//! IPC commands: thin wrappers that delegate to the services (AGENTS.md §13).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_returns_the_crate_version() {
        assert_eq!(app_version(), env!("CARGO_PKG_VERSION"));
    }
}
