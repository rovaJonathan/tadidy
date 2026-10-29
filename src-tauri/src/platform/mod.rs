//! What the services need from the operating system, as traits: the pasteboard and the
//! workspace (frontmost app). The AppKit implementations live in `crate::macos`; the fakes in
//! [`fakes`] let the logic around them be tested without touching the real clipboard.

pub mod fakes;
pub mod pasteboard;
pub mod workspace;

pub use pasteboard::{
    IGNORED_TYPES, Pasteboard, Representation, UTI_PLAIN_TEXT, UTI_TADIDY_MARKER,
};
pub use workspace::{SourceApp, Workspace};

/// A failure of the platform layer. Never carries clip content (AGENTS.md §2).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlatformError {
    #[error("the pasteboard refused the write")]
    PasteboardWrite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_and_settings_error_messages_are_fixed_texts() {
        assert_eq!(
            PlatformError::KeyEvent.to_string(),
            "the paste keystroke could not be sent"
        );
        assert_eq!(
            PlatformError::AccessibilitySettings.to_string(),
            "the Accessibility settings could not be opened"
        );
    }

    #[test]
    fn the_write_error_message_carries_no_clip_content() {
        assert_eq!(
            PlatformError::PasteboardWrite.to_string(),
            "the pasteboard refused the write"
        );
    }
}
