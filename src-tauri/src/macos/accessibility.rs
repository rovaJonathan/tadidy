//! `AXIsProcessTrustedWithOptions` and the Accessibility pane of System Settings, behind the
//! [`AccessibilityProbe`] trait. The answer depends on the TCC database and the binary's
//! signature, so it is covered by the manual QA of TAD-005, not by a unit test.

use std::process::Command;

use objc2_application_services::{AXIsProcessTrustedWithOptions, kAXTrustedCheckOptionPrompt};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFString, kCFBooleanTrue};

use crate::platform::{AccessibilityProbe, PlatformError};

const SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// The Accessibility permission of this process.
#[derive(Debug, Default, Clone, Copy)]
pub struct AxProbe;

impl AccessibilityProbe for AxProbe {
    fn is_trusted(&self) -> bool {
        // SAFETY: a null options dictionary is allowed and means "do not prompt".
        unsafe { AXIsProcessTrustedWithOptions(None) }
    }

    fn request_and_open_settings(&self) -> Result<(), PlatformError> {
        // SAFETY: `kCFBooleanTrue` and `kAXTrustedCheckOptionPrompt` are constants exported by
        // CoreFoundation and ApplicationServices, valid for the life of the process.
        let (prompt, key) = unsafe { (kCFBooleanTrue, kAXTrustedCheckOptionPrompt) };
        let prompt = prompt.ok_or(PlatformError::AccessibilitySettings)?;
        let options = CFDictionary::<CFString, CFBoolean>::from_slices(&[key], &[prompt]);
        // SAFETY: the dictionary maps the documented `CFString` key to a `CFBoolean`. With
        // `prompt` the call lists this binary under Accessibility; the answer is not needed.
        unsafe { AXIsProcessTrustedWithOptions(Some(options.as_opaque())) };

        let status = Command::new("/usr/bin/open")
            .arg(SETTINGS_URL)
            .status()
            .map_err(|_| PlatformError::AccessibilitySettings)?;
        if status.success() {
            Ok(())
        } else {
            Err(PlatformError::AccessibilitySettings)
        }
    }
}
