//! The server-send half of the ability telemetry coverage gate (AB-C7).
//!
//! [`LEDGER_METHODS`] lists the server-to-client ability methods this
//! ledger writes an `abilities.wire` row for: the "server send row" column
//! of `docs/analysis/ability-mechanics/telemetry-coverage.md`.
//! `tools/telemetry-coverage/abilities.py` scans its `(index, "name")`
//! tuples; every other method of the script's set needs an exception there
//! with a reason. The tests below fail when a listed method has no layout
//! in the ledger's decoder, so the row would carry no payload fields.

/// Server-to-client ability methods with an `abilities.wire` row (flat
/// SGWPlayer client-method index, `.def` name).
pub(crate) const LEDGER_METHODS: &[(u16, &str)] = &[
    (1, "onSequence"),
    (12, "onTimerUpdate"),
    (14, "onEffectResults"),
    (19, "onStateFieldUpdate"),
    (20, "onStatUpdate"),
    // The respawn resync, grants, training, respec, GM changes and the
    // cell's player init (`origin` names the trigger); the base's
    // world-entry bundle writes the same rows itself (`base-world-entry`
    // `world_entry::map_loaded_wire_rows`).
    (21, "onStatBaseUpdate"),
    // The ability system's own feedback lines (refusals, no ally); other
    // systems' chat does not go through this ledger.
    (28, "onPlayerCommunication"),
    (101, "onKnownAbilitiesUpdate"),
    (121, "onErrorCode"),
    (141, "onAbilityTreeInfo"),
];

#[cfg(test)]
mod tests {
    use super::super::decode::{method_name, Decoded};
    use super::*;

    /// The smallest well-formed payload of each method (empty arrays).
    fn minimal(method: &str) -> Vec<u8> {
        vec![
            0;
            match method {
                "onSequence" => 26,
                "onTimerUpdate" => 21,
                "onEffectResults" => 21,
                "onStateFieldUpdate" | "onStatUpdate" | "onStatBaseUpdate" => 4,
                "onKnownAbilitiesUpdate" | "onAbilityTreeInfo" => 4,
                "onErrorCode" => 7,
                "onPlayerCommunication" => 10,
                other => panic!("no minimal payload for {other}"),
            }
        ]
    }

    /// Every listed method is named by the row and decoded into its own
    /// fields: neither `Other` (no layout) nor `Short` (a layout that does
    /// not fit its smallest payload).
    #[test]
    fn every_ledger_method_has_a_decoded_row() {
        for &(index, name) in LEDGER_METHODS {
            assert_eq!(method_name(index), name, "index {index}");
            let d = Decoded::parse(index, &minimal(name));
            assert!(
                !matches!(d, Decoded::Other | Decoded::Short),
                "{name}: {d:?}"
            );
        }
    }

    #[test]
    fn indices_match_the_wire_constants() {
        use crate::cell::client_methods::{
            being, combatant, communicator, player, spawnable_entity,
        };
        let want = [
            (spawnable_entity::ON_SEQUENCE, "onSequence"),
            (being::ON_TIMER_UPDATE, "onTimerUpdate"),
            (being::ON_EFFECT_RESULTS, "onEffectResults"),
            (being::ON_STATE_FIELD_UPDATE, "onStateFieldUpdate"),
            (combatant::ON_STAT_UPDATE, "onStatUpdate"),
            (combatant::ON_STAT_BASE_UPDATE, "onStatBaseUpdate"),
            (
                communicator::ON_PLAYER_COMMUNICATION,
                "onPlayerCommunication",
            ),
            (player::ON_KNOWN_ABILITIES_UPDATE, "onKnownAbilitiesUpdate"),
            (player::ON_ERROR_CODE, "onErrorCode"),
            (player::ON_ABILITY_TREE_INFO, "onAbilityTreeInfo"),
        ];
        assert_eq!(LEDGER_METHODS, &want[..]);
    }
}
