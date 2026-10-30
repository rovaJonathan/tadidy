//! The clipboard watcher: polls the pasteboard's change count and records each text copy as a
//! clip (CAP-01, CAP-02, CAP-08, CAP-09, CAP-12). Logs carry ids, kinds, sizes and durations,
//! never a clip's content (AGENTS.md §2).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::capture;
use crate::clip::{HistoryLimit, content_hash};
use crate::clock::Clock;
use crate::error::AppError;
use crate::events::{CLIPS_CHANGED, ClipsChanged, EventSink};
use crate::platform::{
    IdleProbe, Pasteboard, UTI_PLAIN_TEXT, UTI_TADIDY_MARKER, Workspace, is_idle,
};
use crate::state::AppState;
use crate::storage::clips::{self, InsertOutcome};
use crate::storage::{Db, StorageError, app_icons};

/// How often the change count is read while the user is around.
pub const POLL_ACTIVE: Duration = Duration::from_millis(300);
/// How often once the session is idle (saves battery).
pub const POLL_IDLE: Duration = Duration::from_secs(1);

/// What one [`Watcher::tick`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    /// The change count did not move.
    Unchanged,
    /// The pasteboard changed but holds nothing to record.
    Ignored,
    /// A new clip, by id.
    Added(String),
    /// Known content copied again: the clip moved to the top.
    Updated(String),
    /// Tadidy's own write: nothing recorded.
    OwnWrite,
}

/// A failure of one tick. Carries a code, never a clip's content or a path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WatcherError {
    #[error("the history database could not be used ({code})")]
    Storage { code: String },
}

impl WatcherError {
    pub fn code(&self) -> &str {
        match self {
            Self::Storage { code } => code,
        }
    }
}

impl From<StorageError> for WatcherError {
    fn from(error: StorageError) -> Self {
        Self::Storage {
            code: AppError::from(error).code,
        }
    }
}

/// What the watcher needs from the system.
pub struct Services {
    pub pasteboard: Arc<dyn Pasteboard>,
    pub workspace: Arc<dyn Workspace>,
    pub idle: Arc<dyn IdleProbe>,
    pub events: Arc<dyn EventSink>,
    pub clock: Arc<dyn Clock>,
}

pub struct Watcher {
    services: Services,
    db: Db,
    limit: HistoryLimit,
    last_change_count: i64,
}

impl Watcher {
    /// Remembers the pasteboard's current change count without capturing it: what was copied
    /// before Tadidy started is not recorded again at every launch.
    pub fn new(services: Services, db: Db, limit: HistoryLimit) -> Self {
        let last_change_count = services.pasteboard.change_count();
        Self {
            services,
            db,
            limit,
            last_change_count,
        }
    }

    /// Looks at the pasteboard once. While the change count has not moved, nothing else is read.
    pub fn tick(&mut self) -> Result<TickOutcome, WatcherError> {
        let change_count = self.services.pasteboard.change_count();
        if change_count == self.last_change_count {
            return Ok(TickOutcome::Unchanged);
        }
        // Remembered first: a copy that fails to record is not retried at every tick.
        self.last_change_count = change_count;
        let seen = Instant::now();

        if self
            .services
            .pasteboard
            .types()
            .iter()
            .any(|uti| uti == UTI_TADIDY_MARKER)
        {
            return self.own_write();
        }
        let source = self.services.workspace.frontmost_app();
        let Some(text) = self.services.pasteboard.read(UTI_PLAIN_TEXT) else {
            return Ok(TickOutcome::Ignored);
        };
        let now = self.services.clock.now_ms();
        let Some(clip) = capture::text_clip(&text, source.as_ref(), now) else {
            return Ok(TickOutcome::Ignored);
        };

        // The icon is fetched outside the database lock: it is an AppKit call.
        let icon = match &source {
            Some(app)
                if !self
                    .db
                    .with_conn(|conn| app_icons::has(conn, &app.bundle_id))? =>
            {
                self.services.workspace.app_icon_png(app)
            }
            _ => None,
        };
        // Written, then announced: the front reloads from the database when it hears about it.
        let (outcome, removed) = self.db.with_conn(|conn| {
            if let (Some(app), Some(png)) = (&source, &icon) {
                app_icons::put(conn, &app.bundle_id, png)?;
            }
            let outcome = clips::insert_or_bump(conn, &clip)?;
            // The clip is committed: a trim that fails must not hide it from the front. The
            // next capture trims what is left over.
            let removed = clips::enforce_limit(conn, self.limit.get()).unwrap_or_else(|error| {
                tracing::error!(
                    code = WatcherError::from(error).code(),
                    "the history could not be trimmed"
                );
                Vec::new()
            });
            Ok::<_, StorageError>((outcome, removed))
        })?;

        let mut changed = ClipsChanged {
            removed: (!removed.is_empty()).then_some(removed),
            ..ClipsChanged::default()
        };
        let (result, id) = match outcome {
            InsertOutcome::Inserted(id) => {
                changed.added = Some(id.clone());
                (TickOutcome::Added(id.clone()), id)
            }
            InsertOutcome::Bumped(id) => {
                changed.updated = Some(id.clone());
                (TickOutcome::Updated(id.clone()), id)
            }
        };
        self.services.events.emit(CLIPS_CHANGED, json!(changed));
        tracing::info!(
            id,
            kind = clip.kind.as_str(),
            bytes = clip.byte_size,
            elapsed_ms = seen.elapsed().as_millis(),
            "clip recorded"
        );
        Ok(result)
    }

    /// Tadidy put a clip back on the pasteboard: the clip moves to the top, nothing is recorded
    /// and no copy is counted (CAP-12).
    fn own_write(&self) -> Result<TickOutcome, WatcherError> {
        let Some(text) = self.services.pasteboard.read(UTI_PLAIN_TEXT) else {
            return Ok(TickOutcome::OwnWrite);
        };
        let hash = content_hash(&text);
        let now = self.services.clock.now_ms();
        let moved = self
            .db
            .with_conn(|conn| clips::move_to_top(conn, &hash, now))?;
        if let Some(id) = moved {
            let changed = ClipsChanged {
                updated: Some(id),
                ..ClipsChanged::default()
            };
            self.services.events.emit(CLIPS_CHANGED, json!(changed));
        }
        Ok(TickOutcome::OwnWrite)
    }

    /// Runs [`Watcher::tick`] on the `tadidy-watcher` thread until the handle is stopped. An
    /// error is logged with its code and the loop goes on.
    pub fn spawn(mut self) -> WatcherHandle {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let spawned = std::thread::Builder::new()
            .name("tadidy-watcher".to_owned())
            .spawn(move || {
                while !stopped.load(Ordering::Acquire) {
                    if let Err(error) = self.tick() {
                        tracing::error!(code = error.code(), "the watcher could not record a copy");
                    }
                    let idle = self.services.idle.seconds_since_last_input();
                    std::thread::park_timeout(poll_interval(idle));
                }
            });
        let thread = spawned
            .inspect_err(|error| tracing::error!(%error, "the watcher thread could not start"))
            .ok();
        WatcherHandle { stop, thread }
    }
}

/// How long to sleep before the next tick.
pub fn poll_interval(idle_secs: f64) -> Duration {
    if is_idle(idle_secs) {
        POLL_IDLE
    } else {
        POLL_ACTIVE
    }
}

/// Stops the watcher thread when [`WatcherHandle::stop`] is called or the handle is dropped.
pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WatcherHandle {
    pub fn stop(self) {
        drop(self);
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            // A thread that panicked has nothing left to tell.
            let _ = thread.join();
        }
    }
}

/// Starts the watcher once the database is open and keeps its handle in the app state. Returns
/// `false`, starting nothing, when the database is unavailable.
pub fn start(state: &AppState, services: Services) -> bool {
    let Some(db) = state.db.clone() else {
        tracing::warn!("the clipboard watcher is not started: no history database");
        return false;
    };
    let handle = Watcher::new(services, db, state.history_limit.clone()).spawn();
    *state.watcher.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, MutexGuard};
    use std::time::Instant;

    use serde_json::json;
    use tempfile::TempDir;
    use tracing_subscriber::fmt::MakeWriter;

    use crate::clip::{ClipKind, ClipRecord, content_hash};
    use crate::clock::FixedClock;
    use crate::crypto::FakeKeyStore;
    use crate::events::{CLIPS_CHANGED, RecordingEventSink};
    use crate::platform::fakes::{
        FakeAccessibility, FakeIdle, FakeKeySender, FakePasteboard, FakeWorkspace,
    };
    use crate::platform::{
        PlatformError, Representation, SourceApp, UTI_PLAIN_TEXT, UTI_TADIDY_MARKER,
    };
    use crate::storage::{app_icons, clips, open_in_dir};

    const CANARY: &str = "canary-7f3a9c-clip-text";

    /// `tracing` decides once per callsite whether anyone listens. A scoped subscriber that
    /// installs while another test thread first reaches the same callsite can miss its events,
    /// so the tests that tick a watcher run one at a time.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn editor() -> SourceApp {
        SourceApp::new("com.example.editor", "Editor", 77)
    }

    fn text(value: &str) -> Representation {
        Representation::new(UTI_PLAIN_TEXT, value.as_bytes().to_vec())
    }

    fn changed(
        added: Option<&str>,
        updated: Option<&str>,
        removed: Option<&[&str]>,
    ) -> serde_json::Value {
        json!({
            "added": added,
            "updated": updated,
            "removed": removed,
        })
    }

    /// A pasteboard that counts how often a watcher reads its types or data.
    struct SpyPasteboard {
        inner: Arc<FakePasteboard>,
        reads: AtomicUsize,
    }

    impl Pasteboard for SpyPasteboard {
        fn change_count(&self) -> i64 {
            self.inner.change_count()
        }

        fn types(&self) -> Vec<String> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            self.inner.types()
        }

        fn read(&self, uti: &str) -> Option<Vec<u8>> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            self.inner.read(uti)
        }

        fn write(&self, items: &[Representation]) -> Result<i64, PlatformError> {
            self.inner.write(items)
        }
    }

    /// An event sink that counts the clips in the database at the moment each event arrives.
    struct CommitCheckSink {
        db: Db,
        clips_at_emit: Mutex<Vec<u32>>,
    }

    impl EventSink for CommitCheckSink {
        fn emit(&self, _event: &'static str, _payload: serde_json::Value) {
            let count = self.db.with_conn(clips::count).unwrap();
            self.clips_at_emit.lock().unwrap().push(count);
        }
    }

    struct Harness {
        _one_at_a_time: MutexGuard<'static, ()>,
        _dir: TempDir,
        db: Db,
        pasteboard: Arc<FakePasteboard>,
        workspace: Arc<FakeWorkspace>,
        idle: Arc<FakeIdle>,
        events: Arc<RecordingEventSink>,
        clock: Arc<FixedClock>,
        limit: HistoryLimit,
    }

    impl Harness {
        fn new() -> Self {
            let one_at_a_time = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
            let dir = tempfile::tempdir().unwrap();
            let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
            let workspace = Arc::new(FakeWorkspace::new());
            workspace.set_frontmost(Some(editor()));
            Self {
                _one_at_a_time: one_at_a_time,
                _dir: dir,
                db,
                pasteboard: Arc::new(FakePasteboard::new()),
                workspace,
                idle: Arc::new(FakeIdle::new()),
                events: Arc::new(RecordingEventSink::default()),
                clock: Arc::new(FixedClock::new(1_000)),
                limit: HistoryLimit::default(),
            }
        }

        fn services_with(
            &self,
            pasteboard: Arc<dyn Pasteboard>,
            events: Arc<dyn EventSink>,
        ) -> Services {
            Services {
                pasteboard,
                workspace: self.workspace.clone(),
                idle: self.idle.clone(),
                events,
                clock: self.clock.clone(),
            }
        }

        fn services(&self) -> Services {
            self.services_with(self.pasteboard.clone(), self.events.clone())
        }

        fn watcher(&self) -> Watcher {
            Watcher::new(self.services(), self.db.clone(), self.limit.clone())
        }

        /// Another app copies `value`, then one tick.
        fn copy(&self, watcher: &mut Watcher, value: &str) -> TickOutcome {
            self.pasteboard.set(vec![text(value)]);
            watcher.tick().unwrap()
        }

        fn list(&self) -> Vec<ClipRecord> {
            self.db
                .with_conn(|conn| clips::list_recent(conn, None, 100))
                .unwrap()
        }

        fn count(&self) -> u32 {
            self.db.with_conn(clips::count).unwrap()
        }
    }

    fn added_id(outcome: TickOutcome) -> String {
        match outcome {
            TickOutcome::Added(id) => id,
            other => panic!("expected Added, got {other:?}"),
        }
    }

    #[test]
    fn text_on_the_pasteboard_at_startup_is_not_captured() {
        let h = Harness::new();
        h.pasteboard.set(vec![text("already there")]);
        let mut watcher = h.watcher();

        assert_eq!(watcher.tick().unwrap(), TickOutcome::Unchanged);

        assert_eq!(h.count(), 0);
        assert!(h.events.events().is_empty());
    }

    #[test]
    fn an_unchanged_change_count_reads_neither_types_nor_data() {
        let h = Harness::new();
        let spy = Arc::new(SpyPasteboard {
            inner: h.pasteboard.clone(),
            reads: AtomicUsize::new(0),
        });
        let mut watcher = Watcher::new(
            h.services_with(spy.clone(), h.events.clone()),
            h.db.clone(),
            h.limit.clone(),
        );

        assert_eq!(watcher.tick().unwrap(), TickOutcome::Unchanged);
        assert_eq!(watcher.tick().unwrap(), TickOutcome::Unchanged);

        assert_eq!(spy.reads.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_copy_becomes_a_text_clip_with_its_source_app() {
        let h = Harness::new();
        h.clock.set(5_000);
        let mut watcher = h.watcher();

        let id = added_id(h.copy(&mut watcher, "hello"));

        let clips = h.list();
        assert_eq!(clips.len(), 1);
        let clip = &clips[0];
        assert_eq!(clip.id, id);
        assert_eq!(clip.kind, ClipKind::Text);
        assert_eq!(clip.plain_text.as_deref(), Some("hello"));
        assert_eq!(clip.content_hash, content_hash(b"hello"));
        assert_eq!(clip.source_bundle.as_deref(), Some("com.example.editor"));
        assert_eq!(clip.source_name.as_deref(), Some("Editor"));
        assert_eq!(clip.copy_count, 1);
        assert_eq!(clip.last_copied_at, 5_000);
        assert_eq!(
            h.events.events(),
            vec![(CLIPS_CHANGED, changed(Some(&id), None, None))]
        );
    }

    #[test]
    fn a_copy_with_no_frontmost_app_is_recorded_without_a_source() {
        let h = Harness::new();
        h.workspace.set_frontmost(None);
        let mut watcher = h.watcher();

        h.copy(&mut watcher, "hello");

        let clips = h.list();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].source_bundle, None);
        assert_eq!(clips[0].source_name, None);
    }

    #[test]
    fn the_source_icon_is_stored_once() {
        let h = Harness::new();
        h.workspace.set_icon("com.example.editor", vec![1, 2, 3]);
        let mut watcher = h.watcher();

        h.copy(&mut watcher, "first");
        h.workspace.set_icon("com.example.editor", vec![9, 9, 9]);
        h.copy(&mut watcher, "second");

        let icon =
            h.db.with_conn(|conn| app_icons::get(conn, "com.example.editor"))
                .unwrap();
        assert_eq!(icon, Some(vec![1, 2, 3]));
        let rows: u32 =
            h.db.with_conn(|conn| {
                conn.query_row("SELECT count(*) FROM app_icons", [], |row| row.get(0))
            })
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn an_app_whose_icon_is_unavailable_still_gets_its_clip() {
        let h = Harness::new();
        let mut watcher = h.watcher();

        h.copy(&mut watcher, "hello");

        assert_eq!(h.count(), 1);
        let has =
            h.db.with_conn(|conn| app_icons::has(conn, "com.example.editor"))
                .unwrap();
        assert!(!has);
    }

    #[test]
    fn the_event_is_emitted_after_the_row_is_committed() {
        let h = Harness::new();
        let sink = Arc::new(CommitCheckSink {
            db: h.db.clone(),
            clips_at_emit: Mutex::new(Vec::new()),
        });
        let mut watcher = Watcher::new(
            h.services_with(h.pasteboard.clone(), sink.clone()),
            h.db.clone(),
            h.limit.clone(),
        );

        h.copy(&mut watcher, "hello");

        assert_eq!(*sink.clips_at_emit.lock().unwrap(), vec![1]);
    }

    #[test]
    fn the_same_text_copied_again_is_bumped_and_goes_first() {
        let h = Harness::new();
        let mut watcher = h.watcher();
        h.clock.set(1_000);
        let first = added_id(h.copy(&mut watcher, "same"));
        h.clock.set(2_000);
        let other = added_id(h.copy(&mut watcher, "other"));
        h.clock.set(3_000);

        let outcome = h.copy(&mut watcher, "same");

        assert_eq!(outcome, TickOutcome::Updated(first.clone()));
        let clips = h.list();
        assert_eq!(
            clips.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            vec![first.as_str(), other.as_str()]
        );
        assert_eq!(clips[0].copy_count, 2);
        assert_eq!(clips[0].last_copied_at, 3_000);
        assert_eq!(
            h.events.events().last(),
            Some(&(CLIPS_CHANGED, changed(None, Some(&first), None)))
        );
    }

    #[test]
    fn a_write_by_tadidy_moves_the_clip_to_the_top_without_a_new_copy() {
        let h = Harness::new();
        let mut watcher = h.watcher();
        h.clock.set(1_000);
        let first = added_id(h.copy(&mut watcher, "pasted"));
        h.clock.set(2_000);
        h.copy(&mut watcher, "newer");
        h.clock.set(3_000);
        h.pasteboard
            .write(&[
                text("pasted"),
                Representation::new(UTI_TADIDY_MARKER, Vec::new()),
            ])
            .unwrap();

        let outcome = watcher.tick().unwrap();

        assert_eq!(outcome, TickOutcome::OwnWrite);
        assert_eq!(h.count(), 2);
        let clips = h.list();
        assert_eq!(clips[0].id, first);
        assert_eq!(clips[0].copy_count, 1);
        assert_eq!(clips[0].last_copied_at, 3_000);
        assert_eq!(
            h.events.events().last(),
            Some(&(CLIPS_CHANGED, changed(None, Some(&first), None)))
        );
    }

    #[test]
    fn a_write_by_tadidy_of_unknown_text_records_and_announces_nothing() {
        let h = Harness::new();
        let mut watcher = h.watcher();
        h.pasteboard
            .write(&[
                text("never stored"),
                Representation::new(UTI_TADIDY_MARKER, Vec::new()),
            ])
            .unwrap();

        assert_eq!(watcher.tick().unwrap(), TickOutcome::OwnWrite);

        assert_eq!(h.count(), 0);
        assert!(h.events.events().is_empty());
    }

    #[test]
    fn a_write_by_tadidy_is_not_attributed_to_the_frontmost_app() {
        let h = Harness::new();
        let mut watcher = h.watcher();
        h.workspace
            .set_frontmost(Some(SourceApp::new("com.apple.TextEdit", "TextEdit", 5)));
        h.copy(&mut watcher, "pasted");
        h.workspace.set_frontmost(Some(editor()));
        h.pasteboard
            .write(&[
                text("pasted"),
                Representation::new(UTI_TADIDY_MARKER, Vec::new()),
            ])
            .unwrap();

        watcher.tick().unwrap();

        assert_eq!(
            h.list()[0].source_bundle.as_deref(),
            Some("com.apple.TextEdit")
        );
    }

    #[test]
    fn empty_whitespace_or_non_utf8_text_is_ignored() {
        let h = Harness::new();
        let mut watcher = h.watcher();

        for bytes in [b"".to_vec(), b" \n\t".to_vec(), vec![0xff, 0xfe]] {
            h.pasteboard
                .set(vec![Representation::new(UTI_PLAIN_TEXT, bytes)]);
            assert_eq!(watcher.tick().unwrap(), TickOutcome::Ignored);
        }

        assert_eq!(h.count(), 0);
        assert!(h.events.events().is_empty());
    }

    #[test]
    fn a_copy_without_plain_text_is_ignored() {
        let h = Harness::new();
        let mut watcher = h.watcher();
        h.pasteboard.set(vec![Representation::new(
            "public.html",
            b"<b>x</b>".to_vec(),
        )]);

        assert_eq!(watcher.tick().unwrap(), TickOutcome::Ignored);

        assert_eq!(h.count(), 0);
        assert!(h.events.events().is_empty());
    }

    #[test]
    fn an_empty_pasteboard_is_ignored() {
        let h = Harness::new();
        let mut watcher = h.watcher();
        h.pasteboard.set(Vec::new());

        assert_eq!(watcher.tick().unwrap(), TickOutcome::Ignored);
    }

    #[test]
    fn the_eleventh_clip_removes_the_oldest_one_under_a_limit_of_ten() {
        let h = Harness::new();
        h.limit.set(Some(10));
        let mut watcher = h.watcher();
        let mut ids = Vec::new();
        for n in 0..10 {
            h.clock.set(1_000 + n);
            ids.push(added_id(h.copy(&mut watcher, &format!("clip {n}"))));
        }
        h.clock.set(2_000);

        let newest = added_id(h.copy(&mut watcher, "clip 10"));

        assert_eq!(h.count(), 10);
        assert!(h.list().iter().all(|clip| clip.id != ids[0]));
        assert_eq!(
            h.events.events().last(),
            Some(&(
                CLIPS_CHANGED,
                changed(Some(&newest), None, Some(&[ids[0].as_str()]))
            ))
        );
    }

    #[test]
    fn a_failed_trim_still_announces_the_clip_that_was_committed() {
        let h = Harness::new();
        h.limit.set(Some(1));
        let mut watcher = h.watcher();
        h.clock.set(1_000);
        let first = added_id(h.copy(&mut watcher, "first"));
        h.db.with_conn(|conn| {
            conn.execute_batch(
                "CREATE TEMP TRIGGER no_trim BEFORE DELETE ON clips \
                 BEGIN SELECT RAISE(ABORT, 'trim refused'); END;",
            )
        })
        .unwrap();
        h.clock.set(2_000);

        let second = added_id(h.copy(&mut watcher, "second"));

        assert_eq!(h.count(), 2);
        assert_eq!(
            h.events.events().last(),
            Some(&(CLIPS_CHANGED, changed(Some(&second), None, None)))
        );

        // The next capture trims what the failed one left behind, newest first.
        h.db.with_conn(|conn| conn.execute_batch("DROP TRIGGER no_trim;"))
            .unwrap();
        h.clock.set(3_000);
        let third = added_id(h.copy(&mut watcher, "third"));

        assert_eq!(h.count(), 1);
        assert_eq!(
            h.events.events().last(),
            Some(&(
                CLIPS_CHANGED,
                changed(Some(&third), None, Some(&[second.as_str(), first.as_str()]))
            ))
        );
    }

    #[test]
    fn an_unlimited_history_never_removes_anything() {
        let h = Harness::new();
        h.limit.set(None);
        let mut watcher = h.watcher();

        for n in 0..12 {
            h.clock.set(1_000 + n);
            h.copy(&mut watcher, &format!("clip {n}"));
        }

        assert_eq!(h.count(), 12);
    }

    #[test]
    fn the_poll_slows_down_once_the_session_has_been_idle_for_five_minutes() {
        assert_eq!(poll_interval(0.0), Duration::from_millis(300));
        assert_eq!(poll_interval(299.0), Duration::from_millis(300));
        assert_eq!(poll_interval(300.0), Duration::from_secs(1));
        assert_eq!(poll_interval(3_600.0), Duration::from_secs(1));
    }

    #[test]
    fn a_database_failure_is_reported_once_and_the_next_tick_goes_on() {
        let h = Harness::new();
        let mut watcher = h.watcher();
        h.db.with_conn(|conn| conn.execute_batch("DROP TABLE representations;"))
            .unwrap();

        let error = {
            h.pasteboard.set(vec![text("lost")]);
            watcher.tick().unwrap_err()
        };

        assert_eq!(error.code(), "storage_error");
        assert_eq!(watcher.tick().unwrap(), TickOutcome::Unchanged);
    }

    #[test]
    fn the_error_message_carries_only_its_code() {
        let error = WatcherError::from(StorageError::KeyMissing);

        assert_eq!(error.code(), "storage_key_missing");
        assert_eq!(
            error.to_string(),
            "the history database could not be used (storage_key_missing)"
        );
    }

    #[derive(Clone, Default)]
    struct LogBuffer(Arc<Mutex<Vec<u8>>>);

    impl io::Write for LogBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for LogBuffer {
        type Writer = LogBuffer;

        fn make_writer(&'a self) -> LogBuffer {
            self.clone()
        }
    }

    #[test]
    fn logs_never_contain_clip_text() {
        let h = Harness::new();
        let logs = LogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer(logs.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            let mut watcher = h.watcher();
            h.copy(&mut watcher, CANARY);
            h.copy(&mut watcher, CANARY);
            h.pasteboard
                .write(&[
                    text(CANARY),
                    Representation::new(UTI_TADIDY_MARKER, Vec::new()),
                ])
                .unwrap();
            watcher.tick().unwrap();
            h.db.with_conn(|conn| conn.execute_batch("DROP TABLE representations;"))
                .unwrap();
            h.pasteboard.set(vec![text(&format!("{CANARY} again"))]);
            assert!(watcher.tick().is_err());
        });

        let logged = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
        assert!(
            logged.contains("clip recorded"),
            "nothing was logged: {logged}"
        );
        assert!(!logged.contains(CANARY), "{logged}");
    }

    fn wait_for(mut done: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if done() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn the_spawned_thread_records_copies_until_it_is_stopped() {
        let h = Harness::new();
        let watcher = h.watcher();
        h.pasteboard.set(vec![text("from the thread")]);

        let handle = watcher.spawn();

        assert!(wait_for(|| h.count() == 1), "the copy was not recorded");
        h.pasteboard.set(vec![text("second")]);
        assert!(
            wait_for(|| h.count() == 2),
            "the second copy was not recorded"
        );
        let start = Instant::now();
        handle.stop();
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "stop took too long"
        );
        h.pasteboard.set(vec![text("after stop")]);
        std::thread::sleep(Duration::from_millis(800));
        assert_eq!(h.count(), 2);
    }

    /// An idle probe that counts the readings: the loop reads it right after each tick.
    #[derive(Default)]
    struct CountingIdle(AtomicUsize);

    impl IdleProbe for CountingIdle {
        fn seconds_since_last_input(&self) -> f64 {
            self.0.fetch_add(1, Ordering::Relaxed);
            0.0
        }
    }

    #[test]
    fn the_watcher_thread_survives_a_database_error() {
        let h = Harness::new();
        let ticks = Arc::new(CountingIdle::default());
        let watcher = Watcher::new(
            Services {
                idle: ticks.clone(),
                ..h.services()
            },
            h.db.clone(),
            h.limit.clone(),
        );
        h.db.with_conn(|conn| conn.execute_batch("ALTER TABLE representations RENAME TO parked;"))
            .unwrap();
        h.pasteboard.set(vec![text("lost")]);
        let handle = watcher.spawn();
        // The first tick sees "lost" and fails; the loop reads the idle time once it is over.
        assert!(
            wait_for(|| ticks.0.load(Ordering::Relaxed) >= 1),
            "the first tick never ended"
        );
        assert_eq!(h.count(), 0);
        h.db.with_conn(|conn| conn.execute_batch("ALTER TABLE parked RENAME TO representations;"))
            .unwrap();

        h.pasteboard.set(vec![text("kept")]);

        assert!(
            wait_for(|| h.count() == 1),
            "the thread stopped after an error"
        );
        assert_eq!(h.list()[0].plain_text.as_deref(), Some("kept"));
        handle.stop();
    }

    fn state_with(db: Option<Db>) -> AppState {
        AppState::new(
            Arc::new(FakeAccessibility::new()),
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            Arc::new(FakeWorkspace::new()),
        )
        .with_db(db)
    }

    #[test]
    fn start_runs_the_watcher_when_the_database_is_open() {
        let h = Harness::new();
        let state = state_with(Some(h.db.clone()));
        state.history_limit.set(Some(10));

        assert!(start(&state, h.services()));
        h.pasteboard.set(vec![text("hello")]);

        assert!(wait_for(|| h.count() == 1));
        assert!(state.watcher.lock().unwrap().is_some());
    }

    #[test]
    fn start_does_nothing_without_a_database() {
        let h = Harness::new();
        let state = state_with(None);

        assert!(!start(&state, h.services()));

        assert!(state.watcher.lock().unwrap().is_none());
    }
}
