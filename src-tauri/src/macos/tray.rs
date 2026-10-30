//! The native status item and its `NSMenu`. Creating them needs the main thread (muda), so no
//! unit test can run this code: the menu model and the click rules live in `crate::tray` and
//! are tested there; this part is covered by the manual QA of TAD-003.

use tauri::image::Image;
use tauri::menu::{AboutMetadataBuilder, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

use crate::events::TauriEventSink;
use crate::tray::{
    ABOUT_ID, TRAY_ID, TrayAction, TrayItem, action_for, menu_items, request_clear_history,
    toggles_the_popover,
};
use crate::windows::{self, POPOVER_LABEL};

/// The 36 × 36 px (@2x) template image: macOS scales a template to the menu bar's 18 pt
/// height, so the larger file stays sharp on Retina. `tray-template.png` is its @1x twin.
const ICON: &[u8] = include_bytes!("../../icons/tray/tray-template@2x.png");

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    for item in menu_items() {
        match item {
            TrayItem::Entry {
                id: ABOUT_ID,
                label,
                ..
            } => menu.append(&about_item(app, label)?)?,
            TrayItem::Entry {
                id,
                label,
                enabled,
                accelerator,
            } => menu.append(&MenuItem::with_id(app, id, label, enabled, accelerator)?)?,
            TrayItem::Separator => menu.append(&PredefinedMenuItem::separator(app)?)?,
        }
    }
    Ok(menu)
}

/// The native about panel. Its copyright line is the bundle's `NSHumanReadableCopyright`
/// (`Info.plist`), which AppKit adds by itself. The item fires no menu event, so it cannot
/// activate the app first: manual QA checks the panel opens in front (TAD-018).
fn about_item<R: Runtime>(app: &AppHandle<R>, label: &str) -> tauri::Result<PredefinedMenuItem<R>> {
    let package = app.package_info();
    let metadata = AboutMetadataBuilder::new()
        .name(Some(package.name.clone()))
        .version(Some(package.version.to_string()))
        .build();
    PredefinedMenuItem::about(app, Some(label), Some(metadata))
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match action_for(id) {
        Some(TrayAction::Quit) => app.exit(0),
        Some(TrayAction::Preferences) => {
            if let Err(error) = windows::open_preferences(app) {
                tracing::error!(%error, "could not open the preferences window");
            }
        }
        Some(TrayAction::ClearHistory) => match app.get_webview_window(POPOVER_LABEL) {
            Some(popover) => request_clear_history(&popover, &TauriEventSink::new(app.clone())),
            None => tracing::error!("the popover window is missing"),
        },
        None => {}
    }
}

/// Adds the tray icon: a left click toggles the popover (MNU-02), a right click opens the
/// native menu (MNU-03).
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON)?)
        .icon_as_template(true)
        .show_menu_on_left_click(false)
        .menu(&build_menu(app)?)
        .on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            // The positioner needs the icon's position to place the popover under it.
            tauri_plugin_positioner::on_tray_event(app, &event);
            if toggles_the_popover(&event) {
                windows::toggle_popover(app);
            }
        })
        .build(app)?;
    Ok(())
}
