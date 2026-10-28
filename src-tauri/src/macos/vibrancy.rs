//! Native translucent material behind the popover (DESIGN.md §3). Needs a real NSWindow, so
//! it is covered by the manual QA of TAD-003, not by a unit test.

use tauri::{Runtime, WebviewWindow};
use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};

/// The popover's corner radius in points (`--radius-window`).
const POPOVER_RADIUS: f64 = 10.0;

/// `Popover` material, always active (the popover never owns the key window's look),
/// with 10 pt rounded corners.
pub fn apply_popover_material<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<(), window_vibrancy::Error> {
    apply_vibrancy(
        window,
        NSVisualEffectMaterial::Popover,
        Some(NSVisualEffectState::Active),
        Some(POPOVER_RADIUS),
    )
}
