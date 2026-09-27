//! The D-ORG10 text rules: the one implementation, for every organization
//! text field and every caller (cell methods, base methods, GM commands),
//! and for the social-systems campaign's player text (D-SS12: the chat
//! line, [`TextField::ChatText`]), which reuses them rather than forking.
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
//! # Rank names
//!
//! Free text (any script), but normalised like a name: ends trimmed and
//! every internal run of whitespace collapsed to one space; empty after
//! that is rejected.
//!
//! # Forbidden characters (D-ORG10, D-ORG23)
//!
//! Every field rejects C0/C1 controls (a newline is allowed in MOTD and
//! notes), every Unicode format character (general category `Cf`: bidi
//! marks and embeddings, zero-width characters, the soft hyphen, the
//! Mongolian vowel separator, invisible operators, tag characters and the
//! rest of the class) and the line and paragraph separators (`Zl`, `Zp`).
//! The `Cf` set is a fixed range table, [`FORMAT_RANGES`], so no Unicode
//! data dependency is needed.
//!
//! # Lone surrogates
//!
//! A `&str` cannot hold one, so this module never sees them: the `WSTRING`
//! decoders in `cimmeria-wire` reject invalid UTF-16 before text reaches
//! here, and report it as [`TextReject::LoneSurrogate`].

use std::fmt;

use super::limits::{
    MAX_CHAT_TEXT_UNITS, MAX_MAIL_BODY_UNITS, MAX_MAIL_RECIPIENT_UNITS, MAX_MAIL_SUBJECT_UNITS,
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
    /// A chat line (`sendPlayerCommunication` text, social-systems D-SS12).
    /// Free text, one line, not normalised.
    ChatText,
    /// A gate-mail subject (`sendMailMessage` CM 44, D-SS12): 1-128 units,
    /// one line, not normalised.
    MailSubject,
    /// A gate-mail body (CM 44, D-SS12): up to 1,000 units; the only mail
    /// field that may span lines.
    MailBody,
    /// One gate-mail recipient name as the client typed it (CM 44). The
    /// server resolves it against `sgw_player` (D-SS13) and echoes it back
    /// in `FailedRecipients`, so it is bounded like any other text.
    MailRecipient,
}

impl TextField {
    /// Shortest accepted length in UTF-16 units. MOTD and notes may be
    /// cleared; a name or a rank name may not be blank.
    pub fn min_units(self) -> usize {
        match self {
            TextField::Name => MIN_NAME_UNITS,
            TextField::RankName | TextField::MailSubject | TextField::MailRecipient => 1,
            TextField::Motd
            | TextField::Note
            | TextField::OfficerNote
            | TextField::ChatText
            | TextField::MailBody => 0,
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
            TextField::ChatText => MAX_CHAT_TEXT_UNITS,
            TextField::MailSubject => MAX_MAIL_SUBJECT_UNITS,
            TextField::MailBody => MAX_MAIL_BODY_UNITS,
            TextField::MailRecipient => MAX_MAIL_RECIPIENT_UNITS,
        }
    }

    /// MOTD, notes and a mail body may span lines; nothing else may.
    fn allows_newline(self) -> bool {
        matches!(
            self,
            TextField::Motd | TextField::Note | TextField::OfficerNote | TextField::MailBody
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            TextField::Name => "name",
            TextField::Motd => "motd",
            TextField::Note => "note",
            TextField::OfficerNote => "officer_note",
            TextField::RankName => "rank_name",
            TextField::ChatText => "chat_text",
            TextField::MailSubject => "mail_subject",
            TextField::MailBody => "mail_body",
            TextField::MailRecipient => "mail_recipient",
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
    /// A bidirectional mark, embedding, override or isolate (U+061C,
    /// U+200E-200F, U+202A-202E, U+2066-2069).
    Bidi(char),
    /// A zero-width character (U+200B-200D, U+2060, U+FEFF).
    ZeroWidth(char),
    /// Any other format character (general category `Cf`): the soft hyphen,
    /// U+180E, invisible operators U+2061-2064, tag characters
    /// U+E0000-E007F, and the rest of [`FORMAT_RANGES`].
    Format(char),
    /// A line or paragraph separator (U+2028, U+2029; categories `Zl`, `Zp`).
    LineSeparator(char),
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
            TextReject::Format(_) => "format_char",
            TextReject::LineSeparator(_) => "line_separator",
            TextReject::NameCharset(_) => "name_charset",
        }
    }
}

impl fmt::Display for TextReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextReject::TooShort { units, min } => {
                write!(f, "too short ({units} < {min} UTF-16 units)")
            }
            TextReject::TooLong { units, max } => {
                write!(f, "too long ({units} > {max} UTF-16 units)")
            }
            TextReject::LoneSurrogate => f.write_str("invalid UTF-16 (unpaired surrogate)"),
            TextReject::Control(c) => write!(f, "control character U+{:04X}", u32::from(*c)),
            TextReject::Bidi(c) => write!(f, "bidi control U+{:04X}", u32::from(*c)),
            TextReject::ZeroWidth(c) => {
                write!(f, "zero-width character U+{:04X}", u32::from(*c))
            }
            TextReject::Format(c) => write!(f, "format character U+{:04X}", u32::from(*c)),
            TextReject::LineSeparator(c) => {
                write!(f, "line separator U+{:04X}", u32::from(*c))
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

/// Unicode general category `Cf` (format characters), as inclusive
/// ranges. From the Unicode 15.1 `UnicodeData.txt` `Cf` entries, with the
/// tag block widened to the whole of U+E0000-E007F.
pub const FORMAT_RANGES: &[(char, char)] = &[
    ('\u{00AD}', '\u{00AD}'),
    ('\u{0600}', '\u{0605}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{06DD}', '\u{06DD}'),
    ('\u{070F}', '\u{070F}'),
    ('\u{0890}', '\u{0891}'),
    ('\u{08E2}', '\u{08E2}'),
    ('\u{180E}', '\u{180E}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2066}', '\u{206F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFF9}', '\u{FFFB}'),
    ('\u{110BD}', '\u{110BD}'),
    ('\u{110CD}', '\u{110CD}'),
    ('\u{13430}', '\u{1343F}'),
    ('\u{1BCA0}', '\u{1BCA3}'),
    ('\u{1D173}', '\u{1D17A}'),
    ('\u{E0000}', '\u{E007F}'),
];

fn is_format(c: char) -> bool {
    FORMAT_RANGES.iter().any(|&(lo, hi)| (lo..=hi).contains(&c))
}

fn is_bidi_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

fn is_zero_width(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}')
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
        if is_format(c) {
            return Err(TextReject::Format(c));
        }
        if matches!(c, '\u{2028}' | '\u{2029}') {
            return Err(TextReject::LineSeparator(c));
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

/// Trim both ends and collapse every internal run of whitespace to one
/// ASCII space.
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Validate `text` as `field` under D-ORG10.
///
/// Returns the text to store: the normalised text for [`TextField::Name`]
/// and [`TextField::RankName`], the input unchanged for every other field.
pub fn validate(field: TextField, text: &str) -> Result<String, TextReject> {
    check_forbidden(field, text)?;
    if field == TextField::RankName {
        let name = collapse_whitespace(text);
        check_length(field, &name)?;
        return Ok(name);
    }
    if field != TextField::Name {
        check_length(field, text)?;
        return Ok(text.to_owned());
    }
    // Charset before normalising, so a tab or a non-breaking space is
    // reported as itself rather than surviving as a word boundary.
    if let Some(c) = text.chars().find(|&c| !is_name_char(c)) {
        return Err(TextReject::NameCharset(c));
    }
    let name = collapse_whitespace(text);
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
