//! The native status item and its `NSMenu`. Creating them needs the main thread (muda), so no
//! unit test can run this code: the menu model and the click rules live in `crate::tray` and
//! are tested there; this part is covered by the manual QA of TAD-003.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Runtime};

use crate::tray::{MenuAction, TRAY_ID, TrayItem, menu_action, menu_items, toggles_the_popover};
use crate::windows;

/// The 36 × 36 px (@2x) template image: macOS scales a template to the menu bar's 18 pt
/// height, so the larger file stays sharp on Retina. `tray-template.png` is its @1x twin.
const ICON: &[u8] = include_bytes!("../../icons/tray/tray-template@2x.png");

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    for item in menu_items() {
        match item {
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

/// Adds the tray icon: a left click toggles the popover (MNU-02), a right click opens the
/// native menu (MNU-03).
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON)?)
        .icon_as_template(true)
        .show_menu_on_left_click(false)
        .menu(&build_menu(app)?)
        .on_menu_event(|app, event| {
            if menu_action(event.id().as_ref()) == Some(MenuAction::Quit) {
                app.exit(0);
            }
        })
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
