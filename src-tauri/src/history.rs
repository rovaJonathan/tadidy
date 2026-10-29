//! The actions that remove clips from the history. Each one tells the webview what changed
//! (`clips://changed`) once the database has committed.

use crate::error::AppError;
use crate::events::EventSink;
use crate::storage::Db;

/// Removes the clips with these ids (HIS-05) and announces the ones that existed.
pub fn delete_clips(db: &Db, sink: &impl EventSink, ids: &[String]) -> Result<(), AppError> {
    let _ = (db, sink, ids);
    todo!("TAD-012")
}

/// Empties the history (HIS-06), pinned clips spared when `keep_pinned`, and returns how many
/// clips went. The webview is told to reload.
pub fn clear_history(db: &Db, sink: &impl EventSink, keep_pinned: bool) -> Result<u32, AppError> {
    let _ = (db, sink, keep_pinned);
    todo!("TAD-012")
}
