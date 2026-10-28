//! Events the core emits to the webview (AGENTS.md §3). Services emit through the
//! `EventSink` trait so their tests use `RecordingEventSink` instead of a running app.

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
