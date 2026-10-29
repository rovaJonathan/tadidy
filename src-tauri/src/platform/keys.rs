//! The synthetic keystroke that pastes into the frontmost app.

use super::PlatformError;

/// Posts keyboard events to the system.
pub trait KeySender: Send + Sync {
    /// Posts `⌘V` (key down then key up, `Command` only) to the frontmost app. Needs the
    /// Accessibility permission: without it macOS drops the event and nothing tells us.
    fn send_paste(&self) -> Result<(), PlatformError>;
}
