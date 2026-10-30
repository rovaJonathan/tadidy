//! The popover window (MNU-02, MNU-04). The decision "should this tray click show or hide
//! the popover?" is a pure type with an injectable clock so it is tested without a window.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window,
    WindowEvent,
};
use tauri_plugin_positioner::{Position as TrayPosition, WindowExt};

use crate::events::{TauriEventSink, emit_panel_shown};
use crate::paster::{PasteOrigin, WindowHider};
use crate::state::AppState;

pub const POPOVER_LABEL: &str = "popover";
pub const PANEL_LABEL: &str = "panel";
pub const SETTINGS_LABEL: &str = "settings";

/// Popover size in logical pixels (DESIGN.md §3).
const POPOVER_WIDTH: f64 = 380.0;
const POPOVER_HEIGHT: f64 = 520.0;

/// Quick Panel size in logical pixels (DESIGN.md §3).
pub const PANEL_WIDTH: f64 = 640.0;
pub const PANEL_HEIGHT: f64 = 460.0;

/// Preferences window size in logical pixels (DESIGN.md §3).
const SETTINGS_WIDTH: f64 = 640.0;
const SETTINGS_HEIGHT: f64 = 480.0;

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
        // Before the window takes the focus: afterwards it is the front app itself.
        prepare_to_show(self.app_handle(), PasteOrigin::Popover);
        // Without a tray position yet (no tray event seen) the window keeps its place.
        if let Err(error) = self.move_window(TrayPosition::TrayBottomCenter) {
            tracing::warn!(%error, "could not place the popover under the tray icon");
        }
        if let Err(error) = self.show().and_then(|()| self.set_focus()) {
            tracing::error!(%error, "could not show the popover");
            return;
        }
        emit_panel_shown(
            &TauriEventSink::new(self.app_handle().clone()),
            self.label(),
        );
    }

    fn conceal(&self) {
        if let Err(error) = self.hide() {
            tracing::error!(%error, "could not hide the popover");
        }
    }
}

/// How the app shows itself to macOS: `Accessory` is a menu-bar app (no Dock icon, not in `⌘Tab`),
/// `Regular` is an ordinary app, which is what a window needs to take the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    Regular,
    Accessory,
}

/// Decides when the activation policy changes, so the rule is tested without AppKit: `Regular`
/// while the Preferences window exists, `Accessory` the rest of the time (spec *Windows*).
#[derive(Debug, Default)]
pub struct PolicySwitch {
    regular: bool,
}

impl PolicySwitch {
    /// The Preferences window is about to be shown: the policy to apply, if it has to change.
    pub fn window_opened(&mut self) -> Option<Policy> {
        todo!("TAD-017: Regular while the Preferences window exists")
    }

    /// The Preferences window is gone: the policy to apply, if it has to change.
    pub fn window_closed(&mut self) -> Option<Policy> {
        todo!("TAD-017: back to Accessory")
    }
}

/// Managed by Tauri: shared by `open_preferences` and the window event handler.
pub type PolicyState = Mutex<PolicySwitch>;

/// Applies a [`Policy`] to the running app; a fake records it in the tests.
pub trait PolicyApplier {
    fn apply(&self, policy: Policy);
}

impl<R: Runtime> PolicyApplier for AppHandle<R> {
    fn apply(&self, policy: Policy) {
        #[cfg(target_os = "macos")]
        {
            let policy = match policy {
                Policy::Regular => tauri::ActivationPolicy::Regular,
                Policy::Accessory => tauri::ActivationPolicy::Accessory,
            };
            if let Err(error) = self.set_activation_policy(policy) {
                tracing::error!(%error, "could not change the activation policy");
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = policy;
    }
}

/// Shows the Preferences window, created on demand, and gives it the focus. A second call finds
/// the first window and only brings it to the front.
pub fn open_preferences<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    open_preferences_with(app, app)
}

/// [`open_preferences`] with the policy applier passed in.
pub fn open_preferences_with<R: Runtime>(
    app: &AppHandle<R>,
    applier: &impl PolicyApplier,
) -> tauri::Result<()> {
    let _ = (app, applier);
    todo!("TAD-017: show the settings window")
}

/// The Preferences window is gone: the app is a menu-bar app again.
pub fn settings_closed(applier: &impl PolicyApplier, state: &PolicyState) {
    let _ = (applier, state);
    todo!("TAD-017: back to Accessory")
}

/// What happens just before the popover or the Quick Panel is shown. Remembers the app in front
/// (only for the popover: it takes the focus, so it is the one that has to give it back), and
/// checks the Accessibility permission so the banner follows a change made in System Settings.
pub fn prepare_to_show<R: Runtime>(app: &AppHandle<R>, origin: PasteOrigin) {
    let Some(state) = app.try_state::<AppState>() else {
        tracing::error!("the app state is missing");
        return;
    };
    if origin == PasteOrigin::Popover {
        state.remember_frontmost_app();
    }
    state
        .accessibility_monitor
        .check(&TauriEventSink::new(app.clone()));
}

/// How long a worker thread waits for the main thread before giving up on a job.
const MAIN_THREAD_WAIT: Duration = Duration::from_secs(2);

/// Runs `job` on the main thread and returns once it has run, so the steps that follow still come
/// after it (AGENTS.md §13). Gives up after [`MAIN_THREAD_WAIT`]: never call it from the main
/// thread itself, which cannot run the job while it waits.
pub fn run_on_main_thread_and_wait<R: Runtime>(
    app: &AppHandle<R>,
    job: impl FnOnce() + Send + 'static,
) {
    let (done, finished) = std::sync::mpsc::channel();
    let sent = app.run_on_main_thread(move || {
        job();
        // The waiter may have given up already: nobody is left to tell.
        let _ = done.send(());
    });
    if sent.is_err() || finished.recv_timeout(MAIN_THREAD_WAIT).is_err() {
        tracing::error!("the main thread did not run the window job");
    }
}

impl<R: Runtime> WindowHider for AppHandle<R> {
    fn hide(&self, origin: PasteOrigin) {
        match origin {
            PasteOrigin::Popover => match self.get_webview_window(POPOVER_LABEL) {
                Some(popover) => popover.conceal(),
                None => tracing::error!("the popover window is missing"),
            },
            // `Panel::hide` is a bare AppKit call: it must run on the main thread, and the paster
            // runs on a command's worker thread.
            #[cfg(target_os = "macos")]
            PasteOrigin::Panel => {
                let panel = crate::macos::panel::NsQuickPanel::new(self.clone());
                run_on_main_thread_and_wait(self, move || panel.conceal());
            }
            #[cfg(not(target_os = "macos"))]
            PasteOrigin::Panel => tracing::warn!("the Quick Panel needs macOS"),
        }
    }
}

/// A screen rectangle, in the physical pixels of the desktop space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Where a window of `size` goes to be centered on `screen`.
pub fn center_in(screen: Rect, size: Size) -> Position {
    Position {
        x: screen.x + (screen.width - size.width) / 2.0,
        y: screen.y + (screen.height - size.height) / 2.0,
    }
}

/// The screen that contains the mouse pointer, or the first one when it is outside all of them.
pub fn target_screen(pointer: Position, screens: &[Rect]) -> Option<Rect> {
    screens
        .iter()
        .find(|screen| {
            (screen.x..screen.x + screen.width).contains(&pointer.x)
                && (screen.y..screen.y + screen.height).contains(&pointer.y)
        })
        .or_else(|| screens.first())
        .copied()
}

/// What the Quick Panel toggle needs from the panel, so it runs against a fake in the tests.
pub trait QuickPanel {
    fn is_shown(&self) -> bool;
    /// Centers the panel on the screen under the pointer, shows it and makes it the key
    /// window without activating the app.
    fn reveal(&self);
    fn conceal(&self);
}

/// The global shortcut (QPN-01): shows the Quick Panel, or hides it.
pub fn toggle_panel(panel: &impl QuickPanel) {
    if panel.is_shown() {
        panel.conceal();
    } else {
        panel.reveal();
    }
}

/// Entry point of the global shortcut.
pub fn toggle_quick_panel<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    toggle_panel(&crate::macos::panel::NsQuickPanel::new(app.clone()));
    #[cfg(not(target_os = "macos"))]
    tracing::warn!(app = app.package_info().name, "the Quick Panel needs macOS");
}

/// A monitor's rectangle in logical points, the unit of `CGDisplayBounds`. tao reports a
/// monitor's position and size in pixels of that monitor's own scale, so the rectangles of
/// monitors with different scales do not tile one space until they are divided by their scale.
pub fn logical_rect(position: (i32, i32), size: (u32, u32), scale: f64) -> Rect {
    Rect {
        x: f64::from(position.0) / scale,
        y: f64::from(position.1) / scale,
        width: f64::from(size.0) / scale,
        height: f64::from(size.1) / scale,
    }
}

/// The pointer in logical points. tao's `cursor_position` multiplies the global point by the
/// primary monitor's scale, whichever monitor the pointer is on.
pub fn pointer_in_points(cursor: (f64, f64), primary_scale: f64) -> Position {
    Position {
        x: cursor.0 / primary_scale,
        y: cursor.1 / primary_scale,
    }
}

/// Centers `window` on the screen that contains the mouse pointer (QPN-03, default option).
pub fn center_on_pointer_screen<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
) -> tauri::Result<()> {
    // Everything is worked out in logical points, the unit AppKit places windows in: the
    // physical values tao reports use a different scale for the pointer, for each monitor
    // and for the window being moved.
    let cursor = app.cursor_position()?;
    let primary_scale = app
        .primary_monitor()?
        .map_or(1.0, |monitor| monitor.scale_factor());
    let screens: Vec<Rect> = app
        .available_monitors()?
        .iter()
        .map(|monitor| {
            let (position, size) = (monitor.position(), monitor.size());
            logical_rect(
                (position.x, position.y),
                (size.width, size.height),
                monitor.scale_factor(),
            )
        })
        .collect();
    let pointer = pointer_in_points((cursor.x, cursor.y), primary_scale);
    let Some(screen) = target_screen(pointer, &screens) else {
        return Ok(());
    };
    let origin = center_in(
        screen,
        Size {
            width: PANEL_WIDTH,
            height: PANEL_HEIGHT,
        },
    );
    window.set_position(tauri::LogicalPosition::new(origin.x, origin.y))
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

    // --- Preferences window ----------------------------------------------------------------

    #[test]
    fn policy_the_first_window_asks_for_regular() {
        let mut switch = PolicySwitch::default();

        assert_eq!(switch.window_opened(), Some(Policy::Regular));
    }

    #[test]
    fn policy_a_window_already_open_changes_nothing() {
        let mut switch = PolicySwitch::default();
        switch.window_opened();

        assert_eq!(switch.window_opened(), None);
    }

    #[test]
    fn policy_closing_the_window_goes_back_to_accessory() {
        let mut switch = PolicySwitch::default();
        switch.window_opened();

        assert_eq!(switch.window_closed(), Some(Policy::Accessory));
    }

    #[test]
    fn policy_closing_when_nothing_was_open_changes_nothing() {
        let mut switch = PolicySwitch::default();

        assert_eq!(switch.window_closed(), None);
    }

    #[test]
    fn policy_a_window_opened_again_after_closing_asks_for_regular_again() {
        let mut switch = PolicySwitch::default();
        switch.window_opened();
        switch.window_closed();

        assert_eq!(switch.window_opened(), Some(Policy::Regular));
    }

    #[derive(Default)]
    struct RecordingPolicy {
        applied: std::cell::RefCell<Vec<Policy>>,
    }

    impl PolicyApplier for RecordingPolicy {
        fn apply(&self, policy: Policy) {
            self.applied.borrow_mut().push(policy);
        }
    }

    fn app_for_preferences() -> tauri::App<MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(PolicyState::default());
        app
    }

    fn settings_windows(app: &tauri::App<MockRuntime>) -> usize {
        app.webview_windows()
            .keys()
            .filter(|label| label.as_str() == SETTINGS_LABEL)
            .count()
    }

    #[test]
    fn opening_preferences_creates_the_settings_window_as_a_regular_app() {
        let app = app_for_preferences();
        let policy = RecordingPolicy::default();

        open_preferences_with(app.handle(), &policy).unwrap();

        let window = app.get_webview_window(SETTINGS_LABEL).unwrap();
        assert_eq!(window.title().unwrap(), "Tadidy Preferences");
        assert!(window.is_decorated().unwrap());
        assert_eq!(*policy.applied.borrow(), vec![Policy::Regular]);
    }

    #[test]
    fn opening_preferences_twice_keeps_one_window_and_one_policy_change() {
        let app = app_for_preferences();
        let policy = RecordingPolicy::default();

        open_preferences_with(app.handle(), &policy).unwrap();
        open_preferences_with(app.handle(), &policy).unwrap();

        assert_eq!(settings_windows(&app), 1);
        assert_eq!(*policy.applied.borrow(), vec![Policy::Regular]);
    }

    #[test]
    fn closing_the_settings_window_goes_back_to_accessory() {
        let app = app_for_preferences();
        let policy = RecordingPolicy::default();
        open_preferences_with(app.handle(), &policy).unwrap();

        settings_closed(&policy, &app.state::<PolicyState>());

        assert_eq!(
            *policy.applied.borrow(),
            vec![Policy::Regular, Policy::Accessory]
        );
    }

    #[test]
    fn the_destroyed_settings_window_lets_the_next_open_ask_for_regular_again() {
        let app = app_for_preferences();
        let policy = RecordingPolicy::default();
        open_preferences_with(app.handle(), &policy).unwrap();
        let window = app.get_webview_window(SETTINGS_LABEL).unwrap();

        on_window_event(&window.as_ref().window(), &WindowEvent::Destroyed);

        let state = app.state::<PolicyState>();
        assert_eq!(state.lock().unwrap().window_opened(), Some(Policy::Regular));
    }

    #[test]
    fn another_window_being_destroyed_leaves_the_policy_alone() {
        let app = app_for_preferences();
        let policy = RecordingPolicy::default();
        open_preferences_with(app.handle(), &policy).unwrap();
        let (_app2, popover) = app_with_popover();

        on_window_event(&popover.as_ref().window(), &WindowEvent::Destroyed);

        let state = app.state::<PolicyState>();
        assert_eq!(state.lock().unwrap().window_opened(), None);
    }

    const SCREEN: Rect = Rect {
        x: 1440.0,
        y: -200.0,
        width: 2560.0,
        height: 1440.0,
    };

    #[test]
    fn center_in_centers_on_the_given_screen() {
        let size = Size {
            width: 640.0,
            height: 460.0,
        };

        let position = center_in(SCREEN, size);

        assert_eq!(
            position,
            Position {
                x: 2400.0,
                y: 290.0
            }
        );
    }

    #[test]
    fn the_target_screen_contains_the_pointer() {
        let main = Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        };

        let at = |x, y| target_screen(Position { x, y }, &[main, SCREEN]);

        assert_eq!(at(10.0, 10.0), Some(main));
        assert_eq!(at(1440.0, 0.0), Some(SCREEN));
        assert_eq!(at(3999.0, 1239.0), Some(SCREEN));
    }

    #[test]
    fn a_pointer_outside_every_screen_falls_back_to_the_first() {
        let pointer = Position {
            x: -50.0,
            y: 5000.0,
        };

        assert_eq!(target_screen(pointer, &[SCREEN]), Some(SCREEN));
        assert_eq!(target_screen(pointer, &[]), None);
    }

    #[derive(Default)]
    struct FakePanel {
        visible: Cell<bool>,
    }

    impl QuickPanel for FakePanel {
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
    fn panel_toggle_shows_when_hidden() {
        let panel = FakePanel::default();

        toggle_panel(&panel);

        assert!(panel.is_shown());
    }

    #[test]
    fn panel_toggle_hides_when_visible() {
        let panel = FakePanel::default();
        panel.reveal();

        toggle_panel(&panel);

        assert!(!panel.is_shown());
    }

    #[test]
    fn revealing_the_popover_emits_panel_shown_with_its_label() {
        use tauri::Listener;
        let (app, popover) = app_with_popover();
        app.handle()
            .plugin(tauri_plugin_positioner::init())
            .unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        app.listen_any(crate::events::PANEL_SHOWN, move |event| {
            sender.send(event.payload().to_owned()).unwrap();
        });

        popover.reveal();

        let payload = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(payload, r#"{"window":"popover"}"#);
    }

    use crate::events::ACCESSIBILITY_CHANGED;
    use crate::platform::SourceApp;
    use crate::platform::fakes::{FakeAccessibility, FakeKeySender, FakePasteboard, FakeWorkspace};
    use std::sync::Arc;

    fn textedit() -> SourceApp {
        SourceApp::new("com.apple.TextEdit", "TextEdit", 4242)
    }

    /// A mock app whose state sees TextEdit in front and a trusted (or not) process.
    fn app_with_state(trusted: bool) -> tauri::App<MockRuntime> {
        let app = tauri::test::mock_app();
        let workspace = Arc::new(FakeWorkspace::new());
        workspace.set_frontmost(Some(textedit()));
        let accessibility = Arc::new(FakeAccessibility::new());
        accessibility.set_trusted(trusted);
        app.manage(AppState::new(
            accessibility,
            Arc::new(FakeKeySender::new()),
            Arc::new(FakePasteboard::new()),
            workspace,
        ));
        app
    }

    fn remembered(app: &tauri::App<MockRuntime>) -> Option<SourceApp> {
        app.state::<AppState>()
            .last_active_app
            .lock()
            .unwrap()
            .clone()
    }

    #[test]
    fn showing_the_popover_remembers_the_app_that_was_in_front() {
        let app = app_with_state(true);
        app.manage(PopoverState::default());
        app.handle()
            .plugin(tauri_plugin_positioner::init())
            .unwrap();
        let popover = create_popover(&app).unwrap();

        popover.reveal();

        assert_eq!(remembered(&app), Some(textedit()));
    }

    #[test]
    fn preparing_the_popover_remembers_the_app_and_the_quick_panel_does_not() {
        let app = app_with_state(true);

        prepare_to_show(app.handle(), PasteOrigin::Panel);
        assert_eq!(remembered(&app), None);

        prepare_to_show(app.handle(), PasteOrigin::Popover);
        assert_eq!(remembered(&app), Some(textedit()));
    }

    #[test]
    fn every_show_checks_the_permission_and_announces_only_a_change() {
        use tauri::Listener;
        let app = app_with_state(false);
        let (sender, receiver) = std::sync::mpsc::channel();
        app.listen_any(ACCESSIBILITY_CHANGED, move |event| {
            sender.send(event.payload().to_owned()).unwrap();
        });

        prepare_to_show(app.handle(), PasteOrigin::Popover);
        prepare_to_show(app.handle(), PasteOrigin::Panel);

        let payload = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(payload, r#"{"trusted":false}"#);
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
    }

    #[test]
    fn preparing_a_show_without_the_app_state_does_not_panic() {
        let app = tauri::test::mock_app();

        prepare_to_show(app.handle(), PasteOrigin::Popover);
    }

    #[test]
    fn a_job_sent_to_the_main_thread_has_run_when_the_helper_returns() {
        let app = tauri::test::mock_app();
        let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = ran.clone();

        run_on_main_thread_and_wait(app.handle(), move || {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        assert!(ran.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn hiding_the_popover_works_with_or_without_its_window() {
        // `AppHandle` has its own `hide()` (the whole app): the trait method is called by path.
        // The Quick Panel is an `NSPanel`: hiding it is the manual QA of TAD-015.
        let (app, _popover) = app_with_popover();
        WindowHider::hide(app.handle(), PasteOrigin::Popover);

        let empty = tauri::test::mock_app();
        WindowHider::hide(empty.handle(), PasteOrigin::Popover);
    }

    /// A Retina laptop (1512 × 982 pt, scale 2) with a 1× external display on its right.
    fn laptop_and_external() -> (Rect, Rect) {
        (
            logical_rect((0, 0), (3024, 1964), 2.0),
            logical_rect((1512, 0), (1920, 1080), 1.0),
        )
    }

    #[test]
    fn a_monitor_rect_is_in_points_whatever_its_scale() {
        let (laptop, external) = laptop_and_external();

        assert_eq!(
            laptop,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 1512.0,
                height: 982.0
            }
        );
        assert_eq!(
            external,
            Rect {
                x: 1512.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0
            }
        );
    }

    #[test]
    fn the_pointer_is_converted_with_the_primary_monitors_scale() {
        let pointer = pointer_in_points((4800.0, 1000.0), 2.0);

        assert_eq!(
            pointer,
            Position {
                x: 2400.0,
                y: 500.0
            }
        );
    }

    #[test]
    fn the_panel_follows_the_pointer_onto_a_one_x_screen_beside_a_retina_one() {
        let (laptop, external) = laptop_and_external();
        let size = Size {
            width: PANEL_WIDTH,
            height: PANEL_HEIGHT,
        };

        let on_external = target_screen(
            pointer_in_points((4800.0, 1000.0), 2.0),
            &[laptop, external],
        );
        let on_laptop = target_screen(pointer_in_points((600.0, 300.0), 2.0), &[laptop, external]);

        assert_eq!(on_external, Some(external));
        assert_eq!(on_laptop, Some(laptop));
        assert_eq!(
            center_in(external, size),
            Position {
                x: 2152.0,
                y: 310.0
            }
        );
    }
}
