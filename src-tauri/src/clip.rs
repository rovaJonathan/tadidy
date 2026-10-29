#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_the_lowercase_hex_sha256() {
        assert_eq!(
            content_hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn content_hash_of_nothing_is_the_empty_digest() {
        assert_eq!(
            content_hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn a_cursor_survives_encode_and_decode() {
        let cursor = Cursor {
            last_copied_at: 1_761_700_000_123,
            id: "0192f3a4-7b1c-7d2e-8f00-123456789abc".to_owned(),
        };

        assert_eq!(Cursor::decode(&cursor.encode()), Some(cursor));
    }

    #[test]
    fn decoding_an_invalid_cursor_gives_none() {
        for input in ["", "abc", "12", "12:", ":id", "x:id"] {
            assert_eq!(Cursor::decode(input), None, "{input:?}");
        }
    }

    #[test]
    fn the_history_limit_defaults_to_500_clips() {
        assert_eq!(DEFAULT_HISTORY_SIZE, 500);
        assert_eq!(HistoryLimit::default().get(), Some(500));
    }

    #[test]
    fn the_history_limit_can_be_set_and_made_unlimited() {
        let limit = HistoryLimit::default();

        limit.set(Some(10));
        assert_eq!(limit.get(), Some(10));

        limit.set(None);
        assert_eq!(limit.get(), None);
    }

    #[test]
    fn a_stored_zero_means_unlimited() {
        let limit = HistoryLimit::default();

        limit.set(Some(0));

        assert_eq!(limit.get(), None);
    }

    #[test]
    fn clones_of_the_history_limit_share_one_value() {
        let limit = HistoryLimit::default();
        let other = limit.clone();

        other.set(Some(25));

        assert_eq!(limit.get(), Some(25));
    }

    #[test]
    fn clip_kinds_round_trip_through_their_column_value() {
        for (kind, text) in [
            (ClipKind::Text, "text"),
            (ClipKind::Rich, "rich"),
            (ClipKind::Image, "image"),
            (ClipKind::File, "file"),
            (ClipKind::Url, "url"),
            (ClipKind::Color, "color"),
        ] {
            assert_eq!(kind.as_str(), text);
            assert_eq!(ClipKind::parse(text), Some(kind));
            assert_eq!(serde_json::to_value(kind).unwrap(), text);
        }
        assert_eq!(ClipKind::parse("bogus"), None);
    }
}
