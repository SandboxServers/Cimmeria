//! Cutting text to a width.

/// `Limit-UatText`: `text` cut to `max` characters, ending in `...` when cut.
pub fn clip_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::clip_text;

    #[test]
    fn short_text_is_unchanged() {
        assert_eq!(clip_text("abc", 3), "abc");
    }

    #[test]
    fn long_text_is_cut_to_the_width_with_three_dots() {
        assert_eq!(clip_text("abcdefgh", 6), "abc...");
    }

    #[test]
    fn it_counts_characters_not_bytes() {
        assert_eq!(
            clip_text("\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}", 5),
            "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}"
        );
        assert_eq!(
            clip_text("\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}", 5),
            "\u{e9}\u{e9}..."
        );
    }
}
