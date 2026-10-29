//! The database key in the macOS Keychain, behind the [`KeyStore`] trait. Needs the real
//! Keychain, so it is covered by the manual QA of TAD-007, not by a unit test.

use core_foundation::base::{CFType, TCFType};
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use security_framework::base::Error;
use security_framework::passwords::{PasswordOptions, generic_password};
use security_framework_sys::access_control::kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly;
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecValueData,
};
use security_framework_sys::keychain_item::SecItemAdd;

use crate::crypto::{DbKey, KeyStore, KeyStoreError};

/// `kSecAttrService` of the key's item.
pub const SERVICE: &str = "app.tadidy";
/// `kSecAttrAccount` of the key's item.
pub const ACCOUNT: &str = "database-key";

/// `errSecItemNotFound`: no item matches the query.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
/// `errSecDuplicateItem`: an item with this service and account already exists.
const ERR_SEC_DUPLICATE_ITEM: i32 = -25299;

// SAFETY: the symbol is a `CFStringRef` constant exported by the Security framework, which
// `security-framework-sys` already links; `security-framework-sys` does not declare this one.
#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecAttrAccessible: CFStringRef;
}

/// A generic-password item. The default is the app's own item; a probe or a test of the adapter
/// passes another service so it never touches the real key.
#[derive(Debug, Clone)]
pub struct KeychainKeyStore {
    service: String,
    account: String,
}

impl KeychainKeyStore {
    pub fn new(service: &str, account: &str) -> Self {
        Self {
            service: service.to_owned(),
            account: account.to_owned(),
        }
    }
}

impl Default for KeychainKeyStore {
    fn default() -> Self {
        Self::new(SERVICE, ACCOUNT)
    }
}

impl KeyStore for KeychainKeyStore {
    fn get(&self) -> Result<Option<DbKey>, KeyStoreError> {
        match generic_password(PasswordOptions::new_generic_password(
            &self.service,
            &self.account,
        )) {
            Ok(bytes) => {
                let bytes = bytes.try_into().map_err(|_| KeyStoreError::Malformed)?;
                Ok(Some(DbKey::from_bytes(bytes)))
            }
            Err(error) if is_not_found(&error) => Ok(None),
            Err(_) => Err(KeyStoreError::Unavailable),
        }
    }

    /// Readable after the first unlock (the watcher runs while the screen is locked) and never
    /// copied to another device or a backup. An existing item is never replaced.
    fn set(&self, key: &DbKey) -> Result<(), KeyStoreError> {
        // The high-level `PasswordOptions` has no `kSecAttrAccessible`, and its access-control
        // flag needs the data-protection keychain (errSecMissingEntitlement for this app).
        // SAFETY: each key is a `kSec*` constant owned by the framework; `wrap_under_get_rule`
        // retains it, so the dictionary and the constants release independently.
        let pairs: Vec<(CFString, CFType)> = unsafe {
            vec![
                (
                    CFString::wrap_under_get_rule(kSecClass),
                    CFString::wrap_under_get_rule(kSecClassGenericPassword).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrService),
                    CFString::new(&self.service).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrAccount),
                    CFString::new(&self.account).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecValueData),
                    CFData::from_buffer(key.as_bytes()).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrAccessible),
                    CFString::wrap_under_get_rule(kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly)
                        .as_CFType(),
                ),
            ]
        };
        let attributes = CFDictionary::from_CFType_pairs(&pairs);
        // SAFETY: `attributes` is a valid dictionary for the whole call; the result is not
        // requested, so the null out-pointer is allowed.
        let status = unsafe { SecItemAdd(attributes.as_concrete_TypeRef(), std::ptr::null_mut()) };
        match status {
            0 => Ok(()),
            ERR_SEC_DUPLICATE_ITEM => Err(KeyStoreError::AlreadyExists),
            _ => Err(KeyStoreError::Unavailable),
        }
    }
}

fn is_not_found(error: &Error) -> bool {
    error.code() == ERR_SEC_ITEM_NOT_FOUND
}
