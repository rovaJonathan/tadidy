//! The clips schema and repository against a real SQLCipher database in a temporary folder, with
//! a fixed test key. Never the Keychain, never the user's data folder (AGENTS.md §13). Every
//! text below is a synthetic string.

// `allow-unwrap-in-tests` covers the `#[test]` functions, not the helpers around them.
#![allow(clippy::unwrap_used)]

use rusqlite::{Connection, params};
use tadidy_lib::clip::{ClipKind, Cursor, NewClip, content_hash};
use tadidy_lib::platform::{Representation, UTI_PLAIN_TEXT};
use tadidy_lib::storage::clips::{
    InsertOutcome, LIST_AFTER_SQL, LIST_FIRST_PAGE_SQL, OLDEST_UNPINNED_SQL, clear, count, delete,
    enforce_limit, get, insert_or_bump, list_recent, mark_used, move_to_top,
};
use tadidy_lib::storage::{Db, app_icons, open};
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

fn inserted_id(outcome: InsertOutcome) -> String {
    match outcome {
        InsertOutcome::Inserted(id) => id,
        other => panic!("expected Inserted, got {other:?}"),
    }
}

fn insert(db: &Db, clip: &NewClip) -> InsertOutcome {
    db.with_conn(|conn| insert_or_bump(conn, clip)).unwrap()
}

fn pin(db: &Db, id: &str) {
    db.with_conn(|conn| conn.execute("UPDATE clips SET pinned = 1 WHERE id = ?1", [id]))
        .unwrap();
}

fn names(conn: &Connection, kind: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = ?1 AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap();
    stmt.query_map([kind], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

fn fts_matches(conn: &Connection, query: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(
            "SELECT clips.id FROM clips_fts JOIN clips ON clips.rowid = clips_fts.rowid
             WHERE clips_fts MATCH ?1",
        )
        .unwrap();
    stmt.query_map([query], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn scalar(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

/// The `detail` column of `EXPLAIN QUERY PLAN` for `sql`, every parameter bound to an integer.
fn plan(conn: &Connection, sql: &str) -> Vec<String> {
    let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
    let arity = stmt.parameter_count();
    stmt.query_map(
        rusqlite::params_from_iter(std::iter::repeat_n(1, arity)),
        |row| row.get::<_, String>(3),
    )
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

// --- migration -------------------------------------------------------------------------------

#[test]
fn migrating_a_new_database_creates_the_whole_data_model() {
    let (_dir, db) = open_db();

    db.with_conn(|conn| {
        assert_eq!(user_version(conn), 2);
        let tables = names(conn, "table");
        for table in [
            "app_icons",
            "clips",
            "clips_fts",
            "collections",
            "excluded_apps",
            "representations",
            "settings",
            "thumbnails",
        ] {
            assert!(tables.contains(&table.to_owned()), "missing table {table}");
        }
        assert_eq!(
            names(conn, "index")
                .into_iter()
                .filter(|name| name.starts_with("idx_"))
                .collect::<Vec<_>>(),
            [
                "idx_clips_kind",
                "idx_clips_last_copied",
                "idx_clips_recent"
            ]
        );
        assert_eq!(names(conn, "trigger"), ["clips_ad", "clips_ai", "clips_au"]);
    });
}

#[test]
fn migrating_again_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let key = tadidy_lib::crypto::DbKey::from_bytes(KEY);
    let first = open(&path, &key).unwrap();
    let before = first.with_conn(|conn| {
        (
            names(conn, "table"),
            names(conn, "index"),
            names(conn, "trigger"),
        )
    });
    drop(first);

    let second = open(&path, &key).unwrap();

    second.with_conn(|conn| {
        assert_eq!(user_version(conn), 2);
        assert_eq!(
            (
                names(conn, "table"),
                names(conn, "index"),
                names(conn, "trigger")
            ),
            before
        );
    });
}

// --- insert and bump -------------------------------------------------------------------------

#[test]
fn a_new_clip_is_inserted_with_a_count_of_one() {
    let (_dir, db) = open_db();

    let id = inserted_id(insert(&db, &text_clip("first", 1_000)));

    let found = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap();
    assert_eq!(found.clip.id, id);
    assert_eq!(found.clip.kind, ClipKind::Text);
    assert_eq!(found.clip.copy_count, 1);
    assert_eq!(found.clip.created_at, 1_000);
    assert_eq!(found.clip.last_copied_at, 1_000);
    assert!(!found.clip.pinned);
    assert_eq!(found.clip.plain_text.as_deref(), Some("first"));
    assert_eq!(found.representations.len(), 1);
    assert_eq!(found.representations[0].uti, UTI_PLAIN_TEXT);
    assert_eq!(found.representations[0].data, b"first");
}

#[test]
fn a_clip_id_is_a_uuid_v7() {
    let (_dir, db) = open_db();

    let id = inserted_id(insert(&db, &text_clip("first", 1_000)));

    let uuid = uuid::Uuid::parse_str(&id).unwrap();
    assert_eq!(uuid.get_version_num(), 7);
}

#[test]
fn the_same_content_later_bumps_the_existing_clip() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("same", 1_000)));
    insert(&db, &text_clip("other", 1_500));

    let outcome = insert(&db, &text_clip("same", 2_000));

    assert_eq!(outcome, InsertOutcome::Bumped(id.clone()));
    assert_eq!(db.with_conn(count).unwrap(), 2);
    let found = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap();
    assert_eq!(found.clip.copy_count, 2);
    assert_eq!(found.clip.created_at, 1_000);
    assert_eq!(found.clip.last_copied_at, 2_000);
    let newest = db.with_conn(|conn| list_recent(conn, None, 10)).unwrap();
    assert_eq!(newest[0].id, id);
}

#[test]
fn a_bump_keeps_one_set_of_representations() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("same", 1_000)));

    insert(&db, &text_clip("same", 2_000));

    let found = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap();
    assert_eq!(found.representations.len(), 1);
}

#[test]
fn a_bump_records_the_latest_source_app() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("same", 1_000)));
    let mut again = text_clip("same", 2_000);
    again.source_bundle = Some("com.example.Browser".to_owned());
    again.source_name = Some("Browser".to_owned());

    insert(&db, &again);

    let found = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap();
    assert_eq!(
        found.clip.source_bundle.as_deref(),
        Some("com.example.Browser")
    );
    assert_eq!(found.clip.source_name.as_deref(), Some("Browser"));
}

#[test]
fn a_bump_keeps_the_bundle_and_the_name_of_one_app_together() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("same", 1_000)));
    let source = |db: &Db| {
        let found = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap();
        (found.clip.source_bundle, found.clip.source_name)
    };

    // A copier with a bundle id but no name must not inherit the previous app's name.
    let mut nameless = text_clip("same", 2_000);
    nameless.source_bundle = Some("com.example.Browser".to_owned());
    nameless.source_name = None;
    insert(&db, &nameless);
    assert_eq!(source(&db), (Some("com.example.Browser".to_owned()), None));

    // A copier of unknown origin leaves the known pair as it is.
    let mut unknown = text_clip("same", 3_000);
    unknown.source_bundle = None;
    unknown.source_name = None;
    insert(&db, &unknown);
    assert_eq!(source(&db), (Some("com.example.Browser".to_owned()), None));
}

#[test]
fn representations_come_back_in_the_order_they_were_stored() {
    let (_dir, db) = open_db();
    let mut clip = text_clip("rich", 1_000);
    clip.representations = vec![
        Representation::new("public.rtf", b"rtf".to_vec()),
        Representation::new("public.html", b"html".to_vec()),
        Representation::new(UTI_PLAIN_TEXT, b"rich".to_vec()),
    ];
    let id = inserted_id(insert(&db, &clip));

    let found = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap();

    let utis: Vec<&str> = found
        .representations
        .iter()
        .map(|r| r.uti.as_str())
        .collect();
    assert_eq!(utis, ["public.rtf", "public.html", UTI_PLAIN_TEXT]);
}

#[test]
fn getting_an_unknown_clip_gives_none() {
    let (_dir, db) = open_db();

    assert!(db.with_conn(|conn| get(conn, "nope")).unwrap().is_none());
}

// --- move to top -----------------------------------------------------------------------------

#[test]
fn moving_to_the_top_changes_the_date_but_not_the_count() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("older", 1_000)));
    insert(&db, &text_clip("newer", 2_000));

    let moved = db
        .with_conn(|conn| move_to_top(conn, &content_hash(b"older"), 3_000))
        .unwrap();

    assert_eq!(moved, Some(id.clone()));
    let found = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap();
    assert_eq!(found.clip.last_copied_at, 3_000);
    assert_eq!(found.clip.copy_count, 1);
    let newest = db.with_conn(|conn| list_recent(conn, None, 10)).unwrap();
    assert_eq!(newest[0].id, id);
}

#[test]
fn moving_an_unknown_hash_to_the_top_gives_none() {
    let (_dir, db) = open_db();

    let moved = db
        .with_conn(|conn| move_to_top(conn, &content_hash(b"unknown"), 3_000))
        .unwrap();

    assert_eq!(moved, None);
}

// --- mark used -------------------------------------------------------------------------------

#[test]
fn marking_a_clip_used_sets_last_used_at_and_nothing_else() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("pasted", 1_000)));
    let before = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap().clip;
    assert_eq!(before.last_used_at, None);

    let marked = db.with_conn(|conn| mark_used(conn, &id, 5_000)).unwrap();

    assert!(marked);
    let after = db.with_conn(|conn| get(conn, &id)).unwrap().unwrap().clip;
    assert_eq!(after.last_used_at, Some(5_000));
    assert_eq!(
        tadidy_lib::clip::ClipRecord {
            last_used_at: None,
            ..after
        },
        before
    );
}

#[test]
fn marking_an_unknown_clip_used_changes_nothing() {
    let (_dir, db) = open_db();
    insert(&db, &text_clip("pasted", 1_000));

    let marked = db
        .with_conn(|conn| mark_used(conn, "no-such-id", 5_000))
        .unwrap();

    assert!(!marked);
}

// --- list ------------------------------------------------------------------------------------

#[test]
fn the_list_is_paged_with_the_cursor_of_the_last_row() {
    let (_dir, db) = open_db();
    for i in 0..25 {
        insert(&db, &text_clip(&format!("clip {i}"), 1_000 + i));
    }

    let mut pages = Vec::new();
    let mut cursor: Option<Cursor> = None;
    loop {
        let page = db
            .with_conn(|conn| list_recent(conn, cursor.as_ref(), 10))
            .unwrap();
        if page.is_empty() {
            break;
        }
        cursor = page.last().map(|clip| clip.cursor());
        pages.push(page);
    }

    assert_eq!(pages.iter().map(Vec::len).collect::<Vec<_>>(), [10, 10, 5]);
    let times: Vec<i64> = pages.iter().flatten().map(|c| c.last_copied_at).collect();
    assert_eq!(times, (0..25).rev().map(|i| 1_000 + i).collect::<Vec<_>>());
}

#[test]
fn clips_copied_in_the_same_millisecond_are_paged_without_duplicates_or_gaps() {
    let (_dir, db) = open_db();
    for i in 0..25 {
        insert(&db, &text_clip(&format!("clip {i}"), 1_000));
    }

    let mut ids = Vec::new();
    let mut cursor: Option<Cursor> = None;
    loop {
        let page = db
            .with_conn(|conn| list_recent(conn, cursor.as_ref(), 7))
            .unwrap();
        if page.is_empty() {
            break;
        }
        cursor = page.last().map(|clip| clip.cursor());
        ids.extend(page.into_iter().map(|clip| clip.id));
    }

    let mut sorted = ids.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    sorted.dedup();
    assert_eq!(ids.len(), 25);
    assert_eq!(ids, sorted, "ordered by id descending, no duplicate");
}

#[test]
fn the_list_plans_use_the_newest_first_index_without_a_temporary_sort() {
    let (_dir, db) = open_db();

    db.with_conn(|conn| {
        for sql in [LIST_FIRST_PAGE_SQL, LIST_AFTER_SQL, OLDEST_UNPINNED_SQL] {
            let details = plan(conn, sql);
            assert!(
                details.iter().any(|d| d.contains("idx_clips_last_copied")),
                "{sql}: {details:?}"
            );
            assert!(
                !details.iter().any(|d| d.contains("TEMP B-TREE")),
                "{sql}: {details:?}"
            );
        }
    });
}

// --- history limit ---------------------------------------------------------------------------

#[test]
fn the_limit_deletes_the_oldest_unpinned_clips_and_spares_the_pinned_ones() {
    let (_dir, db) = open_db();
    // The two pinned clips are the oldest of all: only `pinned` keeps them.
    let pinned: Vec<String> = (0..2)
        .map(|i| inserted_id(insert(&db, &text_clip(&format!("pinned {i}"), 1 + i))))
        .collect();
    for id in &pinned {
        pin(&db, id);
    }
    let unpinned: Vec<String> = (0..12)
        .map(|i| inserted_id(insert(&db, &text_clip(&format!("clip {i}"), 100 + i))))
        .collect();

    let mut deleted = db.with_conn(|conn| enforce_limit(conn, Some(10))).unwrap();

    // The order of the ids returned is not part of the contract.
    deleted.sort();
    let mut oldest = unpinned[..2].to_vec();
    oldest.sort();
    assert_eq!(deleted, oldest);
    assert_eq!(db.with_conn(count).unwrap(), 12);
    for id in &pinned {
        assert!(db.with_conn(|conn| get(conn, id)).unwrap().is_some());
    }
    db.with_conn(|conn| {
        assert_eq!(scalar(conn, "SELECT count(*) FROM representations"), 12);
    });
}

#[test]
fn a_history_under_the_limit_is_left_alone() {
    let (_dir, db) = open_db();
    for i in 0..3 {
        insert(&db, &text_clip(&format!("clip {i}"), 1_000 + i));
    }

    let deleted = db.with_conn(|conn| enforce_limit(conn, Some(10))).unwrap();

    assert!(deleted.is_empty());
    assert_eq!(db.with_conn(count).unwrap(), 3);
}

#[test]
fn no_limit_deletes_nothing() {
    let (_dir, db) = open_db();
    for i in 0..30 {
        insert(&db, &text_clip(&format!("clip {i}"), 1_000 + i));
    }

    let deleted = db.with_conn(|conn| enforce_limit(conn, None)).unwrap();

    assert!(deleted.is_empty());
    assert_eq!(db.with_conn(count).unwrap(), 30);
}

// --- delete and clear ------------------------------------------------------------------------

#[test]
fn delete_reports_only_the_ids_it_removed() {
    let (_dir, db) = open_db();
    let first = inserted_id(insert(&db, &text_clip("first", 1_000)));
    let second = inserted_id(insert(&db, &text_clip("second", 2_000)));

    let removed = db
        .with_conn(|conn| delete(conn, &[first.clone(), "unknown".to_owned()]))
        .unwrap();

    assert_eq!(removed, std::slice::from_ref(&first));
    assert!(db.with_conn(|conn| get(conn, &first)).unwrap().is_none());
    assert!(db.with_conn(|conn| get(conn, &second)).unwrap().is_some());
    db.with_conn(|conn| {
        assert_eq!(scalar(conn, "SELECT count(*) FROM representations"), 1);
    });
}

#[test]
fn clearing_can_keep_the_pinned_clips() {
    let (_dir, db) = open_db();
    let ids: Vec<String> = (0..5)
        .map(|i| inserted_id(insert(&db, &text_clip(&format!("clip {i}"), 1_000 + i))))
        .collect();
    pin(&db, &ids[0]);
    pin(&db, &ids[1]);

    let kept_pinned = db.with_conn(|conn| clear(conn, true)).unwrap();
    let remaining = db.with_conn(|conn| list_recent(conn, None, 10)).unwrap();

    assert_eq!(kept_pinned, 3);
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().all(|clip| clip.pinned));

    let everything = db.with_conn(|conn| clear(conn, false)).unwrap();

    assert_eq!(everything, 2);
    assert_eq!(db.with_conn(count).unwrap(), 0);
}

#[test]
fn removing_clips_gives_the_freed_pages_back() {
    let (_dir, db) = open_db();
    let mut big = text_clip("big", 1_000);
    big.representations = vec![Representation::new("public.png", vec![7u8; 1 << 20])];
    let id = inserted_id(insert(&db, &big));
    db.with_conn(|conn| assert_eq!(scalar(conn, "PRAGMA freelist_count"), 0));

    db.with_conn(|conn| delete(conn, &[id])).unwrap();

    db.with_conn(|conn| assert_eq!(scalar(conn, "PRAGMA freelist_count"), 0));
}

// --- full-text index -------------------------------------------------------------------------

#[test]
fn the_index_finds_accented_text_without_the_accent_until_the_clip_is_deleted() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("un café noir", 1_000)));

    let before = db.with_conn(|conn| fts_matches(conn, "cafe"));
    db.with_conn(|conn| delete(conn, std::slice::from_ref(&id)))
        .unwrap();
    let after = db.with_conn(|conn| fts_matches(conn, "cafe"));

    assert_eq!(before, [id]);
    assert!(after.is_empty());
}

#[test]
fn a_bump_does_not_write_to_the_index_but_a_new_source_app_does() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("same", 1_000)));
    let writes = |db: &Db, clip: &NewClip| {
        db.with_conn(|conn| {
            let before = conn.total_changes();
            insert_or_bump(conn, clip).unwrap();
            conn.total_changes() - before
        })
    };

    let plain_bump = writes(&db, &text_clip("same", 2_000));
    let mut other_source = text_clip("same", 3_000);
    other_source.source_name = Some("Browser".to_owned());
    let reindexed_bump = writes(&db, &other_source);

    // Only the `clips` row changes in a plain bump; the update trigger adds the index rewrite.
    assert_eq!(plain_bump, 1);
    assert!(reindexed_bump > plain_bump, "{reindexed_bump}");
    db.with_conn(|conn| {
        assert_eq!(fts_matches(conn, "Browser"), std::slice::from_ref(&id));
        assert!(fts_matches(conn, "Editor").is_empty());
        assert_eq!(fts_matches(conn, "same"), [id]);
    });
}

#[test]
fn the_update_trigger_only_listens_to_the_three_indexed_columns() {
    let (_dir, db) = open_db();

    let sql: String = db.with_conn(|conn| {
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = 'clips_au'",
            [],
            |row| row.get(0),
        )
        .unwrap()
    });

    assert!(
        sql.contains("UPDATE OF plain_text, title, source_name ON clips"),
        "{sql}"
    );
}

// --- app icons -------------------------------------------------------------------------------

#[test]
fn an_app_icon_is_cached_by_bundle_id() {
    let (_dir, db) = open_db();
    db.with_conn(|conn| {
        assert!(!app_icons::has(conn, "com.example.Editor").unwrap());
        assert_eq!(app_icons::get(conn, "com.example.Editor").unwrap(), None);

        app_icons::put(conn, "com.example.Editor", b"png-1").unwrap();

        assert!(app_icons::has(conn, "com.example.Editor").unwrap());
        assert_eq!(
            app_icons::get(conn, "com.example.Editor").unwrap(),
            Some(b"png-1".to_vec())
        );
    });
}

#[test]
fn putting_an_icon_again_replaces_it() {
    let (_dir, db) = open_db();
    db.with_conn(|conn| {
        app_icons::put(conn, "com.example.Editor", b"png-1").unwrap();

        app_icons::put(conn, "com.example.Editor", b"png-2").unwrap();

        assert_eq!(
            app_icons::get(conn, "com.example.Editor").unwrap(),
            Some(b"png-2".to_vec())
        );
        assert_eq!(scalar(conn, "SELECT count(*) FROM app_icons"), 1);
    });
}

// --- cascades --------------------------------------------------------------------------------

#[test]
fn deleting_a_collection_keeps_its_clips() {
    let (_dir, db) = open_db();
    let id = inserted_id(insert(&db, &text_clip("filed", 1_000)));
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO collections (id, name, sort) VALUES ('c1', 'Work', 1.0)",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE clips SET collection_id = 'c1' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        conn.execute("DELETE FROM collections WHERE id = 'c1'", [])
            .unwrap();

        let collection: Option<String> = conn
            .query_row(
                "SELECT collection_id FROM clips WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(collection, None);
    });
}
