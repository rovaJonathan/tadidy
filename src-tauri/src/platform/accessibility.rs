//! The Accessibility (TCC) permission that synthetic keystrokes depend on.

use super::PlatformError;

/// `AXIsProcessTrusted` and the System Settings pane that grants it.
pub trait AccessibilityProbe: Send + Sync {
    /// Whether the process may post keyboard events. Never prompts.
    fn is_trusted(&self) -> bool;
    /// Asks macOS to list Tadidy under *Privacy & Security → Accessibility* and opens that
    /// pane. Only called from a user action: the permission is never requested silently.
    fn request_and_open_settings(&self) -> Result<(), PlatformError>;
}
