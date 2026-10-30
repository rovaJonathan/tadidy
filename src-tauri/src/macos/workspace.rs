//! `NSWorkspace` behind the [`Workspace`] trait. Needs the running desktop, so it is covered
//! by the manual QA of TAD-004, not by a unit test.

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};

use super::app_icon;
use crate::platform::{PlatformError, SourceApp, Workspace};

/// The shared workspace. Stateless: every call asks AppKit again.
#[derive(Debug, Default, Clone, Copy)]
pub struct NsWorkspace;

impl Workspace for NsWorkspace {
    /// `None` when no app is in front or when it has no bundle identifier (no `Info.plist`):
    /// the exclusion list is keyed by bundle id, so such an app cannot be told apart.
    fn frontmost_app(&self) -> Option<SourceApp> {
        autoreleasepool(|_| {
            let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
            Some(SourceApp::new(
                app.bundleIdentifier()?.to_string(),
                app.localizedName()
                    .map(|name| name.to_string())
                    .unwrap_or_default(),
                app.processIdentifier(),
            ))
        })
    }

    fn app_icon_png(&self, app: &SourceApp) -> Option<Vec<u8>> {
        autoreleasepool(|_| {
            let running = NSRunningApplication::runningApplicationWithProcessIdentifier(app.pid)?;
            // The pid may have been reused by another app since `frontmost_app` returned.
            if running.bundleIdentifier()?.to_string() != app.bundle_id {
                return None;
            }
            app_icon::png(&running)
        })
    }

    fn activate(&self, app: &SourceApp) -> Result<(), PlatformError> {
        autoreleasepool(|_| {
            let running = NSRunningApplication::runningApplicationWithProcessIdentifier(app.pid)
                .ok_or(PlatformError::Activation)?;
            // The pid may have been reused by another app since `frontmost_app` returned.
            if running
                .bundleIdentifier()
                .map(|id| id.to_string())
                .as_deref()
                != Some(app.bundle_id.as_str())
            {
                return Err(PlatformError::Activation);
            }
            // Tadidy is the active app (the popover had the focus), so macOS lets it hand the
            // focus over; every window of the app comes forward, as a click on its Dock icon.
            if running.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows) {
                Ok(())
            } else {
                Err(PlatformError::Activation)
            }
        })
    }
}
