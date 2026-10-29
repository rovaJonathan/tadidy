//! In-memory [`Pasteboard`], [`Workspace`], [`KeySender`] and [`AccessibilityProbe`] for the tests of every service that depends on
//! them (`src-tauri/tests/` included). They honour the same contract as the AppKit
//! implementations in `crate::macos`.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::{
    AccessibilityProbe, KeySender, Pasteboard, PlatformError, Representation, SourceApp, Workspace,
};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Default)]
struct PasteboardState {
    items: Vec<Representation>,
    change_count: i64,
    writes: Vec<Vec<Representation>>,
}

/// A pasteboard that lives in memory. `set` is what another app copying looks like; `write`
/// is what Tadidy pasting looks like, and is recorded.
#[derive(Default)]
pub struct FakePasteboard {
    state: Mutex<PasteboardState>,
}

impl FakePasteboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the content as another application would, bumping the change count.
    pub fn set(&self, items: Vec<Representation>) {
        let mut state = lock(&self.state);
        state.items = items;
        state.change_count += 1;
    }

    /// Every item list written through [`Pasteboard::write`], oldest first.
    pub fn writes(&self) -> Vec<Vec<Representation>> {
        lock(&self.state).writes.clone()
    }
}

impl Pasteboard for FakePasteboard {
    fn change_count(&self) -> i64 {
        lock(&self.state).change_count
    }

    fn types(&self) -> Vec<String> {
        lock(&self.state)
            .items
            .iter()
            .map(|item| item.uti.clone())
            .collect()
    }

    fn read(&self, uti: &str) -> Option<Vec<u8>> {
        lock(&self.state)
            .items
            .iter()
            .find(|item| item.uti == uti)
            .map(|item| item.data.clone())
    }

    fn write(&self, items: &[Representation]) -> Result<i64, PlatformError> {
        let mut state = lock(&self.state);
        state.items = items.to_vec();
        state.change_count += 1;
        state.writes.push(items.to_vec());
        Ok(state.change_count)
    }
}

#[derive(Default)]
struct WorkspaceState {
    frontmost: Option<SourceApp>,
    icons: HashMap<String, Vec<u8>>,
}

/// A workspace whose frontmost app and icons are set by the test.
#[derive(Default)]
pub struct FakeWorkspace {
    state: Mutex<WorkspaceState>,
}

impl FakeWorkspace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_frontmost(&self, app: Option<SourceApp>) {
        lock(&self.state).frontmost = app;
    }

    pub fn set_icon(&self, bundle_id: &str, png: Vec<u8>) {
        lock(&self.state).icons.insert(bundle_id.to_owned(), png);
    }
}

impl Workspace for FakeWorkspace {
    fn frontmost_app(&self) -> Option<SourceApp> {
        lock(&self.state).frontmost.clone()
    }

    fn app_icon_png(&self, app: &SourceApp) -> Option<Vec<u8>> {
        lock(&self.state).icons.get(&app.bundle_id).cloned()
    }
}

#[derive(Default)]
struct KeySenderState {
    pastes: usize,
    failing: bool,
}

/// A key sender that only counts the pastes it was asked to send.
#[derive(Default)]
pub struct FakeKeySender {
    state: Mutex<KeySenderState>,
}

impl FakeKeySender {
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes every later [`KeySender::send_paste`] fail.
    pub fn fail_sends(&self) {
        lock(&self.state).failing = true;
    }

    /// How many pastes were sent successfully.
    pub fn paste_count(&self) -> usize {
        lock(&self.state).pastes
    }
}

impl KeySender for FakeKeySender {
    fn send_paste(&self) -> Result<(), PlatformError> {
        let mut state = lock(&self.state);
        if state.failing {
            return Err(PlatformError::KeyEvent);
        }
        state.pastes += 1;
        Ok(())
    }
}

#[derive(Default)]
struct AccessibilityState {
    trusted: bool,
    requests: usize,
    failing: bool,
}

/// An Accessibility probe that is untrusted until the test says otherwise.
#[derive(Default)]
pub struct FakeAccessibility {
    state: Mutex<AccessibilityState>,
}

impl FakeAccessibility {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_trusted(&self, trusted: bool) {
        lock(&self.state).trusted = trusted;
    }

    /// Makes every later [`AccessibilityProbe::request_and_open_settings`] fail.
    pub fn fail_requests(&self) {
        lock(&self.state).failing = true;
    }

    /// How many times the permission was requested, failed requests included.
    pub fn requests(&self) -> usize {
        lock(&self.state).requests
    }
}

impl AccessibilityProbe for FakeAccessibility {
    fn is_trusted(&self) -> bool {
        lock(&self.state).trusted
    }

    fn request_and_open_settings(&self) -> Result<(), PlatformError> {
        let mut state = lock(&self.state);
        state.requests += 1;
        if state.failing {
            return Err(PlatformError::AccessibilitySettings);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{UTI_PLAIN_TEXT, UTI_TADIDY_MARKER};

    fn text(value: &str) -> Representation {
        Representation::new(UTI_PLAIN_TEXT, value.as_bytes().to_vec())
    }

    fn html(value: &str) -> Representation {
        Representation::new("public.html", value.as_bytes().to_vec())
    }

    fn textedit() -> SourceApp {
        SourceApp::new("com.apple.TextEdit", "TextEdit", 4242)
    }

    #[test]
    fn set_bumps_change_count() {
        let pasteboard = FakePasteboard::new();
        let before = pasteboard.change_count();

        pasteboard.set(vec![text("one")]);
        pasteboard.set(vec![text("two"), html("<b>two</b>")]);

        assert_eq!(pasteboard.change_count(), before + 2);
        assert_eq!(pasteboard.types(), vec![UTI_PLAIN_TEXT, "public.html"]);
    }

    #[test]
    fn write_replaces_all_types_and_returns_new_count() {
        let pasteboard = FakePasteboard::new();
        pasteboard.set(vec![html("<b>old</b>")]);
        let before = pasteboard.change_count();

        let returned = pasteboard
            .write(&[
                text("new"),
                Representation::new(UTI_TADIDY_MARKER, Vec::new()),
            ])
            .unwrap();

        assert_eq!(returned, pasteboard.change_count());
        assert_eq!(returned, before + 1);
        assert_eq!(pasteboard.types(), vec![UTI_PLAIN_TEXT, UTI_TADIDY_MARKER]);
        assert_eq!(pasteboard.read("public.html"), None);
        assert_eq!(pasteboard.read(UTI_PLAIN_TEXT), Some(b"new".to_vec()));
        assert_eq!(pasteboard.read(UTI_TADIDY_MARKER), Some(Vec::new()));
    }

    #[test]
    fn write_is_recorded() {
        let pasteboard = FakePasteboard::new();
        let items = vec![text("pasted")];

        pasteboard.write(&items).unwrap();

        assert_eq!(pasteboard.writes(), vec![items]);
    }

    #[test]
    fn read_unknown_uti_is_none() {
        let pasteboard = FakePasteboard::new();
        pasteboard.set(vec![text("one")]);

        assert_eq!(pasteboard.read("public.png"), None);
    }

    #[test]
    fn an_empty_pasteboard_has_no_types() {
        assert!(FakePasteboard::new().types().is_empty());
    }

    #[test]
    fn fake_workspace_returns_configured_app() {
        let workspace = FakeWorkspace::new();
        assert_eq!(workspace.frontmost_app(), None);

        workspace.set_frontmost(Some(textedit()));
        workspace.set_icon("com.apple.TextEdit", vec![0x89, b'P', b'N', b'G']);

        assert_eq!(workspace.frontmost_app(), Some(textedit()));
        assert_eq!(
            workspace.app_icon_png(&textedit()),
            Some(vec![0x89, b'P', b'N', b'G'])
        );
    }

    #[test]
    fn fake_workspace_has_no_icon_for_an_unknown_app() {
        let workspace = FakeWorkspace::new();

        assert_eq!(workspace.app_icon_png(&textedit()), None);
    }

    #[test]
    fn fake_key_sender_counts_pastes_and_can_fail() {
        let keys = FakeKeySender::new();
        assert_eq!(keys.paste_count(), 0);

        keys.send_paste().unwrap();
        keys.send_paste().unwrap();
        assert_eq!(keys.paste_count(), 2);

        keys.fail_sends();
        assert_eq!(keys.send_paste(), Err(PlatformError::KeyEvent));
        assert_eq!(keys.paste_count(), 2);
    }

    #[test]
    fn fake_accessibility_is_untrusted_until_set() {
        let accessibility = FakeAccessibility::new();
        assert!(!accessibility.is_trusted());

        accessibility.set_trusted(true);

        assert!(accessibility.is_trusted());
        assert_eq!(accessibility.requests(), 0);
    }

    #[test]
    fn fake_accessibility_records_requests_and_can_fail() {
        let accessibility = FakeAccessibility::new();

        accessibility.request_and_open_settings().unwrap();
        assert_eq!(accessibility.requests(), 1);

        accessibility.fail_requests();
        assert_eq!(
            accessibility.request_and_open_settings(),
            Err(PlatformError::AccessibilitySettings)
        );
        assert_eq!(accessibility.requests(), 2);
    }
}
