//! Folder-relative tree comparison for the adoption checks (C-LF1). Pure: it reads
//! no files, so the light and verified checks share one rule and one log event.
use super::verification::Tree;
use std::collections::{BTreeMap, BTreeSet};
use tracing::Level;

/// The longest value a sample list may log, in bytes.
const SAMPLE_LIMIT: usize = 230;
/// The samples shown per list.
const SAMPLES: usize = 3;

/// The differences between an expected tree and an actual one.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct TreeDiff {
    /// Expected, absent from actual.
    pub missing: Vec<(String, u64, [u8; 32])>,
    /// Present in both, with a different size or digest.
    pub changed: Vec<Changed>,
    /// Actual, absent from expected.
    pub unexpected: Vec<(String, u64, [u8; 32])>,
    /// `(expected key, actual key)` for the same bytes under another spelling.
    pub case_only: Vec<(String, String)>,
}

/// A path whose bytes differ between the two trees.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Changed {
    pub path: String,
    pub expected: (u64, [u8; 32]),
    pub actual: (u64, [u8; 32]),
}

/// Pairs keys exactly first, then by `str::to_lowercase()` for the keys left over.
/// `presence_only` keys (matched case-insensitively) are never `changed`; a missing one is still `missing`.
/// Output vectors are sorted by key.
pub(super) fn diff(expected: &Tree, actual: &Tree, presence_only: &[&str]) -> TreeDiff {
    let presence: BTreeSet<String> = presence_only.iter().map(|key| key.to_lowercase()).collect();
    let pairs = pair_keys(expected, actual);
    let paired: BTreeSet<&str> = pairs.values().copied().collect();
    let mut out = TreeDiff::default();
    for (key, &(size, sha)) in expected {
        let Some(&other) = pairs.get(key.as_str()) else {
            out.missing.push((key.clone(), size, sha));
            continue;
        };
        let (actual_size, actual_sha) = actual[other];
        if (size, sha) == (actual_size, actual_sha) {
            if key != other {
                out.case_only.push((key.clone(), other.to_owned()));
            }
        } else if !presence.contains(&key.to_lowercase()) {
            out.changed.push(Changed {
                path: key.clone(),
                expected: (size, sha),
                actual: (actual_size, actual_sha),
            });
        }
    }
    for (key, &(size, sha)) in actual {
        if !paired.contains(key.as_str()) {
            out.unexpected.push((key.clone(), size, sha));
        }
    }
    out
}

/// Expected key to actual key: exact matches first, then one lowercase match per
/// leftover key, in key order. A key with no partner is left out of the map, so two
/// keys that fold together never panic; the extra one is reported as missing or unexpected.
fn pair_keys<'a>(expected: &'a Tree, actual: &'a Tree) -> BTreeMap<&'a str, &'a str> {
    let mut pairs: BTreeMap<&str, &str> = expected
        .keys()
        .filter(|key| actual.contains_key(*key))
        .map(|key| (key.as_str(), key.as_str()))
        .collect();
    let mut spare: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for key in actual.keys().filter(|key| !expected.contains_key(*key)) {
        spare
            .entry(key.to_lowercase())
            .or_default()
            .push(key.as_str());
    }
    for key in expected.keys().filter(|key| !actual.contains_key(*key)) {
        let partner = spare
            .get_mut(&key.to_lowercase())
            .filter(|keys| !keys.is_empty())
            .map(|keys| keys.remove(0));
        if let Some(other) = partner {
            pairs.insert(key.as_str(), other);
        }
    }
    pairs
}

impl TreeDiff {
    /// True when the folder cannot be accepted: a missing or a changed file.
    pub(super) fn blocking(&self) -> bool {
        !self.missing.is_empty() || !self.changed.is_empty()
    }

    /// True when the trees agree exactly, spelling included.
    pub(super) fn is_empty(&self) -> bool {
        self.missing.is_empty()
            && self.changed.is_empty()
            && self.unexpected.is_empty()
            && self.case_only.is_empty()
    }
}

/// The result of one adoption tree check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TreeOutcome {
    Matched,
    Accepted,
    Mismatch,
    Unrecorded,
}

impl TreeOutcome {
    /// The `outcome` field's value.
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Accepted => "accepted",
            Self::Mismatch => "mismatch",
            Self::Unrecorded => "unrecorded",
        }
    }
}

/// The fields of C-LF2, without `event`; pure so tests need no capture.
pub(super) fn tree_checked_fields(
    check: &str,
    outcome: TreeOutcome,
    diff: Option<&TreeDiff>,
) -> Vec<(&'static str, String)> {
    let mut fields = vec![
        ("check", check.to_owned()),
        ("outcome", outcome.code().to_owned()),
    ];
    let Some(diff) = diff else {
        return fields;
    };
    fields.extend([
        ("missing", diff.missing.len().to_string()),
        ("changed", diff.changed.len().to_string()),
        ("unexpected", diff.unexpected.len().to_string()),
        ("case_only", diff.case_only.len().to_string()),
    ]);
    let lists: [(&'static str, Vec<String>); 4] = [
        (
            "missing_paths",
            diff.missing
                .iter()
                .map(|(path, size, sha)| format!("{path} {size} {}", hex12(sha)))
                .collect(),
        ),
        (
            "changed_paths",
            diff.changed.iter().map(changed_sample).collect(),
        ),
        (
            "unexpected_paths",
            diff.unexpected
                .iter()
                .map(|(path, size, sha)| format!("{path} {size} {}", hex12(sha)))
                .collect(),
        ),
        (
            "case_paths",
            diff.case_only
                .iter()
                .map(|(expected, actual)| format!("{expected}>{actual}"))
                .collect(),
        ),
    ];
    for (name, samples) in lists {
        if let Some(value) = join_samples(&samples) {
            fields.push((name, value));
        }
    }
    fields
}

/// Emits C-LF2 at the level C-LF2 gives: INFO matched, WARN accepted and unrecorded,
/// ERROR mismatch. Counts are numbers; the sample lists are capped by `join_samples`.
pub(super) fn log_tree_checked(check: &str, outcome: TreeOutcome, diff: Option<&TreeDiff>) {
    let fields = tree_checked_fields(check, outcome, diff);
    let text = |key: &str| {
        fields
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.as_str())
    };
    let counts = diff.map(|d| {
        [
            d.missing.len(),
            d.changed.len(),
            d.unexpected.len(),
            d.case_only.len(),
        ]
        .map(|n| n as u64)
    });
    let count = |index: usize| counts.map(|c| c[index]);
    // `tracing` needs a constant level, so the one event body is a macro per level.
    macro_rules! emit {
        ($level:path) => {
            tracing::event!(
                target: "adoption",
                $level,
                event = "adoption_tree_checked",
                check = check,
                outcome = outcome.code(),
                missing = count(0),
                changed = count(1),
                unexpected = count(2),
                case_only = count(3),
                missing_paths = text("missing_paths"),
                changed_paths = text("changed_paths"),
                unexpected_paths = text("unexpected_paths"),
                case_paths = text("case_paths"),
                "adoption tree check: {}",
                outcome.code()
            )
        };
    }
    match outcome {
        TreeOutcome::Matched => emit!(Level::INFO),
        TreeOutcome::Accepted | TreeOutcome::Unrecorded => emit!(Level::WARN),
        TreeOutcome::Mismatch => emit!(Level::ERROR),
    }
}

/// `path expected>actual exp_h12>act_h12` for a changed path.
fn changed_sample(change: &Changed) -> String {
    let (expected_size, expected_sha) = change.expected;
    let (actual_size, actual_sha) = change.actual;
    format!(
        "{} {expected_size}>{actual_size} {}>{}",
        change.path,
        hex12(&expected_sha),
        hex12(&actual_sha)
    )
}

/// Up to `SAMPLES` samples joined by `"; "`. A sample that would pass `SAMPLE_LIMIT`
/// ends the list; a first sample that is too long is cut on a char boundary.
fn join_samples(samples: &[String]) -> Option<String> {
    let mut value = String::new();
    for sample in samples.iter().take(SAMPLES) {
        let next = if value.is_empty() {
            sample.clone()
        } else {
            format!("{value}; {sample}")
        };
        if next.len() <= SAMPLE_LIMIT {
            value = next;
        } else {
            if value.is_empty() {
                value = cut(&next, SAMPLE_LIMIT);
            }
            break;
        }
    }
    (!value.is_empty()).then_some(value)
}

/// The longest prefix of `text` within `limit` bytes that ends on a char boundary.
fn cut(text: &str, limit: usize) -> String {
    let mut end = limit.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

/// The first 12 lowercase hex digits of a SHA-256.
fn hex12(sha: &[u8; 32]) -> String {
    sha[..6].iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[path = "tree_diff_tests.rs"]
mod tests;
