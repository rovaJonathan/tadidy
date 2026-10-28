pub mod commands;
pub mod macos;
pub mod state;

use state::AppState;

pub fn run() {
    if let Err(error) = tauri::Builder::default()
        .manage(AppState::default())
        .run(tauri::generate_context!())
    {
        eprintln!("Tadidy failed to start: {error}");
        std::process::exit(1);
    }
}
