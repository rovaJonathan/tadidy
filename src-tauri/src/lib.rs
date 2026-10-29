pub mod commands;
pub mod error;
pub mod events;
pub mod macos;
pub mod platform;
pub mod state;
pub mod tray;
pub mod windows;

use state::AppState;
use tracing_subscriber::EnvFilter;

/// The platform services of the running app: the AppKit and CoreGraphics ones.
#[cfg(target_os = "macos")]
fn real_state() -> AppState {
    AppState::new(
        std::sync::Arc::new(macos::accessibility::AxProbe),
        std::sync::Arc::new(macos::keys::CgKeySender),
    )
}

pub fn run() {
    // Filter with RUST_LOG, e.g. `tadidy_lib=debug`. Never log clip content (AGENTS.md §2).
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    if let Err(error) = tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(real_state())
        .manage(windows::PopoverState::default())
        .setup(|app| {
            // Menu-bar app: no Dock icon and not in ⌘Tab, also in `tauri dev` where the
            // bundle's Info.plist (LSUIElement) does not apply (MNU-06).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // Created hidden at startup so the tray click only has to show it (< 100 ms).
            let popover = windows::create_popover(app)?;
            #[cfg(target_os = "macos")]
            macos::vibrancy::apply_popover_material(&popover)?;
            #[cfg(target_os = "macos")]
            macos::tray::setup(app.handle())?;
            Ok(())
        })
        .on_window_event(windows::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::hide_window,
            commands::check_accessibility,
            commands::open_accessibility_settings
        ])
        .run(tauri::generate_context!())
    {
        tracing::error!(%error, "Tadidy failed to start");
        std::process::exit(1);
    }
}
