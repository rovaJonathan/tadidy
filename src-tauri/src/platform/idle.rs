//! How long the user has been away, so the watcher can poll less often.

/// Once the session has been without input for this long, it counts as idle.
pub const IDLE_AFTER_SECS: f64 = 300.0;

pub trait IdleProbe: Send + Sync {
    /// Seconds since the last keyboard, mouse or trackpad event of the session.
    fn seconds_since_last_input(&self) -> f64;
}

/// Whether a session without input for `idle_secs` seconds counts as idle.
pub fn is_idle(idle_secs: f64) -> bool {
    idle_secs >= IDLE_AFTER_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_is_idle_from_five_minutes_without_input() {
        assert!(!is_idle(0.0));
        assert!(!is_idle(299.9));
        assert!(is_idle(300.0));
        assert!(is_idle(86_400.0));
    }

    #[test]
    fn a_reading_that_is_not_a_time_never_counts_as_idle() {
        assert!(!is_idle(f64::NAN));
        assert!(!is_idle(-1.0));
    }
}
