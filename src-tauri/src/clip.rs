//! The domain types of a clip: its kind, what the watcher hands to the repository, the rows the
//! repository returns, and the history limit. Clip content never reaches a log or a test failure
//! message: the types that hold it print their sizes, not their text (AGENTS.md §2).

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::platform::Representation;

/// Number of clips kept by default (HIS-02).
pub const DEFAULT_HISTORY_SIZE: u32 = 500;

/// What a clip is, the value of the `kind` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ClipKind {
    Text,
    Rich,
    Image,
    File,
    Url,
    Color,
}

impl ClipKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Rich => "rich",
            Self::Image => "image",
            Self::File => "file",
            Self::Url => "url",
            Self::Color => "color",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "text" => Self::Text,
            "rich" => Self::Rich,
            "image" => Self::Image,
            "file" => Self::File,
            "url" => Self::Url,
            "color" => Self::Color,
            _ => return None,
        })
    }
}

impl ToSql for ClipKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}

impl FromSql for ClipKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Self::parse(value.as_str()?).ok_or(FromSqlError::InvalidType)
    }
}

/// SHA-256 of a clip's primary representation, lowercase hex: the identity used to deduplicate
/// (CAP-09).
pub fn content_hash(primary: &[u8]) -> String {
    Sha256::digest(primary)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A clip as the watcher captured it, ready to be stored.
#[derive(Clone)]
pub struct NewClip {
    pub kind: ClipKind,
    pub content_hash: String,
    pub plain_text: Option<String>,
    pub preview: Option<String>,
    pub source_bundle: Option<String>,
    pub source_name: Option<String>,
    pub byte_size: i64,
    pub representations: Vec<Representation>,
    /// Milliseconds since the Unix epoch.
    pub now: i64,
}

// Printed by hand: a derived `Debug` would put the clip's text in a log or a test failure.
impl fmt::Debug for NewClip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewClip")
            .field("kind", &self.kind)
            .field("byte_size", &self.byte_size)
            .field("representations", &self.representations)
            .field("now", &self.now)
            .finish_non_exhaustive()
    }
}

/// One row of `clips`.
#[derive(Clone, PartialEq)]
pub struct ClipRecord {
    pub id: String,
    pub kind: ClipKind,
    pub content_hash: String,
    pub plain_text: Option<String>,
    pub preview: Option<String>,
    pub title: Option<String>,
    pub source_bundle: Option<String>,
    pub source_name: Option<String>,
    pub byte_size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub pinned: bool,
    pub pin_order: Option<f64>,
    pub collection_id: Option<String>,
    pub copy_count: i64,
    pub created_at: i64,
    pub last_copied_at: i64,
    pub last_used_at: Option<i64>,
}

impl ClipRecord {
    /// Where to resume `list_recent` after this row.
    pub fn cursor(&self) -> Cursor {
        Cursor {
            last_copied_at: self.last_copied_at,
            id: self.id.clone(),
        }
    }
}

// Printed by hand, like [`NewClip`]: ids, kind and sizes only.
impl fmt::Debug for ClipRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipRecord")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("byte_size", &self.byte_size)
            .field("pinned", &self.pinned)
            .field("copy_count", &self.copy_count)
            .field("last_copied_at", &self.last_copied_at)
            .finish_non_exhaustive()
    }
}

/// Page size when the front does not ask for one, and the most it can ask for.
pub const DEFAULT_PAGE_SIZE: u32 = 50;
pub const MAX_PAGE_SIZE: u32 = 200;

/// What `list_clips` is asked for. The filters of the spec that come later (kinds, app, tab,
/// collection) join as optional fields.
#[derive(Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClipQuery {
    /// What the user typed in the search field; blank means no search.
    #[ts(optional)]
    pub search: Option<String>,
    /// The `next_cursor` of the previous page; none for the first page.
    #[ts(optional)]
    pub cursor: Option<String>,
    /// Page size: 50 by default, 200 at most.
    #[ts(optional)]
    pub limit: Option<u32>,
}

// Printed by hand: the search text is never logged, only its length (AGENTS.md §2).
impl fmt::Debug for ClipQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipQuery")
            .field("search_len", &self.search.as_ref().map(String::len))
            .field("cursor", &self.cursor)
            .field("limit", &self.limit)
            .finish()
    }
}

/// A clip as the list shows it: no representations, no full text.
#[derive(Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClipSummary {
    pub id: String,
    pub kind: ClipKind,
    /// The text shown on two lines; none for an image.
    pub preview: Option<String>,
    pub source_bundle: Option<String>,
    pub source_name: Option<String>,
    pub pinned: bool,
    #[ts(type = "number")]
    pub copy_count: i64,
    /// Milliseconds since the Unix epoch.
    #[ts(type = "number")]
    pub last_copied_at: i64,
}

impl ClipSummary {
    /// Where to resume the list after this row.
    pub fn cursor(&self) -> Cursor {
        Cursor {
            last_copied_at: self.last_copied_at,
            id: self.id.clone(),
        }
    }
}

// Printed by hand, like [`ClipRecord`]: the preview is clip content.
impl fmt::Debug for ClipSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipSummary")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("pinned", &self.pinned)
            .field("copy_count", &self.copy_count)
            .field("last_copied_at", &self.last_copied_at)
            .finish_non_exhaustive()
    }
}

/// One page of a list.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    /// What to hand back as `cursor` to get the next page; none on the last one.
    pub next_cursor: Option<String>,
    /// How many clips match the query in all, not only on this page.
    pub total: u32,
}

/// A clip and every data format stored for it.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipWithRepresentations {
    pub clip: ClipRecord,
    pub representations: Vec<Representation>,
}

/// A position in the newest-first list: the sort key of the last row already served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub last_copied_at: i64,
    pub id: String,
}

impl Cursor {
    /// An opaque string for the front to hand back.
    pub fn encode(&self) -> String {
        format!("{}:{}", self.last_copied_at, self.id)
    }

    pub fn decode(value: &str) -> Option<Self> {
        let (last_copied_at, id) = value.split_once(':')?;
        if id.is_empty() {
            return None;
        }
        Some(Self {
            last_copied_at: last_copied_at.parse().ok()?,
            id: id.to_owned(),
        })
    }
}

/// How many clips the history keeps; one instance is shared by the settings and the purger.
/// `0` is stored for "unlimited".
#[derive(Debug, Clone)]
pub struct HistoryLimit(Arc<AtomicU32>);

impl HistoryLimit {
    pub fn get(&self) -> Option<u32> {
        match self.0.load(Ordering::Relaxed) {
            0 => None,
            max => Some(max),
        }
    }

    pub fn set(&self, max: Option<u32>) {
        self.0.store(max.unwrap_or(0), Ordering::Relaxed);
    }
}

impl Default for HistoryLimit {
    fn default() -> Self {
        Self(Arc::new(AtomicU32::new(DEFAULT_HISTORY_SIZE)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_the_lowercase_hex_sha256() {
        assert_eq!(
            content_hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn content_hash_of_nothing_is_the_empty_digest() {
        assert_eq!(
            content_hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn a_cursor_survives_encode_and_decode() {
        let cursor = Cursor {
            last_copied_at: 1_761_700_000_123,
            id: "0192f3a4-7b1c-7d2e-8f00-123456789abc".to_owned(),
        };

        assert_eq!(Cursor::decode(&cursor.encode()), Some(cursor));
    }

    #[test]
    fn decoding_an_invalid_cursor_gives_none() {
        for input in ["", "abc", "12", "12:", ":id", "x:id"] {
            assert_eq!(Cursor::decode(input), None, "{input:?}");
        }
    }

    #[test]
    fn the_history_limit_defaults_to_500_clips() {
        assert_eq!(DEFAULT_HISTORY_SIZE, 500);
        assert_eq!(HistoryLimit::default().get(), Some(500));
    }

    #[test]
    fn the_history_limit_can_be_set_and_made_unlimited() {
        let limit = HistoryLimit::default();

        limit.set(Some(10));
        assert_eq!(limit.get(), Some(10));

        limit.set(None);
        assert_eq!(limit.get(), None);
    }

    #[test]
    fn a_stored_zero_means_unlimited() {
        let limit = HistoryLimit::default();

        limit.set(Some(0));

        assert_eq!(limit.get(), None);
    }

    #[test]
    fn clones_of_the_history_limit_share_one_value() {
        let limit = HistoryLimit::default();
        let other = limit.clone();

        other.set(Some(25));

        assert_eq!(limit.get(), Some(25));
    }

    #[test]
    fn clip_kinds_round_trip_through_their_column_value() {
        for (kind, text) in [
            (ClipKind::Text, "text"),
            (ClipKind::Rich, "rich"),
            (ClipKind::Image, "image"),
            (ClipKind::File, "file"),
            (ClipKind::Url, "url"),
            (ClipKind::Color, "color"),
        ] {
            assert_eq!(kind.as_str(), text);
            assert_eq!(ClipKind::parse(text), Some(kind));
            assert_eq!(serde_json::to_value(kind).unwrap(), text);
        }
        assert_eq!(ClipKind::parse("bogus"), None);
    }

    fn summary(preview: &str) -> ClipSummary {
        ClipSummary {
            id: "0192f3a4-7b1c-7d2e-8f00-123456789abc".to_owned(),
            kind: ClipKind::Text,
            preview: Some(preview.to_owned()),
            source_bundle: Some("com.example.Editor".to_owned()),
            source_name: None,
            pinned: false,
            copy_count: 2,
            last_copied_at: 1_761_700_000_123,
        }
    }

    #[test]
    fn a_query_deserializes_from_the_camel_case_the_front_sends() {
        let query: ClipQuery =
            serde_json::from_str(r#"{ "search": "sel", "cursor": "1:a", "limit": 20 }"#).unwrap();

        assert_eq!(query.search.as_deref(), Some("sel"));
        assert_eq!(query.cursor.as_deref(), Some("1:a"));
        assert_eq!(query.limit, Some(20));
    }

    #[test]
    fn every_field_of_a_query_is_optional() {
        let query: ClipQuery = serde_json::from_str("{}").unwrap();

        assert_eq!(query.search, None);
        assert_eq!(query.cursor, None);
        assert_eq!(query.limit, None);
    }

    #[test]
    fn a_page_of_summaries_serializes_to_camel_case() {
        let page = Page {
            items: vec![summary("hello")],
            next_cursor: None,
            total: 1,
        };

        assert_eq!(
            serde_json::to_value(page).unwrap(),
            serde_json::json!({
                "items": [{
                    "id": "0192f3a4-7b1c-7d2e-8f00-123456789abc",
                    "kind": "text",
                    "preview": "hello",
                    "sourceBundle": "com.example.Editor",
                    "sourceName": null,
                    "pinned": false,
                    "copyCount": 2,
                    "lastCopiedAt": 1_761_700_000_123_i64,
                }],
                "nextCursor": null,
                "total": 1,
            })
        );
    }

    #[test]
    fn a_summary_points_to_the_place_after_it_in_the_list() {
        assert_eq!(
            summary("hello").cursor(),
            Cursor {
                last_copied_at: 1_761_700_000_123,
                id: "0192f3a4-7b1c-7d2e-8f00-123456789abc".to_owned(),
            }
        );
    }

    #[test]
    fn debug_output_of_a_query_and_a_summary_holds_no_text() {
        let query = ClipQuery {
            search: Some("my secret search".to_owned()),
            ..ClipQuery::default()
        };

        let printed = format!("{query:?} {:?}", summary("my secret preview"));

        assert!(!printed.contains("secret"), "{printed}");
        assert!(printed.contains("search_len: Some(16)"), "{printed}");
    }
}
