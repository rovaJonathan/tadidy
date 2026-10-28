//! The popover window (MNU-02, MNU-04). The decision "should this tray click show or hide
//! the popover?" is a pure type with an injectable clock so it is tested without a window.

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

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
}
