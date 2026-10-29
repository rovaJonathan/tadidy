//! How long the user has been away, so the watcher can poll less often.

pub trait IdleProbe: Send + Sync {
    /// Seconds since the last keyboard, mouse or trackpad event of the session.
    fn seconds_since_last_input(&self) -> f64;
}
