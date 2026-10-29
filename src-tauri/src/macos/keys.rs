//! The `⌘V` keystroke through CoreGraphics, behind the [`KeySender`] trait. Needs the
//! Accessibility permission and a frontmost app, so it is covered by the manual QA of TAD-005
//! (`paste_probe`), not by a unit test.

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::platform::{KeySender, PlatformError};

/// `kVK_ANSI_V`: the physical position of `V` on an ANSI keyboard, not the character.
const KVK_ANSI_V: CGKeyCode = 9;

/// Posts `⌘V` to the HID event tap.
#[derive(Debug, Default, Clone, Copy)]
pub struct CgKeySender;

impl KeySender for CgKeySender {
    fn send_paste(&self) -> Result<(), PlatformError> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|()| PlatformError::KeyEvent)?;
        for key_down in [true, false] {
            let event = CGEvent::new_keyboard_event(source.clone(), KVK_ANSI_V, key_down)
                .map_err(|()| PlatformError::KeyEvent)?;
            // Explicit flags, so a modifier the user still holds (`⇧` of `⌘⇧V`) is not merged in.
            event.set_flags(CGEventFlags::CGEventFlagCommand);
            event.post(CGEventTapLocation::HID);
        }
        Ok(())
    }
}
