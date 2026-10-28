//! The popover window (MNU-02, MNU-04). The decision "should this tray click show or hide
//! the popover?" is a pure type with an injectable clock so it is tested without a window.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window,
    WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

pub const POPOVER_LABEL: &str = "popover";

/// Popover size in logical pixels (DESIGN.md §3).
const POPOVER_WIDTH: f64 = 380.0;
const POPOVER_HEIGHT: f64 = 520.0;

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

/// Managed by Tauri: shared by the tray click handler and the window event handler.
pub type PopoverState = Mutex<PopoverToggle>;

/// Creates the popover hidden, so showing it later takes well under 100 ms. Vibrancy is
/// applied by the caller (`macos::vibrancy`): it needs a real window.
pub fn create_popover<R: Runtime>(manager: &impl Manager<R>) -> tauri::Result<WebviewWindow<R>> {
    WebviewWindowBuilder::new(
        manager,
        POPOVER_LABEL,
        WebviewUrl::App("popover.html".into()),
    )
    .inner_size(POPOVER_WIDTH, POPOVER_HEIGHT)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .build()
}

/// What the toggle logic needs from the popover's window, so it runs against a fake in the
/// tests: Tauri's mock runtime ignores `show` and `hide`.
pub trait PopoverWindow {
    fn is_shown(&self) -> bool;
    /// Places the window under the tray icon, shows it and gives it the focus.
    fn reveal(&self);
    fn conceal(&self);
}

impl<R: Runtime> PopoverWindow for WebviewWindow<R> {
    fn is_shown(&self) -> bool {
        self.is_visible().unwrap_or(false)
    }

    fn reveal(&self) {
        // Without a tray position yet (no tray event seen) the window keeps its place.
        if let Err(error) = self.move_window(Position::TrayBottomCenter) {
            tracing::warn!(%error, "could not place the popover under the tray icon");
        }
        if let Err(error) = self.show().and_then(|()| self.set_focus()) {
            tracing::error!(%error, "could not show the popover");
        }
    }

    fn conceal(&self) {
        if let Err(error) = self.hide() {
            tracing::error!(%error, "could not hide the popover");
        }
    }
}

fn lock(state: &PopoverState) -> MutexGuard<'_, PopoverToggle> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A left click on the tray icon (MNU-02): shows the popover under the icon, or hides it.
pub fn toggle(popover: &impl PopoverWindow, state: &PopoverState, now: Instant) {
    // The lock is never held while the window is touched.
    let visible = popover.is_shown();
    let action = lock(state).click(now, visible);
    match action {
        ToggleAction::Show => popover.reveal(),
        ToggleAction::Hide => popover.conceal(),
        ToggleAction::Ignore => {}
    }
}

/// The popover lost focus (MNU-04): hide it and remember when, so the tray click that caused
/// the focus loss does not reopen it.
pub fn focus_lost(popover: &impl PopoverWindow, state: &PopoverState, now: Instant) {
    lock(state).focus_lost(now);
    popover.conceal();
}

/// Entry point of the tray's left click.
pub fn toggle_popover<R: Runtime>(app: &AppHandle<R>) {
    let (Some(popover), Some(state)) = (
        app.get_webview_window(POPOVER_LABEL),
        app.try_state::<PopoverState>(),
    ) else {
        tracing::error!("the popover window or its state is missing");
        return;
    };
    toggle(&popover, &state, Instant::now());
}

/// Window event hook: the popover hides when it loses focus; it is never destroyed.
pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if window.label() != POPOVER_LABEL || !matches!(event, WindowEvent::Focused(false)) {
        return;
    }
    let (Some(popover), Some(state)) = (
        window.app_handle().get_webview_window(POPOVER_LABEL),
        window.try_state::<PopoverState>(),
    ) else {
        tracing::error!("the popover window or its state is missing");
        return;
    };
    focus_lost(&popover, &state, Instant::now());
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

    /// Asks the window about itself while checking that the toggle state is not locked.
    struct LockProbe<'a> {
        state: &'a PopoverState,
    }

    impl PopoverWindow for LockProbe<'_> {
        fn is_shown(&self) -> bool {
            assert!(
                self.state.try_lock().is_ok(),
                "the window was queried while the state was locked"
            );
            false
        }
        fn reveal(&self) {}
        fn conceal(&self) {}
    }

    #[test]
    fn the_window_is_queried_without_holding_the_state_lock() {
        let state = PopoverState::default();

        toggle(&LockProbe { state: &state }, &state, Instant::now());
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
