//! The history reset end to end: a database whose key is gone, a reset, then a clip copied by
//! another app is recorded without restarting. Fakes for the system, a temporary folder, never the
//! Keychain or the user's data folder (AGENTS.md §13).

#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use tadidy_lib::clock::FixedClock;
use tadidy_lib::crypto::FakeKeyStore;
use tadidy_lib::error::AppError;
use tadidy_lib::events::RecordingEventSink;
use tadidy_lib::platform::fakes::{
    FakeAccessibility, FakeIdle, FakeKeySender, FakePasteboard, FakeWorkspace,
};
use tadidy_lib::platform::{Representation, UTI_PLAIN_TEXT};
use tadidy_lib::state::AppState;
use tadidy_lib::storage::{clips, open_for_startup, open_in_dir};
use tadidy_lib::watcher::Services;

#[test]
fn a_clip_copied_after_the_reset_is_recorded_without_restarting() {
    let dir = tempfile::tempdir().unwrap();
    open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
    let store = Arc::new(FakeKeyStore::default());
    let pasteboard = Arc::new(FakePasteboard::new());
    let services = Services {
        pasteboard: pasteboard.clone(),
        workspace: Arc::new(FakeWorkspace::new()),
        idle: Arc::new(FakeIdle::new()),
        events: Arc::new(RecordingEventSink::default()),
        clock: Arc::new(FixedClock::new(1_000)),
    };
    let opened = open_for_startup(dir.path(), store.as_ref());
    assert!(opened.is_err());
    let state = AppState::new(
        Arc::new(FakeAccessibility::new()),
        Arc::new(FakeKeySender::new()),
        Arc::new(FakePasteboard::new()),
        Arc::new(FakeWorkspace::new()),
    )
    .with_storage(opened.map_err(|error: AppError| error))
    .with_storage_reset(dir.path().to_path_buf(), store, services);

    state.reset_history().unwrap();
    pasteboard.set(vec![Representation::new(UTI_PLAIN_TEXT, b"hello".to_vec())]);

    let db = state.db().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while db.with_conn(|conn| clips::count(conn).unwrap()) == 0 {
        assert!(Instant::now() < deadline, "the clip was never recorded");
        std::thread::sleep(Duration::from_millis(20));
    }
}
