#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{DbKey, FakeKeyStore};
    use crate::error::AppError;

    #[test]
    fn open_in_dir_creates_the_database_and_the_key_on_first_launch() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();

        open_in_dir(dir.path(), &store).unwrap();

        assert!(dir.path().join(DB_FILE_NAME).exists());
        assert!(store.get().unwrap().is_some());
    }

    #[test]
    fn open_in_dir_creates_the_data_folder() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("app.tadidy");

        open_in_dir(&data_dir, &FakeKeyStore::default()).unwrap();

        assert!(data_dir.join(DB_FILE_NAME).exists());
    }

    #[test]
    fn open_in_dir_reopens_with_the_stored_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeKeyStore::default();
        open_in_dir(dir.path(), &store).unwrap();

        assert!(open_in_dir(dir.path(), &store).is_ok());
    }

    #[test]
    fn open_in_dir_refuses_to_replace_a_database_whose_key_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let before = std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap();

        let result = open_in_dir(dir.path(), &FakeKeyStore::default());

        assert!(matches!(result, Err(StorageError::KeyMissing)));
        assert_eq!(
            std::fs::read(dir.path().join(DB_FILE_NAME)).unwrap(),
            before
        );
    }

    #[test]
    fn open_in_dir_reports_a_wrong_key() {
        let dir = tempfile::tempdir().unwrap();
        open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let other = FakeKeyStore::with_key(DbKey::from_bytes([9; 32]));

        let result = open_in_dir(dir.path(), &other);

        assert!(matches!(result, Err(StorageError::WrongKey)));
    }

    #[test]
    fn storage_errors_map_to_stable_codes_without_leaking_details() {
        let code = |error: StorageError| AppError::from(error).code;

        assert_eq!(code(StorageError::KeyMissing), "storage_key_missing");
        assert_eq!(code(StorageError::WrongKey), "storage_wrong_key");
        assert_eq!(
            code(StorageError::Sqlite(rusqlite::Error::InvalidQuery)),
            "storage_error"
        );
    }

    #[test]
    fn with_conn_runs_on_the_shared_connection() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        db.with_conn(|conn| conn.execute("INSERT INTO settings VALUES ('k', '1')", []))
            .unwrap();
        let value: String = db
            .clone()
            .with_conn(|conn| {
                conn.query_row("SELECT value FROM settings WHERE key = 'k'", [], |row| {
                    row.get(0)
                })
            })
            .unwrap();

        assert_eq!(value, "1");
    }

    #[test]
    fn debug_does_not_print_the_connection() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();

        assert_eq!(format!("{db:?}"), "Db");
    }
}
