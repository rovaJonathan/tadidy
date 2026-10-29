//! The actions that remove clips from the history. Each one tells the webview what changed
//! (`clips://changed`) once the database has committed.

use serde_json::json;

use crate::error::AppError;
use crate::events::{CLIPS_CHANGED, ClipsChanged, EventSink};
use crate::storage::{Db, clips};

/// Removes the clips with these ids (HIS-05) and announces the ones that existed.
pub fn delete_clips(db: &Db, sink: &impl EventSink, ids: &[String]) -> Result<(), AppError> {
    let removed = db.with_conn(|conn| clips::delete(conn, ids))?;
    if !removed.is_empty() {
        // Ids and counts only: no clip content in a log (AGENTS.md §2).
        tracing::info!(count = removed.len(), "clips deleted");
        sink.emit(
            CLIPS_CHANGED,
            json!(ClipsChanged {
                removed: Some(removed),
                ..ClipsChanged::default()
            }),
        );
    }
    Ok(())
}

/// Empties the history (HIS-06), pinned clips spared when `keep_pinned`, and returns how many
/// clips went. The webview is told to reload.
pub fn clear_history(db: &Db, sink: &impl EventSink, keep_pinned: bool) -> Result<u32, AppError> {
    let removed = db.with_conn(|conn| clips::clear(conn, keep_pinned))?;
    if removed > 0 {
        tracing::info!(count = removed, keep_pinned, "history cleared");
        sink.emit(CLIPS_CHANGED, json!(ClipsChanged::default()));
    }
    Ok(removed)
}
