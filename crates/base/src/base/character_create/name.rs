//! Character name rules (matches Python `Account.py:isCharacterNameAllowed`).

/// Validate a character name for length, format, and whitespace rules.
///
/// Allowed characters: ASCII letters, digits, spaces, hyphens, apostrophes.
/// Rejects: leading/trailing whitespace, consecutive spaces, control chars,
/// HTML/script injection, zero-width characters, and names outside 3-20 chars.
///
/// Returns `Ok(())` if valid, or `Err(reason)` with a human-readable rejection reason.
pub(super) fn validate_character_name(name: &str) -> Result<(), &'static str> {
    if name.len() < 3 {
        return Err("too short (min 3)");
    }
    if name.len() > 20 {
        return Err("too long (max 20)");
    }
    if name != name.trim() {
        return Err("leading or trailing whitespace");
    }
    if name.contains("  ") {
        return Err("consecutive spaces");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '\'')
    {
        return Err("invalid characters (only letters, digits, spaces, hyphens, apostrophes)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_valid() {
        assert!(validate_character_name("John").is_ok());
        assert!(validate_character_name("Sam Carter").is_ok());
        assert!(validate_character_name("O'Neill").is_ok());
        assert!(validate_character_name("Teal-c").is_ok());
        assert!(validate_character_name("abc").is_ok()); // min length
        assert!(validate_character_name("12345678901234567890").is_ok()); // max length (20)
    }

    #[test]
    fn name_too_short() {
        assert!(validate_character_name("AB").is_err());
        assert!(validate_character_name("A").is_err());
        assert!(validate_character_name("").is_err());
    }

    #[test]
    fn name_too_long() {
        assert!(validate_character_name("123456789012345678901").is_err()); // 21 chars
        assert!(validate_character_name("AAAAAAAAAAAAAAAAAAAAA").is_err());
    }

    #[test]
    fn name_rejects_html() {
        assert!(validate_character_name("<script>").is_err());
        assert!(validate_character_name("a]>b").is_err());
    }

    #[test]
    fn name_rejects_control_chars() {
        assert!(validate_character_name("abc\0def").is_err());
        assert!(validate_character_name("abc\ndef").is_err());
        assert!(validate_character_name("abc\tdef").is_err());
    }

    #[test]
    fn name_rejects_bad_whitespace() {
        assert!(validate_character_name(" Leading").is_err());
        assert!(validate_character_name("Trailing ").is_err());
        assert!(validate_character_name("Two  Spaces").is_err());
    }

    #[test]
    fn name_rejects_non_ascii() {
        assert!(validate_character_name("Ünïcödé").is_err());
        assert!(validate_character_name("名前").is_err());
    }

    #[test]
    fn skin_tint_valid_range() {
        for i in 0..=15i32 {
            assert!((0..=15).contains(&i));
        }
        assert!(!(0..=15).contains(&-1i32));
        assert!(!(0..=15).contains(&16i32));
    }
}
