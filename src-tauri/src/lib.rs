pub mod capture;
pub mod clip;
pub mod clock;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod events;
pub mod history;
pub mod login_item;
pub mod macos;
pub mod paster;
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

/// The platform services of the running app: the AppKit and CoreGraphics ones, the global
/// shortcut plugin and the system's login item.
#[cfg(target_os = "macos")]
fn real_state(db: Option<storage::Db>, app: &tauri::AppHandle) -> AppState {
    AppState::new(
        std::sync::Arc::new(macos::accessibility::AxProbe),
        std::sync::Arc::new(macos::keys::CgKeySender),
        std::sync::Arc::new(macos::pasteboard::NsPasteboard),
        std::sync::Arc::new(macos::workspace::NsWorkspace),
    )
    .with_db(db)
    .with_shortcuts(std::sync::Arc::new(shortcuts::AppShortcuts(app.clone())))
    .with_login_item(std::sync::Arc::new(macos::login_item::SmLoginItem))
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
        .manage(windows::PolicyState::default())
        .setup(|app| {
            // Menu-bar app: no Dock icon and not in ⌘Tab, also in `tauri dev` where the
            // bundle's Info.plist (LSUIElement) does not apply (MNU-06).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // The history database; without it (key missing) the app still runs, see `storage`.
            #[cfg(target_os = "macos")]
            app.manage(real_state(
                storage::open_or_report(
                    &app.path().app_data_dir()?,
                    &macos::keychain::KeychainKeyStore::default(),
                ),
                app.handle(),
            ));
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
            // The login item is read back and the stored Quick Panel shortcut registered.
            app.state::<AppState>().apply_stored_settings();
            Ok(())
        })
        .on_window_event(windows::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::hide_window,
            commands::check_accessibility,
            commands::open_accessibility_settings,
            commands::get_settings,
            commands::set_settings,
            commands::get_shortcut_status,
            commands::open_preferences,
            commands::list_clips,
            commands::delete_clips,
            commands::clear_history,
            commands::get_app_icon,
            commands::paste_clip,
            commands::copy_clip
        ])
        .run(tauri::generate_context!())
    {
        tracing::error!(%error, "Tadidy failed to start");
        std::process::exit(1);
    }
}
