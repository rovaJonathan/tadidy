//! Developer probe for the manual QA of TAD-004: polls the general pasteboard every 300 ms and
//! prints the change count, the types with their byte sizes and the frontmost app. It never
//! prints the data itself (AGENTS.md §2). The icon of the frontmost app, rewritten at every
//! change of app, goes to a temporary PNG so it can be opened in Preview.
//!
//! It polls from its own thread while the main thread runs the run loop, like the watcher
//! inside Tauri: `frontmostApplication` is not refreshed without a running main run loop.
//!
//! `cargo run --manifest-path src-tauri/Cargo.toml --example pasteboard_probe`

#[cfg(target_os = "macos")]
fn main() {
    use tadidy_lib::macos::run_loop;

    std::thread::spawn(poll);
    run_loop::run_forever()
}

#[cfg(target_os = "macos")]
fn poll() {
    use std::time::Duration;
    use tadidy_lib::macos::{pasteboard::NsPasteboard, workspace::NsWorkspace};
    use tadidy_lib::platform::{Pasteboard, Workspace};

    let pasteboard = NsPasteboard;
    let workspace = NsWorkspace;
    let icon_path = std::env::temp_dir().join("tadidy-probe-icon.png");
    let mut last_count = None;
    let mut last_app = None;

    println!("polling every 300 ms, Ctrl-C to stop");
    loop {
        let count = pasteboard.change_count();
        if last_count != Some(count) {
            last_count = Some(count);
            println!("change count {count}");
            for uti in pasteboard.types() {
                let size = pasteboard.read(&uti).map_or(0, |data| data.len());
                println!("  {uti}: {size} bytes");
            }
        }

        let app = workspace.frontmost_app();
        if app != last_app {
            match &app {
                Some(app) => {
                    println!(
                        "frontmost app {} ({}, pid {})",
                        app.name, app.bundle_id, app.pid
                    );
                    if let Some(png) = workspace.app_icon_png(app)
                        && std::fs::write(&icon_path, &png).is_ok()
                    {
                        println!("  icon: {} bytes -> {}", png.len(), icon_path.display());
                    }
                }
                None => println!("frontmost app: none"),
            }
            last_app = app;
        }

        std::thread::sleep(Duration::from_millis(300));
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the pasteboard probe only runs on macOS");
}
