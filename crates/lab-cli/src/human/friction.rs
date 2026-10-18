//! Groups player friction rows into clusters for the human-friction report.

use std::collections::{BTreeMap, BTreeSet};

use super::rows::Row;

/// Keys that name a person or a character and never leave a cluster sample.
pub const PERSONAL_KEYS: &[&str] = &[
    "account_name",
    "player_name",
    "entity_name",
    "target_entity_name",
];

/// Subject keys, in the order they are tried for a cluster's subject.
const SUBJECT_KEYS: &[&str] = &[
    "tag",
    "template_name",
    "item_id",
    "step_id",
    "mission_id",
    "region_tag",
    "dialog_id",
];

/// One group of player friction rows that share a signal, reason, world and subject.
#[derive(Debug, Clone, PartialEq)]
pub struct FrictionCluster {
    pub signal: String,
    pub reason: String,
    pub world: String,
    pub subject: String,
    /// Rows in the cluster.
    pub count: usize,
    /// Distinct players; a row without a `player_id` counts as its own player.
    pub players: usize,
    /// The earliest `timestamp` in the cluster, compared as text.
    pub first_seen: Option<String>,
    /// The latest `timestamp` in the cluster, compared as text.
    pub last_seen: Option<String>,
    /// The first row of the cluster with `PERSONAL_KEYS` removed.
    pub sample: Row,
}

/// Accumulates the rows of one cluster before it is turned into a [`FrictionCluster`].
struct Acc {
    count: usize,
    player_ids: BTreeSet<String>,
    anonymous: usize,
    first_seen: Option<String>,
    last_seen: Option<String>,
    sample: Row,
}

/// Clusters the player friction rows. Lab and unknown sessions are dropped (D-PC1).
///
/// Output is sorted by count, descending, then by key, so the order is stable.
pub fn cluster_friction(rows: &[Row]) -> Vec<FrictionCluster> {
    let mut groups: BTreeMap<(String, String, String, String), Acc> = BTreeMap::new();
    for row in rows {
        if row.str("signal").is_none() || row.str("cimmeria.session_kind") != Some("player") {
            continue;
        }
        let key = (
            row.text("signal").unwrap_or_else(|| "-".to_string()),
            row.text("reason").unwrap_or_else(|| "-".to_string()),
            row.text("world").unwrap_or_else(|| "-".to_string()),
            subject_of(row),
        );
        let timestamp = row.text("timestamp");
        let acc = groups.entry(key).or_insert_with(|| Acc {
            count: 0,
            player_ids: BTreeSet::new(),
            anonymous: 0,
            first_seen: None,
            last_seen: None,
            sample: strip_personal(row),
        });
        acc.count += 1;
        match row.text("player_id") {
            Some(id) => {
                acc.player_ids.insert(id);
            }
            None => acc.anonymous += 1,
        }
        if let Some(ts) = timestamp {
            if acc.first_seen.as_ref().is_none_or(|first| ts < *first) {
                acc.first_seen = Some(ts.clone());
            }
            if acc.last_seen.as_ref().is_none_or(|last| ts > *last) {
                acc.last_seen = Some(ts);
            }
        }
    }
    let mut clusters: Vec<FrictionCluster> = groups
        .into_iter()
        .map(|((signal, reason, world, subject), acc)| FrictionCluster {
            signal,
            reason,
            world,
            subject,
            count: acc.count,
            players: acc.player_ids.len() + acc.anonymous,
            first_seen: acc.first_seen,
            last_seen: acc.last_seen,
            sample: acc.sample,
        })
        .collect();
    clusters.sort_by_key(|c| std::cmp::Reverse(c.count));
    clusters
}

/// The first present subject key of a row, or `"-"`.
fn subject_of(row: &Row) -> String {
    SUBJECT_KEYS
        .iter()
        .find_map(|key| row.text(key))
        .unwrap_or_else(|| "-".to_string())
}

/// A copy of a row with every `PERSONAL_KEYS` entry removed.
fn strip_personal(row: &Row) -> Row {
    let mut sample = row.clone();
    for key in PERSONAL_KEYS {
        sample.0.remove(*key);
    }
    sample
}
