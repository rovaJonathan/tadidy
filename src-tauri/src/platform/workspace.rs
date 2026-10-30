//! The running applications, as seen by the watcher and the paster.

use super::PlatformError;

/// The application that was in front when something happened.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SourceApp {
    pub bundle_id: String,
    pub name: String,
    pub pid: i32,
}

impl SourceApp {
    pub fn new(bundle_id: impl Into<String>, name: impl Into<String>, pid: i32) -> Self {
        Self {
            bundle_id: bundle_id.into(),
            name: name.into(),
            pid,
        }
    }
}

/// `NSWorkspace`.
pub trait Workspace: Send + Sync {
    /// The frontmost application, `None` when macOS reports none.
    fn frontmost_app(&self) -> Option<SourceApp>;
    /// The app's icon as a 64 × 64 px PNG (32 pt @2x). `None` once the app has quit: ask while
    /// it is still running. The fake does not model quitting, so it always returns a set icon.
    fn app_icon_png(&self, app: &SourceApp) -> Option<Vec<u8>>;
    /// Brings the app to the front, so the keystroke that follows reaches it. Fails once the
    /// app has quit.
    fn activate(&self, app: &SourceApp) -> Result<(), PlatformError>;
}
