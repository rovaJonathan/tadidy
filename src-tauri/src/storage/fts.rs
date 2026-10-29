//! Turns what the user typed into an FTS5 `MATCH` expression. The input is never handed to FTS5
//! as is: its query syntax (`"`, `*`, `-`, `NEAR`, `AND`…) would turn a stray character into a
//! syntax error.

/// Every word of `input` becomes a quoted prefix term (`"sel"*`); the terms are joined with a
/// space, which FTS5 reads as AND. `None` when nothing searchable is left.
pub fn match_query(input: &str) -> Option<String> {
    let _ = input;
    todo!("TAD-012")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_becomes_a_quoted_prefix_term() {
        assert_eq!(match_query("sel").as_deref(), Some(r#""sel"*"#));
    }

    #[test]
    fn words_are_joined_with_a_space_so_that_all_of_them_must_match() {
        assert_eq!(
            match_query("  creme \t brul\n").as_deref(),
            Some(r#""creme"* "brul"*"#)
        );
    }

    #[test]
    fn empty_or_blank_input_gives_none() {
        for input in ["", "   ", "\t\n"] {
            assert_eq!(match_query(input), None, "{input:?}");
        }
    }

    #[test]
    fn fts5_syntax_characters_never_reach_the_expression() {
        for (input, expected) in [
            (r#"say "hi""#, r#""say"* "hi"*"#),
            ("sel*", r#""sel"*"#),
            ("(a OR b)", r#""a"* "OR"* "b"*"#),
            ("foo-bar", r#""foo"* "bar"*"#),
            ("col:value", r#""col"* "value"*"#),
            ("^start", r#""start"*"#),
            ("a + b", r#""a"* "b"*"#),
        ] {
            assert_eq!(match_query(input).as_deref(), Some(expected), "{input:?}");
        }
    }

    #[test]
    fn keywords_are_searched_as_plain_words() {
        assert_eq!(
            match_query("NEAR AND NOT").as_deref(),
            Some(r#""NEAR"* "AND"* "NOT"*"#)
        );
    }

    #[test]
    fn input_without_any_letter_or_digit_gives_none() {
        for input in [r#"""#, "*", "()", "- -", "...", "\"*\""] {
            assert_eq!(match_query(input), None, "{input:?}");
        }
    }

    #[test]
    fn letters_with_diacritics_and_other_scripts_are_kept() {
        assert_eq!(
            match_query("Crème brûlée 日本").as_deref(),
            Some(r#""Crème"* "brûlée"* "日本"*"#)
        );
    }
}
