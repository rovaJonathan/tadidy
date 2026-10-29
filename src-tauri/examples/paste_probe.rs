//! Developer probe for the manual QA of TAD-005: prints whether this process may post key
//! events, waits 3 s so the target app can be put in front, then sends `⌘V` if it may. What
//! gets pasted is whatever is on the clipboard; the probe never reads or prints it
//! (AGENTS.md §2).
//!
//! With `--request` it asks macOS to list this binary under Accessibility and opens the
//! pane, then exits.
//!
//! The permission is granted to the program that launched the probe (the terminal), as
//! macOS attributes the event to its responsible process.
//!
//! `cargo run --manifest-path src-tauri/Cargo.toml --example paste_probe [-- --request]`

#[cfg(target_os = "macos")]
fn main() {
    use std::time::Duration;
    use tadidy_lib::macos::{accessibility::AxProbe, keys::CgKeySender};
    use tadidy_lib::platform::{AccessibilityProbe, KeySender};

    let probe = AxProbe;

    if std::env::args().any(|arg| arg == "--request") {
        match probe.request_and_open_settings() {
            Ok(()) => println!("requested; System Settings opened on Accessibility"),
            Err(error) => println!("request failed: {error}"),
        }
        return;
    }

    let trusted = probe.is_trusted();
    println!("trusted: {trusted}");
    println!("focus the target app within 3 s");
    std::thread::sleep(Duration::from_secs(3));

    if !trusted {
        println!("not trusted: no key event sent");
        return;
    }
    match CgKeySender.send_paste() {
        Ok(()) => println!("sent Command+V"),
        Err(error) => println!("send failed: {error}"),
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the paste probe only runs on macOS");
}
