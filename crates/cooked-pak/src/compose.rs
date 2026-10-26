//! Rebuilding a category from a stock PAK and the server's changed records.
//!
//! The client's cache copy is the stock PAK with the server's overrides laid
//! over it. [`compose`] does that layering, then checks the result against the
//! digest the server published, so a copy that does not match the server is
//! never written.

use std::collections::BTreeMap;

use crate::catalog::CategoryBody;
use crate::digest::category_digest;

/// Why a category could not be composed.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ComposeError {
    /// Some listed keys are in neither the changed records nor the base.
    #[error("{missing} keys missing from the base, first key {first}")]
    BaseMissing {
        /// How many listed keys the base lacks.
        missing: u32,
        /// The first such key, in ascending order.
        first: u32,
    },
    /// The composed records do not hash to the published digest.
    #[error("composed digest does not match the published digest")]
    DigestMismatch,
}

/// Lays the body's changed records over `base` and checks the digest.
///
/// For each key in `body.keys` the result takes `changed[key]`, else
/// `base[key]`. Base keys that the body does not list are dropped.
pub fn compose(
    body: &CategoryBody,
    base: &BTreeMap<u32, Vec<u8>>,
) -> Result<BTreeMap<u32, Vec<u8>>, ComposeError> {
    let mut out = BTreeMap::new();
    let mut missing: u32 = 0;
    let mut first = None;
    for &key in &body.keys {
        match body.changed.get(&key).or_else(|| base.get(&key)) {
            Some(bytes) => {
                out.insert(key, bytes.clone());
            }
            None => {
                missing += 1;
                first.get_or_insert(key);
            }
        }
    }
    if let Some(first) = first {
        return Err(ComposeError::BaseMissing { missing, first });
    }
    let digest = category_digest(
        body.category_id,
        body.version,
        out.iter().map(|(key, bytes)| (*key, bytes.as_slice())),
    );
    if digest != body.digest {
        return Err(ComposeError::DigestMismatch);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATEGORY: u32 = 4;
    const VERSION: u32 = 5;

    fn digest_of(records: &BTreeMap<u32, Vec<u8>>) -> [u8; 32] {
        category_digest(
            CATEGORY,
            VERSION,
            records.iter().map(|(key, bytes)| (*key, bytes.as_slice())),
        )
    }

    fn body(keys: &[u32], changed: &[(u32, &[u8])], digest: [u8; 32]) -> CategoryBody {
        CategoryBody {
            category_id: CATEGORY,
            version: VERSION,
            keys: keys.to_vec(),
            changed: changed
                .iter()
                .map(|(key, bytes)| (*key, bytes.to_vec()))
                .collect(),
            digest,
        }
    }

    fn base() -> BTreeMap<u32, Vec<u8>> {
        BTreeMap::from([
            (1, b"one".to_vec()),
            (2, b"two".to_vec()),
            (3, b"three".to_vec()),
            (9, b"extra".to_vec()),
        ])
    }

    #[test]
    fn compose_takes_changed_over_base_and_drops_extra_base_keys() {
        let expected = BTreeMap::from([
            (1, b"one".to_vec()),
            (2, b"CHANGED".to_vec()),
            (3, b"three".to_vec()),
        ]);
        let body = body(&[1, 2, 3], &[(2, b"CHANGED")], digest_of(&expected));
        assert_eq!(compose(&body, &base()), Ok(expected));
    }

    #[test]
    fn compose_reports_missing_base_keys() {
        let base = BTreeMap::from([(1, b"one".to_vec())]);
        let body = body(&[1, 2, 3], &[], [0; 32]);
        assert_eq!(
            compose(&body, &base),
            Err(ComposeError::BaseMissing {
                missing: 2,
                first: 2
            })
        );
    }

    #[test]
    fn compose_rejects_a_wrong_digest() {
        let body = body(&[1, 2, 3], &[], [0; 32]);
        assert_eq!(compose(&body, &base()), Err(ComposeError::DigestMismatch));
    }
}
