//! Runs the main thread's run loop. `NSWorkspace.frontmostApplication` is a KVO property that
//! AppKit only refreshes while the main run loop runs: Tauri runs it, a plain command-line
//! program does not. Used by the TAD-004 probe to read the frontmost app the way the watcher
//! will, from another thread. Covered by the manual QA of TAD-004.

use objc2::rc::autoreleasepool;
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};

/// How long each turn of the loop may wait for a source, in seconds. A loop with no source
/// returns at once, so the turns also keep it from spinning.
const TURN: f64 = 0.1;

/// Never returns. Call it on the main thread.
pub fn run_forever() -> ! {
    loop {
        autoreleasepool(|_| {
            let until = NSDate::dateWithTimeIntervalSinceNow(TURN);
            // SAFETY: `NSDefaultRunLoopMode` is a constant AppKit initialises before `main`.
            let mode = unsafe { NSDefaultRunLoopMode };
            NSRunLoop::currentRunLoop().runMode_beforeDate(mode, &until);
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
