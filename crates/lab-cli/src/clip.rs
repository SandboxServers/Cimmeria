//! Cutting text to a width.

/// `Limit-UatText`: `text` cut to `max` UTF-16 code units (PowerShell's
/// `String.Length`), ending in `...` when cut. A character outside the Basic
/// Multilingual Plane (an emoji) counts 2. The cut never splits such a pair:
/// a character that does not fit whole is left out, so the result is at most
/// `max` units (PowerShell would leave half a pair there).
pub fn clip_text(text: &str, max: usize) -> String {
    if text.encode_utf16().count() <= max {
        return text.to_string();
    }
    let keep = max.saturating_sub(3);
    let mut used = 0;
    let mut out = String::new();
    for c in text.chars() {
        let units = c.len_utf16();
        if used + units > keep {
            break;
        }
        used += units;
        out.push(c);
    }
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

    #[test]
    fn it_counts_utf16_units_so_an_emoji_is_two() {
        // "ab" + U+1F600 + "cd" is 5 characters but 6 units.
        let text = "ab\u{1F600}cd";
        assert_eq!(clip_text(text, 6), text);
        assert_eq!(clip_text(text, 5), "ab...");
    }

    #[test]
    fn it_never_splits_a_surrogate_pair() {
        // "abc" + emoji + "def" is 8 units; the budget 7 keeps 4, and the
        // emoji would end at unit 5, so the cut is before it.
        let cut = clip_text("abc\u{1F600}def", 7);
        assert_eq!(cut, "abc...");
        assert!(cut.encode_utf16().count() <= 7);
    }

    #[test]
    fn an_emoji_that_ends_exactly_on_the_boundary_is_kept() {
        // "ab" + emoji + "cdefg" is 9 units; the budget 7 keeps 4 = "ab" + the emoji.
        let cut = clip_text("ab\u{1F600}cdefg", 7);
        assert_eq!(cut, "ab\u{1F600}...");
        assert_eq!(cut.encode_utf16().count(), 7);
    }

    #[test]
    fn ascii_budgets_are_unchanged() {
        let text = "abcdefghij";
        for max in 3..=12 {
            let cut = clip_text(text, max);
            assert_eq!(cut.chars().count(), text.len().min(max), "max {max}");
            if max < text.len() {
                assert_eq!(cut, format!("{}...", &text[..max - 3]));
            } else {
                assert_eq!(cut, text);
            }
        }
    }
}
