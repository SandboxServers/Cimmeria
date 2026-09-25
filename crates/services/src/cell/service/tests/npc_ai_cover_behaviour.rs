//! NA22 cover behaviour on a meshless `Castle` fixture: spawn hold, the
//! in-range seek (audit C3), flank release and re-pick, Cover Stance on
//! arrival and its removal on leave / leash / death, the `use_cover` rule
//! and the melee-only carve-out.
//!
//! The real-seed, real-navmesh versions of the headline cases are in
//! [`super::npc_ai_cover_seed`]. The fixture and node helper come from
//! [`super::npc_ai_cover`].

use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::{COVER_DEFENSE, HEALTH};
use tokio::sync::mpsc;

use super::npc_ai_cover::{make_cover_fixture, node};
use crate::cell::cover::{
    Cover, CoverSlotKey, COVER_STANCE_ABILITY, COVER_STANCE_EFFECT, COVER_STANCE_REMOVE_EFFECT,
};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::SpawnRecord;

const NPC: u32 = 200;
const PLAYER: u32 = 100;
const SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 50,
    node_id: 0,
};

/// The seeded Cover Stance effect rows (`effects.sql` 4565 / 1742), which
/// the live-DB test pins against the seed.
pub(super) fn seed_cover_stance_effects(mgr: &mut SpaceManager) {
    for (effect_id, script) in [
        (COVER_STANCE_EFFECT, "CoverStance"),
        (COVER_STANCE_REMOVE_EFFECT, "RemoveCoverStance"),
    ] {
        mgr.effect_defs.insert(
            effect_id,
            EffectDef {
                effect_id,
                ability_id: COVER_STANCE_ABILITY,
                script_name: Some(script.to_string()),
                ..Default::default()
            },
        );
    }
}

pub(super) fn cover_defense(mgr: &SpaceManager, id: u32) -> i32 {
    mgr.get_entity(id)
        .unwrap()
        .stats
        .get(COVER_DEFENSE)
        .unwrap()
        .cur
}

fn held(mgr: &SpaceManager, id: u32) -> Option<CoverSlotKey> {
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .slot_for_entity(EntityId(id as i32))
}

fn add_player(mgr: &mut SpaceManager, pos: [f32; 3]) {
    mgr.create_entity(PLAYER, "Castle", pos, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    let h = p.stats.get_mut(HEALTH).unwrap();
    h.update(0, 100, 100);
    h.clear_dirty();
    mgr.get_entity_mut(NPC)
        .unwrap()
        .threat_list
        .insert(PLAYER, 1.0);
}

fn move_player(mgr: &mut SpaceManager, pos: [f32; 3]) {
    mgr.update_entity_position(PLAYER, pos, [0, 0, 0], [0.0; 3]);
}

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// NPC at the slot (4,0,0) facing +X, holding it, player 21 u in front.
fn npc_in_slot() -> SpaceManager {
    let mut mgr = make_cover_fixture(
        [4.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        vec![node(50, 0, 4.0, 0.0, 0.0)],
    );
    seed_cover_stance_effects(&mut mgr);
    add_player(&mut mgr, [25.0, 0.0, 0.0]);
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(NPC as i32), SLOT)
        .unwrap();
    mgr
}

/// Audit C3: an NPC with a shot takes a slot a short walk away. Before NA22
/// `in_range` skipped cover entirely; reverting the step-2 early return
/// leaves the reservation table empty.
#[tokio::test]
async fn in_range_npc_under_fire_reserves_a_nearby_slot() {
    let mut mgr = make_cover_fixture([0.0; 3], [0.0; 3], vec![node(50, 0, 4.0, 0.0, 0.0)]);
    add_player(&mut mgr, [20.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;
    assert_eq!(held(&mgr, NPC), Some(SLOT));
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(
        npc.nav_path.back().copied(),
        Some(Vector3::new(4.0, 0.0, 0.0)),
        "the NPC walks to the slot (a direct waypoint on a meshless space)"
    );
}

/// On arrival the NPC stops dead at the slot and gains Cover Stance
/// (+100 COVER_DEFENSE, effect 4565); when the slot is flanked it loses it
/// again (effect 1742). Reverting the grant or the revoke fails the matching
/// assertion.
#[tokio::test]
async fn cover_stance_is_granted_on_arrival_and_removed_on_leaving() {
    let mut mgr = npc_in_slot();
    mgr.get_entity_mut(NPC).unwrap().velocity = [0.3, 0.0, 0.0];
    ai_tick(&mut mgr).await;
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.velocity, [0.0; 3], "stopped at the slot");
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.position, Vector3::new(4.0, 0.0, 0.0));
    assert_eq!(cover_defense(&mgr, NPC), 100, "Cover Stance on arrival");

    // A second tick in the slot does not stack the buff.
    ai_tick(&mut mgr).await;
    assert_eq!(cover_defense(&mgr, NPC), 100);

    // The player walks round behind the cover.
    move_player(&mut mgr, [-15.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;
    assert_eq!(held(&mgr, NPC), None, "flanked: released");
    assert_eq!(
        cover_defense(&mgr, NPC),
        0,
        "Cover Stance removed on leaving"
    );
}

/// Flanking releases the slot and the next tick re-picks one that defends
/// against the new threat position.
#[tokio::test]
async fn flanking_releases_the_slot_and_the_npc_repicks() {
    let mut mgr = make_cover_fixture(
        [4.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        vec![
            node(50, 0, 4.0, 0.0, 0.0),
            node(51, 0, 6.0, 3.0, std::f32::consts::PI),
        ],
    );
    add_player(&mut mgr, [-20.0, 0.0, 0.0]);
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(NPC as i32), SLOT)
        .unwrap();

    ai_tick(&mut mgr).await;
    assert_eq!(held(&mgr, NPC), None, "the flanked slot is released");
    ai_tick(&mut mgr).await;
    assert_eq!(
        held(&mgr, NPC),
        Some(CoverSlotKey::new(51, 0)),
        "re-picked the slot facing the new threat"
    );
}

/// Leashing drops the slot and the stance.
#[tokio::test]
async fn leash_releases_the_slot_and_removes_cover_stance() {
    let mut mgr = npc_in_slot();
    crate::cell::cover::grant_cover_stance(&mut mgr, NPC);
    assert_eq!(cover_defense(&mgr, NPC), 100);
    // Home is 96 u away: past the leash band.
    mgr.get_entity_mut(NPC).unwrap().spawn_position = Some(Vector3::new(100.0, 0.0, 0.0));
    ai_tick(&mut mgr).await;
    assert_eq!(held(&mgr, NPC), None);
    assert_eq!(cover_defense(&mgr, NPC), 0, "stance removed on leash");
}

/// Death drops the slot and the stance (`abilities::death`).
#[tokio::test]
async fn death_releases_the_slot_and_removes_cover_stance() {
    let mut mgr = npc_in_slot();
    crate::cell::cover::grant_cover_stance(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        crate::cell::abilities::kill_npc_out_of_band(NPC, PLAYER, true, false, &tx, &mut mgr).await
    );
    assert_eq!(held(&mgr, NPC), None);
    assert_eq!(cover_defense(&mgr, NPC), 0, "stance removed on death");
}

/// An NPC whose every ability is a melee swing never takes cover, even with
/// `use_cover` on and a slot inside its swing reach of the target (the slot
/// at 1.5 u is 0.5 u from a target 2 u away, so without the gate the cover
/// step reserves it).
#[tokio::test]
async fn melee_only_npc_never_takes_cover() {
    let mut mgr = make_cover_fixture([0.0; 3], [0.0; 3], vec![node(50, 0, 1.5, 0.0, 0.0)]);
    add_player(&mut mgr, [2.0, 0.0, 0.0]);
    let melee = crate::cell::combat::NPC_DEFAULT_ABILITY;
    super::npc_ai::seed_default_ability(&mut mgr, 0, 0);
    mgr.ability_defs.get_mut(&melee).unwrap().is_ranged = false;
    mgr.get_entity_mut(NPC)
        .unwrap()
        .abilities
        .add_ability(melee);
    ai_tick(&mut mgr).await;
    assert_eq!(
        held(&mgr, NPC),
        None,
        "a melee-only NPC must not reserve cover"
    );
}

fn guard_record(x: f32, z: f32) -> SpawnRecord {
    SpawnRecord {
        spawn_id: 29,
        world_name: "Castle".to_string(),
        x,
        y: 0.0,
        z,
        heading: 0.0,
        tag: Some("CoverGuard".to_string()),
        template_id: 24,
        template_name: "NID Guard".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(1),
        alignment: Some(0),
        faction: Some(10),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![],
        respawn_secs: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
    }
}

/// `entity_templates.use_cover` resolution: NULL means "a hostile NPC takes
/// cover"; the column overrides either way; stationary and props never do.
#[test]
fn use_cover_resolution_follows_the_template_then_the_default_rule() {
    use crate::cell::space_manager::resolve_use_cover as resolve;
    let hostile = guard_record(0.0, 0.0);
    assert!(resolve(&hostile), "NULL + faction 10 -> takes cover");
    let friendly = SpawnRecord {
        faction: Some(1),
        ..guard_record(0.0, 0.0)
    };
    assert!(!resolve(&friendly), "NULL + non-hostile -> no cover");
    assert!(resolve(&SpawnRecord {
        use_cover: Some(true),
        ..friendly.clone()
    }));
    assert!(!resolve(&SpawnRecord {
        use_cover: Some(false),
        ..hostile.clone()
    }));
    assert!(!resolve(&SpawnRecord {
        use_cover: Some(true),
        is_stationary: true,
        ..hostile.clone()
    }));
    assert!(!resolve(&SpawnRecord {
        use_cover: Some(true),
        static_mesh: Some("Props.Turret".to_string()),
        ..hostile
    }));
}

/// Audit C4: an NPC authored at a cover marker spawns holding it; one with
/// `use_cover = false`, or stationary, does not. Before NA22 nothing was
/// reserved at spawn.
#[test]
fn npc_spawned_at_a_cover_marker_holds_the_slot() {
    for (record, expect) in [
        (guard_record(4.3, 0.4), Some(SLOT)),
        (
            SpawnRecord {
                use_cover: Some(false),
                ..guard_record(4.3, 0.4)
            },
            None,
        ),
        (
            SpawnRecord {
                is_stationary: true,
                ..guard_record(4.3, 0.4)
            },
            None,
        ),
        // Too far from the marker.
        (guard_record(6.0, 0.0), None),
    ] {
        let mut mgr = make_cover_fixture([0.0; 3], [0.0; 3], vec![node(50, 0, 4.0, 0.0, 0.0)]);
        mgr.spawn_npc_from_record(300, &record).unwrap();
        assert_eq!(held(&mgr, 300), expect, "{record:?}");
    }
}

/// The startup population spawns before cover loads, so the per-spawn hold
/// finds nothing; `cover_loaded` sweeps it.
#[test]
fn cover_load_sweeps_npcs_that_spawned_before_it() {
    let mut mgr = make_cover_fixture([0.0; 3], [0.0; 3], Vec::new());
    mgr.cover = Cover::empty();
    mgr.spawn_npc_from_record(300, &guard_record(4.3, 0.4))
        .unwrap();
    assert_eq!(held(&mgr, 300), None);
    mgr.cover = Cover::from_loaded(Vec::new(), vec![node(50, 0, 4.0, 0.0, 0.0)]);
    mgr.cover_loaded();
    assert_eq!(held(&mgr, 300), Some(SLOT));
}

/// NA02's cover telemetry names the NA22 branches: a pick while already in
/// range logs `move_to_cover` with `in_range=true`, and a target leaving
/// attack range logs `cover_released_out_of_range`.
#[tokio::test]
async fn cover_rows_name_the_new_branches() {
    use crate::test_support::LogCapture;
    let logs = LogCapture::install();

    let mut mgr = make_cover_fixture([0.0; 3], [0.0; 3], vec![node(50, 0, 4.0, 0.0, 0.0)]);
    add_player(&mut mgr, [20.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;
    // Put the NPC at the slot, then walk the player out of range.
    mgr.update_entity_position(NPC, [4.0, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
    mgr.get_entity_mut(NPC).unwrap().spawn_position = Some(Vector3::new(4.0, 0.0, 0.0));
    move_player(&mut mgr, [45.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;

    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "npc_ai")
        .collect();
    assert!(
        rows.iter()
            .any(|c| c.has_field("decision_outcome", "move_to_cover")
                && c.has_field("in_range", "true")),
        "in-range pick row: {rows:#?}"
    );
    assert!(
        rows.iter()
            .any(|c| c.has_field("decision_outcome", "cover_released_out_of_range")),
        "out-of-range release row: {rows:#?}"
    );
}

/// NA16's attack LoS policy names the cover rule: an NPC at its slot (it
/// holds Cover Stance) fires across a navmesh `Blocked` under
/// `los_policy=in_cover_slot`, and loses the exemption when it leaves the
/// slot. A stationary NPC keeps NA16's own rules.
#[tokio::test]
async fn attack_los_policy_exempts_an_npc_at_its_cover_slot() {
    use crate::cell::space_manager::AttackLosPolicy;
    use cimmeria_entity::navigation::LineOfSight;

    let mut mgr = npc_in_slot();
    assert_eq!(
        mgr.attack_los_policy(NPC, PLAYER, false, LineOfSight::Blocked),
        AttackLosPolicy::Strict(false),
        "not yet at the slot"
    );
    ai_tick(&mut mgr).await; // arrives, takes the stance
    let policy = mgr.attack_los_policy(NPC, PLAYER, false, LineOfSight::Blocked);
    assert_eq!(policy, AttackLosPolicy::InCoverSlot);
    assert!(policy.permits());
    assert_eq!(policy.label(), "in_cover_slot");
    assert_eq!(
        mgr.attack_los_policy(NPC, PLAYER, true, LineOfSight::Blocked),
        AttackLosPolicy::StationaryRelaxed,
        "the stationary rule is unchanged"
    );

    move_player(&mut mgr, [-15.0, 0.0, 0.0]); // flanks the slot
    ai_tick(&mut mgr).await;
    assert_eq!(
        mgr.attack_los_policy(NPC, PLAYER, false, LineOfSight::Blocked),
        AttackLosPolicy::Strict(false),
        "the exemption goes with the slot"
    );
}
