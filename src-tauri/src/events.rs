//! Events the core emits to the webview (AGENTS.md §3). Services emit through the
//! `EventSink` trait so their tests use `RecordingEventSink` instead of a running app.

use std::sync::{Mutex, PoisonError};

use serde_json::Value;
use tauri::{AppHandle, Emitter};

pub const CLIPS_CHANGED: &str = "clips://changed";
pub const SETTINGS_CHANGED: &str = "settings://changed";
pub const ACCESSIBILITY_CHANGED: &str = "accessibility://changed";
pub const PANEL_SHOWN: &str = "panel://shown";

pub trait EventSink: Send + Sync {
    fn emit(&self, event: &'static str, payload: Value);
}

/// Emits to every webview of the running app.
pub struct TauriEventSink {
    app: AppHandle,
}

impl TauriEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
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
    use serde_json::json;
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
}
