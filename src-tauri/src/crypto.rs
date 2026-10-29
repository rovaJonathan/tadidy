//! The database key and where it lives. The key is 256 random bits, kept in the macOS Keychain
//! and in memory only: it is never written to disk, logged, put in an error or sent to the
//! webview (AGENTS.md §2). The Keychain sits behind [`KeyStore`] so the logic is tested with a
//! fake (AGENTS.md §9).

use std::fmt;
use std::sync::Mutex;

use crate::storage::StorageError;

/// Length of the SQLCipher key, in bytes (AES-256).
const KEY_LEN: usize = 32;

/// The key of the encrypted database. No `Serialize`, and `Debug` hides the bytes.
#[derive(Clone)]
pub struct DbKey([u8; KEY_LEN]);

impl DbKey {
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    /// A new key from the operating system's random generator.
    pub fn generate() -> Result<Self, KeyStoreError> {
        let mut bytes = [0; KEY_LEN];
        getrandom::fill(&mut bytes).map_err(|_| KeyStoreError::Random)?;
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    /// The raw-key form SQLCipher reads as is: `x'<64 hex digits>'`. A passphrase would go
    /// through PBKDF2 at every open, which is slow for a key that is already random.
    pub(crate) fn to_pragma_value(&self) -> String {
        let hex: String = self.0.iter().map(|byte| format!("{byte:02x}")).collect();
        format!("x'{hex}'")
    }
}

impl fmt::Debug for DbKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DbKey(<redacted>)")
    }
}

/// A failure of the key store. Never carries the key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyStoreError {
    #[error("the key store is unavailable")]
    Unavailable,
    #[error("the stored key is malformed")]
    Malformed,
    #[error("the system random generator failed")]
    Random,
}

/// Where the database key is kept between launches.
pub trait KeyStore: Send + Sync {
    fn get(&self) -> Result<Option<DbKey>, KeyStoreError>;
    fn set(&self, key: &DbKey) -> Result<(), KeyStoreError>;
}

/// An in-memory [`KeyStore`] for tests; the real one is the Keychain.
#[derive(Debug, Default)]
pub struct FakeKeyStore {
    key: Mutex<Option<DbKey>>,
    failing: bool,
}

impl FakeKeyStore {
    pub fn with_key(key: DbKey) -> Self {
        Self {
            key: Mutex::new(Some(key)),
            failing: false,
        }
    }

    /// A store whose every call fails, as a locked Keychain would.
    pub fn failing() -> Self {
        Self {
            key: Mutex::new(None),
            failing: true,
        }
    }
}

impl KeyStore for FakeKeyStore {
    fn get(&self) -> Result<Option<DbKey>, KeyStoreError> {
        if self.failing {
            return Err(KeyStoreError::Unavailable);
        }
        Ok(self.key.lock().unwrap_or_else(|e| e.into_inner()).clone())
    }

    fn set(&self, key: &DbKey) -> Result<(), KeyStoreError> {
        if self.failing {
            return Err(KeyStoreError::Unavailable);
        }
        *self.key.lock().unwrap_or_else(|e| e.into_inner()) = Some(key.clone());
        Ok(())
    }
}

/// The key to open the database with. A stored key is returned as is. With no key, a new one is
/// created only when there is no database yet: a database without its key can never be read
/// again, and a fresh key must not silently replace it (the caller offers a reset instead).
pub fn load_or_create_key(store: &dyn KeyStore, db_exists: bool) -> Result<DbKey, StorageError> {
    if let Some(key) = store.get()? {
        return Ok(key);
    }
    if db_exists {
        return Err(StorageError::KeyMissing);
    }
    let key = DbKey::generate()?;
    store.set(&key)?;
    Ok(key)
}

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
