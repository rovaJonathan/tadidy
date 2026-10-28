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
