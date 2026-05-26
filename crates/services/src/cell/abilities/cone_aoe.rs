//! Cone-shaped AoE target collection (#61, #419).
//!
//! 99 effects in the DB carry `target_collection_method = 'TCM_AECone'`,
//! but until this module landed they only damaged the primary target —
//! shotgun blasts, grenade arcs, and "secondary target" splash all
//! resolved as plain single-target hits. This implements the cone
//! collection + secondary fan-out.
//!
//! ## Cone geometry
//!
//! The cone is anchored at the **attacker's position**, oriented toward
//! the **primary target's position**. Length comes from `tcm_param1`
//! (range tier name → meters via `EffectDef::tcm_range_meters`), width
//! comes from `tcm_param2` (Narrow/Medium/Wide → half-angle via
//! `EffectDef::tcm_half_angle_radians`).
//!
//! An entity `E` is inside the cone when:
//!   1. `distance(attacker, E) <= cone_length`
//!   2. `angle_between(target - attacker, E - attacker) <= half_angle`
//!   3. `E` is not the primary target (it already took damage upstream)
//!   4. `E` is in the attacker's space (no cross-space leak)
//!   5. `E` is hostile (faction = 10) and alive
//!
//! ## Why dispatch lives here, not in `apply_damage_to_target`
//!
//! `apply_damage_to_target` is reused by AoE-secondary callers — if we
//! put cone fan-out inside it, every secondary call would recursively
//! fan out to its own cone, causing exponential blowup. The fan-out
//! happens **above** `apply_damage_to_target` (called once per ability
//! invocation from `use_ability` after the primary commits).
//!
//! ## What's NOT here
//!
//! - **Radius (TCM_AERadius) fan-out.** That's already covered for the
//!   ground-target path by `handle_use_ability_on_ground`. The 300
//!   radius effects on single-target abilities (e.g. proximity-mine
//!   detonations) are a follow-up — they need explicit detonation
//!   triggers, not a fan-out at primary cast.
//! - **Cone for ground-target abilities.** A ground-target ability with
//!   a TCM_AECone effect would have no primary entity to orient toward.
//!   When that case shows up, anchor the cone at the ground click with
//!   the attacker's facing direction instead.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    AbilityDef, EffectDef, EF_DOT, EF_INTERRUPT_CHANCE, EF_MENTAL_RESIST_ROLL, EF_STUN,
    EF_SUPPRESSION, TCM_AE_CONE,
};

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::damage_apply::apply_damage_to_target;

/// Faction sentinel used across combat — must match the value in
/// [`super::dispatch::handle_use_ability_on_ground`].
const HOSTILE_FACTION: u8 = 10;

/// Should this effect drive a cone fan-out at primary-cast time?
///
/// Returns `true` only for effects authored as TCM_AECone. Other TCM
/// values are handled elsewhere (single, radius via ground-target).
pub fn is_cone_effect(effect: &EffectDef) -> bool {
    effect.target_collection_method == TCM_AE_CONE
}

/// Collect every hostile entity inside the cone defined by `(source,
/// primary_target, length, half_angle)`. The primary target is excluded
/// because it already took damage on the single-target path.
///
/// Returns entity ids sorted by distance from the apex so callers can
/// reason deterministically (e.g. limit to N nearest secondaries when
/// that becomes a feature).
pub fn collect_cone_targets(
    space_mgr: &SpaceManager,
    attacker_id: u32,
    primary_target_id: u32,
    cone_length: f32,
    half_angle: f32,
) -> Vec<u32> {
    let attacker = match space_mgr.get_entity(attacker_id) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let primary = match space_mgr.get_entity(primary_target_id) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let attacker_space = attacker.space_id;
    let apex = [
        attacker.position.x,
        attacker.position.y,
        attacker.position.z,
    ];

    // Direction vector from attacker to primary target in the X/Z plane
    // (Y is up in this engine). A vertical-only offset between attacker
    // and primary (rare — same X/Z) collapses the cone direction; skip
    // fan-out in that degenerate case rather than picking an arbitrary
    // facing.
    let dx = primary.position.x - apex[0];
    let dz = primary.position.z - apex[2];
    let dir_len = (dx * dx + dz * dz).sqrt();
    if dir_len < 1e-3 {
        tracing::debug!(
            attacker_id,
            primary_target_id,
            "cone_aoe: primary stacked on attacker — skipping cone fan-out"
        );
        return Vec::new();
    }
    let cone_dx = dx / dir_len;
    let cone_dz = dz / dir_len;

    let cos_half_angle = half_angle.cos();
    let length_sq = cone_length * cone_length;

    let mut hits: Vec<(u32, f32)> = Vec::new();
    for npc_eid in space_mgr.all_npc_entity_ids() {
        if npc_eid == primary_target_id || npc_eid == attacker_id {
            continue;
        }
        let npc = match space_mgr.get_entity(npc_eid) {
            Some(e) => e,
            None => continue,
        };
        if npc.space_id != attacker_space {
            continue;
        }
        if combat::is_dead_state(npc.state_field) {
            continue;
        }
        if npc.faction != HOSTILE_FACTION {
            continue;
        }
        let ex = npc.position.x - apex[0];
        let ez = npc.position.z - apex[2];
        let dist_sq_xz = ex * ex + ez * ez;
        // Use planar distance for cone containment; the original game's
        // cones are effectively 2D (cylindrical sections in 3D), and
        // anyone within the X/Z cone is "in the line of fire" regardless
        // of vertical offset short of going through a floor — which the
        // cell doesn't validate anyway pre-navmesh-LOS.
        if dist_sq_xz > length_sq {
            continue;
        }
        // Skip the apex itself (entities directly on the attacker would
        // make `to_npc_len` zero and the dot-product undefined).
        if dist_sq_xz < 1e-6 {
            continue;
        }
        let to_npc_len = dist_sq_xz.sqrt();
        let nx = ex / to_npc_len;
        let nz = ez / to_npc_len;
        let dot = nx * cone_dx + nz * cone_dz;
        if dot >= cos_half_angle {
            hits.push((npc_eid, dist_sq_xz));
        }
    }
    hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.into_iter().map(|(eid, _)| eid).collect()
}

/// Read a packed flags integer and surface any non-trivial categories as
/// structured tracing events so operators can see which effects are
/// landing. v1 doesn't fully implement Stun / Suppression / Interrupt /
/// Resist-roll mechanics — they show up as separate effect scripts in
/// Phase G — but the flag inspection happens at fan-out time so the
/// observability is consistent.
pub fn log_effect_flag_categories(
    entity_id: u32,
    target_id: u32,
    ability_id: i32,
    effect: &EffectDef,
) {
    let flags = effect.flags;
    if flags == 0 {
        return;
    }
    // Bit checks — flags are a packed bitmask so multiple categories
    // can fire from one effect.
    let mut categories: Vec<&'static str> = Vec::new();
    if flags & EF_STUN == EF_STUN {
        categories.push("stun");
    }
    if flags & EF_INTERRUPT_CHANCE == EF_INTERRUPT_CHANCE {
        categories.push("interrupt_chance");
    }
    if flags & EF_MENTAL_RESIST_ROLL == EF_MENTAL_RESIST_ROLL {
        categories.push("mental_resist_roll");
    }
    if flags & EF_SUPPRESSION == EF_SUPPRESSION {
        categories.push("suppression");
    }
    if flags & EF_DOT == EF_DOT {
        categories.push("dot");
    }
    if categories.is_empty() {
        return;
    }
    tracing::debug!(
        target: "abilities",
        event = "effect_flag_categories",
        entity_id,
        target_id,
        ability_id,
        effect_id = effect.effect_id,
        flags,
        categories = ?categories,
        "Effect carries category flags — v1 logs them; full mechanics arrive in Phase G"
    );
}

/// Dispatch every cone effect on `ability_def` against secondary targets
/// found around `(attacker, primary_target)`. Each secondary takes a
/// fresh `effect_seq` (so per-target wire packets remain correlatable)
/// and `needs_ammo_stat_send = false` (the primary call already flushed
/// the bandolier).
///
/// Returns the entity ids of every NPC that **died** during the cone
/// fan-out so the caller can fire `entity_death` events for kill-count
/// missions.
pub async fn fan_out_cone_effects(
    entity_id: u32,
    primary_target_id: u32,
    ability_id: i32,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Vec<u32> {
    let Some(def) = ability_def else {
        return Vec::new();
    };

    // Collect (cone_length, half_angle) per cone-effect, deduplicating
    // by geometry so two cone effects with identical reach + spread
    // share one target collection pass.
    let mut cone_specs: Vec<(f32, f32)> = Vec::new();
    for &eid in &def.effect_ids {
        let Some(effect) = space_mgr.effect_defs.get(&eid) else {
            continue;
        };
        if !is_cone_effect(effect) {
            continue;
        }
        let length = EffectDef::tcm_range_meters(&effect.tcm_param1);
        let half = EffectDef::tcm_half_angle_radians(&effect.tcm_param2);
        if !cone_specs
            .iter()
            .any(|(l, a)| (*l - length).abs() < 0.01 && (*a - half).abs() < 0.001)
        {
            cone_specs.push((length, half));
        }
    }

    if cone_specs.is_empty() {
        return Vec::new();
    }

    // Union the targets from each cone spec — an entity in a Narrow
    // cone is also in any wider cone with same length, so the union is
    // dominated by the widest+longest, but we compute per-spec so
    // exotic spec combinations (e.g. short-narrow + long-wide on the
    // same ability) work correctly.
    let mut union_targets: Vec<u32> = Vec::new();
    for &(length, half) in &cone_specs {
        for eid in collect_cone_targets(space_mgr, entity_id, primary_target_id, length, half) {
            if !union_targets.contains(&eid) {
                union_targets.push(eid);
            }
        }
    }

    if union_targets.is_empty() {
        tracing::debug!(
            entity_id,
            primary_target_id,
            ability_id,
            cone_count = cone_specs.len(),
            "cone_aoe: no secondaries in any cone spec — primary-only damage"
        );
        return Vec::new();
    }

    tracing::info!(
        entity_id,
        primary_target_id,
        ability_id,
        cone_count = cone_specs.len(),
        secondary_count = union_targets.len(),
        "cone_aoe: fanning out to cone secondaries"
    );

    // Snapshot HEALTH for every secondary so we can detect alive→dead
    // transitions after damage commits.
    let alive_before: Vec<(u32, bool)> = union_targets
        .iter()
        .map(|&eid| {
            let alive = space_mgr.get_entity(eid).is_some_and(|e| {
                e.stats
                    .get(cimmeria_entity::stats::HEALTH)
                    .is_some_and(|s| s.cur > 0)
            });
            (eid, alive)
        })
        .collect();

    for &secondary_eid in &union_targets {
        let secondary_seq = space_mgr
            .get_entity_mut(entity_id)
            .map(|e| e.abilities.next_effect_id())
            .unwrap_or(0);
        apply_damage_to_target(
            entity_id,
            secondary_eid,
            ability_id,
            ability_def,
            secondary_seq as u32,
            // Primary already flushed; secondaries must not re-flush
            // or the wire-packet log shows N+1 ammo updates per fire.
            false,
            tx,
            space_mgr,
        )
        .await;
    }

    // Collect alive→dead transitions so the caller can fire entity_death
    // for each kill (kill-count missions on AoE secondaries).
    let mut deaths = Vec::new();
    for (eid, was_alive) in alive_before {
        if !was_alive {
            continue;
        }
        let now_dead = space_mgr.get_entity(eid).is_some_and(|e| {
            e.stats
                .get(cimmeria_entity::stats::HEALTH)
                .is_some_and(|s| s.cur <= 0)
        });
        if now_dead {
            deaths.push(eid);
        }
    }
    deaths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;

    fn spawn_npc(mgr: &mut SpaceManager, eid: u32, world: &str, pos: [f32; 3]) {
        mgr.spawn_npc(eid, world, pos, [0.0; 3]).unwrap();
        if let Some(npc) = mgr.get_entity_mut(eid) {
            npc.faction = HOSTILE_FACTION;
        }
    }

    fn make_mgr_with_attacker() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /><Space WorldName="W2" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /><Space WorldName="W2" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
        }
        mgr
    }

    #[test]
    fn cone_collects_entities_inside_arc_and_excludes_outside() {
        // Cone pointing toward +X (primary at (10, 0, 0)). 45° half-angle
        // means anything within ±45° around +X within 10m gets hit.
        let mut mgr = make_mgr_with_attacker();
        spawn_npc(&mut mgr, 2, "W", [10.0, 0.0, 0.0]); // primary — excluded
        spawn_npc(&mut mgr, 3, "W", [5.0, 0.0, 3.0]); // ~31° off axis — INSIDE
        spawn_npc(&mut mgr, 4, "W", [4.0, 0.0, -3.0]); // ~37° off axis other side — INSIDE
        spawn_npc(&mut mgr, 5, "W", [0.0, 0.0, 8.0]); // 90° off axis — OUTSIDE
        spawn_npc(&mut mgr, 6, "W", [-5.0, 0.0, 0.0]); // 180° (behind) — OUTSIDE
        spawn_npc(&mut mgr, 7, "W", [25.0, 0.0, 0.0]); // forward but past range — OUTSIDE

        let hits = collect_cone_targets(
            &mgr,
            1, // attacker
            2, // primary target
            10.0,
            std::f32::consts::FRAC_PI_4, // 45° half-angle
        );
        assert!(hits.contains(&3), "(5,0,3) inside 45° cone");
        assert!(hits.contains(&4), "(4,0,-3) inside 45° cone");
        assert!(!hits.contains(&2), "primary target excluded");
        assert!(!hits.contains(&5), "(0,0,8) at 90° outside 45° cone");
        assert!(!hits.contains(&6), "behind attacker outside cone");
        assert!(!hits.contains(&7), "(25,0,0) past 10m range");
    }

    #[test]
    fn cone_excludes_cross_space_entities() {
        let mut mgr = make_mgr_with_attacker();
        spawn_npc(&mut mgr, 2, "W", [10.0, 0.0, 0.0]); // primary
        spawn_npc(&mut mgr, 3, "W2", [5.0, 0.0, 3.0]); // same coords, different space

        let hits = collect_cone_targets(&mgr, 1, 2, 10.0, std::f32::consts::FRAC_PI_4);
        assert!(
            !hits.contains(&3),
            "cross-space NPC must not be collected even when geometrically inside cone"
        );
    }

    #[test]
    fn cone_excludes_dead_and_non_hostile_entities() {
        let mut mgr = make_mgr_with_attacker();
        spawn_npc(&mut mgr, 2, "W", [10.0, 0.0, 0.0]); // primary

        // Dead hostile inside cone
        spawn_npc(&mut mgr, 3, "W", [5.0, 0.0, 3.0]);
        if let Some(e) = mgr.get_entity_mut(3) {
            e.set_state_flag(combat::BSF_DEAD);
        }
        // Neutral (faction != 10) inside cone
        spawn_npc(&mut mgr, 4, "W", [4.0, 0.0, -3.0]);
        if let Some(e) = mgr.get_entity_mut(4) {
            e.faction = 5; // neutral
        }

        let hits = collect_cone_targets(&mgr, 1, 2, 10.0, std::f32::consts::FRAC_PI_4);
        assert!(!hits.contains(&3), "dead hostile excluded");
        assert!(!hits.contains(&4), "neutral (non-hostile) excluded");
    }

    #[test]
    fn cone_skips_when_primary_stacked_on_attacker() {
        let mut mgr = make_mgr_with_attacker();
        // Primary exactly at attacker position — no direction to orient cone
        spawn_npc(&mut mgr, 2, "W", [0.0, 0.0, 0.0]);
        spawn_npc(&mut mgr, 3, "W", [5.0, 0.0, 0.0]); // would be in any forward cone

        let hits = collect_cone_targets(&mgr, 1, 2, 10.0, std::f32::consts::FRAC_PI_4);
        assert!(
            hits.is_empty(),
            "degenerate cone (primary on attacker) must skip — no defined direction"
        );
    }

    #[test]
    fn narrow_cone_excludes_what_wide_cone_includes() {
        let mut mgr = make_mgr_with_attacker();
        spawn_npc(&mut mgr, 2, "W", [10.0, 0.0, 0.0]); // primary
        spawn_npc(&mut mgr, 3, "W", [5.0, 0.0, 4.0]); // ~38° off axis

        // Wide cone (67.5° half-angle) includes it
        let wide =
            collect_cone_targets(&mgr, 1, 2, 10.0, EffectDef::tcm_half_angle_radians("Wide"));
        assert!(wide.contains(&3), "Wide (67.5°) cone includes 38° entity");

        // Narrow cone (22.5° half-angle) excludes it
        let narrow = collect_cone_targets(
            &mgr,
            1,
            2,
            10.0,
            EffectDef::tcm_half_angle_radians("Narrow"),
        );
        assert!(
            !narrow.contains(&3),
            "Narrow (22.5°) cone excludes 38° entity ({narrow:?})"
        );
    }

    #[test]
    fn is_cone_effect_recognizes_tcm_aecone_only() {
        let cone = EffectDef {
            target_collection_method: TCM_AE_CONE.to_string(),
            ..Default::default()
        };
        let single = EffectDef {
            target_collection_method: cimmeria_entity::abilities::TCM_SINGLE.to_string(),
            ..Default::default()
        };
        let radius = EffectDef {
            target_collection_method: cimmeria_entity::abilities::TCM_AE_RADIUS.to_string(),
            ..Default::default()
        };
        assert!(is_cone_effect(&cone));
        assert!(!is_cone_effect(&single));
        assert!(!is_cone_effect(&radius));
    }
}
