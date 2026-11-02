//! Starting over when the history database can no longer be opened (key gone, key not matching).
//! The old files are moved aside as `.bak`, never deleted (AGENTS.md §2: the old key and the
//! clip content are never read or logged, here only file names move).

use std::path::Path;

use crate::crypto::KeyStore;
use crate::storage::{Db, StorageError};

/// Moves `tadidy.db`, `tadidy.db-wal` and `tadidy.db-shm` aside as `.bak`, stores a new key and
/// opens a new database. Nothing is lost on a failure: the files are back where they were and
/// an older `.bak` is replaced only once everything else succeeded.
pub fn reset(_dir: &Path, _store: &dyn KeyStore) -> Result<Db, StorageError> {
    todo!("TAD-022")
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
