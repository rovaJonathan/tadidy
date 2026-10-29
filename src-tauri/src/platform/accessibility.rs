//! The Accessibility (TCC) permission that synthetic keystrokes depend on.

use std::process::Command;

use super::PlatformError;

const SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// The command that opens *Privacy & Security → Accessibility* in System Settings.
pub fn open_settings_command() -> Command {
    let mut command = Command::new("/usr/bin/open");
    command.arg(SETTINGS_URL);
    command
}

/// `AXIsProcessTrusted` and the System Settings pane that grants it.
pub trait AccessibilityProbe: Send + Sync {
    /// Whether the process may post keyboard events. Never prompts.
    fn is_trusted(&self) -> bool;
    /// Asks macOS to list Tadidy under *Privacy & Security → Accessibility* and opens that
    /// pane. Only called from a user action: the permission is never requested silently.
    fn request_and_open_settings(&self) -> Result<(), PlatformError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_command_opens_the_accessibility_pane() {
        let command = open_settings_command();

        assert_eq!(command.get_program(), "/usr/bin/open");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(
            args,
            ["x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"]
        );
    }
}
