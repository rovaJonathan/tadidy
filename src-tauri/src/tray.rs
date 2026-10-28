//! The tray icon's native menu (MNU-03). The model is a pure function so the labels, the
//! order and the enabled state are tested without AppKit; `lib.rs` turns it into a menu.

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

    #[test]
    fn build_menu_follows_the_model() {
        let app = tauri::test::mock_app();

        let menu = build_menu(app.handle()).unwrap();

        assert_eq!(menu.items().unwrap().len(), menu_items().len());
    }

    #[test]
    fn setup_registers_the_tray_icon() {
        let app = tauri::test::mock_app();

        setup(app.handle()).unwrap();

        assert!(app.tray_by_id(TRAY_ID).is_some());
    }
}
