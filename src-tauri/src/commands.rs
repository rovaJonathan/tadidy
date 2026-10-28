//! IPC commands: thin wrappers that delegate to the services (AGENTS.md §13).

/// The version of the running app, from the crate manifest.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    #[test]
    fn app_version_returns_the_crate_version() {
        assert_eq!(app_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn hide_window_hides_the_calling_window_without_destroying_it() {
        let app = tauri::test::mock_app();
        let window = tauri::WebviewWindowBuilder::new(&app, "popover", Default::default())
            .build()
            .unwrap();
        window.show().unwrap();

        hide_window(window.clone()).unwrap();

        assert!(!window.is_visible().unwrap());
        assert!(app.get_webview_window("popover").is_some());
    }
}
