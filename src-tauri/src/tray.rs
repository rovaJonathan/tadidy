//! The tray icon's native menu (MNU-03). The model is a pure function so the labels, the
//! order and the enabled state are tested without AppKit; `lib.rs` turns it into a menu.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

use crate::windows;

pub const TRAY_ID: &str = "tadidy";

/// The 36 × 36 px (@2x) template image: macOS scales a template to the menu bar's 18 pt
/// height, so the larger file stays sharp on Retina. `tray-template.png` is its @1x twin.
const ICON: &[u8] = include_bytes!("../icons/tray/tray-template@2x.png");

/// One row of the tray menu, in the order the prototype's `#tray-menu` shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayItem {
    Entry {
        id: &'static str,
        label: &'static str,
        enabled: bool,
        /// Tauri accelerator string, shown as the menu's keyboard equivalent.
        accelerator: Option<&'static str>,
    },
    Separator,
}

const fn entry(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    accelerator: Option<&'static str>,
) -> TrayItem {
    TrayItem::Entry {
        id,
        label,
        enabled,
        accelerator,
    }
}

/// The tray menu. Preferences…, Clear history… and About are wired by TAD-018 and Check for
/// updates… by the P2 updater ticket (SYS-04): until then they are shown but disabled.
pub fn menu_items() -> Vec<TrayItem> {
    vec![
        entry("preferences", "Preferences…", false, Some("CmdOrCtrl+,")),
        entry("clear-history", "Clear history…", false, None),
        TrayItem::Separator,
        entry("about", "About Tadidy", false, None),
        entry("check-updates", "Check for updates…", false, None),
        TrayItem::Separator,
        entry("quit", "Quit Tadidy", true, Some("CmdOrCtrl+Q")),
    ]
}

/// What choosing a menu item does. Only Quit is wired in this ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Quit,
}

pub fn menu_action(id: &str) -> Option<MenuAction> {
    (id == "quit").then_some(MenuAction::Quit)
}

pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
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
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::toggle_popover(app);
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[TrayItem]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                TrayItem::Entry { label, .. } => (*label).to_owned(),
                TrayItem::Separator => "-".to_owned(),
            })
            .collect()
    }

    #[test]
    fn menu_lists_the_prototype_items_in_order() {
        let items = menu_items();

        assert_eq!(
            labels(&items),
            [
                "Preferences…",
                "Clear history…",
                "-",
                "About Tadidy",
                "Check for updates…",
                "-",
                "Quit Tadidy",
            ]
        );
        let ids: Vec<_> = items
            .iter()
            .filter_map(|item| match item {
                TrayItem::Entry { id, .. } => Some(*id),
                TrayItem::Separator => None,
            })
            .collect();
        assert_eq!(
            ids,
            [
                "preferences",
                "clear-history",
                "about",
                "check-updates",
                "quit"
            ]
        );
        let accelerators: Vec<_> = items
            .iter()
            .filter_map(|item| match item {
                TrayItem::Entry {
                    id, accelerator, ..
                } => accelerator.map(|accelerator| (*id, accelerator)),
                TrayItem::Separator => None,
            })
            .collect();
        assert_eq!(
            accelerators,
            [("preferences", "CmdOrCtrl+,"), ("quit", "CmdOrCtrl+Q")]
        );
    }

    #[test]
    fn only_quit_is_enabled() {
        let enabled: Vec<_> = menu_items()
            .into_iter()
            .filter_map(|item| match item {
                TrayItem::Entry {
                    id, enabled: true, ..
                } => Some(id),
                _ => None,
            })
            .collect();

        assert_eq!(enabled, ["quit"]);
    }

    #[test]
    fn only_quit_has_an_action() {
        assert_eq!(menu_action("quit"), Some(MenuAction::Quit));
        for id in [
            "preferences",
            "clear-history",
            "about",
            "check-updates",
            "?",
        ] {
            assert_eq!(menu_action(id), None, "{id}");
        }
    }

    fn click(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: tauri::tray::TrayIconId::new(TRAY_ID),
            position: tauri::PhysicalPosition::new(0.0, 0.0),
            rect: tauri::Rect::default(),
            button,
            button_state,
        }
    }

    #[test]
    fn a_left_click_release_toggles_the_popover() {
        assert!(toggles_the_popover(&click(
            MouseButton::Left,
            MouseButtonState::Up
        )));
    }

    #[test]
    fn other_clicks_do_not_toggle_the_popover() {
        // The press is ignored (the release acts), and the right click opens the menu.
        for (button, state) in [
            (MouseButton::Left, MouseButtonState::Down),
            (MouseButton::Right, MouseButtonState::Up),
            (MouseButton::Right, MouseButtonState::Down),
        ] {
            assert!(
                !toggles_the_popover(&click(button, state)),
                "{button:?} {state:?}"
            );
        }
    }

    #[test]
    fn pointer_movement_does_not_toggle_the_popover() {
        let event = TrayIconEvent::Leave {
            id: tauri::tray::TrayIconId::new(TRAY_ID),
            position: tauri::PhysicalPosition::new(0.0, 0.0),
            rect: tauri::Rect::default(),
        };

        assert!(!toggles_the_popover(&event));
    }
}
