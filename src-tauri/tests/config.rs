//! Checks on `tauri.conf.json` that no window exists yet to reveal.

use serde_json::Value;

#[test]
fn csp_allows_the_ipc_custom_protocol() {
    // Without it, the first `invoke` fails and Tauri falls back to postMessage for the session.
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let csp = config["app"]["security"]["csp"].as_str().unwrap();
    let connect_src: Vec<&str> = csp
        .split(';')
        .map(str::trim)
        .find(|directive| directive.starts_with("connect-src"))
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    assert!(connect_src.contains(&"ipc:"), "{csp}");
    assert!(connect_src.contains(&"http://ipc.localhost"), "{csp}");
}
