use super::*;

fn make_entity() -> CellEntity {
    CellEntity::new(EntityId(1), SpaceId(100), Vector3::new(10.0, 0.0, 20.0))
}

#[test]
fn new_entity_defaults() {
    let entity = make_entity();
    assert_eq!(entity.entity_id, EntityId(1));
    assert_eq!(entity.space_id, SpaceId(100));
    assert_eq!(entity.position, Vector3::new(10.0, 0.0, 20.0));
    assert_eq!(entity.direction, Vector3::zero());
    assert!(entity.is_on_ground);
    assert!(entity.witnesses.is_empty());
    assert_eq!(entity.aoi_radius, 100.0);
    assert_eq!(entity.ring_source_id, None);
    assert_eq!(entity.destination_ring_id, None);
}

#[test]
fn set_and_get_position() {
    let mut entity = make_entity();
    let new_pos = Vector3::new(50.0, 5.0, 60.0);
    entity.set_position(new_pos);
    assert_eq!(*entity.get_position(), new_pos);
}

#[test]
fn add_and_remove_witness() {
    let mut entity = make_entity();
    entity.add_witness(EntityId(2));
    entity.add_witness(EntityId(3));
    assert_eq!(entity.get_witnesses().len(), 2);
    assert!(entity.get_witnesses().contains(&EntityId(2)));

    entity.remove_witness(EntityId(2));
    assert_eq!(entity.get_witnesses().len(), 1);
    assert!(!entity.get_witnesses().contains(&EntityId(2)));
}

#[test]
fn duplicate_witness_is_idempotent() {
    let mut entity = make_entity();
    entity.add_witness(EntityId(2));
    entity.add_witness(EntityId(2));
    assert_eq!(entity.get_witnesses().len(), 1);
}

#[test]
fn remove_absent_witness_is_noop() {
    let mut entity = make_entity();
    entity.remove_witness(EntityId(99)); // no panic
    assert!(entity.get_witnesses().is_empty());
}

#[test]
fn is_in_aoi_within_radius() {
    let entity = make_entity(); // pos = (10, 0, 20), radius = 100
    let nearby = Vector3::new(20.0, 0.0, 25.0);
    assert!(entity.is_in_aoi(&nearby));
}

#[test]
fn is_in_aoi_outside_radius() {
    let entity = make_entity(); // pos = (10, 0, 20), radius = 100
    let far_away = Vector3::new(500.0, 0.0, 500.0);
    assert!(!entity.is_in_aoi(&far_away));
}

#[test]
fn is_in_aoi_at_exact_boundary() {
    let mut entity = make_entity();
    entity.aoi_radius = 10.0;
    // Point exactly 10 units away on the X axis
    let boundary = Vector3::new(20.0, 0.0, 20.0);
    assert!(entity.is_in_aoi(&boundary));
}

// ── New field defaults ─────────────────────────────────────────────────

#[test]
fn new_entity_ammo_defaults_empty() {
    let entity = make_entity();
    // Stage C: shadow scalars are gone — assert the helper-derived view
    // (no items in bandolier => zero everything) and that no slot is dirty.
    assert_eq!(entity.active_ammo(), 0);
    assert_eq!(entity.active_clip_size(), 0);
    assert_eq!(entity.active_ammo_type(), 0);
    assert!(entity.bandolier_items.is_empty());
    assert!(entity.bandolier_ammo_dirty.is_empty());
}

#[test]
fn new_entity_ai_state_defaults_idle() {
    let entity = make_entity();
    assert_eq!(entity.ai_state, AiState::Idle);
    assert!(entity.threat_list.is_empty());
    assert!(entity.spawn_position.is_none());
    assert_eq!(entity.ai_cooldown_ticks, 0);
}

#[test]
fn new_entity_saved_missions_loaded_false() {
    let entity = make_entity();
    assert!(!entity.saved_missions_loaded);
}

#[test]
fn ai_state_equality() {
    assert_eq!(AiState::Idle, AiState::Idle);
    assert_eq!(AiState::Fighting, AiState::Fighting);
    assert_eq!(AiState::Dead, AiState::Dead);
    assert_eq!(AiState::Leashing, AiState::Leashing);
    assert_ne!(AiState::Idle, AiState::Fighting);
    assert_ne!(AiState::Dead, AiState::Leashing);
}

#[test]
fn threat_list_operations() {
    let mut entity = make_entity();
    entity.threat_list.insert(10, 50.0);
    entity.threat_list.insert(20, 100.0);
    assert_eq!(entity.threat_list.len(), 2);
    assert_eq!(entity.threat_list[&20], 100.0);

    // Accumulate threat
    *entity.threat_list.entry(10).or_insert(0.0) += 25.0;
    assert_eq!(entity.threat_list[&10], 75.0);

    // Top threat target
    let top = entity
        .threat_list
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(&id, _)| id);
    assert_eq!(top, Some(20));

    entity.threat_list.clear();
    assert!(entity.threat_list.is_empty());
}

#[test]
fn spawn_position_stores_and_retrieves() {
    let mut entity = make_entity();
    assert!(entity.spawn_position.is_none());

    let spawn = Vector3::new(100.0, 5.0, 200.0);
    entity.spawn_position = Some(spawn);
    assert_eq!(entity.spawn_position.unwrap(), spawn);
}

// ── Bandolier ammo helpers ──────────────────────────────────────────────

fn make_bandolier_item(item_id: i32, clip: i32) -> BandolierItem {
    BandolierItem {
        item_id,
        clip_size: clip,
        default_ammo_type: 1,
        current_ammo: clip,
        cur_ammo_type: 1,
    }
}

#[test]
fn active_ammo_helpers_with_no_item_return_zero() {
    let entity = make_entity();
    assert_eq!(entity.active_ammo(), 0);
    assert_eq!(entity.active_clip_size(), 0);
    assert_eq!(entity.active_ammo_type(), 0);
}

#[test]
fn active_ammo_helpers_read_from_active_slot() {
    let mut entity = make_entity();
    entity.active_bandolier_slot = 1;
    entity
        .bandolier_items
        .insert(0, make_bandolier_item(100, 30));
    entity.bandolier_items.insert(
        1,
        BandolierItem {
            item_id: 200,
            clip_size: 12,
            default_ammo_type: 2,
            current_ammo: 8,
            cur_ammo_type: 2,
        },
    );

    assert_eq!(entity.active_ammo(), 8);
    assert_eq!(entity.active_clip_size(), 12);
    assert_eq!(entity.active_ammo_type(), 2);
}

/// Seed an AmmoSlot stat so the `set_current()` clamp doesn't pin to 0.
/// Stage B's world-entry init does this for real; tests have to mirror it
/// because Stage A leaves the helper callable but unseeded by default.
fn seed_ammo_stat(entity: &mut CellEntity, slot_id: i32, clip: i32) {
    let stat_id = crate::stats::AMMO_SLOT_1 + slot_id;
    if let Some(stat) = entity.stats.get_mut(stat_id) {
        stat.update(0, clip, clip);
        stat.clear_dirty();
    }
}

#[test]
fn set_slot_ammo_clamps_and_marks_dirty() {
    let mut entity = make_entity();
    entity
        .bandolier_items
        .insert(0, make_bandolier_item(100, 30));
    seed_ammo_stat(&mut entity, 0, 30);

    // Clamp above clip_size.
    let result = entity.set_slot_ammo(0, 999);
    assert_eq!(result, Some(30));
    assert!(entity.bandolier_ammo_dirty.contains(&0));
    assert_eq!(entity.stats.get(crate::stats::AMMO_SLOT_1).unwrap().cur, 30);

    // Clamp below zero.
    entity.bandolier_ammo_dirty.clear();
    let result = entity.set_slot_ammo(0, -5);
    assert_eq!(result, Some(0));
    assert!(entity.bandolier_ammo_dirty.contains(&0));
    assert_eq!(entity.stats.get(crate::stats::AMMO_SLOT_1).unwrap().cur, 0);
}

#[test]
fn set_slot_ammo_unequipped_returns_none() {
    let mut entity = make_entity();
    let result = entity.set_slot_ammo(2, 5);
    assert_eq!(result, None);
    assert!(entity.bandolier_ammo_dirty.is_empty());
}

#[test]
fn refill_active_slot_fills_to_clip_size() {
    let mut entity = make_entity();
    entity.bandolier_items.insert(
        0,
        BandolierItem {
            item_id: 100,
            clip_size: 30,
            default_ammo_type: 1,
            current_ammo: 5,
            cur_ammo_type: 1,
        },
    );
    seed_ammo_stat(&mut entity, 0, 30);

    let result = entity.refill_active_slot();
    assert_eq!(result, Some(30));
    assert_eq!(entity.bandolier_items[&0].current_ammo, 30);
    let stat = entity.stats.get(crate::stats::AMMO_SLOT_1).unwrap();
    assert_eq!(stat.cur, 30);
}

#[test]
fn refill_active_slot_unequipped_returns_none() {
    let mut entity = make_entity();
    let result = entity.refill_active_slot();
    assert_eq!(result, None);
}

// ── Holster / appearance-components tests ──────────────────────────────
//
// These pin the wire-format contract from issue #333: `BeingAppearance`'s
// `ComponentList` is what the SGW client picks the holster pose from
// (`CompositedAppearanceProxy::ApplyToPawn` at
// `ghidra://SGW.exe@0x00ec0840` — keys on whichever weapon-shaped entry
// it finds in the list, defaults to `WEAP_Melee = 4` when none), not
// any bit of `bStateField`. The receiver-side dispatch
// `GameBeing_OnStateFieldUpdate` at `ghidra://SGW.exe@0x00e01c90`
// confirms it tests only bits 0-7. So the entire holster mechanism on
// the server side reduces to "do or don't include `weapon_visual` in
// the `appearance_components()` output."

#[test]
fn appearance_components_no_weapon_is_components_unchanged() {
    let mut entity = make_entity();
    entity.components = vec!["torso".into(), "head".into(), "boots".into()];
    entity.weapon_visual = None;

    // Holstered or not, with no weapon there's nothing to filter.
    entity.weapon_holstered = true;
    assert_eq!(entity.appearance_components(), entity.components);

    entity.weapon_holstered = false;
    assert_eq!(entity.appearance_components(), entity.components);
}

#[test]
fn appearance_components_holstered_filters_weapon_visual() {
    let mut entity = make_entity();
    entity.components = vec![
        "torso".into(),
        "pistol".into(), // the weapon
        "head".into(),
    ];
    entity.weapon_visual = Some("pistol".into());
    entity.weapon_holstered = true;

    let wire = entity.appearance_components();
    assert!(
        !wire.contains(&"pistol".to_string()),
        "weapon visual must be filtered out when holstered"
    );
    assert!(wire.contains(&"torso".to_string()));
    assert!(wire.contains(&"head".to_string()));
    assert_eq!(wire.len(), 2);
}

#[test]
fn appearance_components_drawn_includes_weapon_visual() {
    let mut entity = make_entity();
    entity.components = vec!["torso".into(), "pistol".into(), "head".into()];
    entity.weapon_visual = Some("pistol".into());
    entity.weapon_holstered = false;

    let wire = entity.appearance_components();
    assert_eq!(
        wire, entity.components,
        "weapon visual must be present in the wire ComponentList when drawn"
    );
}

#[test]
fn set_weapon_holstered_signals_rebroadcast_only_when_relevant() {
    let mut entity = make_entity();
    entity.weapon_visual = Some("pistol".into());
    entity.weapon_holstered = false;

    // Real state change with a weapon present — caller should rebroadcast.
    assert!(
        entity.set_weapon_holstered(true),
        "holster transition with weapon visual present must signal a rebroadcast"
    );
    assert!(entity.weapon_holstered);

    // No-op call (already in the requested state).
    assert!(
        !entity.set_weapon_holstered(true),
        "no-op call must not request a rebroadcast"
    );

    // Toggle back — another real transition.
    assert!(entity.set_weapon_holstered(false));
    assert!(!entity.weapon_holstered);
}

#[test]
fn set_weapon_holstered_without_weapon_does_not_signal_rebroadcast() {
    // A player with an empty bandolier slot has `weapon_visual = None`.
    // The bool can still flip server-side (in case a future weapon-pickup
    // path needs to seed it), but the ComponentList won't change either
    // way, so the caller has nothing to broadcast.
    let mut entity = make_entity();
    entity.weapon_visual = None;
    entity.weapon_holstered = false;

    assert!(
        !entity.set_weapon_holstered(true),
        "without a weapon visual, holster toggle is wire-invisible"
    );
    assert!(entity.weapon_holstered, "the bool still flips internally");
}

#[test]
fn new_entity_defaults_to_holstered_with_no_weapon() {
    // Spawn invariant: a fresh entity has no weapon and is "holstered"
    // by default. When a player loads in with a populated active
    // bandolier slot, `weapon_visual` gets set to the weapon's visual
    // component and `weapon_holstered = true` keeps it out of the wire
    // ComponentList until a fire/reload/explicit-draw event flips it.
    let entity = make_entity();
    assert_eq!(entity.weapon_visual, None);
    assert!(entity.weapon_holstered);
    assert_eq!(entity.appearance_components(), entity.components);
}
