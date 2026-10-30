//! Puts a clip back where the user wants it (PST-01, PST-02): on the pasteboard, then into the
//! app that was in front, with `⌘V`. Without the Accessibility permission it stops at the
//! pasteboard (SYS-03). Everything the system does goes through a trait, so the order of the
//! steps is tested with fakes.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::clip::ClipWithRepresentations;
use crate::clock::Clock;
use crate::error::AppError;
use crate::events::{ACCESSIBILITY_CHANGED, EventSink};
use crate::platform::{
    AccessibilityProbe, KeySender, Pasteboard, Representation, SourceApp, UTI_PLAIN_TEXT,
    UTI_TADIDY_MARKER, Workspace,
};
use crate::storage::{Db, clips};
use crate::windows::{PANEL_LABEL, POPOVER_LABEL};

/// How long the target app gets to take the focus back before `⌘V` is sent. One value, so QA
/// can raise it if a loaded machine misses the keystroke.
pub const PASTE_DELAY: Duration = Duration::from_millis(50);

/// How the clip is written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PasteMode {
    /// Every stored representation.
    Default,
    /// Plain text only (PST-03).
    Plain,
}

/// What `paste_clip` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PasteResult {
    Pasted,
    /// The clip is on the pasteboard but `⌘V` was not sent: no Accessibility permission.
    CopiedOnly,
}

/// Payload of `accessibility://changed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AccessibilityChanged {
    pub trusted: bool,
}

/// The window a paste or a copy was asked from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteOrigin {
    Popover,
    Panel,
}

impl PasteOrigin {
    /// The origin of the window that called the command; any other label is refused.
    pub fn from_label(label: &str) -> Result<Self, AppError> {
        match label {
            POPOVER_LABEL => Ok(Self::Popover),
            PANEL_LABEL => Ok(Self::Panel),
            _ => Err(AppError::new(
                "invalid_window",
                "Only the popover and the Quick Panel can paste.",
            )),
        }
    }
}

/// Hides the window a paste or a copy came from.
pub trait WindowHider: Send + Sync {
    fn hide(&self, origin: PasteOrigin);
}

/// Keeps the last known state of the Accessibility permission and announces its changes.
pub struct AccessibilityMonitor {
    probe: Arc<dyn AccessibilityProbe>,
    /// `None` until the first check.
    last: Mutex<Option<bool>>,
}

impl AccessibilityMonitor {
    pub fn new(probe: Arc<dyn AccessibilityProbe>) -> Self {
        Self {
            probe,
            last: Mutex::new(None),
        }
    }

    /// Whether the permission is granted now. Emits `accessibility://changed` when that is not
    /// what the last check saw (the first check counts as a change).
    pub fn check(&self, events: &dyn EventSink) -> bool {
        let trusted = self.probe.is_trusted();
        // Not held while emitting.
        let changed = self
            .last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(trusted)
            != Some(trusted);
        if changed {
            events.emit(
                ACCESSIBILITY_CHANGED,
                json!(AccessibilityChanged { trusted }),
            );
        }
        trusted
    }
}

/// What the paster needs from the system and the app.
pub struct PasterServices {
    pub pasteboard: Arc<dyn Pasteboard>,
    pub keys: Arc<dyn KeySender>,
    pub workspace: Arc<dyn Workspace>,
    pub monitor: Arc<AccessibilityMonitor>,
    pub events: Arc<dyn EventSink>,
    pub hider: Arc<dyn WindowHider>,
    pub clock: Arc<dyn Clock>,
    /// The app that was in front before the popover opened.
    pub last_active_app: Arc<Mutex<Option<SourceApp>>>,
    pub db: Db,
}

pub struct Paster {
    services: PasterServices,
    delay: Duration,
}

impl Paster {
    pub fn new(services: PasterServices, delay: Duration) -> Self {
        Self { services, delay }
    }

    /// PST-01: writes the clip, hides the window, gives the focus back, sends `⌘V`. Without the
    /// Accessibility permission it stops after the write and says so with `CopiedOnly`.
    pub fn paste(
        &self,
        id: &str,
        mode: PasteMode,
        origin: PasteOrigin,
    ) -> Result<PasteResult, AppError> {
        let started = Instant::now();
        self.write(id, mode)?;
        if !self.services.monitor.check(&*self.services.events) {
            tracing::info!(id, ?mode, result = "copied_only", "clip copied, not pasted");
            return Ok(PasteResult::CopiedOnly);
        }
        self.services.hider.hide(origin);
        self.give_the_focus_back(origin);
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        self.services
            .keys
            .send_paste()
            .map_err(|_| AppError::new("paste_failed", "The paste keystroke could not be sent."))?;
        tracing::info!(
            id,
            ?mode,
            result = "pasted",
            elapsed_ms = started.elapsed().as_millis(),
            "clip pasted"
        );
        Ok(PasteResult::Pasted)
    }

    /// PST-02: writes the clip and hides the window, no keystroke.
    pub fn copy(&self, id: &str, origin: PasteOrigin) -> Result<(), AppError> {
        self.write(id, PasteMode::Default)?;
        self.services.hider.hide(origin);
        self.give_the_focus_back(origin);
        tracing::info!(id, "clip copied");
        Ok(())
    }

    /// Puts the clip and the marker on the pasteboard and records the use.
    fn write(&self, id: &str, mode: PasteMode) -> Result<(), AppError> {
        let clip = self
            .services
            .db
            .with_conn(|conn| clips::get(conn, id))?
            .ok_or_else(|| AppError::new("not_found", "The clip does not exist."))?;
        self.services
            .pasteboard
            .write(&representations(&clip, mode))
            .map_err(|_| {
                AppError::new(
                    "pasteboard_write",
                    "The clip could not be put on the clipboard.",
                )
            })?;
        // The clip is on the pasteboard: a failed bookkeeping write must not undo the paste.
        let now = self.services.clock.now_ms();
        if let Err(error) = self
            .services
            .db
            .with_conn(|conn| clips::mark_used(conn, id, now))
        {
            tracing::warn!(code = %AppError::from(error).code, "the use of a clip was not recorded");
        }
        Ok(())
    }

    /// The popover is a real window: closing it leaves the focus in no app. The Quick Panel never
    /// took it. An app that quit since is not an error: macOS picks the next one.
    fn give_the_focus_back(&self, origin: PasteOrigin) {
        if origin != PasteOrigin::Popover {
            return;
        }
        let previous = self
            .services
            .last_active_app
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(app) = previous
            && let Err(error) = self.services.workspace.activate(&app)
        {
            tracing::warn!(%error, "the previous app could not be brought back to the front");
        }
    }
}

/// What goes on the pasteboard: the stored representations (or the plain text alone) and the
/// marker that tells the watcher the write is Tadidy's own.
fn representations(clip: &ClipWithRepresentations, mode: PasteMode) -> Vec<Representation> {
    let mut items = match mode {
        PasteMode::Default => clip.representations.clone(),
        PasteMode::Plain => plain_text_only(clip),
    };
    items.push(Representation::new(UTI_TADIDY_MARKER, Vec::new()));
    items
}

/// A clip with no text (an image, say) has no plain version: it is written whole.
fn plain_text_only(clip: &ClipWithRepresentations) -> Vec<Representation> {
    if let Some(text) = clip
        .representations
        .iter()
        .find(|representation| representation.uti == UTI_PLAIN_TEXT)
    {
        return vec![text.clone()];
    }
    match &clip.clip.plain_text {
        Some(text) => vec![Representation::new(UTI_PLAIN_TEXT, text.as_bytes())],
        None => clip.representations.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{ClipKind, NewClip, content_hash};
    use crate::clock::FixedClock;
    use crate::crypto::FakeKeyStore;
    use crate::events::{ACCESSIBILITY_CHANGED, RecordingEventSink};
    use crate::platform::fakes::{
        FakeAccessibility, FakeIdle, FakeKeySender, FakePasteboard, FakeWorkspace,
    };
    use crate::platform::{PlatformError, Representation, UTI_PLAIN_TEXT, UTI_TADIDY_MARKER};
    use crate::storage::clips::{self, InsertOutcome};
    use crate::storage::open_in_dir;
    use crate::watcher::{Services, TickOutcome, Watcher};
    use serde_json::json;
    use std::time::Instant;

    const HTML: &str = "public.html";
    const TEXT: &str = "pasted text";

    /// Every call the fakes receive, in order, with the instant it arrived.
    #[derive(Default)]
    struct Calls(Mutex<Vec<(String, Instant)>>);

    impl Calls {
        fn push(&self, name: impl Into<String>) {
            self.0.lock().unwrap().push((name.into(), Instant::now()));
        }

        fn names(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .map(|(name, _)| name.clone())
                .collect()
        }

        fn instant_of(&self, name: &str) -> Instant {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|(call, _)| call == name)
                .map(|(_, at)| *at)
                .unwrap()
        }
    }

    struct LoggedPasteboard {
        inner: Arc<FakePasteboard>,
        calls: Arc<Calls>,
    }

    impl Pasteboard for LoggedPasteboard {
        fn change_count(&self) -> i64 {
            self.inner.change_count()
        }
        fn types(&self) -> Vec<String> {
            self.inner.types()
        }
        fn read(&self, uti: &str) -> Option<Vec<u8>> {
            self.inner.read(uti)
        }
        fn write(&self, items: &[Representation]) -> Result<i64, PlatformError> {
            self.calls.push("write");
            self.inner.write(items)
        }
    }

    struct LoggedWorkspace {
        inner: Arc<FakeWorkspace>,
        calls: Arc<Calls>,
    }

    impl Workspace for LoggedWorkspace {
        fn frontmost_app(&self) -> Option<SourceApp> {
            self.inner.frontmost_app()
        }
        fn app_icon_png(&self, app: &SourceApp) -> Option<Vec<u8>> {
            self.inner.app_icon_png(app)
        }
        fn activate(&self, app: &SourceApp) -> Result<(), PlatformError> {
            self.calls.push(format!("activate:{}", app.bundle_id));
            self.inner.activate(app)
        }
    }

    struct LoggedKeys {
        inner: Arc<FakeKeySender>,
        calls: Arc<Calls>,
    }

    impl KeySender for LoggedKeys {
        fn send_paste(&self) -> Result<(), PlatformError> {
            self.calls.push("paste");
            self.inner.send_paste()
        }
    }

    struct LoggedHider(Arc<Calls>);

    impl WindowHider for LoggedHider {
        fn hide(&self, origin: PasteOrigin) {
            self.0.push(format!("hide:{origin:?}"));
        }
    }

    fn textedit() -> SourceApp {
        SourceApp::new("com.apple.TextEdit", "TextEdit", 4242)
    }

    struct Harness {
        _dir: tempfile::TempDir,
        db: Db,
        calls: Arc<Calls>,
        pasteboard: Arc<FakePasteboard>,
        workspace: Arc<FakeWorkspace>,
        keys: Arc<FakeKeySender>,
        accessibility: Arc<FakeAccessibility>,
        events: Arc<RecordingEventSink>,
        clock: Arc<FixedClock>,
        last_active_app: Arc<Mutex<Option<SourceApp>>>,
        monitor: Arc<AccessibilityMonitor>,
        clip_id: String,
    }

    impl Harness {
        /// A trusted process, TextEdit remembered as the previous app, and one rich clip stored.
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
            let accessibility = Arc::new(FakeAccessibility::new());
            accessibility.set_trusted(true);
            let clip = NewClip {
                kind: ClipKind::Rich,
                content_hash: content_hash(TEXT.as_bytes()),
                plain_text: Some(TEXT.to_owned()),
                preview: Some(TEXT.to_owned()),
                source_bundle: None,
                source_name: None,
                byte_size: 11,
                representations: vec![
                    Representation::new(UTI_PLAIN_TEXT, TEXT.as_bytes()),
                    Representation::new(HTML, "<b>pasted text</b>".as_bytes()),
                ],
                now: 1_000,
            };
            let clip_id = match db
                .with_conn(|conn| clips::insert_or_bump(conn, &clip))
                .unwrap()
            {
                InsertOutcome::Inserted(id) => id,
                other => panic!("expected Inserted, got {other:?}"),
            };
            Self {
                _dir: dir,
                db,
                calls: Arc::default(),
                pasteboard: Arc::new(FakePasteboard::new()),
                workspace: Arc::new(FakeWorkspace::new()),
                keys: Arc::new(FakeKeySender::new()),
                events: Arc::new(RecordingEventSink::default()),
                clock: Arc::new(FixedClock::new(5_000)),
                last_active_app: Arc::new(Mutex::new(Some(textedit()))),
                monitor: Arc::new(AccessibilityMonitor::new(accessibility.clone())),
                accessibility,
                clip_id,
            }
        }

        fn paster(&self, delay: Duration) -> Paster {
            Paster::new(
                PasterServices {
                    pasteboard: Arc::new(LoggedPasteboard {
                        inner: self.pasteboard.clone(),
                        calls: self.calls.clone(),
                    }),
                    keys: Arc::new(LoggedKeys {
                        inner: self.keys.clone(),
                        calls: self.calls.clone(),
                    }),
                    workspace: Arc::new(LoggedWorkspace {
                        inner: self.workspace.clone(),
                        calls: self.calls.clone(),
                    }),
                    monitor: self.monitor.clone(),
                    events: self.events.clone(),
                    hider: Arc::new(LoggedHider(self.calls.clone())),
                    clock: self.clock.clone(),
                    last_active_app: self.last_active_app.clone(),
                    db: self.db.clone(),
                },
                delay,
            )
        }

        fn paste(&self, mode: PasteMode, origin: PasteOrigin) -> Result<PasteResult, AppError> {
            self.paster(Duration::ZERO)
                .paste(&self.clip_id, mode, origin)
        }

        fn copy(&self, origin: PasteOrigin) -> Result<(), AppError> {
            self.paster(Duration::ZERO).copy(&self.clip_id, origin)
        }

        fn last_used_at(&self) -> Option<i64> {
            self.db
                .with_conn(|conn| clips::get(conn, &self.clip_id))
                .unwrap()
                .unwrap()
                .clip
                .last_used_at
        }
    }

    fn marker() -> Representation {
        Representation::new(UTI_TADIDY_MARKER, Vec::new())
    }

    fn full_clip_and_marker() -> Vec<Representation> {
        vec![
            Representation::new(UTI_PLAIN_TEXT, TEXT.as_bytes()),
            Representation::new(HTML, "<b>pasted text</b>".as_bytes()),
            marker(),
        ]
    }

    // --- paste ---------------------------------------------------------------------------

    #[test]
    fn a_popover_paste_writes_hides_gives_the_focus_back_waits_then_sends_command_v() {
        let harness = Harness::new();
        let delay = Duration::from_millis(30);

        let result = harness
            .paster(delay)
            .paste(&harness.clip_id, PasteMode::Default, PasteOrigin::Popover)
            .unwrap();

        assert_eq!(result, PasteResult::Pasted);
        assert_eq!(
            harness.calls.names(),
            [
                "write",
                "hide:Popover",
                "activate:com.apple.TextEdit",
                "paste"
            ]
        );
        assert_eq!(harness.pasteboard.writes(), vec![full_clip_and_marker()]);
        let waited = harness
            .calls
            .instant_of("paste")
            .duration_since(harness.calls.instant_of("activate:com.apple.TextEdit"));
        assert!(waited >= delay, "waited only {waited:?}");
    }

    #[test]
    fn a_panel_paste_activates_no_app() {
        let harness = Harness::new();

        let result = harness
            .paste(PasteMode::Default, PasteOrigin::Panel)
            .unwrap();

        assert_eq!(result, PasteResult::Pasted);
        assert_eq!(harness.calls.names(), ["write", "hide:Panel", "paste"]);
        assert!(harness.workspace.activations().is_empty());
    }

    #[test]
    fn a_popover_paste_without_a_remembered_app_still_pastes() {
        let harness = Harness::new();
        *harness.last_active_app.lock().unwrap() = None;

        let result = harness
            .paste(PasteMode::Default, PasteOrigin::Popover)
            .unwrap();

        assert_eq!(result, PasteResult::Pasted);
        assert_eq!(harness.calls.names(), ["write", "hide:Popover", "paste"]);
    }

    #[test]
    fn a_previous_app_that_quit_does_not_stop_the_paste() {
        let harness = Harness::new();
        harness.workspace.fail_activations();

        let result = harness
            .paste(PasteMode::Default, PasteOrigin::Popover)
            .unwrap();

        assert_eq!(result, PasteResult::Pasted);
        assert_eq!(harness.keys.paste_count(), 1);
    }

    #[test]
    fn plain_mode_writes_only_the_plain_text_and_the_marker() {
        let harness = Harness::new();

        harness.paste(PasteMode::Plain, PasteOrigin::Panel).unwrap();

        assert_eq!(
            harness.pasteboard.writes(),
            vec![vec![
                Representation::new(UTI_PLAIN_TEXT, TEXT.as_bytes()),
                marker()
            ]]
        );
    }

    #[test]
    fn a_failing_keystroke_is_reported_after_the_clip_was_written() {
        let harness = Harness::new();
        harness.keys.fail_sends();

        let error = harness
            .paste(PasteMode::Default, PasteOrigin::Panel)
            .unwrap_err();

        assert_eq!(error.code, "paste_failed");
        assert_eq!(harness.pasteboard.writes().len(), 1);
    }

    // --- untrusted -----------------------------------------------------------------------

    #[test]
    fn without_accessibility_only_the_write_happens_and_the_result_is_copied_only() {
        let harness = Harness::new();
        harness.accessibility.set_trusted(false);

        let result = harness
            .paste(PasteMode::Default, PasteOrigin::Popover)
            .unwrap();

        assert_eq!(result, PasteResult::CopiedOnly);
        assert_eq!(harness.calls.names(), ["write"]);
        assert_eq!(harness.pasteboard.writes(), vec![full_clip_and_marker()]);
    }

    #[test]
    fn the_loss_of_accessibility_is_announced_once_not_at_every_paste() {
        let harness = Harness::new();
        harness.accessibility.set_trusted(false);

        harness
            .paste(PasteMode::Default, PasteOrigin::Popover)
            .unwrap();
        harness
            .paste(PasteMode::Default, PasteOrigin::Popover)
            .unwrap();

        assert_eq!(
            harness.events.events(),
            vec![(ACCESSIBILITY_CHANGED, json!({ "trusted": false }))]
        );
    }

    // --- copy ----------------------------------------------------------------------------

    #[test]
    fn a_popover_copy_writes_hides_and_gives_the_focus_back_without_a_keystroke() {
        let harness = Harness::new();

        harness.copy(PasteOrigin::Popover).unwrap();

        assert_eq!(
            harness.calls.names(),
            ["write", "hide:Popover", "activate:com.apple.TextEdit"]
        );
        assert_eq!(harness.pasteboard.writes(), vec![full_clip_and_marker()]);
        assert_eq!(harness.keys.paste_count(), 0);
    }

    #[test]
    fn a_panel_copy_writes_and_hides_only() {
        let harness = Harness::new();

        harness.copy(PasteOrigin::Panel).unwrap();

        assert_eq!(harness.calls.names(), ["write", "hide:Panel"]);
    }

    #[test]
    fn a_copy_works_without_accessibility_and_announces_nothing() {
        let harness = Harness::new();
        harness.accessibility.set_trusted(false);

        harness.copy(PasteOrigin::Panel).unwrap();

        assert_eq!(harness.calls.names(), ["write", "hide:Panel"]);
        assert!(harness.events.events().is_empty());
    }

    // --- bookkeeping ---------------------------------------------------------------------

    #[test]
    fn a_paste_a_copy_and_a_copy_only_paste_all_set_last_used_at() {
        for case in 0..3 {
            let harness = Harness::new();
            assert_eq!(harness.last_used_at(), None);

            match case {
                0 => drop(
                    harness
                        .paste(PasteMode::Default, PasteOrigin::Panel)
                        .unwrap(),
                ),
                1 => harness.copy(PasteOrigin::Panel).unwrap(),
                _ => {
                    harness.accessibility.set_trusted(false);
                    harness
                        .paste(PasteMode::Default, PasteOrigin::Panel)
                        .unwrap();
                }
            }

            assert_eq!(harness.last_used_at(), Some(5_000), "case {case}");
        }
    }

    #[test]
    fn an_unknown_id_fails_with_not_found_and_touches_nothing() {
        let harness = Harness::new();
        let paster = harness.paster(Duration::ZERO);

        let pasted = paster
            .paste("no-such-id", PasteMode::Default, PasteOrigin::Popover)
            .unwrap_err();
        let copied = paster.copy("no-such-id", PasteOrigin::Popover).unwrap_err();

        assert_eq!(pasted.code, "not_found");
        assert_eq!(copied.code, "not_found");
        assert!(harness.calls.names().is_empty());
        assert!(harness.pasteboard.writes().is_empty());
    }

    #[test]
    fn paste_marks_used_and_watcher_ignores_it() {
        let harness = Harness::new();
        let watcher_services = Services {
            pasteboard: harness.pasteboard.clone(),
            workspace: harness.workspace.clone(),
            idle: Arc::new(FakeIdle::new()),
            events: harness.events.clone(),
            clock: harness.clock.clone(),
        };
        let mut watcher = Watcher::new(
            watcher_services,
            harness.db.clone(),
            crate::clip::HistoryLimit::default(),
        );
        harness.clock.set(9_000);

        harness
            .paste(PasteMode::Default, PasteOrigin::Panel)
            .unwrap();
        let outcome = watcher.tick().unwrap();

        assert_eq!(outcome, TickOutcome::OwnWrite);
        let clips = harness
            .db
            .with_conn(|conn| clips::list_recent(conn, None, 10))
            .unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].id, harness.clip_id);
        assert_eq!(clips[0].copy_count, 1);
        assert_eq!(clips[0].last_copied_at, 9_000);
        assert_eq!(clips[0].last_used_at, Some(9_000));
    }

    // --- origin --------------------------------------------------------------------------

    #[test]
    fn the_origin_is_the_label_of_the_calling_window() {
        assert_eq!(PasteOrigin::from_label("popover"), Ok(PasteOrigin::Popover));
        assert_eq!(PasteOrigin::from_label("panel"), Ok(PasteOrigin::Panel));
    }

    #[test]
    fn any_other_window_is_refused() {
        for label in ["settings", "onboarding", "", "Popover"] {
            let error = PasteOrigin::from_label(label).unwrap_err();

            assert_eq!(error.code, "invalid_window", "{label:?}");
        }
    }

    // --- accessibility monitor -----------------------------------------------------------

    #[test]
    fn the_monitor_reports_the_probe_and_announces_the_first_state_it_sees() {
        let accessibility = Arc::new(FakeAccessibility::new());
        accessibility.set_trusted(true);
        let monitor = AccessibilityMonitor::new(accessibility);
        let events = RecordingEventSink::default();

        assert!(monitor.check(&events));

        assert_eq!(
            events.events(),
            vec![(ACCESSIBILITY_CHANGED, json!({ "trusted": true }))]
        );
    }

    #[test]
    fn the_monitor_emits_only_when_the_permission_changes() {
        let accessibility = Arc::new(FakeAccessibility::new());
        let monitor = AccessibilityMonitor::new(accessibility.clone());
        let events = RecordingEventSink::default();

        assert!(!monitor.check(&events));
        assert!(!monitor.check(&events));
        accessibility.set_trusted(true);
        assert!(monitor.check(&events));
        assert!(monitor.check(&events));
        accessibility.set_trusted(false);
        assert!(!monitor.check(&events));

        assert_eq!(
            events.events(),
            vec![
                (ACCESSIBILITY_CHANGED, json!({ "trusted": false })),
                (ACCESSIBILITY_CHANGED, json!({ "trusted": true })),
                (ACCESSIBILITY_CHANGED, json!({ "trusted": false })),
            ]
        );
    }

    #[test]
    fn modes_and_results_use_the_names_of_the_contract() {
        assert_eq!(json!(PasteMode::Default), json!("default"));
        assert_eq!(json!(PasteMode::Plain), json!("plain"));
        assert_eq!(json!(PasteResult::Pasted), json!("pasted"));
        assert_eq!(json!(PasteResult::CopiedOnly), json!("copied_only"));
        assert_eq!(
            serde_json::from_value::<PasteMode>(json!("plain")).unwrap(),
            PasteMode::Plain
        );
    }
}
