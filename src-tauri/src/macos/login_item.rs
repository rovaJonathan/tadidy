//! Launch at login through `SMAppService.mainApp`, behind the [`LoginItem`] trait. It only works
//! from a signed `.app` bundle, so it is covered by the manual QA of TAD-017, not by a unit test
//! (ADR 0007).

use smappservice_rs::{AppService, ServiceStatus, ServiceType};

use crate::login_item::{LoginItem, LoginItemError};

/// The app's own login item.
#[derive(Debug, Default, Clone, Copy)]
pub struct SmLoginItem;

impl LoginItem for SmLoginItem {
    fn is_enabled(&self) -> Result<bool, LoginItemError> {
        // `RequiresApproval` is a login item the user switched off in System Settings.
        Ok(AppService::new(ServiceType::MainApp).status() == ServiceStatus::Enabled)
    }

    fn set_enabled(&self, on: bool) -> Result<(), LoginItemError> {
        let service = AppService::new(ServiceType::MainApp);
        let status = service.status();
        // Registering an app that is registered, or releasing one that is not, is an error for
        // the system and nothing to do for us.
        let result = match (on, status) {
            (true, ServiceStatus::Enabled) => return Ok(()),
            (true, _) => service.register(),
            (false, ServiceStatus::Enabled | ServiceStatus::RequiresApproval) => {
                service.unregister()
            }
            (false, _) => return Ok(()),
        };
        let after = service.status();
        tracing::debug!(on, before = %status, after = %after, "login item changed");
        // The system may accept the registration and keep the item off until the user allows it
        // in System Settings (or refuse it for that reason): either way, only the user can finish.
        if on && after == ServiceStatus::RequiresApproval {
            return Err(LoginItemError::RequiresApproval);
        }
        result.map_err(|error| {
            tracing::warn!(code = error.code(), "the login item could not be changed");
            LoginItemError::Failed
        })
    }
}
