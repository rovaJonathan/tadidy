//! The tray icon's native menu (MNU-03). The model is a pure function so the labels, the
//! order and the enabled state are tested without AppKit; `lib.rs` turns it into a menu.

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
}
