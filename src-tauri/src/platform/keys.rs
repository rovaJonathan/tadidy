//! The synthetic keystroke that pastes into the frontmost app.

use super::PlatformError;

/// A key and the modifiers held with it. Only what the paster sends, so only `Command`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChord {
    /// Virtual key code: a physical position on the keyboard, not a character.
    pub key_code: u16,
    pub command: bool,
}

impl KeyChord {
    /// `⌘V`: `kVK_ANSI_V` is the physical `V` of an ANSI keyboard. The flags are explicit, so a
    /// modifier the user still holds (the `⇧` of `⌘⇧V`) is not merged into the event.
    pub const fn paste() -> Self {
        Self {
            key_code: 9,
            command: true,
        }
    }
}

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
