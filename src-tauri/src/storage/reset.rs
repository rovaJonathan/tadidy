//! Starting over when the history database can no longer be opened (key gone, key not matching).
//! The old files are moved aside as `.bak`, never deleted (AGENTS.md §2: the old key and the
//! clip content are never read or logged, here only file names move).

use std::path::Path;

use crate::crypto::{DbKey, KeyStore};
use crate::storage::{DB_FILE_NAME, Db, StorageError, open};

/// The database file and the two files SQLite keeps beside it in WAL mode.
const FILE_SUFFIXES: [&str; 3] = ["", "-wal", "-shm"];
/// Where a file waits while the reset runs, then what it is kept as.
const STAGED: &str = ".bak.new";
const BACKUP: &str = ".bak";

/// Moves `tadidy.db`, `tadidy.db-wal` and `tadidy.db-shm` aside as `.bak`, opens a new database
/// with a new key, then stores that key. Nothing is lost on a failure: the files are back where
/// they were, the stored key was not touched until the new database opened, and an older `.bak`
/// set is replaced (whole) only once everything else succeeded.
pub fn reset(dir: &Path, store: &dyn KeyStore) -> Result<Db, StorageError> {
    reset_with(dir, store, open)
}

/// [`reset`] with the way the new database is opened passed in, so a test can make it fail.
fn reset_with(
    dir: &Path,
    store: &dyn KeyStore,
    open: impl FnOnce(&Path, &DbKey) -> Result<Db, StorageError>,
) -> Result<Db, StorageError> {
    keep_leftovers(dir)?;
    let staged = stage(dir)?;
    match start_over(dir, store, open) {
        Ok(db) => {
            promote(dir, &staged);
            Ok(db)
        }
        Err(error) => {
            // The new database's files must not end up beside the restored ones.
            for name in file_names() {
                let _ = std::fs::remove_file(dir.join(name));
            }
            unstage(dir, &staged);
            Err(error)
        }
    }
}

fn file_names() -> impl Iterator<Item = String> {
    FILE_SUFFIXES
        .into_iter()
        .map(|suffix| format!("{DB_FILE_NAME}{suffix}"))
}

/// A staging file still there belongs to an earlier reset that stopped before it could rename it
/// to `.bak` (it worked but could not write the backup, or it was stopped): the history it moved
/// aside exists only under that name, and staging would overwrite it. The set is kept under a
/// numbered name (`tadidy.db.bak.1`, …) that no other file has, so neither this reset's own backup
/// nor an older one can replace it. A sidecar alone (no main file) is not a backup of anything and
/// stays where it is.
fn keep_leftovers(dir: &Path) -> Result<(), StorageError> {
    let left = |name: &str| dir.join(format!("{name}{STAGED}"));
    if !left(DB_FILE_NAME).is_file() {
        return Ok(());
    }
    let kept = |name: &str, number: u32| dir.join(format!("{name}{BACKUP}.{number}"));
    let number = (1..)
        .find(|&number| file_names().all(|name| !kept(&name, number).exists()))
        .unwrap_or(u32::MAX);
    std::fs::rename(left(DB_FILE_NAME), kept(DB_FILE_NAME, number))?;
    for name in file_names().filter(|name| name != DB_FILE_NAME && left(name).is_file()) {
        if std::fs::rename(left(&name), kept(&name, number)).is_err() {
            tracing::warn!("a database backup could not be renamed");
        }
    }
    Ok(())
}

/// Renames each existing file to its staging name; on a failure the ones already moved go back.
fn stage(dir: &Path) -> Result<Vec<String>, StorageError> {
    let mut staged = Vec::new();
    for name in file_names() {
        let from = dir.join(&name);
        if !from.exists() {
            continue;
        }
        if let Err(error) = std::fs::rename(&from, dir.join(format!("{name}{STAGED}"))) {
            unstage(dir, &staged);
            return Err(error.into());
        }
        staged.push(name);
    }
    Ok(staged)
}

/// The new database first, the key in the store last: the old key is replaced only once nothing
/// can fail any more, so a failure before that leaves it where it was.
fn start_over(
    dir: &Path,
    store: &dyn KeyStore,
    open: impl FnOnce(&Path, &DbKey) -> Result<Db, StorageError>,
) -> Result<Db, StorageError> {
    let key = DbKey::generate()?;
    let db = open(&dir.join(DB_FILE_NAME), &key)?;
    store.replace(&key)?;
    Ok(db)
}

/// Staging names become the backups, replacing the older ones. A backup set never mixes two
/// databases (restored by hand, a `-wal` from another database could be replayed into the main
/// file): the main file goes first, and the sidecars follow only if it did. With no database file
/// to move, or when its backup cannot be written, the staged sidecars wait under their staging
/// names and the older backups stay as they were.
fn promote(dir: &Path, staged: &[String]) {
    let main = staged.iter().any(|name| name == DB_FILE_NAME);
    if !main {
        return;
    }
    if std::fs::rename(
        dir.join(format!("{DB_FILE_NAME}{STAGED}")),
        dir.join(format!("{DB_FILE_NAME}{BACKUP}")),
    )
    .is_err()
    {
        // The old history is still on disk under its staging name; the reset itself worked.
        tracing::warn!("a database backup could not be renamed");
        return;
    }
    promote_sidecars(dir, staged);
}

/// The `-wal` / `-shm` backups of a set whose main file was just promoted: the staged ones replace
/// the older ones, and an older one with no replacement goes.
fn promote_sidecars(dir: &Path, staged: &[String]) {
    for name in file_names().filter(|name| name != DB_FILE_NAME) {
        let backup = dir.join(format!("{name}{BACKUP}"));
        if staged.contains(&name) {
            if std::fs::rename(dir.join(format!("{name}{STAGED}")), &backup).is_err() {
                tracing::warn!("a database backup could not be renamed");
            }
        } else if backup.exists() && std::fs::remove_file(&backup).is_err() {
            tracing::warn!("an older database backup could not be removed");
        }
    }
}

fn unstage(dir: &Path, staged: &[String]) {
    for name in staged {
        if std::fs::rename(dir.join(format!("{name}{STAGED}")), dir.join(name)).is_err() {
            tracing::warn!("a database file could not be put back");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{DbKey, FakeKeyStore};
    use crate::error::AppError;
    use crate::storage::{DB_FILE_NAME, open_in_dir};

    const WAL: &str = "tadidy.db-wal";
    const SHM: &str = "tadidy.db-shm";

    /// A data folder holding a database made with a key that `store` does not know.
    fn orphaned_database() -> (tempfile::TempDir, Vec<u8>) {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let bytes = std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap();
        (dir, bytes)
    }

    fn read(dir: &Path, name: &str) -> Vec<u8> {
        std::fs::read(dir.join(name)).unwrap()
    }

    #[test]
    fn reset_moves_the_database_aside_untouched_and_opens_a_new_one() {
        let (dir, before) = orphaned_database();
        let store = FakeKeyStore::default();

        let db = reset(dir.path(), &store).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak"), before);
        assert_ne!(read(dir.path(), DB_FILE_NAME), before);
        assert!(
            db.with_conn(
                |conn| conn.query_row("SELECT count(*) FROM clips", [], |r| r.get::<_, i64>(0))
            )
            .is_ok()
        );
    }

    #[test]
    fn reset_moves_the_wal_and_shm_files_too() {
        let (dir, _) = orphaned_database();
        std::fs::write(dir.path().join(WAL), b"wal").unwrap();
        std::fs::write(dir.path().join(SHM), b"shm").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db-wal.bak"), b"wal");
        assert_eq!(read(dir.path(), "tadidy.db-shm.bak"), b"shm");
    }

    #[test]
    fn reset_stores_a_new_key_that_opens_the_new_database() {
        let (dir, _) = orphaned_database();
        let stale = DbKey::from_bytes([9; 32]);
        let store = FakeKeyStore::with_key(stale.clone());

        reset(dir.path(), &store).unwrap();

        let stored = store.get().unwrap().unwrap();
        assert_ne!(stored.as_bytes(), stale.as_bytes());
        assert!(open_in_dir(dir.path(), &store).is_ok());
    }

    #[test]
    fn reset_replaces_an_older_backup_only_after_it_succeeded() {
        let (dir, before) = orphaned_database();
        std::fs::write(dir.path().join("tadidy.db.bak"), b"older").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak"), before);
    }

    #[test]
    fn a_second_reset_leaves_no_sidecar_of_the_first_beside_its_backup() {
        let (dir, _) = orphaned_database();
        std::fs::write(dir.path().join(WAL), b"wal").unwrap();
        reset(dir.path(), &FakeKeyStore::default()).unwrap();
        assert!(dir.path().join("tadidy.db-wal.bak").exists());
        // The new database lost its key as well; nothing but the main file is there to move.
        std::fs::remove_file(dir.path().join(WAL)).ok();
        std::fs::remove_file(dir.path().join(SHM)).ok();
        let second = read(dir.path(), DB_FILE_NAME);

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak"), second);
        assert!(!dir.path().join("tadidy.db-wal.bak").exists());
        assert!(!dir.path().join("tadidy.db-shm.bak").exists());
    }

    #[test]
    fn a_wal_is_never_promoted_beside_a_backup_that_was_not_replaced() {
        // The main file's backup cannot be written (a non-empty folder is in the way), but the
        // wal's can: a wal of this database must not end up beside the older database.
        let (dir, _) = orphaned_database();
        std::fs::write(dir.path().join(WAL), b"wal").unwrap();
        let blocker = dir.path().join("tadidy.db.bak");
        std::fs::create_dir(&blocker).unwrap();
        std::fs::write(blocker.join("x"), b"x").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert!(!dir.path().join("tadidy.db-wal.bak").exists());
        // Nothing is lost: it waits under its staging name.
        assert_eq!(read(dir.path(), "tadidy.db-wal.bak.new"), b"wal");
    }

    #[test]
    fn a_stale_wal_without_a_database_file_does_not_replace_an_older_wal_backup() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tadidy.db.bak"), b"older").unwrap();
        std::fs::write(dir.path().join("tadidy.db-wal.bak"), b"older wal").unwrap();
        std::fs::write(dir.path().join(WAL), b"stale wal").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak"), b"older");
        assert_eq!(read(dir.path(), "tadidy.db-wal.bak"), b"older wal");
        // And it is out of the way of the new database.
        assert!(!dir.path().join(WAL).exists());
    }

    #[test]
    fn a_staged_file_left_by_an_earlier_reset_is_never_overwritten() {
        // An earlier reset worked but could not rename `.bak.new` to `.bak`: the history it moved
        // aside exists only under that name.
        let (dir, _) = orphaned_database();
        std::fs::write(dir.path().join("tadidy.db.bak.new"), b"first history").unwrap();

        let result = reset(dir.path(), &FakeKeyStore::failing());

        assert!(result.is_err());
        assert_eq!(read(dir.path(), "tadidy.db.bak.1"), b"first history");
    }

    #[test]
    fn the_history_left_by_an_interrupted_reset_survives_the_next_reset() {
        // Reset #1 moved the history aside, opened an empty database and was stopped before it
        // stored the key: `tadidy.db` is that empty database, the history waits as `.bak.new`.
        // The next reset must not replace the history with the empty database.
        let (dir, empty) = orphaned_database();
        std::fs::write(dir.path().join("tadidy.db.bak.new"), b"first history").unwrap();
        std::fs::write(dir.path().join("tadidy.db-wal.bak.new"), b"first wal").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak.1"), b"first history");
        assert_eq!(read(dir.path(), "tadidy.db-wal.bak.1"), b"first wal");
        assert_eq!(read(dir.path(), "tadidy.db.bak"), empty);
        assert!(!dir.path().join("tadidy.db.bak.new").exists());
        assert!(!dir.path().join("tadidy.db-wal.bak.new").exists());
    }

    #[test]
    fn a_kept_history_never_takes_the_name_of_another_one() {
        let (dir, _) = orphaned_database();
        std::fs::write(dir.path().join("tadidy.db.bak.1"), b"kept before").unwrap();
        std::fs::write(dir.path().join("tadidy.db.bak.new"), b"first history").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak.1"), b"kept before");
        assert_eq!(read(dir.path(), "tadidy.db.bak.2"), b"first history");
    }

    #[test]
    fn a_backup_that_cannot_be_written_does_not_block_every_later_reset() {
        // `tadidy.db.bak` is a non-empty folder, so no database file can become that backup: the
        // reset must still go through, with the history kept under another name.
        let (dir, before) = orphaned_database();
        std::fs::write(dir.path().join("tadidy.db.bak.new"), b"first history").unwrap();
        let blocker = dir.path().join("tadidy.db.bak");
        std::fs::create_dir(&blocker).unwrap();
        std::fs::write(blocker.join("x"), b"x").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak.1"), b"first history");
        // The history this reset moved aside waits under its staging name.
        assert_eq!(read(dir.path(), "tadidy.db.bak.new"), before);
    }

    #[test]
    fn a_reset_with_no_database_file_keeps_the_older_backups() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tadidy.db.bak"), b"older").unwrap();
        std::fs::write(dir.path().join("tadidy.db-wal.bak"), b"older wal").unwrap();

        reset(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(read(dir.path(), "tadidy.db.bak"), b"older");
        assert_eq!(read(dir.path(), "tadidy.db-wal.bak"), b"older wal");
    }

    #[test]
    fn a_failed_reset_puts_every_file_back_and_keeps_the_older_backup() {
        let (dir, before) = orphaned_database();
        std::fs::write(dir.path().join(WAL), b"wal").unwrap();
        std::fs::write(dir.path().join("tadidy.db.bak"), b"older").unwrap();

        let result = reset(dir.path(), &FakeKeyStore::failing());

        assert!(result.is_err());
        assert_eq!(read(dir.path(), DB_FILE_NAME), before);
        assert_eq!(read(dir.path(), WAL), b"wal");
        assert_eq!(read(dir.path(), "tadidy.db.bak"), b"older");
    }

    #[test]
    fn a_database_that_does_not_open_leaves_the_stored_key_and_the_files_alone() {
        let (dir, before) = orphaned_database();
        let stale = DbKey::from_bytes([9; 32]);
        let store = FakeKeyStore::with_key(stale.clone());

        let result = reset_with(dir.path(), &store, |_, _| Err(StorageError::WrongKey));

        assert!(result.is_err());
        assert_eq!(store.get().unwrap().unwrap().as_bytes(), stale.as_bytes());
        assert_eq!(read(dir.path(), DB_FILE_NAME), before);
        assert!(!dir.path().join("tadidy.db.bak.new").exists());
    }

    #[test]
    fn a_file_that_cannot_be_moved_leaves_the_others_where_they_were() {
        let (dir, before) = orphaned_database();
        std::fs::write(dir.path().join(WAL), b"wal").unwrap();
        // A non-empty folder where the wal's staging file goes: that rename fails after the
        // database's own rename succeeded.
        let blocker = dir.path().join("tadidy.db-wal.bak.new");
        std::fs::create_dir(&blocker).unwrap();
        std::fs::write(blocker.join("x"), b"x").unwrap();

        let result = reset(dir.path(), &FakeKeyStore::default());

        assert!(result.is_err());
        assert_eq!(read(dir.path(), DB_FILE_NAME), before);
        assert_eq!(read(dir.path(), WAL), b"wal");
        assert!(!dir.path().join("tadidy.db.bak.new").exists());
    }

    #[test]
    fn a_failure_reaches_the_webview_without_a_path_or_a_key() {
        let (dir, _) = orphaned_database();
        let error = reset(dir.path(), &FakeKeyStore::failing()).unwrap_err();

        let shown = serde_json::to_string(&AppError::from(error)).unwrap();

        assert!(!shown.contains(dir.path().to_str().unwrap()));
        assert!(!shown.contains("/Users"));
    }
}
