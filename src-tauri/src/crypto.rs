#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageError;

    fn test_key() -> DbKey {
        DbKey::from_bytes([7; 32])
    }

    #[test]
    fn debug_never_prints_the_key() {
        assert_eq!(format!("{:?}", test_key()), "DbKey(<redacted>)");
    }

    #[test]
    fn generated_keys_are_random() {
        let (first, second) = (DbKey::generate().unwrap(), DbKey::generate().unwrap());

        assert_ne!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn the_pragma_value_is_the_raw_key_form() {
        assert_eq!(
            test_key().to_pragma_value(),
            format!("x'{}'", "07".repeat(32))
        );
    }

    #[test]
    fn a_key_is_generated_and_stored_when_nothing_exists() {
        let store = FakeKeyStore::default();

        let key = load_or_create_key(&store, false).unwrap();

        assert_eq!(store.get().unwrap().unwrap().as_bytes(), key.as_bytes());
    }

    #[test]
    fn the_same_key_is_returned_next_time() {
        let store = FakeKeyStore::default();

        let first = load_or_create_key(&store, false).unwrap();
        let second = load_or_create_key(&store, true).unwrap();

        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn a_stored_key_is_returned_as_is() {
        let store = FakeKeyStore::with_key(test_key());

        let key = load_or_create_key(&store, true).unwrap();

        assert_eq!(key.as_bytes(), test_key().as_bytes());
    }

    #[test]
    fn no_key_while_the_database_exists_is_key_missing() {
        let store = FakeKeyStore::default();

        let result = load_or_create_key(&store, true);

        assert!(matches!(result, Err(StorageError::KeyMissing)));
        assert!(store.get().unwrap().is_none(), "no new key may be created");
    }

    #[test]
    fn a_key_store_failure_is_reported_as_a_key_store_error() {
        let store = FakeKeyStore::failing();

        let result = load_or_create_key(&store, false);

        assert!(matches!(result, Err(StorageError::KeyStore(_))));
    }
}
