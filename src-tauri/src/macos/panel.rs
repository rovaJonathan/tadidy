//! The Quick Panel as a non-activating `NSPanel` (QPN-02), through `tauri-nspanel`. It takes
//! keyboard input while the app the user was typing in stays active, so `⌘V` lands in the
//! right window. Needs a real window server, so it is covered by the manual QA of TAD-006.

// The `panel_event!` macro requires the explicit `-> ()` of the delegate callbacks.
#![allow(clippy::unused_unit)]

use tauri::{AppHandle, LogicalSize, Manager, Runtime, Size, WebviewUrl, WebviewWindow};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask, tauri_panel,
};
use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};

use crate::events::{TauriEventSink, emit_panel_shown};
use crate::windows::{self, PANEL_HEIGHT, PANEL_LABEL, PANEL_WIDTH, QuickPanel};

/// `--radius-window` in points.
const PANEL_RADIUS: f64 = 10.0;

tauri_panel! {
    panel!(QuickPanelClass {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            is_floating_panel: true
        }
    })

    panel_event!(QuickPanelEvents {
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

/// Creates the Quick Panel hidden, as a non-activating floating panel on every Space and
/// above full-screen apps, with the HUD material behind the webview. It hides when it stops
/// being the key window (a click elsewhere).
pub fn create(app: &AppHandle) -> anyhow::Result<()> {
    let panel = PanelBuilder::<_, QuickPanelClass>::new(app, PANEL_LABEL)
        .url(WebviewUrl::App("panel.html".into()))
        .size(Size::Logical(LogicalSize::new(PANEL_WIDTH, PANEL_HEIGHT)))
        .level(PanelLevel::Floating)
        // Set before the first show: an activating panel would deactivate the target app.
        .add_style_mask(StyleMask::empty().nonactivating_panel())
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary(),
        )
        .hides_on_deactivate(false)
        .with_window(|window| {
            window
                .decorations(false)
                .transparent(true)
                .skip_taskbar(true)
                .resizable(false)
                .visible(false)
        })
        .build()?;

    let events = QuickPanelEvents::new();
    let handle = app.clone();
    events.window_did_resign_key(move |_notification| {
        NsQuickPanel::new(handle.clone()).conceal();
    });
    panel.set_event_handler(Some(events.as_ref()));

    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        apply_hud_material(&window)?;
    }
    Ok(())
}

fn apply_hud_material<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), window_vibrancy::Error> {
    apply_vibrancy(
        window,
        NSVisualEffectMaterial::HudWindow,
        Some(NSVisualEffectState::Active),
        Some(PANEL_RADIUS),
    )
}

/// The real Quick Panel, behind the `QuickPanel` trait the toggle logic works with.
pub struct NsQuickPanel<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> NsQuickPanel<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> QuickPanel for NsQuickPanel<R> {
    fn is_shown(&self) -> bool {
        self.app
            .get_webview_panel(PANEL_LABEL)
            .is_ok_and(|panel| panel.is_visible())
    }

    fn reveal(&self) {
        let (Ok(panel), Some(window)) = (
            self.app.get_webview_panel(PANEL_LABEL),
            self.app.get_webview_window(PANEL_LABEL),
        ) else {
            tracing::error!("the Quick Panel is missing");
            return;
        };
        if let Err(error) = windows::center_on_pointer_screen(&self.app, &window) {
            tracing::warn!(%error, "could not center the Quick Panel");
        }
        // Never `set_focus()`: it activates the app and deactivates the target.
        panel.show_and_make_key();
        emit_panel_shown(&TauriEventSink::new(self.app.clone()), PANEL_LABEL);
    }

    fn conceal(&self) {
        match self.app.get_webview_panel(PANEL_LABEL) {
            Ok(panel) => panel.hide(),
            Err(_) => tracing::error!("the Quick Panel is missing"),
        }
    }
}
