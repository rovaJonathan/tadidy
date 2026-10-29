//! How long the session has been without input, through CoreGraphics, behind the
//! [`IdleProbe`] trait. Needs a real login session, so it is covered by the manual QA of
//! TAD-010, not by a unit test.

use core_graphics::event_source::CGEventSourceStateID;

use crate::platform::IdleProbe;

/// `kCGAnyInputEventType`, `~0`: every keyboard, mouse and trackpad event.
const ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(
        state_id: CGEventSourceStateID,
        event_type: u32,
    ) -> f64;
}

/// The idle time of the combined session state.
#[derive(Debug, Default, Clone, Copy)]
pub struct CgIdleProbe;

impl IdleProbe for CgIdleProbe {
    fn seconds_since_last_input(&self) -> f64 {
        // SAFETY: a pure query taking two plain values; `CombinedSessionState` is a valid state
        // id and `kCGAnyInputEventType` is the documented value for "any input event".
        unsafe {
            CGEventSourceSecondsSinceLastEventType(
                CGEventSourceStateID::CombinedSessionState,
                ANY_INPUT_EVENT_TYPE,
            )
        }
    }
}
