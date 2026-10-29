//! Events the core emits to the webview (AGENTS.md §3). Services emit through the
//! `EventSink` trait so their tests use `RecordingEventSink` instead of a running app.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Runtime, Wry};
use ts_rs::TS;

pub const CLIPS_CHANGED: &str = "clips://changed";
pub const SETTINGS_CHANGED: &str = "settings://changed";
pub const ACCESSIBILITY_CHANGED: &str = "accessibility://changed";
pub const PANEL_SHOWN: &str = "panel://shown";

pub trait EventSink: Send + Sync {
    fn emit(&self, event: &'static str, payload: Value);
}

/// Payload of `panel://shown`: which window was just shown, by label (`popover` or `panel`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PanelShown {
    pub window: String,
}

/// Payload of `clips://changed`: what changed in the history. An all-empty payload means
/// "reload the list".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClipsChanged {
    /// Id of a clip that was just recorded.
    pub added: Option<String>,
    /// Id of a clip that moved to the top or changed.
    pub updated: Option<String>,
    pub removed: Option<Vec<String>>,
}

/// Tells the webview that the popover or the Quick Panel was shown, so it can focus its
/// search field and reset its selection.
pub fn emit_panel_shown(sink: &impl EventSink, window: &str) {
    let payload = PanelShown {
        window: window.to_owned(),
    };
    sink.emit(PANEL_SHOWN, json!(payload));
}

/// Emits to every webview of the running app. Generic over the runtime so tests use
/// Tauri's mock runtime.
pub struct TauriEventSink<R: Runtime = Wry> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriEventSink<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> EventSink for TauriEventSink<R> {
    fn emit(&self, event: &'static str, payload: Value) {
        // The payload is never logged: it may describe a clip.
        if let Err(error) = self.app.emit(event, payload) {
            tracing::warn!(event, %error, "failed to emit event");
        }
    }
}

/// Test fake: keeps every emitted event, in order.
#[derive(Default)]
pub struct RecordingEventSink {
    events: Mutex<Vec<(&'static str, Value)>>,
}

impl RecordingEventSink {
    pub fn events(&self) -> Vec<(&'static str, Value)> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl EventSink for RecordingEventSink {
    fn emit(&self, event: &'static str, payload: Value) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((event, payload));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Listener;

    #[test]
    fn recording_sink_keeps_events_in_order() {
        let sink = RecordingEventSink::default();

        sink.emit(CLIPS_CHANGED, json!({ "added": "0192" }));
        sink.emit(SETTINGS_CHANGED, json!(null));

        assert_eq!(
            sink.events(),
            vec![
                (CLIPS_CHANGED, json!({ "added": "0192" })),
                (SETTINGS_CHANGED, json!(null)),
            ]
        );
    }

    #[test]
    fn tauri_sink_emits_to_the_app_listeners() {
        let app = tauri::test::mock_app();
        let (sender, receiver) = std::sync::mpsc::channel();
        app.listen_any(CLIPS_CHANGED, move |event| {
            sender.send(event.payload().to_owned()).unwrap();
        });

        TauriEventSink::new(app.handle().clone()).emit(CLIPS_CHANGED, json!({ "added": "0192" }));

        let payload = receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(payload, r#"{"added":"0192"}"#);
    }

    #[test]
    fn event_names_match_the_contract() {
        assert_eq!(
            [
                CLIPS_CHANGED,
                SETTINGS_CHANGED,
                ACCESSIBILITY_CHANGED,
                PANEL_SHOWN
            ],
            [
                "clips://changed",
                "settings://changed",
                "accessibility://changed",
                "panel://shown"
            ]
        );
    }

    #[test]
    fn panel_shown_carries_the_window_label() {
        let sink = RecordingEventSink::default();

        emit_panel_shown(&sink, "panel");

        assert_eq!(
            sink.events(),
            vec![(PANEL_SHOWN, json!({ "window": "panel" }))]
        );
    }

    #[test]
    fn clips_changed_serializes_to_camel_case_with_nothing_set_by_default() {
        let changed = ClipsChanged {
            added: Some("0192".to_owned()),
            removed: Some(vec!["a".to_owned(), "b".to_owned()]),
            ..ClipsChanged::default()
        };

        assert_eq!(
            json!(changed),
            json!({ "added": "0192", "updated": null, "removed": ["a", "b"] })
        );
        assert_eq!(
            json!(ClipsChanged::default()),
            json!({ "added": null, "updated": null, "removed": null })
        );
    }
}
