//! The synthetic keystroke that pastes into the frontmost app.

use super::PlatformError;

/// Posts keyboard events to the system.
pub trait KeySender: Send + Sync {
    /// Posts `⌘V` (key down then key up, `Command` only) to the frontmost app. Needs the
    /// Accessibility permission: without it macOS drops the event and nothing tells us.
    fn send_paste(&self) -> Result<(), PlatformError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paste_chord_is_command_and_the_physical_v_key_only() {
        let chord = KeyChord::paste();

        // `kVK_ANSI_V`; no other modifier may ever be added (a held `⇧` must not merge in).
        assert_eq!(chord.key_code, 9);
        assert!(chord.command);
    }
}
