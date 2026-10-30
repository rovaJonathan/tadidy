//! Launch at login (SYS-01). The system owns the truth: the user can switch Tadidy off in System
//! Settings, so the stored `launchAtLogin` is reconciled with it at startup. The `SMAppService`
//! implementation lives in `macos/login_item.rs`; the logic around it runs against a fake.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{PoisonError, RwLock};

use crate::error::AppError;
use crate::settings::Settings;
use crate::storage::{self, Db};

/// Why the login item could not be read or changed. No detail: the cause is logged by code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LoginItemError {
    #[error("the login item could not be changed")]
    Failed,
}

/// The "Open at Login" entry of the app, as the system sees it.
pub trait LoginItem: Send + Sync {
    fn is_enabled(&self) -> Result<bool, LoginItemError>;
    fn set_enabled(&self, on: bool) -> Result<(), LoginItemError>;
}

/// Stands in until the real implementation is wired: every call fails, so a state built without a
/// login item reports `login_item_failed` instead of pretending it worked.
pub struct NoLoginItem;

impl LoginItem for NoLoginItem {
    fn is_enabled(&self) -> Result<bool, LoginItemError> {
        Err(LoginItemError::Failed)
    }

    fn set_enabled(&self, _on: bool) -> Result<(), LoginItemError> {
        Err(LoginItemError::Failed)
    }
}

/// An in-memory login item for the tests of everything that changes or reads it.
#[derive(Default)]
pub struct FakeLoginItem {
    enabled: AtomicBool,
    failing: AtomicBool,
    changes: AtomicU32,
}

impl FakeLoginItem {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            ..Self::default()
        }
    }

    /// Makes every later call fail, as a bundle that is not signed does.
    pub fn fail(&self) {
        self.failing.store(true, Ordering::SeqCst);
    }

    /// The state the system holds now.
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// How many times `set_enabled` succeeded.
    pub fn changes(&self) -> u32 {
        self.changes.load(Ordering::SeqCst)
    }
}

impl LoginItem for FakeLoginItem {
    fn is_enabled(&self) -> Result<bool, LoginItemError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(LoginItemError::Failed);
        }
        Ok(self.enabled())
    }

    fn set_enabled(&self, on: bool) -> Result<(), LoginItemError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(LoginItemError::Failed);
        }
        self.enabled.store(on, Ordering::SeqCst);
        self.changes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Makes the stored `launchAtLogin` match the system, which wins. Called once at startup, before
/// anything shows the setting. A system that cannot be read leaves the stored value alone.
pub fn reconcile(item: &dyn LoginItem, current: &RwLock<Settings>, db: &Db) {
    let Ok(system) = item.is_enabled() else {
        tracing::warn!("the login item state could not be read, keeping the stored value");
        return;
    };
    let mut guard = current.write().unwrap_or_else(PoisonError::into_inner);
    if guard.launch_at_login == system {
        return;
    }
    let next = Settings {
        launch_at_login: system,
        ..guard.clone()
    };
    // The system wins even when the write fails: the next start reconciles again.
    if let Err(error) = db.with_conn(|conn| storage::settings::save(conn, &next)) {
        tracing::warn!(code = %AppError::from(error).code, "the reconciled setting was not stored");
    }
    *guard = next;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::FakeKeyStore;
    use crate::storage::{self, open_in_dir};

    struct Fixture {
        db: Db,
        current: RwLock<Settings>,
        _dir: tempfile::TempDir,
    }

    /// A database and shared settings that both hold `launch_at_login` as stored.
    fn stored(launch_at_login: bool) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let db = open_in_dir(dir.path(), &FakeKeyStore::default()).unwrap();
        let settings = Settings {
            launch_at_login,
            ..Settings::default()
        };
        db.with_conn(|conn| storage::settings::save(conn, &settings))
            .unwrap();
        Fixture {
            db,
            current: RwLock::new(settings),
            _dir: dir,
        }
    }

    fn live(fixture: &Fixture) -> bool {
        fixture.current.read().unwrap().launch_at_login
    }

    fn persisted(fixture: &Fixture) -> bool {
        fixture
            .db
            .with_conn(storage::settings::load)
            .unwrap()
            .launch_at_login
    }

    #[test]
    fn a_login_item_removed_in_system_settings_reads_false_at_startup() {
        let fixture = stored(true);

        reconcile(&FakeLoginItem::new(false), &fixture.current, &fixture.db);

        assert!(!live(&fixture));
        assert!(!persisted(&fixture));
    }

    #[test]
    fn a_login_item_added_in_system_settings_reads_true_at_startup() {
        let fixture = stored(false);

        reconcile(&FakeLoginItem::new(true), &fixture.current, &fixture.db);

        assert!(live(&fixture));
        assert!(persisted(&fixture));
    }

    #[test]
    fn reconciling_never_changes_the_login_item_itself() {
        let item = FakeLoginItem::new(false);
        let fixture = stored(true);

        reconcile(&item, &fixture.current, &fixture.db);

        assert_eq!(item.changes(), 0);
        assert!(!item.enabled());
    }

    #[test]
    fn a_system_that_agrees_changes_nothing() {
        let fixture = stored(true);

        reconcile(&FakeLoginItem::new(true), &fixture.current, &fixture.db);

        assert!(live(&fixture));
        assert!(persisted(&fixture));
    }

    #[test]
    fn an_unreadable_system_state_leaves_the_stored_value() {
        let item = FakeLoginItem::new(false);
        item.fail();
        let fixture = stored(true);

        reconcile(&item, &fixture.current, &fixture.db);

        assert!(live(&fixture));
        assert!(persisted(&fixture));
    }

    #[test]
    fn only_launch_at_login_is_touched() {
        let fixture = stored(true);
        let other = Settings {
            history_size: 50,
            launch_at_login: true,
            ..Settings::default()
        };
        *fixture.current.write().unwrap() = other.clone();

        reconcile(&FakeLoginItem::new(false), &fixture.current, &fixture.db);

        assert_eq!(
            *fixture.current.read().unwrap(),
            Settings {
                launch_at_login: false,
                ..other
            }
        );
    }

    #[test]
    fn the_stand_in_login_item_always_fails() {
        assert_eq!(NoLoginItem.is_enabled(), Err(LoginItemError::Failed));
        assert_eq!(NoLoginItem.set_enabled(true), Err(LoginItemError::Failed));
    }

    #[test]
    fn the_fake_follows_what_it_is_told() {
        let item = FakeLoginItem::new(false);

        item.set_enabled(true).unwrap();

        assert_eq!(item.is_enabled(), Ok(true));
        assert_eq!(item.changes(), 1);
    }
}
