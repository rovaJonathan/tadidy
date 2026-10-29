//! Turns what is on the pasteboard into a clip to store. Pure: no clock, no database, no AppKit.

use crate::clip::{ClipKind, NewClip, content_hash};
use crate::platform::{Representation, SourceApp, UTI_PLAIN_TEXT};

/// How many characters of the text the list shows (CAP-02).
pub const PREVIEW_CHARS: usize = 300;

/// A text clip from the `public.utf8-plain-text` bytes. `None` for text that is not valid UTF-8,
/// empty, or only whitespace: nothing worth keeping.
pub fn text_clip(text: &[u8], source: Option<&SourceApp>, now: i64) -> Option<NewClip> {
    let string = std::str::from_utf8(text).ok()?;
    if string.trim().is_empty() {
        return None;
    }
    Some(NewClip {
        kind: ClipKind::Text,
        content_hash: content_hash(text),
        plain_text: Some(string.to_owned()),
        preview: Some(string.chars().take(PREVIEW_CHARS).collect()),
        source_bundle: source.map(|app| app.bundle_id.clone()),
        source_name: source.map(|app| app.name.clone()),
        byte_size: i64::try_from(text.len()).unwrap_or(i64::MAX),
        representations: vec![Representation::new(UTI_PLAIN_TEXT, text)],
        now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{ClipKind, content_hash};
    use crate::platform::{Representation, UTI_PLAIN_TEXT};

    fn editor() -> SourceApp {
        SourceApp::new("com.example.editor", "Editor", 77)
    }

    #[test]
    fn a_text_becomes_a_text_clip_with_its_source_and_time() {
        let clip = text_clip(b"hello world", Some(&editor()), 1_700_000_000_000).unwrap();

        assert_eq!(clip.kind, ClipKind::Text);
        assert_eq!(clip.plain_text.as_deref(), Some("hello world"));
        assert_eq!(clip.preview.as_deref(), Some("hello world"));
        assert_eq!(clip.source_bundle.as_deref(), Some("com.example.editor"));
        assert_eq!(clip.source_name.as_deref(), Some("Editor"));
        assert_eq!(clip.byte_size, 11);
        assert_eq!(clip.now, 1_700_000_000_000);
    }

    #[test]
    fn the_hash_is_the_sha256_of_the_utf8_bytes() {
        let clip = text_clip("héllo".as_bytes(), None, 1).unwrap();

        assert_eq!(clip.content_hash, content_hash("héllo".as_bytes()));
    }

    #[test]
    fn the_only_representation_is_the_plain_text_itself() {
        let clip = text_clip(b"hello", None, 1).unwrap();

        assert_eq!(
            clip.representations,
            vec![Representation::new(UTI_PLAIN_TEXT, b"hello".to_vec())]
        );
    }

    #[test]
    fn the_byte_size_counts_bytes_not_characters() {
        let clip = text_clip("héllo".as_bytes(), None, 1).unwrap();

        assert_eq!(clip.byte_size, 6);
    }

    #[test]
    fn the_text_is_kept_as_copied_whitespace_included() {
        let clip = text_clip(b"  indented\n", None, 1).unwrap();

        assert_eq!(clip.plain_text.as_deref(), Some("  indented\n"));
        assert_eq!(clip.content_hash, content_hash(b"  indented\n"));
    }

    #[test]
    fn a_copy_without_a_known_source_has_no_source() {
        let clip = text_clip(b"hello", None, 1).unwrap();

        assert_eq!(clip.source_bundle, None);
        assert_eq!(clip.source_name, None);
    }

    #[test]
    fn the_preview_is_the_first_300_characters_not_bytes() {
        let text = "é".repeat(400);

        let clip = text_clip(text.as_bytes(), None, 1).unwrap();

        assert_eq!(clip.preview.as_deref(), Some("é".repeat(300).as_str()));
        assert_eq!(clip.plain_text.as_deref(), Some(text.as_str()));
        assert_eq!(clip.byte_size, 800);
    }

    #[test]
    fn a_text_of_exactly_300_characters_is_its_own_preview() {
        let text = "a".repeat(PREVIEW_CHARS);

        let clip = text_clip(text.as_bytes(), None, 1).unwrap();

        assert_eq!(clip.preview.as_deref(), Some(text.as_str()));
    }

    #[test]
    fn empty_text_is_not_a_clip() {
        assert!(text_clip(b"", Some(&editor()), 1).is_none());
    }

    #[test]
    fn whitespace_only_text_is_not_a_clip() {
        for text in [" ", "\n", " \t\r\n ", "\u{a0}\u{2003}"] {
            assert!(text_clip(text.as_bytes(), None, 1).is_none(), "{text:?}");
        }
    }

    #[test]
    fn bytes_that_are_not_utf8_are_not_a_clip() {
        assert!(text_clip(&[0xff, 0xfe, b'a'], None, 1).is_none());
    }
}
