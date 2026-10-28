pub mod commands;
pub mod error;
pub mod events;
pub mod macos;
pub mod state;
pub mod tray;
pub mod windows;

use state::AppState;
use tracing_subscriber::EnvFilter;

pub fn run() {
    // Filter with RUST_LOG, e.g. `tadidy_lib=debug`. Never log clip content (AGENTS.md §2).
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    if let Err(error) = tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            // Menu-bar app: no Dock icon and not in ⌘Tab, also in `tauri dev` where the
            // bundle's Info.plist (LSUIElement) does not apply (MNU-06).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::app_version])
        .run(tauri::generate_context!())
    {
        tracing::error!(%error, "Tadidy failed to start");
        std::process::exit(1);
    }
}
