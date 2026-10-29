//! The current time, behind a trait so the services that stamp clips are tested with a fixed one.

use std::sync::atomic::{AtomicI64, Ordering};

pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> i64;
}

/// The system clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        0
    }
}

/// A clock that only moves when the test says so.
#[derive(Debug, Default)]
pub struct FixedClock(AtomicI64);

impl FixedClock {
    pub fn new(now_ms: i64) -> Self {
        Self(AtomicI64::new(now_ms))
    }

    pub fn set(&self, now_ms: i64) {
        self.0.store(now_ms, Ordering::Relaxed);
    }
}

impl Clock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_clock_reads_milliseconds_since_the_epoch() {
        // 2020-01-01 in milliseconds: any real clock is past it.
        assert!(SystemClock.now_ms() > 1_577_836_800_000);
    }

    #[test]
    fn a_fixed_clock_stays_put_until_set() {
        let clock = FixedClock::new(1_000);
        assert_eq!(clock.now_ms(), 1_000);
        assert_eq!(clock.now_ms(), 1_000);

        clock.set(2_500);

        assert_eq!(clock.now_ms(), 2_500);
    }
}
