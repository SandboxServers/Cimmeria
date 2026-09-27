//! Tests for [`super::npc_movement_tick`], split out of `npc_movement.rs`
//! when it neared the 700-line cap (NA41).

use super::*;
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;

#[test]
fn npc_movement_tick_advances_along_nav_path() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04;
        npc.move_speed = 5.0;
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(10.0, 0.0, 0.0));
    }

    npc_movement_tick(&mut mgr);

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.position.x, 5.0);
    assert_eq!(npc.position.y, 0.0);
    assert_eq!(npc.position.z, 0.0);
    assert_eq!(npc.nav_path.len(), 1);
}

#[test]
fn npc_movement_tick_does_not_panic_on_empty_path() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04;
        npc.nav_path.clear();
    }
    // Must not panic.
    npc_movement_tick(&mut mgr);
    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.position.x, 0.0, "stationary NPC must not move");
}

#[test]
fn npc_snaps_to_waypoint_when_within_move_speed_and_advances() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04;
        npc.move_speed = 10.0; // larger than distance to first waypoint
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(3.0, 0.0, 4.0)); // dist = 5
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(20.0, 0.0, 0.0));
    }

    npc_movement_tick(&mut mgr);

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.position.x, 3.0, "must snap to first waypoint X");
    assert_eq!(npc.position.y, 0.0, "must snap to first waypoint Y");
    assert_eq!(npc.position.z, 4.0, "must snap to first waypoint Z");
    assert_eq!(
        npc.nav_path.len(),
        1,
        "first waypoint consumed, second remains"
    );
}

#[test]
fn npc_stops_at_final_waypoint() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04;
        npc.move_speed = 20.0; // overshoots the only waypoint
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(5.0, 0.0, 0.0));
    }

    npc_movement_tick(&mut mgr);

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.position.x, 5.0, "must snap to final waypoint X");
    assert_eq!(npc.position.y, 0.0, "must snap to final waypoint Y");
    assert_eq!(npc.position.z, 0.0, "must snap to final waypoint Z");
    assert!(
        npc.nav_path.is_empty(),
        "path must be empty after reaching final waypoint"
    );
}

/// NA41: an NPC handed a one-waypoint path onto the spot it already stands
/// on keeps its facing. The final-waypoint branch took `atan2(dx, dz)` of a
/// zero leg, which is 0, and turned the NPC to due north on arrival.
///
/// Revert-proof: restoring the unconditional `dx.atan2(dz)` gives yaw 0.
#[test]
fn final_waypoint_on_the_npc_keeps_its_yaw() {
    use std::f32::consts::FRAC_PI_2;
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", [5.0, 0.0, 3.0], [0.0; 3])
        .unwrap();
    let npc = mgr.get_entity_mut(200).unwrap();
    npc.is_player = false;
    npc.class_id = 0x04;
    npc.direction = cimmeria_common::Vector3::new(0.0, FRAC_PI_2, 0.0);
    npc.nav_path
        .push_back(cimmeria_common::Vector3::new(5.0, 0.0, 3.0));

    npc_movement_tick(&mut mgr);

    let npc = mgr.get_entity(200).unwrap();
    assert!(npc.nav_path.is_empty(), "the waypoint is consumed");
    assert_eq!(npc.velocity, [0.0; 3]);
    assert!(
        (npc.direction.y - FRAC_PI_2).abs() < 1e-6,
        "a zero-length final leg must keep the yaw (+PI/2), not snap to \
         north; got {}",
        npc.direction.y
    );

    // Control: a real final leg still turns the NPC along it (due west).
    mgr.get_entity_mut(200)
        .unwrap()
        .nav_path
        .push_back(cimmeria_common::Vector3::new(4.9, 0.0, 3.0));
    npc_movement_tick(&mut mgr);
    let yaw = mgr.get_entity(200).unwrap().direction.y;
    assert!((yaw + FRAC_PI_2).abs() < 1e-4, "faces west; got {yaw}");
}

// ── P47 (`.speed`) — tick-level movement effect ─────────────────────────
//
// The stat-level half of P47 lives in
// `crate::cell::console::tests::p47`; these prove the setter's effect
// reaches real tick behavior rather than stopping at the stat value.
// `cargo test --lib legacy_p47_` runs both halves.

/// Build a GM caller + a pathing NPC in one space. The NPC starts at the
/// origin with a single waypoint 80 units down +X and `move_speed = 5.0`,
/// so every per-tick step in these tests (`t = speed / 80`) lands on an
/// exact binary fraction and the position assertions can be exact `f32`
/// equality rather than an epsilon compare.
fn speed_fixture() -> (SpaceManager, u32, u32) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();

    let gm = 1u32;
    mgr.create_entity(gm, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.is_player = true;
        e.access_level = 2;
    }

    let npc = 200u32;
    mgr.create_entity(npc, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.is_player = false;
        e.class_id = 0x04;
        e.move_speed = 5.0;
        e.nav_path
            .push_back(cimmeria_common::Vector3::new(80.0, 0.0, 0.0));
    }
    (mgr, gm, npc)
}

/// Run `.speed <arg>` through the real console dispatch, then one movement
/// tick, and report how far along +X the NPC actually got.
async fn speed_then_tick(arg: &str) -> f32 {
    let (mut mgr, gm, npc) = speed_fixture();
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    crate::cell::console::exec("speed", gm, &[arg], Some(npc), &tx, &mut mgr, &engine).await;
    npc_movement_tick(&mut mgr);
    mgr.get_entity(npc).unwrap().position.x
}

/// The acceptance criterion: `.speed` changes how far the NPC actually
/// moves on the next tick, not merely what the stat reads back.
/// `move_speed = 5.0` at the default mod of 100 steps 5.0 units; `.speed
/// 200` must step exactly 10.0 and `.speed 50` exactly 2.5.
///
/// Reverting `effective_move_speed` back to a bare `npc.move_speed` makes
/// every non-100 row collapse onto 5.0 and fails here.
#[tokio::test]
async fn legacy_p47_speed_scales_the_npc_tick_step_proportionally() {
    for (arg, expected) in [("50", 2.5f32), ("100", 5.0), ("200", 10.0), ("400", 20.0)] {
        let got = speed_then_tick(arg).await;
        assert_eq!(
            got, expected,
            ".speed {arg} must move the NPC exactly {expected} units on the next tick"
        );
    }
}

/// `.speed 0` freezes the NPC in place: no displacement, and the waypoint
/// is NOT consumed (a zero-length step must not be mistaken for "reached
/// the waypoint" — `dist <= move_speed` would be `80.0 <= 0.0`, false).
#[tokio::test]
async fn legacy_p47_speed_zero_freezes_the_npc_without_consuming_its_path() {
    let (mut mgr, gm, npc) = speed_fixture();
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    crate::cell::console::exec("speed", gm, &["0"], Some(npc), &tx, &mut mgr, &engine).await;

    npc_movement_tick(&mut mgr);
    npc_movement_tick(&mut mgr);

    let e = mgr.get_entity(npc).unwrap();
    assert_eq!(e.position.x, 0.0, ".speed 0 must stop the NPC moving");
    assert_eq!(e.position.z, 0.0, ".speed 0 must stop the NPC moving");
    assert_eq!(
        e.nav_path.len(),
        1,
        "a frozen NPC must not consume waypoints"
    );
}

/// The effect accumulates across ticks rather than applying once: two
/// ticks at `.speed 200` cover exactly 20.0 units.
#[tokio::test]
async fn legacy_p47_speed_effect_persists_across_ticks() {
    let (mut mgr, gm, npc) = speed_fixture();
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    crate::cell::console::exec("speed", gm, &["200"], Some(npc), &tx, &mut mgr, &engine).await;

    npc_movement_tick(&mut mgr);
    assert_eq!(mgr.get_entity(npc).unwrap().position.x, 10.0);
    npc_movement_tick(&mut mgr);
    assert_eq!(
        mgr.get_entity(npc).unwrap().position.x,
        20.0,
        "the speed mod must apply on every subsequent tick, not just the first"
    );
}

// ── GC1b-0 — `entity_templates.move_speed` DB wiring ─────────────────────
//
// The tests above pin the per-tick math from a raw `npc.move_speed`
// field write. This one closes the loop from the other end: a
// `SpawnRecord.move_speed` value (what `load_spawns_from_db` reads
// off `entity_templates.move_speed`, COALESCEd to the historical
// 0.6 default) must actually reach `CellEntity.move_speed` via
// `spawn_npc_from_record`, and a template that opts into a faster
// pace (e.g. an escort NPC, ~0.9/tick per the GC1b-0 feasibility
// pass) must move measurably farther per tick than the 0.6 default
// — not just carry a different number that nothing reads.

/// Build a minimal `SpawnRecord` for the movement-speed wiring test.
/// Field values mirror `spawner::tests::spawn_records::make_test_record`
/// (this module can't reach that private test helper across the
/// `spawner`/`service` module boundary, so it's duplicated narrowly).
fn make_spawn_record(move_speed: f32) -> crate::cell::spawner::SpawnRecord {
    crate::cell::spawner::SpawnRecord {
        spawn_id: 1,
        world_name: "Castle".to_string(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: None,
        template_id: 10,
        template_name: "Test Escort".to_string(),
        // Must be "mob" (class_id 0x04) -- `npc_movement_tick` sources
        // its candidate set from `ai_driven_npc_entity_ids`, which admits
        // a "being" (0x01) only in a behaviour state such as Follow
        // (NA24). This fixture stays Idle, so a being would be excluded
        // from the tick and both assertions would read 0.0.
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "GLB_Components.WorldObject_Small".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(1),
        alignment: Some(0),
        faction: Some(1),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
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
        move_speed,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
        vault_scope: cimmeria_entity::cell_entity::VaultScope::Personal,
    }
}

/// A template `move_speed` of 0.9 (GC1b-0's suggested escort speed) must
/// move an NPC farther per tick than the 0.6 historical default — proving
/// the DB column actually changes effective NPC speed, not just that it
/// round-trips through `SpawnRecord`. 0.6 and 0.9 are chosen to match
/// `construction.rs`'s hardcoded default and Marsh's seeded template
/// value exactly, so the assertions are exact `f32` equality.
#[test]
fn spawn_record_move_speed_produces_proportionally_faster_movement() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();

    let default_npc = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(default_npc, &make_spawn_record(0.6))
        .unwrap();
    let escort_npc = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(escort_npc, &make_spawn_record(0.9))
        .unwrap();

    assert_eq!(
        mgr.get_entity(default_npc).unwrap().move_speed,
        0.6,
        "SpawnRecord.move_speed=0.6 must land on CellEntity.move_speed unchanged"
    );
    assert_eq!(
        mgr.get_entity(escort_npc).unwrap().move_speed,
        0.9,
        "SpawnRecord.move_speed=0.9 must land on CellEntity.move_speed unchanged"
    );

    for npc_id in [default_npc, escort_npc] {
        if let Some(e) = mgr.get_entity_mut(npc_id) {
            e.nav_path
                .push_back(cimmeria_common::Vector3::new(100.0, 0.0, 0.0));
        }
    }

    npc_movement_tick(&mut mgr);

    let default_x = mgr.get_entity(default_npc).unwrap().position.x;
    let escort_x = mgr.get_entity(escort_npc).unwrap().position.x;
    assert_eq!(
        default_x, 0.6,
        "the 0.6 default must move exactly 0.6 units in one tick"
    );
    assert_eq!(
        escort_x, 0.9,
        "the 0.9 escort-speed template must move exactly 0.9 units in \
         one tick -- 50% farther than the 0.6 default per tick"
    );
    assert!(
        escort_x > default_x,
        "a higher template move_speed must produce more per-tick \
         movement than the default -- the DB column must actually \
         change effective NPC speed"
    );
}

/// Guards `effective_move_speed`'s two defensive branches directly.
///
/// The absent-stat fallback can't be reached through a real `StatList`:
/// `StatList::new()` always installs `movementSpeedMod` and exposes no
/// public way to remove an entry (same constraint `stats.rs`'s
/// `format_stat_line` documents for its `None` arm), so only the
/// default-mod and negative-`cur` branches are exercised here. A negative
/// `cur` is itself only reachable by poking the field directly — the
/// `.speed` path rejects out-of-range values and `Stat::set_current`
/// clamps — but the floor is what keeps a stray write from driving an NPC
/// backwards along its own path.
#[test]
fn legacy_p47_effective_move_speed_fallbacks_are_safe() {
    let mut stats = cimmeria_entity::stats::StatList::new();
    assert_eq!(
        effective_move_speed(5.0, &stats),
        5.0,
        "the default mod of 100 must leave move_speed unscaled"
    );

    stats.get_mut(MOVEMENT_SPEED_MOD).unwrap().cur = -250;
    assert_eq!(
        effective_move_speed(5.0, &stats),
        0.0,
        "a negative mod must stall the NPC, never reverse it"
    );
}

/// A leg starts when the path gets longer; consuming waypoints never
/// restarts the count.
#[test]
fn leg_step_restarts_only_when_a_new_path_is_installed() {
    let npc = 4_200_001;
    assert_eq!(leg_step_index(npc, 3), 1);
    assert_eq!(leg_step_index(npc, 3), 2);
    assert_eq!(leg_step_index(npc, 2), 3, "waypoint consumed: same leg");
    assert_eq!(leg_step_index(npc, 1), 4);
    assert_eq!(leg_step_index(npc, 4), 1, "longer path = new leg");
    assert_eq!(leg_step_index(npc + 1, 1), 1, "per NPC");
}
