//! Developer probe for the manual QA of TAD-007: stores a random key in a throwaway Keychain
//! item (service `app.tadidy.probe`, never the app's own), reads it back, compares and deletes
//! it. It never prints the key (AGENTS.md §2).
//!
//! `cargo run --manifest-path src-tauri/Cargo.toml --example keychain_probe`

#[cfg(target_os = "macos")]
fn main() {
    use security_framework::passwords::delete_generic_password;
    use tadidy_lib::crypto::{DbKey, KeyStore};
    use tadidy_lib::macos::keychain::KeychainKeyStore;

    const SERVICE: &str = "app.tadidy.probe";
    const ACCOUNT: &str = "database-key";
    let store = KeychainKeyStore::new(SERVICE, ACCOUNT);

    println!("before: {:?}", store.get().map(|key| key.is_some()));
    let Ok(key) = DbKey::generate() else {
        return println!("the random generator failed");
    };
    println!("set: {:?}", store.set(&key));
    let read = store.get().ok().flatten();
    println!(
        "get: found {}, same key: {}",
        read.is_some(),
        read.is_some_and(|read| read.as_bytes() == key.as_bytes())
    );
    println!("cleanup: {:?}", delete_generic_password(SERVICE, ACCOUNT));
}

#[cfg(not(target_os = "macos"))]
fn main() {}
