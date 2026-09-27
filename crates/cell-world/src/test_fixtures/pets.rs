//! Pet fixtures (issue #570): a pet-shaped template in the startup cache and
//! a ready player owner, so every pet packet's tests start from the same
//! world.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::SpawnRecord;

/// Template id the pet fixtures cache. In the pets range (350-369), like the
/// seeded Jaffa Soldier (350).
pub const PET_FIXTURE_TEMPLATE_ID: i32 = 350;
/// Ability set the fixture template carries: the pet bar.
pub const PET_FIXTURE_ABILITIES: [i32; 2] = [592, 1652];

/// A pet template shaped like the PT-S Jaffa row: authored hostile at
/// level 3 with loot, a respawn timer, a patrol and a wander radius, so a
/// test can see the spawn path strip each of them.
pub fn pet_template_record(template_id: i32) -> SpawnRecord {
    SpawnRecord {
        spawn_id: -1,
        world_name: String::new(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: Some("template_tag".to_string()),
        template_id,
        template_name: "Jaffa Soldier (pet)".to_string(),
        class: "pet".to_string(),
        static_mesh: None,
        body_set: "BS_JaffaMale.BS_JaffaMale".to_string(),
        components: Some(vec!["Jaffa_Armor".to_string()]),
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(3),
        alignment: Some(0),
        faction: Some(10),
        name_id: Some(8087),
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        loot_table_id: Some(9),
        is_stationary: false,
        ability_ids: PET_FIXTURE_ABILITIES.to_vec(),
        respawn_secs: Some(30),
        patrol_path: vec![cimmeria_common::Vector3::new(1.0, 0.0, 1.0)],
        patrol_point_delay_secs: 2.0,
        wander_radius: 8.0,
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

/// Cache [`pet_template_record`] under `template_id` on `mgr`.
pub fn seed_pet_template(mgr: &mut SpaceManager, template_id: i32) {
    mgr.spawn_templates
        .insert(template_id, pet_template_record(template_id));
}

/// Create a connected, introducible player `entity_id` in `world` at
/// `position` with `level`: what a pet owner looks like once
/// `InitPlayerState` has run.
pub fn add_pet_owner(
    mgr: &mut SpaceManager,
    entity_id: u32,
    world: &str,
    position: [f32; 3],
    level: u32,
) {
    mgr.create_entity(entity_id, world, position, [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    if let Some(e) = mgr.get_entity_mut(entity_id) {
        e.account_id = Some(entity_id);
        e.player_id = Some(entity_id as i32 + 1000);
        e.archetype_id = Some(1);
        e.level = level;
    }
}

/// Owner entity id [`watched_pet_world`] uses.
pub const PET_FIXTURE_OWNER: u32 = 7;
/// A second player [`watched_pet_world`] puts beside the pet: sees it, does
/// not own it.
pub const PET_FIXTURE_OTHER: u32 = 8;

/// Agnos and Castle (shared, both loaded) plus Castle_CellBlock (instanced),
/// with the pet template cached. The world every pet lifecycle test starts
/// from.
pub fn make_pet_world() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
        <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
        <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    mgr
}

/// [`make_pet_world`] with [`PET_FIXTURE_OWNER`] (level 12) in Agnos, its
/// pet summoned, and [`PET_FIXTURE_OTHER`] standing beside them, after one
/// AoI tick so both players' witness sets hold the pet. Returns the pet id.
pub fn watched_pet_world() -> (SpaceManager, u32) {
    let mut mgr = make_pet_world();
    add_pet_owner(&mut mgr, PET_FIXTURE_OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let pet = mgr
        .spawn_pet_from_template(PET_FIXTURE_OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("pet spawns");
    add_pet_owner(&mut mgr, PET_FIXTURE_OTHER, "Agnos", [12.0, 0.0, 12.0], 5);
    let _ = mgr.compute_aoi_changes();
    (mgr, pet)
}

/// Drain `rx`; the witnesses sent `LeftAoI` for `entity`, sorted.
pub fn drain_left_aoi_for(rx: &mut mpsc::Receiver<CellToBaseMsg>, entity: u32) -> Vec<u32> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } = msg
        {
            if entity_id == entity {
                out.push(witness_id);
            }
        }
    }
    out.sort_unstable();
    out
}

/// Drain `rx`; `(witness, position, velocity)` of every `EntityMoved` for
/// `entity`, sorted by witness.
pub fn drain_entity_moved_for(
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    entity: u32,
) -> Vec<(u32, [f32; 3], [f32; 3])> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            position,
            velocity,
            ..
        } = msg
        {
            if entity_id == entity {
                out.push((witness_id, position, velocity));
            }
        }
    }
    out.sort_by_key(|m| m.0);
    out
}

/// Assert `pet` is gone everywhere: no entity, no registry entry, and no
/// witness set that still holds it.
pub fn assert_pet_fully_gone(mgr: &SpaceManager, owner: u32, pet: u32) {
    assert!(mgr.get_entity(pet).is_none(), "pet entity removed");
    assert!(mgr.pets.owner_of(pet).is_none(), "pet left the registry");
    assert!(mgr.pets.pets_of(owner).is_empty(), "owner has no pets left");
    for space in mgr.spaces.values() {
        for e in space.entities.values() {
            assert!(
                !e.witnesses.contains(&cimmeria_common::EntityId(pet as i32)),
                "no witness set still holds the pet"
            );
        }
    }
}
