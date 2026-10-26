//! The category digest the presync checks cache copies against.
//!
//! The digest is SHA-256 over a fixed framing of the category id, version and
//! every record, so any byte, key, version or category change moves it.

use sha2::{Digest, Sha256};

/// SHA-256 of `b"CPD1" ‖ category_id LE ‖ version LE ‖ count u32 LE ‖
/// (key u32 LE ‖ len u32 LE ‖ bytes)*`.
///
/// The caller passes the entries in ascending key order. A `BTreeMap`
/// iterator does.
pub fn category_digest<'a>(
    category_id: u32,
    version: u32,
    entries: impl IntoIterator<Item = (u32, &'a [u8])>,
) -> [u8; 32] {
    let entries: Vec<(u32, &[u8])> = entries.into_iter().collect();
    let mut hasher = Sha256::new();
    hasher.update(b"CPD1");
    hasher.update(category_id.to_le_bytes());
    hasher.update(version.to_le_bytes());
    hasher.update((entries.len() as u32).to_le_bytes());
    for (key, bytes) in entries {
        hasher.update(key.to_le_bytes());
        hasher.update((bytes.len() as u32).to_le_bytes());
        hasher.update(bytes);
    }
    hasher.finalize().into()
}

/// A digest as 64 lower-case hex characters.
pub fn digest_hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Parses the form [`digest_hex`] writes: exactly 64 lower-case hex
/// characters. Anything else gives `None`.
pub fn parse_digest_hex(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    let (pairs, _) = bytes.as_chunks::<2>();
    for (slot, pair) in out.iter_mut().zip(pairs) {
        *slot = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(out)
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: &[(u32, &[u8])] = &[(1, b"alpha"), (2, b"bravo")];

    #[test]
    fn digest_changes_with_any_byte_key_version_or_category() {
        let base = category_digest(4, 10, KEYS.iter().copied());
        let flipped: &[(u32, &[u8])] = &[(1, b"alpho"), (2, b"bravo")];
        assert_ne!(base, category_digest(4, 10, flipped.iter().copied()));
        let rekeyed: &[(u32, &[u8])] = &[(1, b"alpha"), (3, b"bravo")];
        assert_ne!(base, category_digest(4, 10, rekeyed.iter().copied()));
        assert_ne!(base, category_digest(4, 11, KEYS.iter().copied()));
        assert_ne!(base, category_digest(5, 10, KEYS.iter().copied()));
        assert_eq!(base, category_digest(4, 10, KEYS.iter().copied()));
    }

    #[test]
    fn digest_hex_round_trips() {
        let digest = category_digest(4, 10, KEYS.iter().copied());
        let text = digest_hex(&digest);
        assert_eq!(text.len(), 64);
        assert!(text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        assert_eq!(parse_digest_hex(&text), Some(digest));
        assert_eq!(parse_digest_hex(&text.to_uppercase()), None);
        assert_eq!(parse_digest_hex(&text[..63]), None);
        assert_eq!(parse_digest_hex(&format!("{text}0")), None);
    }
}
