//! The per-category check behind `cooked-pak-tool check` (LP-04). It asks how
//! the player's stock `SourceCache` copy lines up with the served category:
//! which served keys the source lacks, and which unchanged keys hold other
//! bytes there. Those are the keys the composed PAK would get wrong.

use std::collections::{BTreeMap, HashMap};

use cimmeria_cooked_pak::catalog::CategoryBody;
use cimmeria_cooked_pak::decide::{decide, Action};

/// One row of `cooked-pak-tool check`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRow {
    /// The category id.
    pub category_id: u32,
    /// The PAK file name.
    pub pak: String,
    /// The version the server serves.
    pub served_version: u32,
    /// The source PAK's version, `None` when it has no `MetaData`.
    pub source_version: Option<u32>,
    /// Every key the served category lists.
    pub keys: u32,
    /// Keys whose bytes the body carries in full.
    pub changed: u32,
    /// Unchanged keys the source PAK lacks.
    pub base_missing: u32,
    /// Unchanged keys whose source bytes differ from the served bytes.
    pub base_differs: u32,
    /// What the presync would do from an empty cache (the source copy decides
    /// the client's startup state), per [`decide`].
    pub action: Action,
}

impl CheckRow {
    /// Whether the source base lines up: no unchanged key missing or differing.
    pub fn base_ok(&self) -> bool {
        self.base_missing == 0 && self.base_differs == 0
    }
}

/// Check one served category against the source PAK's records.
///
/// `served` is the server's full record map for the category, and `source`
/// the source PAK's `_<key>` entries. Keys the body carries in full are
/// skipped: only the unchanged keys are read from the base.
pub fn check_category(
    category_id: u32,
    pak: &str,
    body: &CategoryBody,
    served: &HashMap<u32, Vec<u8>>,
    source: &BTreeMap<u32, Vec<u8>>,
    source_version: Option<u32>,
) -> CheckRow {
    let mut base_missing = 0;
    let mut base_differs = 0;
    for key in &body.keys {
        if body.changed.contains_key(key) {
            continue;
        }
        match source.get(key) {
            None => base_missing += 1,
            Some(bytes) => {
                if served.get(key).map(Vec::as_slice) != Some(bytes.as_slice()) {
                    base_differs += 1;
                }
            }
        }
    }
    CheckRow {
        category_id,
        pak: pak.to_string(),
        served_version: body.version,
        source_version,
        keys: body.keys.len() as u32,
        changed: body.changed.len() as u32,
        base_missing,
        base_differs,
        action: decide(None, source_version, body.version),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATEGORY: u32 = 4;
    const PAK: &str = "CookedDataItems.pak";

    /// A body that lists keys 1..=5 and carries key 2 in full.
    fn body(version: u32) -> CategoryBody {
        CategoryBody {
            category_id: CATEGORY,
            version,
            keys: vec![1, 2, 3, 4, 5],
            changed: BTreeMap::from([(2, b"changed".to_vec())]),
            digest: [0; 32],
        }
    }

    /// The server's full record map: the same five records, made up.
    fn served() -> HashMap<u32, Vec<u8>> {
        HashMap::from([
            (1, b"one".to_vec()),
            (2, b"changed".to_vec()),
            (3, b"three".to_vec()),
            (4, b"four".to_vec()),
            (5, b"five".to_vec()),
        ])
    }

    #[test]
    fn check_counts_missing_and_differing_base_entries() {
        // Key 2 is changed, so its absence from the source is not a gap. Keys
        // 4 and 5 are absent (2 missing) and key 3 holds other bytes (1 differs).
        let source = BTreeMap::from([
            (1, b"one".to_vec()),
            (2, b"stock".to_vec()),
            (3, b"old".to_vec()),
        ]);
        let row = check_category(CATEGORY, PAK, &body(7), &served(), &source, Some(6));
        assert_eq!(row.keys, 5);
        assert_eq!(row.changed, 1);
        assert_eq!(row.base_missing, 2);
        assert_eq!(row.base_differs, 1);
        assert!(!row.base_ok());
    }

    #[test]
    fn a_matching_base_reports_zero_and_action_write() {
        let source = BTreeMap::from([
            (1, b"one".to_vec()),
            (3, b"three".to_vec()),
            (4, b"four".to_vec()),
            (5, b"five".to_vec()),
        ]);
        let row = check_category(CATEGORY, PAK, &body(7), &served(), &source, Some(6));
        assert_eq!(row.base_missing, 0);
        assert_eq!(row.base_differs, 0);
        assert!(row.base_ok());
        assert_eq!(row.action, Action::Write);
    }
}
