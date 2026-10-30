//! The clips repository: plain functions over a connection, parameters only (AGENTS.md §13).
//! Errors carry codes, never a clip's text.

use rusqlite::{Connection, OptionalExtension, Row, params};
use uuid::Uuid;

use super::{StorageError, fts};
use crate::clip::{
    ClipQuery, ClipRecord, ClipSummary, ClipWithRepresentations, Cursor, DEFAULT_PAGE_SIZE,
    MAX_PAGE_SIZE, NewClip, Page,
};
use crate::platform::Representation;

macro_rules! clip_columns {
    () => {
        "id, kind, content_hash, plain_text, preview, title, source_bundle, source_name, \
         byte_size, width, height, pinned, pin_order, collection_id, copy_count, created_at, \
         last_copied_at, last_used_at"
    };
}

/// The newest-first list, first page. Walks `idx_clips_last_copied`, no sort.
pub const LIST_FIRST_PAGE_SQL: &str = concat!(
    "SELECT ",
    clip_columns!(),
    " FROM clips ORDER BY last_copied_at DESC, id DESC LIMIT ?1"
);

/// The pages after a cursor: the row-value comparison is a range on the same index.
pub const LIST_AFTER_SQL: &str = concat!(
    "SELECT ",
    clip_columns!(),
    " FROM clips WHERE (last_copied_at, id) < (?1, ?2) \
     ORDER BY last_copied_at DESC, id DESC LIMIT ?3"
);

/// What the list shows of a clip: no full text, no representations. Qualified, because the search
/// joins `clips_fts`.
macro_rules! summary_columns {
    () => {
        "clips.id, clips.kind, clips.preview, clips.source_bundle, clips.source_name, \
         clips.pinned, clips.copy_count, clips.last_copied_at"
    };
}

/// The summaries, newest first. Walks `idx_clips_last_copied`, no sort.
const SUMMARIES_SQL: &str = concat!(
    "SELECT ",
    summary_columns!(),
    " FROM clips ORDER BY clips.last_copied_at DESC, clips.id DESC LIMIT ?1"
);

const SUMMARIES_AFTER_SQL: &str = concat!(
    "SELECT ",
    summary_columns!(),
    " FROM clips WHERE (clips.last_copied_at, clips.id) < (?1, ?2) \
     ORDER BY clips.last_copied_at DESC, clips.id DESC LIMIT ?3"
);

/// The summaries of the clips the full-text index matches (`?1`), in the same order.
const SEARCH_SQL: &str = concat!(
    "SELECT ",
    summary_columns!(),
    " FROM clips_fts JOIN clips ON clips.rowid = clips_fts.rowid WHERE clips_fts MATCH ?1 \
     ORDER BY clips.last_copied_at DESC, clips.id DESC LIMIT ?2"
);

const SEARCH_AFTER_SQL: &str = concat!(
    "SELECT ",
    summary_columns!(),
    " FROM clips_fts JOIN clips ON clips.rowid = clips_fts.rowid WHERE clips_fts MATCH ?1 \
     AND (clips.last_copied_at, clips.id) < (?2, ?3) \
     ORDER BY clips.last_copied_at DESC, clips.id DESC LIMIT ?4"
);

/// The unpinned clips past the first `?1`, newest first: the ones the history limit removes.
pub const OLDEST_UNPINNED_SQL: &str = "SELECT id FROM clips INDEXED BY idx_clips_last_copied \
     WHERE pinned = 0 ORDER BY last_copied_at DESC, id DESC LIMIT -1 OFFSET ?1";

/// What [`insert_or_bump`] did, with the id of the clip concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted(String),
    Bumped(String),
}

fn record(row: &Row<'_>) -> rusqlite::Result<ClipRecord> {
    Ok(ClipRecord {
        id: row.get(0)?,
        kind: row.get(1)?,
        content_hash: row.get(2)?,
        plain_text: row.get(3)?,
        preview: row.get(4)?,
        title: row.get(5)?,
        source_bundle: row.get(6)?,
        source_name: row.get(7)?,
        byte_size: row.get(8)?,
        width: row.get(9)?,
        height: row.get(10)?,
        pinned: row.get(11)?,
        pin_order: row.get(12)?,
        collection_id: row.get(13)?,
        copy_count: row.get(14)?,
        created_at: row.get(15)?,
        last_copied_at: row.get(16)?,
        last_used_at: row.get(17)?,
    })
}

fn summary(row: &Row<'_>) -> rusqlite::Result<ClipSummary> {
    Ok(ClipSummary {
        id: row.get(0)?,
        kind: row.get(1)?,
        preview: row.get(2)?,
        source_bundle: row.get(3)?,
        source_name: row.get(4)?,
        pinned: row.get(5)?,
        copy_count: row.get(6)?,
        last_copied_at: row.get(7)?,
    })
}

/// Records a copy. New content becomes a clip with its representations; content already stored
/// (same hash) is bumped: `last_copied_at = now`, `copy_count + 1`, and the source app is the
/// latest one that is known (CAP-09).
pub fn insert_or_bump(conn: &Connection, clip: &NewClip) -> Result<InsertOutcome, StorageError> {
    let tx = conn.unchecked_transaction()?;
    // The FTS update trigger fires only when an indexed column really changes, so a bump from
    // the same app does not rewrite the index.
    let (id, copy_count): (String, i64) = tx.query_row(
        "INSERT INTO clips (id, kind, content_hash, plain_text, preview, source_bundle, \
             source_name, byte_size, created_at, last_copied_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT (content_hash) DO UPDATE SET
             last_copied_at = excluded.last_copied_at,
             copy_count = copy_count + 1,
             source_bundle = coalesce(excluded.source_bundle, source_bundle),
             source_name = CASE WHEN excluded.source_bundle IS NULL
                 THEN source_name ELSE excluded.source_name END
         RETURNING id, copy_count",
        params![
            Uuid::now_v7().to_string(),
            clip.kind,
            clip.content_hash,
            clip.plain_text,
            clip.preview,
            clip.source_bundle,
            clip.source_name,
            clip.byte_size,
            clip.now,
        ],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if copy_count > 1 {
        tx.commit()?;
        return Ok(InsertOutcome::Bumped(id));
    }
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO representations (clip_id, uti, data) VALUES (?1, ?2, ?3)",
        )?;
        for representation in &clip.representations {
            stmt.execute(params![id, representation.uti, representation.data])?;
        }
    }
    tx.commit()?;
    Ok(InsertOutcome::Inserted(id))
}

/// Moves a clip to the top without counting a copy: Tadidy's own writes (CAP-12). `None` when
/// no clip has this hash.
pub fn move_to_top(
    conn: &Connection,
    content_hash: &str,
    now: i64,
) -> Result<Option<String>, StorageError> {
    Ok(conn
        .query_row(
            "UPDATE clips SET last_copied_at = ?2 WHERE content_hash = ?1 RETURNING id",
            params![content_hash, now],
            |row| row.get(0),
        )
        .optional()?)
}

/// Records that a clip was pasted or copied back by Tadidy. `false` when no clip has this id.
pub fn mark_used(conn: &Connection, id: &str, now: i64) -> Result<bool, StorageError> {
    let updated = conn
        .prepare_cached("UPDATE clips SET last_used_at = ?2 WHERE id = ?1")?
        .execute(params![id, now])?;
    Ok(updated > 0)
}

/// Up to `limit` clips, newest first, after `cursor` (from the start when `None`).
pub fn list_recent(
    conn: &Connection,
    cursor: Option<&Cursor>,
    limit: u32,
) -> Result<Vec<ClipRecord>, StorageError> {
    let rows = match cursor {
        None => conn
            .prepare_cached(LIST_FIRST_PAGE_SQL)?
            .query_map(params![limit], record)?
            .collect::<Result<_, _>>()?,
        Some(cursor) => conn
            .prepare_cached(LIST_AFTER_SQL)?
            .query_map(params![cursor.last_copied_at, cursor.id, limit], record)?
            .collect::<Result<_, _>>()?,
    };
    Ok(rows)
}

/// One page of the list `query` asks for, newest first: every clip, or the ones matching its
/// search. `total` counts all of them, not only this page.
pub fn list(conn: &Connection, query: &ClipQuery) -> Result<Page<ClipSummary>, StorageError> {
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| Cursor::decode(value).ok_or(StorageError::InvalidCursor))
        .transpose()?;
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let search = query.search.as_deref().and_then(fts::match_query);

    // One row more than asked for tells whether a next page exists.
    let mut items = summaries(conn, search.as_deref(), cursor.as_ref(), limit + 1)?;
    let next_cursor = (items.len() > limit as usize).then(|| {
        items.truncate(limit as usize);
        items.last().map(|clip| clip.cursor().encode())
    });
    Ok(Page {
        items,
        next_cursor: next_cursor.flatten(),
        total: count_matching(conn, search.as_deref())?,
    })
}

fn summaries(
    conn: &Connection,
    search: Option<&str>,
    cursor: Option<&Cursor>,
    limit: u32,
) -> Result<Vec<ClipSummary>, StorageError> {
    let rows = match (search, cursor) {
        (None, None) => conn
            .prepare_cached(SUMMARIES_SQL)?
            .query_map(params![limit], summary)?
            .collect::<Result<_, _>>()?,
        (None, Some(cursor)) => conn
            .prepare_cached(SUMMARIES_AFTER_SQL)?
            .query_map(params![cursor.last_copied_at, cursor.id, limit], summary)?
            .collect::<Result<_, _>>()?,
        (Some(search), None) => conn
            .prepare_cached(SEARCH_SQL)?
            .query_map(params![search, limit], summary)?
            .collect::<Result<_, _>>()?,
        (Some(search), Some(cursor)) => conn
            .prepare_cached(SEARCH_AFTER_SQL)?
            .query_map(
                params![search, cursor.last_copied_at, cursor.id, limit],
                summary,
            )?
            .collect::<Result<_, _>>()?,
    };
    Ok(rows)
}

/// How many clips the list holds for this search, whatever the page.
fn count_matching(conn: &Connection, search: Option<&str>) -> Result<u32, StorageError> {
    Ok(match search {
        None => conn
            .prepare_cached("SELECT count(*) FROM clips")?
            .query_row([], |row| row.get(0))?,
        Some(search) => conn
            .prepare_cached("SELECT count(*) FROM clips_fts WHERE clips_fts MATCH ?1")?
            .query_row([search], |row| row.get(0))?,
    })
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<ClipWithRepresentations>, StorageError> {
    let sql = concat!("SELECT ", clip_columns!(), " FROM clips WHERE id = ?1");
    let Some(clip) = conn.query_row(sql, [id], record).optional()? else {
        return Ok(None);
    };
    let representations = conn
        .prepare_cached("SELECT uti, data FROM representations WHERE clip_id = ?1 ORDER BY rowid")?
        .query_map([id], |row| {
            Ok(Representation::new(
                row.get::<_, String>(0)?,
                row.get::<_, Vec<u8>>(1)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(Some(ClipWithRepresentations {
        clip,
        representations,
    }))
}

/// Removes the clips (their representations and thumbnail go with them) and returns the ids that
/// existed.
pub fn delete(conn: &Connection, ids: &[String]) -> Result<Vec<String>, StorageError> {
    let removed = remove(conn, ids)?;
    reclaim_space(conn, removed.len())?;
    Ok(removed)
}

/// Empties the history (HIS-06); pinned clips survive when `keep_pinned`. Returns the number of
/// clips removed.
pub fn clear(conn: &Connection, keep_pinned: bool) -> Result<u32, StorageError> {
    let sql = if keep_pinned {
        "DELETE FROM clips WHERE pinned = 0"
    } else {
        "DELETE FROM clips"
    };
    let removed = conn.execute(sql, [])?;
    reclaim_space(conn, removed)?;
    Ok(u32::try_from(removed).unwrap_or(u32::MAX))
}

/// Keeps the `max` newest unpinned clips and removes the rest (HIS-02); pinned clips and
/// snippets do not count. Returns the ids removed; `None` keeps everything.
pub fn enforce_limit(conn: &Connection, max: Option<u32>) -> Result<Vec<String>, StorageError> {
    let Some(max) = max else {
        return Ok(Vec::new());
    };
    // Counting runs on `idx_clips_recent` alone, so the usual case (under the limit) reads no row.
    let unpinned: u32 = conn
        .prepare_cached("SELECT count(*) FROM clips WHERE pinned = 0")?
        .query_row([], |row| row.get(0))?;
    if unpinned <= max {
        return Ok(Vec::new());
    }
    let excess: Vec<String> = conn
        .prepare_cached(OLDEST_UNPINNED_SQL)?
        .query_map([max], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    delete(conn, &excess)
}

pub fn count(conn: &Connection) -> Result<u32, StorageError> {
    Ok(conn.query_row("SELECT count(*) FROM clips", [], |row| row.get(0))?)
}

fn remove(conn: &Connection, ids: &[String]) -> Result<Vec<String>, StorageError> {
    let tx = conn.unchecked_transaction()?;
    let mut removed = Vec::new();
    {
        let mut stmt = tx.prepare("DELETE FROM clips WHERE id = ?1")?;
        for id in ids {
            if stmt.execute([id])? > 0 {
                removed.push(id.clone());
            }
        }
    }
    tx.commit()?;
    Ok(removed)
}

/// Gives the pages freed by a removal back to the file. Never a full `VACUUM`: it may renumber
/// the rowids `clips_fts` is keyed on.
pub fn incremental_vacuum(conn: &Connection) -> Result<(), StorageError> {
    // Each step frees one page and reports a row: the statement has to be run to its end.
    let mut stmt = conn.prepare("PRAGMA incremental_vacuum")?;
    let mut rows = stmt.query([])?;
    while rows.next()?.is_some() {}
    Ok(())
}

fn reclaim_space(conn: &Connection, removed: usize) -> Result<(), StorageError> {
    if removed > 0 {
        incremental_vacuum(conn)?;
    }
    Ok(())
}
