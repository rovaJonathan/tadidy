//! The running applications, as seen by the watcher.

/// The application that was in front when something happened.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SourceApp {
    pub bundle_id: String,
    pub name: String,
    pub pid: i32,
}

/// `NSWorkspace`.
pub trait Workspace: Send + Sync {
    /// The frontmost application, `None` when macOS reports none.
    fn frontmost_app(&self) -> Option<SourceApp>;
    /// The app's icon as a 64 × 64 px PNG (32 pt @2x).
    fn app_icon_png(&self, app: &SourceApp) -> Option<Vec<u8>>;
}
