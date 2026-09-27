//! The D-ORG10 text rules: the one implementation, for every organization
//! text field and every caller (cell methods, base methods, GM commands).
//!
//! Text is **rejected**, never truncated or silently cleaned, so the caller
//! can give the player feedback. Project policy, not recovered data, except
//! the name length, which is the client's own string (audit A-13).
//!
//! # Organization names
//!
//! A name is normalised (ends trimmed, internal runs of spaces collapsed to
//! one) and then restricted to ASCII letters, digits, space and `'` `-` `.`.
//! [`name_key`] case-folds the normalised name; uniqueness is per type on
//! that key (the `name_key` column).
//!
//! D-ORG10 also calls for NFC normalisation. It is deliberately not applied:
//! the character whitelist is ASCII-only, the same rule character names
//! use (`base/character_create.rs`), and every accepted string is therefore
//! already in NFC. Applying NFC first would be *weaker*, because a few
//! non-ASCII code points have canonical singleton decompositions to ASCII
//! (U+212A KELVIN SIGN becomes `K`); rejecting them keeps a name exactly
//! what the player typed. Widening the whitelist past ASCII would need NFC
//! (and a `unicode-normalization` dependency) first.
//!
//! # Lone surrogates
//!
//! A `&str` cannot hold one, so this module never sees them: the `WSTRING`
//! decoders in `cimmeria-wire` reject invalid UTF-16 before text reaches
//! here, and report it as [`TextReject::LoneSurrogate`].

use std::fmt;

use super::limits::{
    MAX_MOTD_UNITS, MAX_NAME_UNITS, MAX_NOTE_UNITS, MAX_OFFICER_NOTE_UNITS, MAX_RANK_NAME_UNITS,
    MIN_NAME_UNITS,
};

/// Which organization text a string is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextField {
    /// A Team or Command name (`onOrganizationCreation`, CM 94).
    Name,
    /// The message of the day (CM 13).
    Motd,
    /// A member's own roster note (CM 14).
    Note,
    /// An officer note on another member (CM 15).
    OfficerNote,
    /// A rank's display name (CM 17).
    RankName,
}

impl TextField {
    /// Shortest accepted length in UTF-16 units. MOTD and notes may be
    /// cleared; a name or a rank name may not be blank.
    pub fn min_units(self) -> usize {
        match self {
            TextField::Name => MIN_NAME_UNITS,
            TextField::RankName => 1,
            TextField::Motd | TextField::Note | TextField::OfficerNote => 0,
        }
    }

    /// Longest accepted length in UTF-16 units (D-ORG10).
    pub fn max_units(self) -> usize {
        match self {
            TextField::Name => MAX_NAME_UNITS,
            TextField::Motd => MAX_MOTD_UNITS,
            TextField::Note => MAX_NOTE_UNITS,
            TextField::OfficerNote => MAX_OFFICER_NOTE_UNITS,
            TextField::RankName => MAX_RANK_NAME_UNITS,
        }
    }

    /// MOTD and notes may span lines; names and rank names may not.
    fn allows_newline(self) -> bool {
        matches!(
            self,
            TextField::Motd | TextField::Note | TextField::OfficerNote
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            TextField::Name => "name",
            TextField::Motd => "motd",
            TextField::Note => "note",
            TextField::OfficerNote => "officer_note",
            TextField::RankName => "rank_name",
        }
    }
}

/// Why a text was rejected. `reason()` is the stable log value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextReject {
    /// Shorter than the field's minimum (after normalisation, for a name).
    TooShort { units: usize, min: usize },
    /// Longer than the field's cap (after normalisation, for a name).
    TooLong { units: usize, max: usize },
    /// The `WSTRING` held an unpaired UTF-16 surrogate.
    LoneSurrogate,
    /// A C0 or C1 control character (a newline is allowed in MOTD and notes).
    Control(char),
    /// A bidirectional embedding, override or isolate (U+202A-202E,
    /// U+2066-2069).
    Bidi(char),
    /// A zero-width character (U+200B-200D, U+FEFF).
    ZeroWidth(char),
    /// A name character outside ASCII letters, digits, space, `'`, `-`, `.`.
    NameCharset(char),
}

impl TextReject {
    pub fn reason(&self) -> &'static str {
        match self {
            TextReject::TooShort { .. } => "too_short",
            TextReject::TooLong { .. } => "too_long",
            TextReject::LoneSurrogate => "lone_surrogate",
            TextReject::Control(_) => "control_char",
            TextReject::Bidi(_) => "bidi_control",
            TextReject::ZeroWidth(_) => "zero_width",
            TextReject::NameCharset(_) => "name_charset",
        }
    }
}

impl fmt::Display for TextReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextReject::TooShort { units, min } => {
                write!(f, "too short ({units} < {min} characters)")
            }
            TextReject::TooLong { units, max } => {
                write!(f, "too long ({units} > {max} characters)")
            }
            TextReject::LoneSurrogate => f.write_str("invalid UTF-16 (unpaired surrogate)"),
            TextReject::Control(c) => write!(f, "control character U+{:04X}", u32::from(*c)),
            TextReject::Bidi(c) => write!(f, "bidi control U+{:04X}", u32::from(*c)),
            TextReject::ZeroWidth(c) => {
                write!(f, "zero-width character U+{:04X}", u32::from(*c))
            }
            TextReject::NameCharset(c) => write!(
                f,
                "'{}' is not allowed (letters, digits, spaces, ' - . only)",
                c.escape_default()
            ),
        }
    }
}

impl std::error::Error for TextReject {}

fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn is_zero_width(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200D}' | '\u{FEFF}')
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, ' ' | '\'' | '-' | '.')
}

/// The characters every field rejects. `char::is_control` is exactly the
/// C0 set, DEL and the C1 set (general category `Cc`).
fn check_forbidden(field: TextField, text: &str) -> Result<(), TextReject> {
    for c in text.chars() {
        if c == '\n' && field.allows_newline() {
            continue;
        }
        if c.is_control() {
            return Err(TextReject::Control(c));
        }
        if is_bidi_control(c) {
            return Err(TextReject::Bidi(c));
        }
        if is_zero_width(c) {
            return Err(TextReject::ZeroWidth(c));
        }
    }
    Ok(())
}

fn check_length(field: TextField, text: &str) -> Result<(), TextReject> {
    let units = text.encode_utf16().count();
    if units < field.min_units() {
        return Err(TextReject::TooShort {
            units,
            min: field.min_units(),
        });
    }
    if units > field.max_units() {
        return Err(TextReject::TooLong {
            units,
            max: field.max_units(),
        });
    }
    Ok(())
}

/// Trim both ends and collapse every internal run of spaces to one.
fn normalise_name(text: &str) -> String {
    text.split(' ')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Validate `text` as `field` under D-ORG10.
///
/// Returns the text to store: the normalised name for [`TextField::Name`],
/// the input unchanged for every other field.
pub fn validate(field: TextField, text: &str) -> Result<String, TextReject> {
    check_forbidden(field, text)?;
    if field != TextField::Name {
        check_length(field, text)?;
        return Ok(text.to_owned());
    }
    // Charset before normalising, so a tab or a non-breaking space is
    // reported as itself rather than surviving as a word boundary.
    if let Some(c) = text.chars().find(|&c| !is_name_char(c)) {
        return Err(TextReject::NameCharset(c));
    }
    let name = normalise_name(text);
    check_length(field, &name)?;
    Ok(name)
}

/// The uniqueness key for a name [`validate`] returned: ASCII case-folded.
///
/// Uniqueness is per organization type on this key (the `name_key` column),
/// so "Tau'ri Command" and "TAU'RI  command" collide.
pub fn name_key(normalised: &str) -> String {
    normalised.to_ascii_lowercase()
}
