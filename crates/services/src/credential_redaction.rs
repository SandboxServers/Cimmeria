//! Log-safe rendering of login credentials (SIDs, tickets, session keys).
//!
//! Credential values must never reach a log sink in full: disk logs, the
//! admin `/ws/logs` stream and SigNoz all retain them, and a harvested
//! Phase 1 SID or Phase 2 ticket is enough to hijack a pending login.
//! Log a [`CredentialPrefix`] under a `*_prefix` field
//! instead — enough to correlate one login's events, far too short to
//! replay.

use std::fmt;

/// Number of leading characters a [`CredentialPrefix`] reveals.
///
/// Six characters of a 20-char hex ticket leave 56 bits unknown; of a
/// 40-char alphanumeric SID, far more. Neither is forgeable from the
/// prefix.
pub(crate) const CREDENTIAL_PREFIX_LEN: usize = 6;

/// Displays at most the first [`CREDENTIAL_PREFIX_LEN`] characters of a
/// credential, followed by `…` when anything was cut.
///
/// Char-based, so it never panics on short or non-ASCII input — the
/// Phase 3 ticket is parsed from an unauthenticated UDP packet and can be
/// any length.
///
/// ```ignore
/// tracing::debug!(sid_prefix = %CredentialPrefix(&sid), "Phase 1 generated SID");
/// ```
pub(crate) struct CredentialPrefix<'a>(pub &'a str);

impl fmt::Display for CredentialPrefix<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut chars = self.0.chars();
        for c in chars.by_ref().take(CREDENTIAL_PREFIX_LEN) {
            write!(f, "{c}")?;
        }
        if chars.next().is_some() {
            f.write_str("…")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_credential_shows_only_the_prefix() {
        let ticket = "0123456789ABCDEF0123";
        let shown = CredentialPrefix(ticket).to_string();
        assert_eq!(shown, "012345…");
        assert!(!shown.contains(ticket));
    }

    #[test]
    fn credential_at_prefix_length_has_no_ellipsis() {
        assert_eq!(CredentialPrefix("ABCDEF").to_string(), "ABCDEF");
    }

    #[test]
    fn short_and_empty_input_does_not_panic() {
        assert_eq!(CredentialPrefix("AB").to_string(), "AB");
        assert_eq!(CredentialPrefix("").to_string(), "");
    }

    #[test]
    fn multibyte_input_is_cut_on_char_boundaries() {
        // Byte-slicing `&s[..6]` would panic here: 'é' is two bytes.
        assert_eq!(CredentialPrefix("éééééééé").to_string(), "éééééé…");
    }
}
