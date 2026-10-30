//! Developer probe for the manual QA of TAD-015: runs the real `Paster` (AppKit and CoreGraphics
//! implementations, a window hider that only prints) on a synthetic sentence.
//!
//! After 3 s it records the frontmost app as the app to give the focus back to; 3 s later it
//! pastes. With `--panel` the origin is the Quick Panel (no re-activation, the app in front
//! keeps the focus); without it the origin is the popover (the recorded app is activated again
//! first), so click another app in between to see it come back.
//!
//! The data lives in a throwaway database with a fixed key, never the Keychain. The sentence is
//! synthetic; the probe never reads or prints the clipboard (AGENTS.md §2).
//!
//! The Accessibility grant goes to the program that launched the probe (the terminal).
//!
//! `cargo run --manifest-path src-tauri/Cargo.toml --example paste_flow_probe [-- --panel]`

#[cfg(target_os = "macos")]
fn main() {
    use tadidy_lib::macos::run_loop;

    let panel = std::env::args().any(|arg| arg == "--panel");
    std::thread::spawn(move || run(panel));
    // `frontmostApplication` is only refreshed while the main run loop runs.
    run_loop::run_forever()
}

#[cfg(target_os = "macos")]
fn run(panel: bool) -> ! {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tadidy_lib::clip::{ClipKind, NewClip, content_hash};
    use tadidy_lib::clock::SystemClock;
    use tadidy_lib::crypto::FakeKeyStore;
    use tadidy_lib::events::RecordingEventSink;
    use tadidy_lib::macos::{
        accessibility::AxProbe, keys::CgKeySender, pasteboard::NsPasteboard, workspace::NsWorkspace,
    };
    use tadidy_lib::paster::{
        AccessibilityMonitor, PASTE_DELAY, PasteMode, PasteOrigin, Paster, PasterServices,
        WindowHider,
    };
    use tadidy_lib::platform::{Representation, UTI_PLAIN_TEXT, Workspace};
    use tadidy_lib::storage::{clips, open_in_dir};

    struct PrintingHider;

    impl WindowHider for PrintingHider {
        fn hide(&self, origin: PasteOrigin) {
            println!("hide the {origin:?} window");
        }
    }

    const SENTENCE: &str = "Tadidy paste flow probe";

    let dir = std::env::temp_dir().join(format!("tadidy-paste-flow-probe-{}", std::process::id()));
    let outcome = (|| {
        std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let db = open_in_dir(&dir, &FakeKeyStore::default()).map_err(|error| error.to_string())?;
        let clip = NewClip {
            kind: ClipKind::Text,
            content_hash: content_hash(SENTENCE.as_bytes()),
            plain_text: Some(SENTENCE.to_owned()),
            preview: Some(SENTENCE.to_owned()),
            source_bundle: None,
            source_name: None,
            byte_size: SENTENCE.len() as i64,
            representations: vec![Representation::new(UTI_PLAIN_TEXT, SENTENCE.as_bytes())],
            now: 1,
        };
        let id = match db
            .with_conn(|conn| clips::insert_or_bump(conn, &clip))
            .map_err(|error| error.to_string())?
        {
            clips::InsertOutcome::Inserted(id) | clips::InsertOutcome::Bumped(id) => id,
        };

        let workspace = Arc::new(NsWorkspace);
        let last_active_app = Arc::new(Mutex::new(None));
        let paster = Paster::new(
            PasterServices {
                pasteboard: Arc::new(NsPasteboard),
                keys: Arc::new(CgKeySender),
                workspace: workspace.clone(),
                monitor: Arc::new(AccessibilityMonitor::new(Arc::new(AxProbe))),
                events: Arc::new(RecordingEventSink::default()),
                hider: Arc::new(PrintingHider),
                clock: Arc::new(SystemClock),
                last_active_app: last_active_app.clone(),
                db,
            },
            PASTE_DELAY,
        );

        println!("put the target app (TextEdit) in front within 3 s");
        std::thread::sleep(Duration::from_secs(3));
        let frontmost = workspace.frontmost_app();
        match &frontmost {
            Some(app) => println!("recorded {} ({})", app.name, app.bundle_id),
            None => println!("no app in front"),
        }
        *last_active_app.lock().map_err(|error| error.to_string())? = frontmost;

        if panel {
            println!("origin: Quick Panel; the app in front keeps the focus");
        } else {
            println!("origin: popover; click another app (Finder) now");
        }
        std::thread::sleep(Duration::from_secs(3));

        let origin = if panel {
            PasteOrigin::Panel
        } else {
            PasteOrigin::Popover
        };
        let result = paster
            .paste(&id, PasteMode::Default, origin)
            .map_err(|error| error.to_string())?;
        println!("result: {result:?}");
        // A process that exits right after posting loses the key event: wait for delivery.
        std::thread::sleep(Duration::from_millis(500));
        Ok::<(), String>(())
    })();

    let _ = std::fs::remove_dir_all(&dir);
    match outcome {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            println!("probe failed: {error}");
            std::process::exit(1)
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the paste flow probe only runs on macOS");
}
