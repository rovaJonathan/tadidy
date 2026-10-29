//! The `⌘V` keystroke through CoreGraphics, behind the [`KeySender`] trait. Needs the
//! Accessibility permission and a frontmost app, so it is covered by the manual QA of TAD-005
//! (`paste_probe`), not by a unit test.

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::platform::keys::KeyChord;
use crate::platform::{KeySender, PlatformError};

/// Posts `⌘V` to the HID event tap.
#[derive(Debug, Default, Clone, Copy)]
pub struct CgKeySender;

impl KeySender for CgKeySender {
    fn send_paste(&self) -> Result<(), PlatformError> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|()| PlatformError::KeyEvent)?;
        let chord = KeyChord::paste();
        for key_down in [true, false] {
            let event = CGEvent::new_keyboard_event(source.clone(), chord.key_code, key_down)
                .map_err(|()| PlatformError::KeyEvent)?;
            // Explicit flags: whatever modifier the user still holds is replaced, not merged.
            event.set_flags(if chord.command {
                CGEventFlags::CGEventFlagCommand
            } else {
                CGEventFlags::CGEventFlagNull
            });
            event.post(CGEventTapLocation::HID);
        }
        Ok(())
    }
}
