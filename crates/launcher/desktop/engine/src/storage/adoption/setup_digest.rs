//! Decodes the digests a setup rule and the file index name, refusing text that is
//! not 64 hex digits (`setup_rule_invalid`).
use super::rejection::{self, Rejection};
use super::*;

/// Decodes exactly 64 hex digits into a digest; `None` for any other text.
pub(super) fn unhex(hex: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    (hex.len() == 64).then_some(out)
}

/// The digest a setup rule's expected bytes name; undecodable text refuses the
/// rule at `path` (`setup_rule_invalid`).
pub(super) fn setup_bytes(hex: &str, path: &str) -> Result<[u8; 32], Error> {
    unhex(hex).ok_or_else(|| {
        rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("setup_rule_invalid").entry(path),
        )
    })
}
