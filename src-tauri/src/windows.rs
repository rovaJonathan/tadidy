//! The popover window (MNU-02, MNU-04). The decision "should this tray click show or hide
//! the popover?" is a pure type with an injectable clock so it is tested without a window.

use std::time::{Duration, Instant};

/// A tray click that arrives this soon after a focus-loss hide is the click that caused the
/// focus loss (the user clicked the icon to close the popover): it must not reopen it.
const REOPEN_DEBOUNCE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleAction {
    Show,
    Hide,
    Ignore,
}

/// Decides what a tray click does. Time is passed in, so tests use a fixed clock.
#[derive(Debug, Default)]
pub struct PopoverToggle {
    hidden_by_focus_loss_at: Option<Instant>,
}

impl PopoverToggle {
    /// The popover was hidden because it lost focus.
    pub fn focus_lost(&mut self, now: Instant) {
        self.hidden_by_focus_loss_at = Some(now);
    }

    /// A left click on the tray icon, while the popover is `visible` or not.
    pub fn click(&mut self, now: Instant, visible: bool) -> ToggleAction {
        if visible {
            return ToggleAction::Hide;
        }
        match self.hidden_by_focus_loss_at.take() {
            Some(hidden_at) if now.duration_since(hidden_at) < REOPEN_DEBOUNCE => {
                ToggleAction::Ignore
            }
            _ => ToggleAction::Show,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    use tauri::test::MockRuntime;
    use tauri::{Manager, WebviewWindow, WindowEvent};

    #[test]
    fn click_right_after_focus_loss_does_not_reopen() {
        let mut toggle = PopoverToggle::default();
        let hidden_at = Instant::now();
        toggle.focus_lost(hidden_at);

        let action = toggle.click(hidden_at + Duration::from_millis(100), false);

        assert_eq!(action, ToggleAction::Ignore);
    }

    #[test]
    fn click_after_debounce_reopens() {
        let mut toggle = PopoverToggle::default();
        let hidden_at = Instant::now();
        toggle.focus_lost(hidden_at);

        let action = toggle.click(hidden_at + Duration::from_millis(250), false);

        assert_eq!(action, ToggleAction::Show);
    }

    #[test]
    fn click_while_visible_hides() {
        let mut toggle = PopoverToggle::default();

        assert_eq!(toggle.click(Instant::now(), true), ToggleAction::Hide);
    }

    #[test]
    fn a_focus_loss_only_debounces_the_next_click() {
        let mut toggle = PopoverToggle::default();
        let hidden_at = Instant::now();
        toggle.focus_lost(hidden_at);
        toggle.click(hidden_at + Duration::from_millis(100), false);

        let action = toggle.click(hidden_at + Duration::from_millis(150), false);

        assert_eq!(action, ToggleAction::Show);
    }

    /// Tauri's mock runtime ignores `show` and `hide` (`is_visible` is always true), so the
    /// show/hide decisions are tested against this fake.
    #[derive(Default)]
    struct FakePopover {
        visible: Cell<bool>,
    }

    impl PopoverWindow for FakePopover {
        fn is_shown(&self) -> bool {
            self.visible.get()
        }
        fn reveal(&self) {
            self.visible.set(true);
        }
        fn conceal(&self) {
            self.visible.set(false);
        }
    }

    #[test]
    fn toggling_a_hidden_popover_shows_it() {
        let popover = FakePopover::default();

        toggle(&popover, &PopoverState::default(), Instant::now());

        assert!(popover.is_shown());
    }

    #[test]
    fn toggling_a_visible_popover_hides_it() {
        let popover = FakePopover::default();
        popover.reveal();

        toggle(&popover, &PopoverState::default(), Instant::now());

        assert!(!popover.is_shown());
    }

    #[test]
    fn losing_focus_hides_the_popover() {
        let popover = FakePopover::default();
        popover.reveal();

        focus_lost(&popover, &PopoverState::default(), Instant::now());

        assert!(!popover.is_shown());
    }

    #[test]
    fn a_tray_click_right_after_the_focus_loss_keeps_the_popover_hidden() {
        let popover = FakePopover::default();
        let state = PopoverState::default();
        let lost_at = Instant::now();
        popover.reveal();
        focus_lost(&popover, &state, lost_at);

        toggle(&popover, &state, lost_at + Duration::from_millis(100));

        assert!(!popover.is_shown());
    }

    #[test]
    fn a_tray_click_after_the_debounce_shows_it_again() {
        let popover = FakePopover::default();
        let state = PopoverState::default();
        let lost_at = Instant::now();
        popover.reveal();
        focus_lost(&popover, &state, lost_at);

        toggle(&popover, &state, lost_at + Duration::from_millis(300));

        assert!(popover.is_shown());
    }

    fn app_with_popover() -> (tauri::App<MockRuntime>, WebviewWindow<MockRuntime>) {
        let app = tauri::test::mock_app();
        app.manage(PopoverState::default());
        let popover = create_popover(&app).unwrap();
        (app, popover)
    }

    /// Whether a tray click right now would be swallowed by the focus-loss debounce.
    fn click_is_debounced(app: &tauri::App<MockRuntime>) -> bool {
        let state = app.state::<PopoverState>();
        let action = state.lock().unwrap().click(Instant::now(), false);
        action == ToggleAction::Ignore
    }

    #[test]
    fn the_popover_is_created_under_its_label() {
        let (app, popover) = app_with_popover();

        assert_eq!(popover.label(), POPOVER_LABEL);
        assert!(app.get_webview_window(POPOVER_LABEL).is_some());
    }

    #[test]
    fn the_popover_window_losing_focus_starts_the_debounce() {
        let (app, popover) = app_with_popover();

        on_window_event(&popover.as_ref().window(), &WindowEvent::Focused(false));

        assert!(click_is_debounced(&app));
        assert!(app.get_webview_window(POPOVER_LABEL).is_some());
    }

    #[test]
    fn gaining_focus_or_another_window_losing_it_starts_no_debounce() {
        let (app, popover) = app_with_popover();
        let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
            .build()
            .unwrap();

        on_window_event(&popover.as_ref().window(), &WindowEvent::Focused(true));
        on_window_event(&other.as_ref().window(), &WindowEvent::Focused(false));

        assert!(!click_is_debounced(&app));
    }
}
