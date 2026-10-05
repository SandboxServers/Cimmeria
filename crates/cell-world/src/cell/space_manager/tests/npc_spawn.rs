//! NPC spawn surface: class id, default ability, NPC-only iteration,
//! world lookup, and template-record application.

use cimmeria_common::Vector3;

use super::make_manager;

#[test]
fn spawn_npc_sets_class_id_and_spawn_position() {
    let mut mgr = make_manager();
    let pos = [50.0, 0.0, 75.0];
    mgr.spawn_npc(500, "Agnos", pos, [0.0; 3]).unwrap();

    let npc = mgr.get_entity(500).unwrap();
    assert_eq!(npc.class_id, 0x04); // SGWMob
    assert!(!npc.is_player);
    assert_eq!(npc.spawn_position.unwrap(), Vector3::new(50.0, 0.0, 75.0));
}

#[test]
fn spawn_npc_gets_default_ability() {
    let mut mgr = make_manager();
    mgr.spawn_npc(500, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    let npc = mgr.get_entity(500).unwrap();
    assert!(npc
        .abilities
        .has_ability(crate::cell::combat::NPC_DEFAULT_ABILITY));
}

#[test]
fn all_npc_entity_ids_returns_only_npcs() {
    let mut mgr = make_manager();
    // Add a player entity
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.connect_entity(1);
    // Add two NPC entities
    mgr.spawn_npc(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();

    let npc_ids = mgr.all_npc_entity_ids();
    assert_eq!(npc_ids.len(), 2);
    assert!(npc_ids.contains(&100));
    assert!(npc_ids.contains(&200));
    // Player should NOT be in the list
    assert!(!npc_ids.contains(&1));
}

#[test]
fn all_npc_entity_ids_empty_when_no_npcs() {
    let mgr = make_manager();
    assert!(mgr.all_npc_entity_ids().is_empty());
}

#[test]
fn get_entity_world_name_for_npc() {
    let mut mgr = make_manager();
    mgr.spawn_npc(500, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    assert_eq!(mgr.get_entity_world_name(500), Some("Agnos".to_string()));
}

#[test]
fn spawn_npc_from_record_sets_template_fields() {
    use crate::cell::spawner::SpawnRecord;
    let mut mgr = make_manager();
    let record = SpawnRecord {
        spawn_id: 1,
        world_name: "Agnos".to_string(),
        x: 10.0,
        y: 0.0,
        z: 20.0,
        heading: 1.5,
        class: "SGWMob".to_string(),
        template_id: 42,
        template_name: "TestGuard".to_string(),
        tag: Some("Guard01".to_string()),
        name_id: Some(1001),
        speaker_id: None,
        event_set_id: None,
        interaction_type: 0,
        flags: 0,
        faction: Some(10),
        alignment: Some(1),
        level: Some(5),
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        static_mesh: None,
        body_set: "BS_NID_Soldier.BS_NID_Soldier".to_string(),
        components: Some(vec!["Comp1".to_string()]),
        loot_table_id: Some(2),
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
        vault_scope: cimmeria_entity::cell_entity::VaultScope::Personal,
        training_dummy: false,
    };

    mgr.spawn_npc_from_record(600, &record).unwrap();

    let npc = mgr.get_entity(600).unwrap();
    assert_eq!(npc.template_id, Some(42));
    assert_eq!(npc.tag.as_deref(), Some("Guard01"));
    assert_eq!(npc.faction, 10);
    assert_eq!(npc.alignment, 1);
    assert_eq!(npc.level, 5);
    assert_eq!(npc.spawn_position.unwrap(), Vector3::new(10.0, 0.0, 20.0));
    // 592 = NPC_DEFAULT_ABILITY (Pistol Shot — was previously 597/Heal Focus).
    assert!(npc.abilities.has_ability(592));
    // Health should be scaled: 200 + (5 * 50) = 450
    assert_eq!(
        npc.stats.get(cimmeria_entity::stats::HEALTH).unwrap().max,
        450
    );
}

/// A record whose only varying field is `interaction_type`, for the
/// static-interaction derivation below.
fn record_with_flags(interaction_type: i64) -> crate::cell::spawner::SpawnRecord {
    crate::cell::spawner::SpawnRecord {
        spawn_id: 400,
        world_name: "Agnos".to_string(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        class: "mob".to_string(),
        template_id: 300,
        template_name: "Debug Hub - Vendor".to_string(),
        tag: Some("DebugHub_Vendor".to_string()),
        name_id: Some(8010),
        speaker_id: None,
        event_set_id: Some(570),
        interaction_type,
        flags: 0,
        faction: Some(1),
        alignment: Some(0),
        level: Some(1),
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
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
        vault_scope: cimmeria_entity::cell_entity::VaultScope::Personal,
        training_dummy: false,
    }
}

/// Any `INT_Vendor*` bit on the template makes the spawned NPC a Vendor, so
/// `handle_interact` reaches its store-open arm. Before the derivation
/// existed nothing set `NpcInteractionType::Vendor`, and a vendor-only
/// template (debug-hub template 300) dead-ended on a right-click; this fails
/// if the assignment in `spawn_npc_from_record_into` is removed.
#[test]
fn spawn_npc_from_record_derives_vendor_from_every_vendor_bit() {
    use cimmeria_entity::cell_entity::NpcInteractionType;
    use cimmeria_entity::interaction_flags::*;
    for bit in [
        INT_VENDOR_ARMOR,
        INT_VENDOR_WEAPONS,
        INT_VENDOR_CONSUMABLES,
        INT_VENDOR_GENERAL,
        INT_VENDOR_MISSION,
        INT_VENDOR_CRAFT_BIO,
        INT_VENDOR_CRAFT_POWER,
        INT_VENDOR_CRAFT_MATERIALS,
        INT_VENDOR_CRAFT_ELECTRONICS,
    ] {
        let mut mgr = make_manager();
        // A vendor bit OR'd with an unrelated one still counts.
        mgr.spawn_npc_from_record(600, &record_with_flags(bit | INT_TRAINER))
            .unwrap();
        assert_eq!(
            mgr.get_entity(600).unwrap().interaction_type,
            Some(NpcInteractionType::Vendor),
            "vendor bit {bit} must derive NpcInteractionType::Vendor",
        );
        assert_eq!(
            mgr.get_entity(600).unwrap().interaction_type_flags,
            bit | INT_TRAINER,
            "the flags themselves pass through unchanged",
        );
    }
}

/// No vendor or banker bit, no static interaction: the trainer, minigame, quest and
/// loot bits are all dispatched elsewhere, and deriving one of them here
/// would shadow that dispatch.
#[test]
fn spawn_npc_from_record_derives_nothing_without_a_vendor_bit() {
    use cimmeria_entity::interaction_flags::*;
    for flags in [
        0,
        INT_TRAINER,
        INT_MINIGAME_LIVEWIRE,
        INT_DHD,
        INT_NON_A_STORY_MISSION_AVAILABLE,
        INT_NORMAL_LOOT,
    ] {
        let mut mgr = make_manager();
        mgr.spawn_npc_from_record(600, &record_with_flags(flags))
            .unwrap();
        assert_eq!(
            mgr.get_entity(600).unwrap().interaction_type,
            None,
            "flags {flags} carry no vendor bit and must derive nothing",
        );
    }
}

/// `INT_BANKER` derives `Banker` with the template's `vault_scope`, for each
/// scope. Fails if the banker branch of `static_interaction_for_flags` is
/// removed (a banker-only template then derives nothing).
#[test]
fn spawn_npc_from_record_derives_banker_with_its_vault_scope() {
    use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope};
    use cimmeria_entity::interaction_flags::INT_BANKER;
    for scope in [VaultScope::Personal, VaultScope::Team, VaultScope::Command] {
        let mut mgr = make_manager();
        let record = crate::cell::spawner::SpawnRecord {
            vault_scope: scope,
            training_dummy: false,
            ..record_with_flags(INT_BANKER)
        };
        mgr.spawn_npc_from_record(600, &record).unwrap();
        assert_eq!(
            mgr.get_entity(600).unwrap().interaction_type,
            Some(NpcInteractionType::Banker { scope }),
        );
    }
}

/// Precedence: a template with both `INT_BANKER` and a vendor bit is a
/// Banker (BV-02, documented on `static_interaction_for_flags`). And
/// `vault_scope` means nothing without the banker bit: a vendor with a
/// non-default scope is still just a Vendor.
#[test]
fn banker_bit_wins_over_vendor_bits_and_scope_needs_the_banker_bit() {
    use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope};
    use cimmeria_entity::interaction_flags::{INT_BANKER, INT_VENDOR_GENERAL};

    let mut mgr = make_manager();
    mgr.spawn_npc_from_record(600, &record_with_flags(INT_BANKER | INT_VENDOR_GENERAL))
        .unwrap();
    assert_eq!(
        mgr.get_entity(600).unwrap().interaction_type,
        Some(NpcInteractionType::Banker {
            scope: VaultScope::Personal
        }),
    );

    let mut mgr = make_manager();
    let record = crate::cell::spawner::SpawnRecord {
        vault_scope: VaultScope::Team,
        training_dummy: false,
        ..record_with_flags(INT_VENDOR_GENERAL)
    };
    mgr.spawn_npc_from_record(600, &record).unwrap();
    assert_eq!(
        mgr.get_entity(600).unwrap().interaction_type,
        Some(NpcInteractionType::Vendor),
    );
}

/// BM-07: `INT_AUCTION` derives `Auctioneer`, the one marker the Black
/// Market's open and trade checks accept. It ranks below the Banker and
/// above the vendor bits. Fails if the auction branch of
/// `static_interaction_for_flags` is removed (the hub auctioneer then
/// derives nothing, and every open at it is refused).
#[test]
fn spawn_npc_from_record_derives_auctioneer_from_the_auction_bit() {
    use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope};
    use cimmeria_entity::interaction_flags::{INT_AUCTION, INT_BANKER, INT_VENDOR_GENERAL};

    for (flags, expected) in [
        (INT_AUCTION, NpcInteractionType::Auctioneer),
        (
            INT_AUCTION | INT_VENDOR_GENERAL,
            NpcInteractionType::Auctioneer,
        ),
        (
            INT_AUCTION | INT_BANKER,
            NpcInteractionType::Banker {
                scope: VaultScope::Personal,
            },
        ),
    ] {
        let mut mgr = make_manager();
        mgr.spawn_npc_from_record(600, &record_with_flags(flags))
            .unwrap();
        assert_eq!(
            mgr.get_entity(600).unwrap().interaction_type,
            Some(expected),
            "flags {flags}",
        );
    }
}
