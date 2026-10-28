//! The macOS layer: the only module allowed to use `unsafe` and the AppKit, CoreGraphics,
//! Keychain and Accessibility FFI (AGENTS.md §13). Kept thin, behind traits, so the logic
//! around it is tested with fakes (AGENTS.md §9). Excluded from the coverage gate.
#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
pub mod vibrancy;
