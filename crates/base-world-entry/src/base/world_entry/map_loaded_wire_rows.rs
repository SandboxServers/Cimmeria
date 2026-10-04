//! The `abilities.wire` rows for the ability methods the world-entry
//! bundle carries (ability-mechanics AB-C7: full accounting of
//! `onStatBaseUpdate`, `onAbilityTreeInfo` and `onKnownAbilitiesUpdate`).
//!
//! The `mapLoaded` body is built at the base (`build_map_loaded_body`), not
//! routed through the cell's wire ledger, so the base writes the rows
//! itself, once the bundle's fragments are on the socket: one row per
//! ability method in the bundle, `event = client_sent`, `origin =
//! world_entry`, with the bundle's Mercury seq range. The payload fields
//! come from the same `PlayerLoadData` and the same stat builder
//! (`world_entry_stat_args`) the bundle was built from, and use the cell
//! ledger's field names: `state_field`; `stat_count`, `stats`
//! (`stat_id:cur/max,...`); `ability_count`, `ability_ids`; `tree_lists`,
//! `tree_sizes`, `tree_total`.

use std::fmt::Write as _;

use crate::mercury::method_idx;
use crate::mercury::{world_entry_stat_args, PlayerLoadData, WORLD_ENTRY_STATE_FIELD};

/// `origin` of these rows.
pub(super) const ORIGIN_WORLD_ENTRY: &str = "world_entry";

/// Ability ids a row lists (the count is exact), as the cell ledger does.
const IDS_LISTED: usize = 64;

/// The ability-set methods the `mapLoaded` body carries, in body order.
pub(super) const WORLD_ENTRY_ABILITY_METHODS: &[(u16, &str)] = &[
    (method_idx::ON_STATE_FIELD_UPDATE, "onStateFieldUpdate"),
    (method_idx::ON_STAT_UPDATE, "onStatUpdate"),
    (method_idx::ON_STAT_BASE_UPDATE, "onStatBaseUpdate"),
    (method_idx::ON_ABILITY_TREE_INFO, "onAbilityTreeInfo"),
    (
        method_idx::ON_KNOWN_ABILITIES_UPDATE,
        "onKnownAbilitiesUpdate",
    ),
];

/// `(count, "stat_id:cur/max,...")` of a `StatUpdateList` payload
/// (`count:u32`, then `stat_id, min, cur, max` as `i32`s). A truncated
/// payload lists what it holds.
fn stat_summary(args: &[u8]) -> (u32, String) {
    let at = |o: usize| {
        args.get(o..o + 4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let count = at(0).map_or(0, |c| c as u32);
    let mut out = String::new();
    for i in 0..count as usize {
        let base = 4 + i * 16;
        let (Some(id), Some(cur), Some(max)) = (at(base), at(base + 8), at(base + 12)) else {
            break;
        };
        if !out.is_empty() {
            out.push(',');
        }
        let _ = write!(out, "{id}:{cur}/{max}");
    }
    (count, out)
}

/// Write the rows for a world-entry bundle sent to `entity_id`'s client in
/// packets `seq_first..=seq_last`.
pub(super) fn log_world_entry_ability_sends(
    entity_id: u32,
    account_id: Option<u32>,
    data: &PlayerLoadData,
    seq_first: u32,
    seq_last: u32,
) {
    if !tracing::enabled!(target: "abilities.wire", tracing::Level::DEBUG) {
        return;
    }
    for &(method_index, method) in WORLD_ENTRY_ABILITY_METHODS {
        macro_rules! row {
            ($($extra:tt)*) => {
                tracing::debug!(
                    target: "abilities.wire",
                    event = "client_sent",
                    stage = "wire",
                    origin = ORIGIN_WORLD_ENTRY,
                    method,
                    method_index,
                    entity_id,
                    recipient_id = entity_id,
                    account_id,
                    player_id = data.player_id,
                    mercury_seq_first = seq_first,
                    mercury_seq_last = seq_last,
                    $($extra)*
                )
            };
        }
        if method_index == method_idx::ON_KNOWN_ABILITIES_UPDATE {
            let ids = data
                .abilities
                .iter()
                .take(IDS_LISTED)
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            row!(
                ability_count = data.abilities.len(),
                ability_ids = ids.as_str(),
                "world-entry bundle sent: onKnownAbilitiesUpdate"
            );
        } else if method_index == method_idx::ON_ABILITY_TREE_INFO {
            let trees = &data.ability_tree.trees;
            let sizes = trees
                .iter()
                .map(|t| t.len().to_string())
                .collect::<Vec<_>>()
                .join("/");
            row!(
                tree_lists = trees.len(),
                tree_sizes = sizes.as_str(),
                tree_total = trees.iter().map(Vec::len).sum::<usize>(),
                "world-entry bundle sent: onAbilityTreeInfo"
            );
        } else if method_index == method_idx::ON_STATE_FIELD_UPDATE {
            row!(
                state_field = WORLD_ENTRY_STATE_FIELD,
                "world-entry bundle sent: onStateFieldUpdate"
            );
        } else {
            let (current, base) = world_entry_stat_args(data);
            let args = if method_index == method_idx::ON_STAT_BASE_UPDATE {
                base
            } else {
                current
            };
            let (count, stats) = stat_summary(&args);
            row!(
                stat_count = count,
                stats = stats.as_str(),
                "world-entry bundle sent: stat update"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;

    /// Every ability method of the bundle gets its row, with the trigger,
    /// the seq range and the payload summary.
    #[test]
    fn every_ability_method_of_the_bundle_gets_a_row() {
        let capture = LogCapture::install();
        let mut data = crate::base::world_entry::methods::default_player_load_data();
        data.player_id = 77;
        data.abilities = vec![597, 1646];
        data.ability_tree.trees = [vec![1, 2, 3], vec![4], vec![]];
        log_world_entry_ability_sends(9001, Some(5), &data, 120, 123);
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.target == "abilities.wire" && c.has_field("event", "client_sent"))
            .collect();
        assert_eq!(rows.len(), WORLD_ENTRY_ABILITY_METHODS.len(), "{rows:#?}");
        for (row, (_, method)) in rows.iter().zip(WORLD_ENTRY_ABILITY_METHODS) {
            assert!(row.has_field("method", method), "{row:#?}");
            assert!(row.has_field("origin", "world_entry"));
            assert!(row.has_field("player_id", "77"));
            assert!(row.has_field("account_id", "5"));
            assert!(row.has_field("mercury_seq_first", "120"));
            assert!(row.has_field("mercury_seq_last", "123"));
        }
        let known = rows
            .iter()
            .find(|r| r.has_field("method", "onKnownAbilitiesUpdate"))
            .unwrap();
        assert!(known.has_field("ability_count", "2"));
        assert!(known.has_field("ability_ids", "597,1646"));
        let tree = rows
            .iter()
            .find(|r| r.has_field("method", "onAbilityTreeInfo"))
            .unwrap();
        assert!(tree.has_field("tree_sizes", "3/1/0"));
        assert!(tree.has_field("tree_total", "4"));
        let state = rows
            .iter()
            .find(|r| r.has_field("method", "onStateFieldUpdate"))
            .unwrap();
        assert!(state.has_field("state_field", "0"), "{state:#?}");
        // The stat rows read back what the bundle's builder sends.
        let (current, base) = world_entry_stat_args(&data);
        for (method, args) in [("onStatUpdate", current), ("onStatBaseUpdate", base)] {
            let row = rows.iter().find(|r| r.has_field("method", method)).unwrap();
            let (count, stats) = stat_summary(&args);
            assert!(count > 0, "fixture: the archetype has stats");
            assert!(row.has_field("stat_count", &count.to_string()), "{row:#?}");
            assert!(row.has_field("stats", &stats), "{row:#?}");
        }
    }
}
