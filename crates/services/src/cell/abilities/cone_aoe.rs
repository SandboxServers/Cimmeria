//! Cone-shaped AoE target collection.
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
//!   5. `E` is a hostile NPC (faction = `HOSTILE_FACTION`) and alive
//!
//! ## PvE only (today)
//!
//! `collect_cone_targets` scans `all_npc_entity_ids()` so player entities
//! are silently excluded — cone AoE doesn't hit other players today. This
//! is correct for the PvE-only design Cimmeria currently targets. When
//! PvP lands, the candidate scan needs to switch to `all_entity_ids()`
//! plus a per-pair hostility check (replacing the flat `faction == 10`
//! sentinel with the future faction table).
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

use super::super::combat::HOSTILE_FACTION;

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

    // Per-effect cone specs — each cone effect carries its own length +
    // half-angle AND its own NVP damage values. We CANNOT union targets
    // across effects and then apply the whole `ability_def`: that would
    // damage every secondary with every cone effect's NVP, even when
    // they only geometrically matched one cone. Instead we collect
    // (effect_id, targets) per cone effect and apply each effect
    // independently using a scoped-down `AbilityDef` carrying only that
    // one effect's id (so the NVP-pick loop inside `apply_damage_to_target`
    // sees only this cone's damage values).
    let cone_effect_ids: Vec<i32> = def
        .effect_ids
        .iter()
        .copied()
        .filter(|eid| space_mgr.effect_defs.get(eid).is_some_and(is_cone_effect))
        .collect();

    if cone_effect_ids.is_empty() {
        return Vec::new();
    }

    // Resolve geometry per cone effect.
    let mut per_effect_targets: Vec<(i32, Vec<u32>)> = Vec::with_capacity(cone_effect_ids.len());
    let mut union_for_death_snapshot: Vec<u32> = Vec::new();
    for &eid in &cone_effect_ids {
        let Some(effect) = space_mgr.effect_defs.get(&eid) else {
            continue;
        };
        let length = EffectDef::tcm_range_meters(&effect.tcm_param1);
        let half = EffectDef::tcm_half_angle_radians(&effect.tcm_param2);
        let targets = collect_cone_targets(space_mgr, entity_id, primary_target_id, length, half);
        for &t in &targets {
            if !union_for_death_snapshot.contains(&t) {
                union_for_death_snapshot.push(t);
            }
        }
        per_effect_targets.push((eid, targets));
    }

    if union_for_death_snapshot.is_empty() {
        tracing::debug!(
            entity_id,
            primary_target_id,
            ability_id,
            cone_count = cone_effect_ids.len(),
            "cone_aoe: no secondaries in any cone spec — primary-only damage"
        );
        return Vec::new();
    }

    tracing::info!(
        entity_id,
        primary_target_id,
        ability_id,
        cone_count = cone_effect_ids.len(),
        unique_secondary_count = union_for_death_snapshot.len(),
        "cone_aoe: fanning out to cone secondaries (per-effect)"
    );

    // Snapshot HEALTH for the UNION of all cone targets — we need this
    // pre-damage to detect alive→dead transitions afterward. Done once
    // (not per-effect) because an entity hit by two effects on the same
    // tick that was alive_before should still count as one kill.
    let alive_before: Vec<(u32, bool)> = union_for_death_snapshot
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

    // Apply damage per-effect: for each cone effect, build a scoped
    // `AbilityDef` carrying ONLY that effect's id, then apply to that
    // effect's geometrically-matched targets. The scoped-down def causes
    // `apply_damage_to_target`'s NVP scan to read only this cone effect's
    // `HealthDamage` / `FocusDamage` / `script_name`, isolating per-effect
    // damage from cross-pollination across cone effects on the same ability.
    for (effect_id, targets) in &per_effect_targets {
        if targets.is_empty() {
            continue;
        }
        let scoped_def = AbilityDef {
            effect_ids: vec![*effect_id],
            ..def.clone()
        };
        let scoped_def_opt = Some(scoped_def);
        for &secondary_eid in targets {
            let secondary_seq = space_mgr
                .get_entity_mut(entity_id)
                .map(|e| e.abilities.next_effect_id())
                .unwrap_or(0);
            apply_damage_to_target(
                entity_id,
                secondary_eid,
                ability_id,
                &scoped_def_opt,
                secondary_seq as u32,
                // Primary already flushed; secondaries must not re-flush
                // or the wire-packet log shows N+1 ammo updates per fire.
                false,
                tx,
                space_mgr,
            )
            .await;
        }
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

    #[test]
    fn log_effect_flag_categories_no_op_on_zero_flags() {
        // Sanity: zero-flag effects don't emit a category log; the
        // function early-returns. We can't easily assert on tracing
        // output without a LogCapture; instead pin the behavior by
        // calling and confirming no panic.
        let effect = EffectDef {
            effect_id: 1,
            ability_id: 1,
            flags: 0,
            ..Default::default()
        };
        log_effect_flag_categories(10, 20, 30, &effect);
    }

    #[test]
    fn log_effect_flag_categories_recognizes_each_category_bit() {
        // Exercise each EF_* path so the match arms aren't dark code.
        // The assertion is structural — the function returns without
        // panic when each flag is set in isolation.
        use cimmeria_entity::abilities::{
            EF_DOT, EF_INTERRUPT_CHANCE, EF_MENTAL_RESIST_ROLL, EF_STUN, EF_SUPPRESSION,
        };
        for flag in [
            EF_STUN,
            EF_INTERRUPT_CHANCE,
            EF_MENTAL_RESIST_ROLL,
            EF_SUPPRESSION,
            EF_DOT,
        ] {
            let effect = EffectDef {
                effect_id: 1,
                ability_id: 1,
                flags: flag,
                ..Default::default()
            };
            log_effect_flag_categories(10, 20, 30, &effect);
        }
    }

    // ── fan_out_cone_effects integration tests ──────────────────────────────

    use cimmeria_entity::abilities::{AbilityDef, EffectDef};
    use cimmeria_entity::cell_entity::BandolierItem;
    use cimmeria_entity::stats::{AMMO_SLOT_1, HEALTH};
    use std::collections::HashMap;
    use tokio::sync::mpsc;

    fn make_cone_ability_with_effect(
        mgr: &mut SpaceManager,
        ability_id: i32,
        effect_id: i32,
        damage: i32,
        range_tier: &str,
        width_tier: &str,
    ) {
        let mut params = HashMap::new();
        params.insert("HealthDamage".to_string(), damage.to_string());
        mgr.effect_defs.insert(
            effect_id,
            EffectDef {
                effect_id,
                ability_id,
                target_collection_method: TCM_AE_CONE.to_string(),
                tcm_param1: range_tier.to_string(),
                tcm_param2: width_tier.to_string(),
                params,
                ..Default::default()
            },
        );
        mgr.ability_defs.insert(
            ability_id,
            AbilityDef {
                ability_id,
                name: format!("ConeAbility{ability_id}"),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: true,
                min_range: 0,
                max_range: 50,
                target_type_id: 0,
                effect_ids: vec![effect_id],
                moniker_ids: vec![],
                required_ammo: 1,
                event_set_id: None,
                velocity: 0.0,
            },
        );
    }

    fn seed_attacker_for_fan_out(mgr: &mut SpaceManager) {
        if let Some(e) = mgr.get_entity_mut(1) {
            e.weapon_holstered = false;
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    instance_id: 0,
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 30,
                    cur_ammo_type: 2,
                },
            );
            if let Some(stat) = e.stats.get_mut(AMMO_SLOT_1) {
                stat.update(0, 30, 30);
                stat.clear_dirty();
            }
        }
    }

    #[tokio::test]
    async fn fan_out_cone_effects_damages_each_secondary_inside_cone() {
        // Attacker at origin, primary at (10,0,0), one secondary at
        // (5,0,3) (inside Medium/Medium cone), one outside.
        let mut mgr = make_mgr_with_attacker();
        seed_attacker_for_fan_out(&mut mgr);
        make_cone_ability_with_effect(&mut mgr, 7000, 7001, 30, "Medium", "Medium");
        // Primary
        spawn_npc(&mut mgr, 100, "W", [10.0, 0.0, 0.0]);
        if let Some(npc) = mgr.get_entity_mut(100) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 200, 200);
            npc.stats.clear_dirty();
        }
        // Secondary INSIDE cone
        spawn_npc(&mut mgr, 101, "W", [5.0, 0.0, 3.0]);
        if let Some(npc) = mgr.get_entity_mut(101) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 200, 200);
            npc.stats.clear_dirty();
        }
        // OUTSIDE cone (behind attacker)
        spawn_npc(&mut mgr, 102, "W", [-5.0, 0.0, 0.0]);
        if let Some(npc) = mgr.get_entity_mut(102) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 200, 200);
            npc.stats.clear_dirty();
        }

        let ability_def = mgr.ability_defs.get(&7000).cloned();
        let (tx, _rx) = mpsc::channel(256);
        let deaths = fan_out_cone_effects(1, 100, 7000, &ability_def, &tx, &mut mgr).await;
        assert!(deaths.is_empty(), "no one died from 30 damage on 200 HP");

        // Inside-cone secondary took damage; behind-attacker NPC did not.
        let secondary_hp = mgr.get_entity(101).unwrap().stats.get(HEALTH).unwrap().cur;
        assert!(
            secondary_hp < 200,
            "in-cone secondary took damage; hp={secondary_hp}"
        );
        let behind_hp = mgr.get_entity(102).unwrap().stats.get(HEALTH).unwrap().cur;
        assert_eq!(behind_hp, 200, "outside-cone NPC untouched");
    }

    #[tokio::test]
    async fn fan_out_cone_effects_returns_alive_to_dead_transitions() {
        // Secondaries at low HP should die from the cone hit; survivor
        // should not be in the deaths Vec.
        let mut mgr = make_mgr_with_attacker();
        seed_attacker_for_fan_out(&mut mgr);
        make_cone_ability_with_effect(&mut mgr, 7100, 7101, 50, "Medium", "Medium");
        spawn_npc(&mut mgr, 200, "W", [10.0, 0.0, 0.0]); // primary
        if let Some(npc) = mgr.get_entity_mut(200) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 200, 200);
            npc.stats.clear_dirty();
        }
        spawn_npc(&mut mgr, 201, "W", [5.0, 0.0, 3.0]); // dies
        if let Some(npc) = mgr.get_entity_mut(201) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 10, 200);
            npc.stats.clear_dirty();
        }
        spawn_npc(&mut mgr, 202, "W", [4.0, 0.0, -3.0]); // survives
        if let Some(npc) = mgr.get_entity_mut(202) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 1000, 1000);
            npc.stats.clear_dirty();
        }

        let ability_def = mgr.ability_defs.get(&7100).cloned();
        let (tx, _rx) = mpsc::channel(256);
        let deaths = fan_out_cone_effects(1, 200, 7100, &ability_def, &tx, &mut mgr).await;
        assert!(deaths.contains(&201), "201 should be in deaths Vec");
        assert!(
            !deaths.contains(&202),
            "202 survived, must not be in deaths"
        );
    }

    #[tokio::test]
    async fn fan_out_cone_effects_no_cone_effects_returns_empty() {
        // Ability with only a TCM_Single effect — no cone fan-out.
        let mut mgr = make_mgr_with_attacker();
        let mut params = HashMap::new();
        params.insert("HealthDamage".to_string(), "10".to_string());
        mgr.effect_defs.insert(
            7200,
            EffectDef {
                effect_id: 7200,
                ability_id: 7201,
                target_collection_method: cimmeria_entity::abilities::TCM_SINGLE.to_string(),
                params,
                ..Default::default()
            },
        );
        mgr.ability_defs.insert(
            7201,
            AbilityDef {
                ability_id: 7201,
                name: "SingleOnly".to_string(),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: true,
                min_range: 0,
                max_range: 50,
                target_type_id: 0,
                effect_ids: vec![7200],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        spawn_npc(&mut mgr, 300, "W", [10.0, 0.0, 0.0]);
        let ability_def = mgr.ability_defs.get(&7201).cloned();
        let (tx, _rx) = mpsc::channel(256);
        let deaths = fan_out_cone_effects(1, 300, 7201, &ability_def, &tx, &mut mgr).await;
        assert!(deaths.is_empty(), "no cone effects = no fan-out");
    }

    #[tokio::test]
    async fn fan_out_cone_effects_handles_missing_ability_def() {
        let mut mgr = make_mgr_with_attacker();
        spawn_npc(&mut mgr, 400, "W", [10.0, 0.0, 0.0]);
        let (tx, _rx) = mpsc::channel(256);
        let deaths = fan_out_cone_effects(1, 400, 9999, &None, &tx, &mut mgr).await;
        assert!(deaths.is_empty(), "missing ability def = no fan-out");
    }

    #[tokio::test]
    async fn fan_out_cone_effects_two_cones_apply_per_effect_not_unioned() {
        // Clara G19: when an ability has TWO cone effects with different
        // geometries, each cone applies its OWN damage to the targets it
        // geometrically matched — NOT the union damage to all matched
        // targets. Pre-fix, the union path applied the last effect's NVP
        // to all targets including ones that only matched the narrower cone.
        let mut mgr = make_mgr_with_attacker();
        seed_attacker_for_fan_out(&mut mgr);

        // Effect 8001 — Medium cone, 30 damage (matches the inner secondary).
        let mut p1 = HashMap::new();
        p1.insert("HealthDamage".to_string(), "30".to_string());
        mgr.effect_defs.insert(
            8001,
            EffectDef {
                effect_id: 8001,
                ability_id: 8000,
                target_collection_method: TCM_AE_CONE.to_string(),
                tcm_param1: "Medium".to_string(),
                tcm_param2: "Medium".to_string(),
                params: p1,
                ..Default::default()
            },
        );
        // Effect 8002 — Long cone, 5 damage (matches the far secondary only).
        let mut p2 = HashMap::new();
        p2.insert("HealthDamage".to_string(), "5".to_string());
        mgr.effect_defs.insert(
            8002,
            EffectDef {
                effect_id: 8002,
                ability_id: 8000,
                target_collection_method: TCM_AE_CONE.to_string(),
                tcm_param1: "Long".to_string(),
                tcm_param2: "Narrow".to_string(),
                params: p2,
                ..Default::default()
            },
        );
        mgr.ability_defs.insert(
            8000,
            AbilityDef {
                ability_id: 8000,
                name: "TwoCones".to_string(),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: true,
                min_range: 0,
                max_range: 50,
                target_type_id: 0,
                effect_ids: vec![8001, 8002],
                moniker_ids: vec![],
                required_ammo: 1,
                event_set_id: None,
                velocity: 0.0,
            },
        );

        spawn_npc(&mut mgr, 500, "W", [10.0, 0.0, 0.0]); // primary
        if let Some(npc) = mgr.get_entity_mut(500) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 1000, 1000);
            npc.stats.clear_dirty();
        }
        // Far secondary at +X 12m — in Long cone (14m range, narrow),
        // OUTSIDE Medium cone (8m range).
        spawn_npc(&mut mgr, 501, "W", [12.0, 0.0, 0.1]);
        if let Some(npc) = mgr.get_entity_mut(501) {
            npc.stats.get_mut(HEALTH).unwrap().update(0, 1000, 1000);
            npc.stats.clear_dirty();
        }

        let ability_def = mgr.ability_defs.get(&8000).cloned();
        let hp_before = mgr.get_entity(501).unwrap().stats.get(HEALTH).unwrap().cur;
        let (tx, _rx) = mpsc::channel(256);
        let _ = fan_out_cone_effects(1, 500, 8000, &ability_def, &tx, &mut mgr).await;
        let hp_after = mgr.get_entity(501).unwrap().stats.get(HEALTH).unwrap().cur;
        let dmg = hp_before - hp_after;
        // If per-effect routing works, npc 501 takes ~5 dmg from effect 8002 only.
        // If broken (old union path), it took the last positive HealthDamage
        // across all effects = 5 OR 30 depending on iteration order — either
        // way, NOT both. The precise per-effect assertion: 501 should take
        // the 5-damage effect's amount, not the 30. We assert dmg < 30 to
        // catch the regression where the Medium cone's 30-damage bleeds into
        // the far target.
        assert!(
            dmg < 30,
            "far secondary in Long cone only must NOT take Medium cone's damage; got {dmg}"
        );
    }
}
