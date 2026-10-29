//! The running applications, as seen by the watcher.

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
    /// The app's icon as a 64 × 64 px PNG (32 pt @2x).
    fn app_icon_png(&self, app: &SourceApp) -> Option<Vec<u8>>;
}
