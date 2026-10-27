//! Generated account names (contract C7). A new account takes its name from
//! the provider's display name, cleaned to the account-name pattern, with a
//! random suffix so the name is likely free. Every result matches
//! `^[A-Za-z0-9_-]{3,20}$`.
//!
//! Uniqueness is `lower(account_name)`, and the repository checks it on
//! insert; this module only proposes candidates.

use rand::{Rng, RngExt};

/// The longest base kept from a display name, before the suffix.
const MAX_BASE_LEN: usize = 15;

/// The shortest name the pattern allows.
const MIN_NAME_LEN: usize = 3;

/// The base used when the display name gives nothing usable.
const FALLBACK_BASE: &str = "player";

/// Random `{base}_{dddd}` tries before the `player{dddddddd}` fallback.
const SUFFIX_TRIES: usize = 8;

/// Prefixes (ASCII case-insensitive) that a generated base must not start
/// with, so players cannot pose as staff or systems.
pub const RESERVED_PREFIXES: [&str; 10] = [
    "gm", "admin", "test", "lab", "cimmeria", "system", "dev", "mod", "staff", "support",
];

/// Turns a display name into the base of an account name.
///
/// Keeps the ASCII `[A-Za-z0-9_-]` characters, at most 15 of them. The base
/// becomes `player` when it is shorter than 3 characters or starts with a
/// reserved prefix.
pub fn base_from_display(display: &str) -> String {
    let base: String = display
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(MAX_BASE_LEN)
        .collect();
    let lower = base.to_ascii_lowercase();
    let reserved = RESERVED_PREFIXES.iter().any(|p| lower.starts_with(p));
    if base.len() < MIN_NAME_LEN || reserved {
        FALLBACK_BASE.to_string()
    } else {
        base
    }
}

/// The candidate names for one base, in the order to try them: eight
/// `{base}_{dddd}` names with random digits, then one `player{dddddddd}`.
pub fn candidates(base: &str, rng: &mut impl Rng) -> Vec<String> {
    let mut out: Vec<String> = (0..SUFFIX_TRIES)
        .map(|_| format!("{base}_{:04}", rng.random_range(0..10_000u32)))
        .collect();
    out.push(format!("player{:08}", rng.random_range(0..100_000_000u32)));
    out
}

/// Whether `s` matches `^[A-Za-z0-9_-]{3,20}$`. Checked by hand, without a
/// regex crate.
pub fn is_valid_account_name(s: &str) -> bool {
    let len = s.len();
    (3..=20).contains(&len)
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// A random display name of 0 to 30 characters from a mixed alphabet,
    /// including non-ASCII letters and punctuation.
    fn random_display(rng: &mut StdRng) -> String {
        const ALPHABET: &[char] = &[
            'a',
            'Z',
            '0',
            '9',
            '_',
            '-',
            ' ',
            '.',
            '!',
            'é',
            '\u{1F600}',
            '中',
        ];
        let len = rng.random_range(0..=30usize);
        (0..len)
            .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())])
            .collect()
    }

    #[test]
    fn names_match_the_account_pattern() {
        let mut rng = StdRng::seed_from_u64(0x0C17_0014);
        for _ in 0..1000 {
            let display = random_display(&mut rng);
            let base = base_from_display(&display);
            for name in candidates(&base, &mut rng) {
                assert!(is_valid_account_name(&name), "bad name {name:?}");
            }
        }
    }

    #[test]
    fn reserved_and_short_names_become_player() {
        let mut rng = StdRng::seed_from_u64(1);
        for display in ["GM_Bob", "admin", "x", "\u{1F600}", "Support Team"] {
            let base = base_from_display(display);
            assert_eq!(base, "player", "display {display:?}");
            let names = candidates(&base, &mut rng);
            assert!(
                names.iter().any(|n| n.starts_with("player_")),
                "no player_dddd candidate for {display:?}"
            );
        }
        assert_eq!(RESERVED_PREFIXES.len(), 10);
        for p in RESERVED_PREFIXES {
            assert_eq!(base_from_display(&format!("{p}Fan")), "player", "{p}");
            assert_eq!(
                base_from_display(&format!("{}Fan", p.to_ascii_uppercase())),
                "player",
                "{p} upper"
            );
        }
        assert_eq!(base_from_display("Hero_gm"), "Hero_gm");
        assert_eq!(base_from_display("Hero_Ann"), "Hero_Ann");
    }

    #[test]
    fn base_keeps_at_most_fifteen_ascii_name_characters() {
        assert_eq!(
            base_from_display("Abcdefghijklmnopqrstuvwxyz"),
            "Abcdefghijklmno"
        );
        assert_eq!(base_from_display("A-b_c d.e"), "A-b_cde");
    }

    #[test]
    fn candidate_shapes_and_pattern_bounds() {
        let mut rng = StdRng::seed_from_u64(7);
        let names = candidates("Fifteen_Chars_X", &mut rng);
        assert_eq!(names.len(), 9);
        assert!(names[..8]
            .iter()
            .all(|n| n.len() == 20 && n.starts_with("Fifteen_Chars_X_")));
        assert!(names[8].starts_with("player") && names[8].len() == 14);
        assert!(names.iter().all(|n| is_valid_account_name(n)));
    }

    #[test]
    fn account_name_pattern_bounds() {
        assert!(is_valid_account_name("abc"));
        assert!(is_valid_account_name(&"a".repeat(20)));
        assert!(!is_valid_account_name("ab"));
        assert!(!is_valid_account_name(&"a".repeat(21)));
        assert!(!is_valid_account_name("a b c"));
        assert!(!is_valid_account_name("abc."));
        assert!(!is_valid_account_name("\u{e9}tt\u{e9}"));
    }
}
