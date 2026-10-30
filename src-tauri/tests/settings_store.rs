//! The settings store and `settings::update` against a real SQLCipher database in a temporary
//! folder, with a fixed test key. Never the Keychain, never the user's data folder (AGENTS.md
//! §13). Every text below is a synthetic string.

// `allow-unwrap-in-tests` covers the `#[test]` functions, not the helpers around them.
#![allow(clippy::unwrap_used)]

use std::sync::{Mutex, RwLock};

use rusqlite::Connection;
use serde_json::{Value, json};
use tadidy_lib::clip::{ClipKind, HistoryLimit, NewClip, content_hash};
use tadidy_lib::crypto::DbKey;
use tadidy_lib::events::{CLIPS_CHANGED, EventSink, RecordingEventSink, SETTINGS_CHANGED};
use tadidy_lib::login_item::FakeLoginItem;
use tadidy_lib::platform::{Representation, UTI_PLAIN_TEXT};
use tadidy_lib::settings::{Effects, Settings, SettingsPatch, update};
use tadidy_lib::shortcuts::{FakeShortcutRegistrar, ShortcutStatus};
use tadidy_lib::storage::clips::{InsertOutcome, count, insert_or_bump};
use tadidy_lib::storage::settings::{load, save};
use tadidy_lib::storage::{Db, open};
use tempfile::TempDir;

fn open_db() -> (TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = open(&dir.path().join("test.db"), &DbKey::from_bytes([0xA; 32])).unwrap();
    (dir, db)
}

fn stored(db: &Db) -> Settings {
    db.with_conn(load).unwrap()
}

fn put_row(db: &Db, key: &str, value: &str) {
    db.with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            [key, value],
        )
    })
    .unwrap();
}

fn rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT count(*) FROM settings", [], |row| row.get(0))
        .unwrap()
}

fn insert_clips(db: &Db, how_many: i64) -> Vec<String> {
    (0..how_many)
        .map(|n| {
            let text = format!("synthetic clip {n}");
            let clip = NewClip {
                kind: ClipKind::Text,
                content_hash: content_hash(text.as_bytes()),
                plain_text: Some(text.clone()),
                preview: Some(text.clone()),
                source_bundle: None,
                source_name: None,
                byte_size: text.len() as i64,
                representations: vec![Representation::new(UTI_PLAIN_TEXT, text.as_bytes())],
                now: 1_000 + n,
            };
            match db.with_conn(|conn| insert_or_bump(conn, &clip)).unwrap() {
                InsertOutcome::Inserted(id) => id,
                other => panic!("expected Inserted, got {other:?}"),
            }
        })
        .collect()
}

/// Everything `update` touches, wired the way `AppState` wires it.
struct Rig {
    _dir: TempDir,
    db: Db,
    current: RwLock<Settings>,
    limit: HistoryLimit,
    events: RecordingEventSink,
    login_item: FakeLoginItem,
    shortcuts: FakeShortcutRegistrar,
    shortcut_status: Mutex<ShortcutStatus>,
    gate: Mutex<()>,
}

impl Rig {
    fn new() -> Self {
        let (dir, db) = open_db();
        Self {
            _dir: dir,
            db,
            current: RwLock::default(),
            limit: HistoryLimit::default(),
            events: RecordingEventSink::default(),
            login_item: FakeLoginItem::new(false),
            shortcuts: FakeShortcutRegistrar::default(),
            shortcut_status: Mutex::default(),
            gate: Mutex::default(),
        }
    }

    fn effects(&self) -> Effects<'_> {
        Effects {
            login_item: &self.login_item,
            shortcuts: &self.shortcuts,
            shortcut_status: &self.shortcut_status,
            gate: &self.gate,
        }
    }

    fn update(&self, patch: SettingsPatch) -> Result<Settings, tadidy_lib::error::AppError> {
        update(
            &self.db,
            &self.current,
            &self.limit,
            &self.events,
            &self.effects(),
            patch,
        )
    }

    fn history_size(size: u32) -> SettingsPatch {
        SettingsPatch {
            history_size: Some(size),
            ..SettingsPatch::default()
        }
    }
}

#[test]
fn an_empty_table_loads_the_defaults() {
    let (_dir, db) = open_db();

    assert_eq!(
        serde_json::to_value(stored(&db)).unwrap(),
        json!({
            "historySize": 500,
            "confirmBeforeDelete": true,
            "launchAtLogin": false,
            "quickPanelShortcut": "Cmd+Shift+V",
        })
    );
}

#[test]
fn saved_settings_load_back_unchanged() {
    let (_dir, db) = open_db();
    let settings = Settings {
        history_size: 0,
        confirm_before_delete: false,
        launch_at_login: true,
        quick_panel_shortcut: "Cmd+Alt+K".to_owned(),
    };

    db.with_conn(|conn| save(conn, &settings)).unwrap();

    assert_eq!(stored(&db), settings);
}

#[test]
fn saving_twice_keeps_one_row_per_field() {
    let (_dir, db) = open_db();

    db.with_conn(|conn| save(conn, &Settings::default()))
        .unwrap();
    db.with_conn(|conn| save(conn, &Settings::default()))
        .unwrap();

    assert_eq!(db.with_conn(rows), 4);
}

#[test]
fn each_field_is_stored_as_json_under_its_camel_case_name() {
    let (_dir, db) = open_db();

    db.with_conn(|conn| save(conn, &Settings::default()))
        .unwrap();

    let value = |key: &str| -> String {
        db.with_conn(|conn| {
            conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
        })
        .unwrap()
    };
    assert_eq!(value("historySize"), "500");
    assert_eq!(value("confirmBeforeDelete"), "true");
    assert_eq!(value("launchAtLogin"), "false");
    assert_eq!(value("quickPanelShortcut"), "\"Cmd+Shift+V\"");
}

#[test]
fn an_unknown_key_is_ignored() {
    let (_dir, db) = open_db();
    db.with_conn(|conn| save(conn, &Settings::default()))
        .unwrap();
    put_row(&db, "somethingFromTheFuture", "{\"a\":1}");

    assert_eq!(stored(&db), Settings::default());
}

#[test]
fn an_unreadable_value_falls_back_to_the_default_of_that_field_only() {
    let (_dir, db) = open_db();
    put_row(&db, "historySize", "not json");
    put_row(&db, "confirmBeforeDelete", "\"yes\"");
    put_row(&db, "launchAtLogin", "true");
    put_row(&db, "quickPanelShortcut", "\"Cmd+Alt+K\"");

    assert_eq!(
        stored(&db),
        Settings {
            launch_at_login: true,
            quick_panel_shortcut: "Cmd+Alt+K".to_owned(),
            ..Settings::default()
        }
    );
}

#[test]
fn a_stored_value_that_breaks_a_rule_falls_back_to_the_default() {
    let (_dir, db) = open_db();
    put_row(&db, "historySize", "5");
    put_row(&db, "quickPanelShortcut", "\"\"");

    assert_eq!(stored(&db), Settings::default());
}

#[test]
fn update_applies_a_patch_and_stores_it() {
    let rig = Rig::new();

    let returned = rig
        .update(SettingsPatch {
            confirm_before_delete: Some(false),
            ..SettingsPatch::default()
        })
        .unwrap();

    let expected = Settings {
        confirm_before_delete: false,
        ..Settings::default()
    };
    assert_eq!(returned, expected);
    assert_eq!(stored(&rig.db), expected);
    assert_eq!(*rig.current.read().unwrap(), expected);
}

#[test]
fn update_emits_one_settings_changed_with_the_full_settings() {
    let rig = Rig::new();

    rig.update(SettingsPatch {
        confirm_before_delete: Some(false),
        ..SettingsPatch::default()
    })
    .unwrap();

    assert_eq!(
        rig.events.events(),
        vec![(
            SETTINGS_CHANGED,
            json!({
                "historySize": 500,
                "confirmBeforeDelete": false,
                "launchAtLogin": false,
                "quickPanelShortcut": "Cmd+Shift+V",
            })
        )]
    );
}

#[test]
fn a_history_size_below_ten_is_refused_and_nothing_is_stored() {
    let rig = Rig::new();

    let error = rig.update(Rig::history_size(5)).unwrap_err();

    assert_eq!(error.code, "invalid_settings");
    assert!(error.message.contains("historySize"), "{}", error.message);
    assert_eq!(rig.db.with_conn(rows), 0);
    assert_eq!(*rig.current.read().unwrap(), Settings::default());
    assert_eq!(rig.limit.get(), Some(500));
    assert!(rig.events.events().is_empty());
}

#[test]
fn zero_and_ten_are_accepted() {
    let rig = Rig::new();

    assert_eq!(rig.update(Rig::history_size(0)).unwrap().history_size, 0);
    assert_eq!(rig.update(Rig::history_size(10)).unwrap().history_size, 10);
}

#[test]
fn lowering_the_history_size_trims_the_oldest_unpinned_clips_at_once() {
    let rig = Rig::new();
    let ids = insert_clips(&rig.db, 20);

    rig.update(Rig::history_size(10)).unwrap();

    assert_eq!(rig.limit.get(), Some(10));
    assert_eq!(rig.db.with_conn(count).unwrap(), 10);
    let removed: Vec<String> = rig
        .events
        .events()
        .into_iter()
        .filter(|(name, _)| *name == CLIPS_CHANGED)
        .flat_map(|(_, payload)| {
            payload["removed"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    let mut expected: Vec<String> = ids[..10].to_vec();
    let mut removed_sorted = removed;
    expected.sort();
    removed_sorted.sort();
    assert_eq!(removed_sorted, expected);
}

/// Notes, when the trim announces its clips, whether the settings could be read right then.
struct ReadDuringTrim<'a> {
    current: &'a RwLock<Settings>,
    readable: Mutex<Option<bool>>,
}

impl EventSink for ReadDuringTrim<'_> {
    fn emit(&self, event: &'static str, _payload: Value) {
        if event == CLIPS_CHANGED {
            *self.readable.lock().unwrap() = Some(self.current.try_read().is_ok());
        }
    }
}

#[test]
fn the_settings_stay_readable_while_the_history_is_trimmed() {
    // `get_settings` runs on the main thread: it must never wait for a trim.
    let rig = Rig::new();
    insert_clips(&rig.db, 12);
    let sink = ReadDuringTrim {
        current: &rig.current,
        readable: Mutex::default(),
    };

    update(
        &rig.db,
        &rig.current,
        &rig.limit,
        &sink,
        &rig.effects(),
        Rig::history_size(10),
    )
    .unwrap();

    assert_eq!(*sink.readable.lock().unwrap(), Some(true));
}

#[test]
fn settings_changed_comes_before_clips_changed() {
    let rig = Rig::new();
    insert_clips(&rig.db, 12);

    rig.update(Rig::history_size(10)).unwrap();

    let names: Vec<&str> = rig.events.events().iter().map(|(name, _)| *name).collect();
    assert_eq!(names, vec![SETTINGS_CHANGED, CLIPS_CHANGED]);
}

#[test]
fn a_history_that_already_fits_emits_no_clips_changed() {
    let rig = Rig::new();
    insert_clips(&rig.db, 5);

    rig.update(Rig::history_size(10)).unwrap();

    let names: Vec<&str> = rig.events.events().iter().map(|(name, _)| *name).collect();
    assert_eq!(names, vec![SETTINGS_CHANGED]);
}

#[test]
fn another_setting_never_trims_the_history() {
    let rig = Rig::new();
    insert_clips(&rig.db, 20);
    rig.limit.set(Some(10));

    rig.update(SettingsPatch {
        launch_at_login: Some(true),
        ..SettingsPatch::default()
    })
    .unwrap();

    assert_eq!(rig.db.with_conn(count).unwrap(), 20);
}

#[test]
fn zero_lifts_the_limit() {
    let rig = Rig::new();
    insert_clips(&rig.db, 20);

    rig.update(Rig::history_size(0)).unwrap();

    assert_eq!(rig.limit.get(), None);
    assert_eq!(rig.db.with_conn(count).unwrap(), 20);
}

#[test]
fn raising_the_history_size_keeps_every_clip() {
    let rig = Rig::new();
    insert_clips(&rig.db, 20);
    rig.update(Rig::history_size(10)).unwrap();

    rig.update(Rig::history_size(100)).unwrap();

    assert_eq!(rig.limit.get(), Some(100));
    assert_eq!(rig.db.with_conn(count).unwrap(), 10);
}
