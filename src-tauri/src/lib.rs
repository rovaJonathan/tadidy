pub mod commands;
pub mod error;
pub mod macos;
pub mod state;

use state::AppState;

pub fn run() {
    if let Err(error) = tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![commands::app_version])
        .run(tauri::generate_context!())
    {
        eprintln!("Tadidy failed to start: {error}");
        std::process::exit(1);
    }
}
