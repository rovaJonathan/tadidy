//! `list_clips` and its siblings against a real SQLCipher database in a temporary folder, with a
//! fixed test key. Never the Keychain, never the user's data folder (AGENTS.md §13). Every text
//! below is a synthetic string.

// `allow-unwrap-in-tests` covers the `#[test]` functions, not the helpers around them.
#![allow(clippy::unwrap_used)]

use tadidy_lib::clip::{ClipKind, ClipQuery, ClipSummary, NewClip, Page, content_hash};
use tadidy_lib::error::AppError;
use tadidy_lib::platform::{Representation, UTI_PLAIN_TEXT};
use tadidy_lib::storage::clips::{InsertOutcome, insert_or_bump, list};
use tadidy_lib::storage::{Db, StorageError, open};
use tempfile::TempDir;

const KEY: [u8; 32] = [0xA; 32];

fn open_db() -> (TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = open(
        &dir.path().join("test.db"),
        &tadidy_lib::crypto::DbKey::from_bytes(KEY),
    )
    .unwrap();
    (dir, db)
}

fn text_clip(text: &str, now: i64) -> NewClip {
    NewClip {
        kind: ClipKind::Text,
        content_hash: content_hash(text.as_bytes()),
        plain_text: Some(text.to_owned()),
        preview: Some(text.to_owned()),
        source_bundle: Some("com.example.Editor".to_owned()),
        source_name: Some("Editor".to_owned()),
        byte_size: text.len() as i64,
        representations: vec![Representation::new(UTI_PLAIN_TEXT, text.as_bytes())],
        now,
    }
}

/// Stores `text` as a clip copied at `now`, and returns its id.
fn add(db: &Db, text: &str, now: i64) -> String {
    match db
        .with_conn(|conn| insert_or_bump(conn, &text_clip(text, now)))
        .unwrap()
    {
        InsertOutcome::Inserted(id) | InsertOutcome::Bumped(id) => id,
    }
}

fn search(text: &str) -> ClipQuery {
    ClipQuery {
        search: Some(text.to_owned()),
        ..ClipQuery::default()
    }
}

fn list_all(db: &Db, query: &ClipQuery) -> Page<ClipSummary> {
    db.with_conn(|conn| list(conn, query)).unwrap()
}

fn ids(page: &Page<ClipSummary>) -> Vec<&str> {
    page.items.iter().map(|clip| clip.id.as_str()).collect()
}

#[test]
fn a_search_matches_the_start_of_words_newest_first() {
    let (_dir, db) = open_db();
    let select = add(&db, "SELECT * FROM users", 1_000);
    add(&db, "vessel", 2_000);
    let selected = add(&db, "Selected items", 3_000);

    let page = list_all(&db, &search("sel"));

    assert_eq!(ids(&page), [selected.as_str(), select.as_str()]);
    assert_eq!(page.total, 2);
    assert_eq!(page.next_cursor, None);
}

#[test]
fn a_search_ignores_diacritics_on_both_sides() {
    let (_dir, db) = open_db();
    let id = add(&db, "Crème brûlée", 1_000);
    add(&db, "Something else", 2_000);

    assert_eq!(ids(&list_all(&db, &search("creme brul"))), [id.as_str()]);
    assert_eq!(ids(&list_all(&db, &search("CRÈME"))), [id.as_str()]);
}

#[test]
fn every_word_of_a_search_must_match() {
    let (_dir, db) = open_db();
    let both = add(&db, "red apple pie", 1_000);
    add(&db, "red cherry", 2_000);

    let page = list_all(&db, &search("pie red"));

    assert_eq!(ids(&page), [both.as_str()]);
    assert_eq!(page.total, 1);
}

#[test]
fn the_source_app_name_is_searched_too() {
    let (_dir, db) = open_db();
    let id = add(&db, "no hint in the text", 1_000);

    assert_eq!(ids(&list_all(&db, &search("editor"))), [id.as_str()]);
}

#[test]
fn a_search_with_fts5_syntax_is_not_an_error() {
    let (_dir, db) = open_db();
    add(&db, "near the end", 1_000);

    for input in [
        r#"""#,
        r#""unterminated"#,
        "*",
        "sel*",
        "(",
        "a (b",
        "-",
        "-foo",
        "NEAR",
        "NEAR(a b)",
        "a AND",
        "OR OR",
        "col:val",
        "^x",
        "{x}",
    ] {
        let result = db.with_conn(|conn| list(conn, &search(input)));

        assert!(result.is_ok(), "{input:?}: {result:?}");
    }
}

#[test]
fn a_search_with_nothing_to_search_lists_everything() {
    let (_dir, db) = open_db();
    add(&db, "one", 1_000);
    add(&db, "two", 2_000);

    for input in ["", "   ", "***", "()"] {
        let page = list_all(&db, &search(input));

        assert_eq!(page.items.len(), 2, "{input:?}");
        assert_eq!(page.total, 2, "{input:?}");
    }
}

#[test]
fn a_search_that_matches_nothing_gives_an_empty_page() {
    let (_dir, db) = open_db();
    add(&db, "one", 1_000);

    let page = list_all(&db, &search("zzz"));

    assert_eq!(page.items, []);
    assert_eq!(page.next_cursor, None);
    assert_eq!(page.total, 0);
}

#[test]
fn an_empty_history_gives_an_empty_page() {
    let (_dir, db) = open_db();

    let page = list_all(&db, &ClipQuery::default());

    assert_eq!(page.items, []);
    assert_eq!(page.next_cursor, None);
    assert_eq!(page.total, 0);
}

#[test]
fn a_summary_carries_what_the_list_shows() {
    let (_dir, db) = open_db();
    let id = add(&db, "hello", 1_234);
    add(&db, "hello", 5_678);

    let page = list_all(&db, &ClipQuery::default());

    assert_eq!(
        page.items,
        [ClipSummary {
            id,
            kind: ClipKind::Text,
            preview: Some("hello".to_owned()),
            source_bundle: Some("com.example.Editor".to_owned()),
            source_name: Some("Editor".to_owned()),
            pinned: false,
            copy_count: 2,
            last_copied_at: 5_678,
        }]
    );
}

fn seed(db: &Db, count: i64) {
    for n in 0..count {
        add(db, &format!("clip number {n}"), 1_000 + n);
    }
}

/// Reads every page of `query` and returns the sizes, the ids in order and the totals seen.
fn walk(db: &Db, query: &ClipQuery) -> (Vec<usize>, Vec<String>, Vec<u32>) {
    let (mut sizes, mut seen, mut totals) = (Vec::new(), Vec::new(), Vec::new());
    let mut query = query.clone();
    loop {
        let page = list_all(db, &query);
        sizes.push(page.items.len());
        seen.extend(page.items.iter().map(|clip| clip.id.clone()));
        totals.push(page.total);
        match page.next_cursor {
            Some(cursor) => query.cursor = Some(cursor),
            None => return (sizes, seen, totals),
        }
    }
}

#[test]
fn pages_of_50_cover_120_clips_without_duplicates_or_gaps() {
    let (_dir, db) = open_db();
    seed(&db, 120);

    let (sizes, seen, totals) = walk(
        &db,
        &ClipQuery {
            limit: Some(50),
            ..ClipQuery::default()
        },
    );

    assert_eq!(sizes, [50, 50, 20]);
    assert_eq!(totals, [120, 120, 120]);
    let mut distinct = seen.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 120);
    let newest_first = list_all(
        &db,
        &ClipQuery {
            limit: Some(200),
            ..ClipQuery::default()
        },
    );
    assert_eq!(seen, ids(&newest_first));
}

#[test]
fn the_page_size_defaults_to_50() {
    let (_dir, db) = open_db();
    seed(&db, 60);

    let page = list_all(&db, &ClipQuery::default());

    assert_eq!(page.items.len(), 50);
    assert!(page.next_cursor.is_some());
}

#[test]
fn the_page_size_is_capped_at_200() {
    let (_dir, db) = open_db();
    seed(&db, 230);

    for limit in [201, 10_000, u32::MAX] {
        let page = list_all(
            &db,
            &ClipQuery {
                limit: Some(limit),
                ..ClipQuery::default()
            },
        );

        assert_eq!(page.items.len(), 200, "{limit}");
    }
}

#[test]
fn a_page_size_of_zero_still_returns_a_clip() {
    let (_dir, db) = open_db();
    seed(&db, 3);

    let page = list_all(
        &db,
        &ClipQuery {
            limit: Some(0),
            ..ClipQuery::default()
        },
    );

    assert_eq!(page.items.len(), 1);
    assert!(page.next_cursor.is_some());
}

#[test]
fn a_history_that_fills_the_last_page_exactly_has_no_next_cursor() {
    let (_dir, db) = open_db();
    seed(&db, 100);

    let (sizes, _, _) = walk(
        &db,
        &ClipQuery {
            limit: Some(50),
            ..ClipQuery::default()
        },
    );

    assert_eq!(sizes, [50, 50]);
}

#[test]
fn a_search_is_paged_with_the_same_cursor() {
    let (_dir, db) = open_db();
    for n in 0..30 {
        add(&db, &format!("needle {n}"), 1_000 + n);
        add(&db, &format!("hay {n}"), 1_000 + n);
    }

    let (sizes, seen, totals) = walk(
        &db,
        &ClipQuery {
            search: Some("nee".to_owned()),
            limit: Some(12),
            ..ClipQuery::default()
        },
    );

    assert_eq!(sizes, [12, 12, 6]);
    assert_eq!(totals, [30, 30, 30]);
    assert_eq!(seen.len(), 30);
}

#[test]
fn clips_copied_in_the_same_millisecond_are_paged_without_losing_any() {
    let (_dir, db) = open_db();
    for n in 0..7 {
        add(&db, &format!("same instant {n}"), 5_000);
    }

    let (sizes, seen, _) = walk(
        &db,
        &ClipQuery {
            search: Some("instant".to_owned()),
            limit: Some(3),
            ..ClipQuery::default()
        },
    );

    assert_eq!(sizes, [3, 3, 1]);
    let mut distinct = seen;
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 7);
}

#[test]
fn an_invalid_cursor_is_refused_with_its_own_code() {
    let (_dir, db) = open_db();
    add(&db, "one", 1_000);

    for cursor in ["", "nonsense", "12:", "x:id"] {
        let result = db.with_conn(|conn| {
            list(
                conn,
                &ClipQuery {
                    cursor: Some(cursor.to_owned()),
                    ..ClipQuery::default()
                },
            )
        });

        let error = result.unwrap_err();
        assert!(matches!(error, StorageError::InvalidCursor), "{cursor:?}");
        assert_eq!(AppError::from(error).code, "invalid_cursor");
    }
}
