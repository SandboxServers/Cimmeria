//! Player vitals telemetry (`target: "vitals"`).
//!
//! A 2026-09-29 colo playtest (a player dying "too fast", then regenerating
//! oddly) could not be read from SigNoz: nothing recorded a player's health
//! or focus, so the only evidence was the player's own screenshots. This
//! module is the one place that writes a player's pools to the log:
//!
//! - [`log_damage_taken`]: one row per hit a player takes through the ability
//!   pipeline (`abilities::damage_apply`), with the pools before and after.
//! - [`log_combat_sample`]: the cell's 2 s tick (`ticks::vitals`) samples
//!   every living player with a non-empty threat set, so a DoT or script
//!   drain that bypasses the hit pipeline is still visible.
//! - The regen tick's `regen_started` / `regen_stopped` rows reuse
//!   [`Vitals`] for their pool fields.
//!
//! Every row is **DEBUG** (exported: `OTEL_FILTER` has `vitals=debug`) and
//! players only: NPC pools are already on `npc_ai.tick`. Volume, per
//! instrumentation-discipline Rule 4: a fighting player costs one sample row
//! per 2 s (1,800 rows/hour) plus one row per hit taken; an idle player costs
//! nothing. No metric, so no label cardinality: `player_id` and `entity_id`
//! are log fields only.

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::stats::{StatList, FOCUS, HEALTH};

use crate::cell::space_manager::SpaceManager;

/// Health and focus, current and max. `0/0` for a pool the entity lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Vitals {
    pub health: i32,
    pub health_max: i32,
    pub focus: i32,
    pub focus_max: i32,
}

impl Vitals {
    /// Read the two pools from a stat list.
    pub fn of(stats: &StatList) -> Self {
        let (health, health_max) = stats.get(HEALTH).map_or((0, 0), |s| (s.cur, s.max));
        let (focus, focus_max) = stats.get(FOCUS).map_or((0, 0), |s| (s.cur, s.max));
        Self {
            health,
            health_max,
            focus,
            focus_max,
        }
    }
}

/// `Some(vitals)` when `entity_id` is a player, for the pre-hit snapshot the
/// damage pipeline takes. `None` for an NPC, so an NPC hit costs one lookup.
pub fn player_snapshot(space_mgr: &SpaceManager, entity_id: u32) -> Option<Vitals> {
    space_mgr
        .get_entity(entity_id)
        .filter(|e| e.is_player)
        .map(|e| Vitals::of(&e.stats))
}

/// The attacker's and ability's names a `damage_taken` row carries, resolved
/// once per hit by the caller (`HitIds`), not again here.
#[derive(Debug, Clone, Copy, Default)]
pub struct HitNames {
    pub attacker: cimmeria_entity::cell_entity::PlayerIdentity,
    pub attacker_name: Option<&'static str>,
    pub ability_name: Option<&'static str>,
}

/// One `vitals` `event = "damage_taken"` row: a player took a hit.
///
/// `before` is the [`player_snapshot`] taken before the damage was applied;
/// the current pools are read here. `health_damage` / `focus_damage` are the
/// differences, so a heal-on-hit or a duel clamp shows as what the player
/// actually lost, not what the formula rolled.
pub fn log_damage_taken(
    space_mgr: &SpaceManager,
    target_eid: u32,
    attacker_eid: u32,
    ability_id: i32,
    result_code: u8,
    before: Vitals,
    names: HitNames,
) {
    let Some(target) = space_mgr.get_entity(target_eid) else {
        return;
    };
    let after = Vitals::of(&target.stats);
    let id = target.identity();
    tracing::debug!(
        target: "vitals",
        event = "damage_taken",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id = target_eid,
        entity_name = id.player_name,
        space_id = target.space_id.0,
        world = space_mgr.world_name_for_space(target.space_id.0 as u32),
        attacker_entity_id = attacker_eid,
        attacker_entity_name = names.attacker_name,
        attacker_player_id = names.attacker.player_id,
        attacker_player_name = names.attacker.player_name,
        ability_id,
        ability_name = names.ability_name,
        result_code,
        result = crate::cell::abilities::metrics::QrOutcome::from_code(result_code).label(),
        health_before = before.health,
        health = after.health,
        health_max = after.health_max,
        health_damage = before.health - after.health,
        focus_before = before.focus,
        focus = after.focus,
        focus_max = after.focus_max,
        focus_damage = before.focus - after.focus,
        threat_count = target.threatened_mobs.len(),
        "vitals: player took damage"
    );
}

/// One `vitals` `event = "combat_sample"` row for a player in combat.
/// `space_mgr` names the player's world.
pub fn log_combat_sample(space_mgr: &SpaceManager, entity: &CellEntity) {
    let v = Vitals::of(&entity.stats);
    let id = entity.identity();
    tracing::debug!(
        target: "vitals",
        event = "combat_sample",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id = entity.entity_id.0,
        entity_name = id.player_name,
        space_id = entity.space_id.0,
        world = space_mgr.world_name_for_space(entity.space_id.0 as u32),
        health = v.health,
        health_max = v.health_max,
        focus = v.focus,
        focus_max = v.focus_max,
        threat_count = entity.threatened_mobs.len(),
        "vitals: in-combat sample"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    fn mgr_with_player() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        let e = mgr.get_entity_mut(1).unwrap();
        e.is_player = true;
        e.player_id = Some(72);
        e.account_id = Some(6);
        e.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
        e.stats.get_mut(FOCUS).unwrap().update(0, 50, 60);
        mgr
    }

    /// The hit row carries the identity and the before/after pools, with the
    /// damage as the difference. Removing the row, or logging `player_id`
    /// Debug-formatted, fails it.
    #[test]
    fn damage_taken_row_carries_pools_and_identity() {
        let mut mgr = mgr_with_player();
        let before = player_snapshot(&mgr, 1).expect("a player snapshots");
        mgr.get_entity_mut(1)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap()
            .update(0, 70, 100);
        mgr.get_entity_mut(1)
            .unwrap()
            .stamp_log_names(Some("Teal'c"), Some("tealc_login"));
        let names = HitNames {
            attacker_name: Some("Jaffa Guard"),
            ability_name: Some("Staff Blast"),
            ..HitNames::default()
        };
        let capture = LogCapture::install();
        log_damage_taken(&mgr, 1, 900, 579, 1, before, names);
        let row = capture
            .find_message(Level::DEBUG, "vitals: player took damage")
            .expect("damage_taken row");
        assert_eq!(row.target, "vitals");
        for (k, v) in [
            ("event", "damage_taken"),
            ("account_id", "6"),
            ("player_id", "72"),
            ("entity_id", "1"),
            ("attacker_entity_id", "900"),
            ("ability_id", "579"),
            // Rule 6 (NT-20): the hit names who was hit, by whom, with what.
            ("player_name", "Teal'c"),
            ("account_name", "tealc_login"),
            ("entity_name", "Teal'c"),
            ("attacker_entity_name", "Jaffa Guard"),
            ("ability_name", "Staff Blast"),
            ("health_before", "100"),
            ("health", "70"),
            ("health_max", "100"),
            ("health_damage", "30"),
            ("focus", "50"),
            ("focus_max", "60"),
            ("focus_damage", "0"),
        ] {
            assert_eq!(row.fields.get(k).map(String::as_str), Some(v), "field {k}");
        }
    }

    /// NPCs get no snapshot, so the pipeline logs nothing for them.
    #[test]
    fn npc_has_no_snapshot() {
        let mut mgr = mgr_with_player();
        mgr.get_entity_mut(1).unwrap().is_player = false;
        assert_eq!(player_snapshot(&mgr, 1), None);
    }

    #[test]
    fn combat_sample_row_carries_pools() {
        let mut mgr = mgr_with_player();
        mgr.get_entity_mut(1).unwrap().threatened_mobs.insert(900);
        let capture = LogCapture::install();
        log_combat_sample(&mgr, mgr.get_entity(1).unwrap());
        let row = capture
            .find_message(Level::DEBUG, "vitals: in-combat sample")
            .expect("combat_sample row");
        assert!(row.has_field("event", "combat_sample"));
        assert!(row.has_field("player_id", "72"));
        assert!(row.has_field("health", "100"));
        assert!(row.has_field("focus_max", "60"));
        assert!(row.has_field("threat_count", "1"));
    }
}
