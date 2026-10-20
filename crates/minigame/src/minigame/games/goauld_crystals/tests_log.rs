//! Log hygiene: client-supplied text is capped and stripped of control
//! characters before it reaches a warning line.

use super::{loggable, LOG_TEXT_CAP};

#[test]
fn loggable_keeps_short_plain_text() {
    assert_eq!(loggable("playfieldclick"), "playfieldclick");
}

#[test]
fn loggable_caps_long_client_text() {
    let long = "a".repeat(LOG_TEXT_CAP * 4);
    let logged = loggable(&long);
    assert_eq!(logged.chars().count(), LOG_TEXT_CAP);
    assert_eq!(logged, "a".repeat(LOG_TEXT_CAP));
}

#[test]
fn loggable_replaces_control_characters() {
    assert_eq!(loggable("ab\ncd\r\tef\u{7}"), "ab?cd??ef?");
}

#[test]
fn loggable_caps_by_characters_not_bytes() {
    // Multi-byte characters count once, so the cap never splits a character.
    let wide = "é".repeat(LOG_TEXT_CAP + 10);
    assert_eq!(loggable(&wide).chars().count(), LOG_TEXT_CAP);
}
