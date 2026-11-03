//! Lint: authored dialog text must not put a digit right after whitespace.
//!
//! The client runs a dialog screen's `Text` through a markup parser before it
//! draws it. On the lab client (2026-11-03) three Cimmeria-authored dialogs
//! drew their text up to a space and then printed `ERROR: syntax error,
//! unexpected SGW_WHITESPACE, expecting SGW_CHARACTER or SGW_COLON`:
//!
//! - 60150 stopped after `Prisoner` (next word `329`),
//! - 60110 stopped after `I can add` (next word `10`),
//! - 60151 carries `Prisoner 329` too.
//!
//! In all of them the failing space is followed by a digit; ordinary
//! punctuation (`. `, `: `, `, `) passes. That rule is inferred from those
//! three renders, NOT read from the client's lexer, so it is a conservative
//! lint over text we author, not a statement of what the grammar accepts.
//! Stock dialogs such as 5357 ("Level 16. Understood.") also have the shape;
//! nobody has recorded whether they render, so they are out of scope: this
//! only guards the ids Cimmeria serves itself. If the lexer is ever read and
//! the rule is narrower or wider, change [`digit_after_whitespace`] and the
//! self-test together.
//!
//! Spell the number out ("three twenty-nine", "ten slots") instead.

use std::path::PathBuf;

use super::DIALOG_OVERRIDES;

/// Cimmeria-authored dialogs live in 60000-65535: above every client-shipped
/// id (they stop at 6427) and within the 16-bit cooked-element bound.
const AUTHORED_IDS: std::ops::RangeInclusive<u32> = 60000..=65535;

/// The first whitespace-then-digit pair in `text`, as a short excerpt.
fn digit_after_whitespace(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    chars.windows(2).enumerate().find_map(|(i, w)| {
        (w[0].is_whitespace() && w[1].is_ascii_digit()).then(|| {
            let start = i.saturating_sub(12);
            let end = (i + 14).min(chars.len());
            chars[start..end].iter().collect()
        })
    })
}

/// The detector flags the three renders that failed and passes their fixes.
#[test]
fn detector_flags_the_known_failures_and_not_their_rewrites() {
    for bad in [
        "Prisoner 329 says there are still people",
        "I can add 10 slots to your vault",
        "up to 100 in all",
        "every\n10 minutes",
    ] {
        assert!(digit_after_whitespace(bad).is_some(), "must flag: {bad}");
    }
    for good in [
        "Prisoner three twenty-nine says there are still people",
        "I can add ten slots to your vault, up to one hundred in all",
        // A digit inside a word, or leading the text, is not the pattern.
        "Container A1 is here",
        "329 is first",
        "Col. Marsh: check them. Now.",
    ] {
        assert!(digit_after_whitespace(good).is_none(), "must pass: {good}");
    }
}

/// Every served override (screen text and button labels) is clean.
#[test]
fn served_dialog_overrides_have_no_digit_after_whitespace() {
    let mut failures = Vec::new();
    for ov in DIALOG_OVERRIDES {
        for screen in ov.screens {
            if let Some(at) = digit_after_whitespace(screen.text) {
                failures.push(format!(
                    "dialog {} screen {}: ...{at}...",
                    ov.dialog_id, screen.screen_id
                ));
            }
            for button in screen.buttons {
                if let Some(at) = digit_after_whitespace(button.text) {
                    failures.push(format!(
                        "dialog {} screen {} button {}: ...{at}...",
                        ov.dialog_id, screen.screen_id, button.button_id
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "the client markup parser rejects whitespace followed by a digit; spell the \
         number out:\n{}",
        failures.join("\n")
    );
}

/// The seeded rows for authored ids are clean too, so a seed edit that the
/// agreement tests would also catch cannot sneak a digit back in on its own.
#[test]
fn authored_dialog_seed_screens_have_no_digit_after_whitespace() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../db/resources/Dialogs/Seed/dialog_screens.sql");
    let seed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("seed file {} must be readable: {e}", path.display()));

    let mut checked = 0usize;
    let mut failures = Vec::new();
    for line in seed.lines() {
        let Some(rest) = line.strip_prefix(
            "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (",
        ) else {
            continue;
        };
        let Some(dialog_id) = rest
            .split(',')
            .next()
            .and_then(|s| s.trim().parse::<u32>().ok())
        else {
            continue;
        };
        if !AUTHORED_IDS.contains(&dialog_id) {
            continue;
        }
        // Text is the quoted literal; the trailing `, 0, 0);` has digits after
        // whitespace of its own and must not be linted.
        let (Some(open), Some(close)) = (line.find('\''), line.rfind('\'')) else {
            continue;
        };
        checked += 1;
        if let Some(at) = digit_after_whitespace(&line[open + 1..close]) {
            failures.push(format!("dialog {dialog_id}: ...{at}..."));
        }
    }
    assert!(
        checked >= 8,
        "expected the authored dialog rows (60100-60170) in the seed, saw {checked}"
    );
    assert!(
        failures.is_empty(),
        "authored dialog seed text has whitespace followed by a digit:\n{}",
        failures.join("\n")
    );
}
