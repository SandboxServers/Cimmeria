//! Regression coverage for the bandolier-related fields woven into the
//! `mapLoaded` body: per-slot AmmoSlot stat seeding, the active-slot
//! indicator, and the active slot's `AmmoTypeId` entity property.

use super::super::*;
use super::sample_player_load_data;
use super::sample_world_entry;

/// Regression: at world entry the `mapLoaded` packet's `onStatUpdate`
/// must seed AmmoSlot{N} stats from the persisted bandolier ammo.
/// Previously it built a fresh `StatList::new()` (defaults `(0, 0, 0)`)
/// so on re-login the bandolier UI showed an empty mag until the next
/// reload — even though the cell-side `bandolier_items.current_ammo`
/// loaded correctly from `sgw_inventory.ammo`.
#[test]
fn build_map_loaded_seeds_ammo_slot_stats_from_bandolier_items() {
    use cimmeria_entity::cell_entity::BandolierItem;

    let data = PlayerLoadData {
        player_id: 1,
        level: 5,
        player_name: "Reloader".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 2,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        weapon_visual: None,
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 0,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: archetype_ability_tree(2),
        items: vec![],
        active_bandolier_slot: 0,
        // Slot 0: 8 of 15 (mid-mag), slot 2: 12 of 12 (full).
        // Slot 1, 3, 4 are empty — their AmmoSlot stats must stay
        // at the default (0, 0, 0).
        bandolier_items: vec![
            (
                0,
                BandolierItem {
                    item_id: 100,
                    clip_size: 15,
                    default_ammo_type: 1,
                    current_ammo: 8,
                    cur_ammo_type: 1,
                },
            ),
            (
                2,
                BandolierItem {
                    item_id: 101,
                    clip_size: 12,
                    default_ammo_type: 7,
                    current_ammo: 12,
                    cur_ammo_type: 7,
                },
            ),
        ],
    };
    let entry = WorldEntryInfo {
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
    // Assert against the raw, unfragmented mapLoaded body to avoid coupling
    // the regression check to Mercury framing/footers.
    let all_bytes = build_map_loaded_body(100, &data, &entry);

    // StatUpdate wire format (16 bytes per stat):
    //   stat_id:i32 LE | min:i32 LE | cur:i32 LE | max:i32 LE
    //
    // Stat IDs: AMMO_SLOT_1=49, AMMO_SLOT_3=51 (skipping the empty slot 1).
    let ammo_slot_1_tuple: [u8; 16] = [
        49, 0, 0, 0, // stat_id = 49 (AMMO_SLOT_1)
        0, 0, 0, 0, // min = 0
        8, 0, 0, 0, // cur = 8
        15, 0, 0, 0, // max = 15
    ];
    let ammo_slot_3_tuple: [u8; 16] = [
        51, 0, 0, 0, // stat_id = 51 (AMMO_SLOT_3 — slot index 2)
        0, 0, 0, 0, // min = 0
        12, 0, 0, 0, // cur = 12
        12, 0, 0, 0, // max = 12
    ];

    let contains = |needle: &[u8]| all_bytes.windows(needle.len()).any(|w| w == needle);

    assert!(
        contains(&ammo_slot_1_tuple),
        "mapLoaded onStatUpdate must include AmmoSlot1 = (0, 8, 15) from persisted bandolier item",
    );
    assert!(
        contains(&ammo_slot_3_tuple),
        "mapLoaded onStatUpdate must include AmmoSlot3 = (0, 12, 12) from persisted bandolier item",
    );

    // Empty slot 1 (stat id 50) must NOT appear with a non-zero cur value.
    // We look for a slot-1 tuple matching some plausible stale value (15)
    // to make sure no leak from elsewhere overwrote it.
    let stale_slot_2: [u8; 16] = [50, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 15, 0, 0, 0];
    assert!(
        !contains(&stale_slot_2),
        "empty slot 1 should not have stale clip_size in its AmmoSlot stat",
    );
}

/// Regression for the "switching back to slot 1 doesn't give my weapon
/// back" bug surfaced in playtesting: on login, the client UI defaults
/// the bandolier slot indicator to wire slot 1 regardless of the
/// persisted `bandolier_slot`. If we don't seed the indicator from
/// `data.active_bandolier_slot` in mapLoaded, the LUA
/// `getActiveSlotForContainer(...) ~= N` keybind guard turns subsequent
/// slot-change keypresses into client-side no-ops once the cell-side
/// state diverges.
///
/// Asserts that `mapLoaded` carries `onActiveSlotUpdate(bag_id=3,
/// wire_slot=server_slot+1)` so the client UI starts from the right slot.
#[test]
fn build_map_loaded_seeds_active_slot_indicator_from_persisted_slot() {
    use cimmeria_entity::cell_entity::BandolierItem;

    let data = PlayerLoadData {
        player_id: 1,
        level: 1,
        player_name: "SeedActive".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 2,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        weapon_visual: None,
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 0,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: archetype_ability_tree(2),
        items: vec![],
        // Persisted bandolier_slot = 2 (server-internal). The wire packet
        // must report slot 3 (= server slot + 1) so the client UI
        // highlights the correct bandolier window.
        active_bandolier_slot: 2,
        bandolier_items: vec![(
            2,
            BandolierItem {
                item_id: 100,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        )],
    };
    let entry = WorldEntryInfo {
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
    let all_bytes = build_map_loaded_body(100, &data, &entry);

    // onActiveSlotUpdate args layout (8 bytes):
    //   bag_id:i32 LE | wire_slot:i32 LE
    // For bandolier (bag 3) at server slot 2, wire slot is 3 (1-indexed).
    let expected_active_slot_args: [u8; 8] = [
        3, 0, 0, 0, // bag_id = 3 (bandolier)
        3, 0, 0, 0, // wire_slot = active_bandolier_slot + 1
    ];
    let contains = |needle: &[u8]| all_bytes.windows(needle.len()).any(|w| w == needle);
    assert!(
        contains(&expected_active_slot_args),
        "mapLoaded must carry onActiveSlotUpdate(3, server_slot+1) so the \
         client UI initialises to the persisted bandolier slot",
    );

    // Negative pin: the *server-internal* slot must not appear as the
    // wire value. If someone reverts the `+1` translation this assertion
    // is what catches it.
    let wrong_wire_args: [u8; 8] = [3, 0, 0, 0, 2, 0, 0, 0];
    assert!(
        !contains(&wrong_wire_args),
        "wire slot must be 1-indexed (`Bag.py:369`); a 0-indexed `2` would \
         desync the client UI by one slot",
    );
}

/// Regression guard: `active_bandolier_slot` selects which bandolier item's
/// `cur_ammo_type` is sent as the `AmmoTypeId` (prop_id = 3) entity property.
/// If this regresses (e.g. always reads slot 0), the client UI shows the
/// wrong ammo for non-default active weapons after re-login.
#[test]
fn active_bandolier_slot_drives_ammo_type_id_property() {
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut data = sample_player_load_data();
    data.active_bandolier_slot = 2;
    data.bandolier_items = vec![
        (
            0,
            BandolierItem {
                item_id: 100,
                clip_size: 15,
                default_ammo_type: 1,
                current_ammo: 8,
                cur_ammo_type: 1,
            },
        ),
        (
            2,
            BandolierItem {
                item_id: 200,
                clip_size: 12,
                default_ammo_type: 7,
                current_ammo: 12,
                cur_ammo_type: 7,
            },
        ),
    ];
    let entry = sample_world_entry();

    let body = build_map_loaded_body(entry.player_entity_id, &data, &entry);

    // onEntityProperty is direct-encoded (index 7 < 61): 0x87 | u16 word_len=12
    // | u32 entity_id | i32 prop_id | i32 value. Match prop_id = 3 (AmmoTypeId)
    // for our entity_id and pull the i32 value.
    let msg_id: u8 = (method_idx::ON_ENTITY_PROPERTY as u8) | 0x80;
    let eid = entry.player_entity_id.to_le_bytes();
    // payload_len = 4 (entity_id) + 8 (prop_id + value args) = 12
    let prefix = [msg_id, 12, 0, eid[0], eid[1], eid[2], eid[3], 3, 0, 0, 0];

    let pos = body
        .windows(prefix.len())
        .position(|w| w == prefix)
        .expect("AmmoTypeId onEntityProperty not found in mapLoaded body");
    let val_off = pos + prefix.len();
    let value = i32::from_le_bytes([
        body[val_off],
        body[val_off + 1],
        body[val_off + 2],
        body[val_off + 3],
    ]);
    assert_eq!(
        value, 7,
        "AmmoTypeId must equal slot 2's cur_ammo_type (active_bandolier_slot=2), not slot 0's",
    );
}

/// Regression guard companion: when `active_bandolier_slot` points at a slot
/// with no item, `AmmoTypeId` falls back to 0 — matches legacy "no weapon
/// equipped" behavior. Without the fallback the wire could leak whatever
/// happened to be in the previously-active slot.
#[test]
fn active_bandolier_slot_with_no_item_emits_ammo_type_zero() {
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut data = sample_player_load_data();
    data.active_bandolier_slot = 1; // no item lives here
    data.bandolier_items = vec![(
        0,
        BandolierItem {
            item_id: 100,
            clip_size: 15,
            default_ammo_type: 9,
            current_ammo: 8,
            cur_ammo_type: 9,
        },
    )];
    let entry = sample_world_entry();

    let body = build_map_loaded_body(entry.player_entity_id, &data, &entry);

    let msg_id: u8 = (method_idx::ON_ENTITY_PROPERTY as u8) | 0x80;
    let eid = entry.player_entity_id.to_le_bytes();
    // payload_len = 4 (entity_id) + 8 (prop_id + value args) = 12
    let prefix = [msg_id, 12, 0, eid[0], eid[1], eid[2], eid[3], 3, 0, 0, 0];

    let pos = body
        .windows(prefix.len())
        .position(|w| w == prefix)
        .expect("AmmoTypeId onEntityProperty not found in mapLoaded body");
    let val_off = pos + prefix.len();
    let value = i32::from_le_bytes([
        body[val_off],
        body[val_off + 1],
        body[val_off + 2],
        body[val_off + 3],
    ]);
    assert_eq!(
        value, 0,
        "AmmoTypeId must fall back to 0 when active slot is empty"
    );
}
