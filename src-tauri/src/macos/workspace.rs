//! `NSWorkspace` behind the [`Workspace`] trait. Needs the running desktop, so it is covered
//! by the manual QA of TAD-004, not by a unit test.

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};

use super::app_icon;
use crate::platform::{SourceApp, Workspace};

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
            app_icon::png(&running)
        })
    }
}
