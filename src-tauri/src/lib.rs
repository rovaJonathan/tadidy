pub mod capture;
pub mod clip;
pub mod clock;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod events;
pub mod macos;
pub mod platform;
pub mod settings;
pub mod shortcuts;
pub mod state;
pub mod storage;
pub mod tray;
pub mod watcher;
pub mod windows;

use state::AppState;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

/// The platform services of the running app: the AppKit and CoreGraphics ones.
#[cfg(target_os = "macos")]
fn real_state(db: Option<storage::Db>) -> AppState {
    AppState::new(
        std::sync::Arc::new(macos::accessibility::AxProbe),
        std::sync::Arc::new(macos::keys::CgKeySender),
    )
    .with_db(db)
}

/// The clipboard watcher's view of the system: the real pasteboard, workspace, idle probe,
/// event channel and clock.
#[cfg(target_os = "macos")]
fn real_watcher_services(app: &tauri::AppHandle) -> watcher::Services {
    use std::sync::Arc;
    watcher::Services {
        pasteboard: Arc::new(macos::pasteboard::NsPasteboard),
        workspace: Arc::new(macos::workspace::NsWorkspace),
        idle: Arc::new(macos::idle::CgIdleProbe),
        events: Arc::new(events::TauriEventSink::new(app.clone())),
        clock: Arc::new(clock::SystemClock),
    }
}

pub fn run() {
    // Filter with RUST_LOG, e.g. `tadidy_lib=debug`. Never log clip content (AGENTS.md §2).
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    if let Err(error) = builder
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(windows::PopoverState::default())
        .setup(|app| {
            // Menu-bar app: no Dock icon and not in ⌘Tab, also in `tauri dev` where the
            // bundle's Info.plist (LSUIElement) does not apply (MNU-06).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // The history database; without it (key missing) the app still runs, see `storage`.
            #[cfg(target_os = "macos")]
            app.manage(real_state(storage::open_or_report(
                &app.path().app_data_dir()?,
                &macos::keychain::KeychainKeyStore::default(),
            )));
            // Records what is copied; not started without the database.
            #[cfg(target_os = "macos")]
            watcher::start(
                &app.state::<AppState>(),
                real_watcher_services(app.handle()),
            );
            // Created hidden at startup so the tray click only has to show it (< 100 ms).
            let popover = windows::create_popover(app)?;
            #[cfg(target_os = "macos")]
            macos::vibrancy::apply_popover_material(&popover)?;
            #[cfg(target_os = "macos")]
            macos::panel::create(app.handle())?;
            #[cfg(target_os = "macos")]
            macos::tray::setup(app.handle())?;
            shortcuts::register_at_startup(
                &shortcuts::AppShortcuts(app.handle().clone()),
                &app.state::<AppState>().shortcut,
            );
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
