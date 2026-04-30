//! Tests for the `world_data` module — kept separate from `mod.rs` glue
//! to keep that file readable.

#![cfg(test)]

use super::*;
use cimmeria_mercury::encryption::MercuryEncryption;

const TEST_KEY: [u8; 32] = [0x42u8; 32];

#[test]
fn build_on_player_data_loaded_uses_correct_msg_id() {
    let out = build_on_player_data_loaded(&TEST_KEY, 1, &[], 42);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&out).unwrap();
    // body starts at offset 1, method_index=115 >= 61, so extended: 0xBD
    assert_eq!(pt[1], 0xBD);
}

#[test]
fn build_setup_world_parameters_uses_correct_msg_id() {
    let out = build_setup_world_parameters(&TEST_KEY, 1, &[], 42);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&out).unwrap();
    // method_index=122 >= 61, so extended: 0xBD
    assert_eq!(pt[1], 0xBD);
}

#[test]
fn build_map_loaded_produces_multiple_packets() {
    let data = PlayerLoadData {
        player_id: 1,
        level: 1,
        player_name: "Test".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 1,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec!["head_test".into()],
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 1,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: Default::default(),
        items: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
    };
    let entry = WorldEntryInfo { player_entity_id: 42, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02, world_stargates: vec![] };
    let (packets, seqs) = build_map_loaded(&TEST_KEY, 5, &[], 42, &data, &entry);
    assert!(!packets.is_empty(), "mapLoaded should produce at least one packet");
    assert_eq!(seqs as usize, packets.len(), "seqs_consumed should match packet count");
    for (i, pkt) in packets.iter().enumerate() {
        assert!(!pkt.is_empty(), "packet {} should not be empty", i);
    }
}

#[test]
fn build_map_loaded_each_packet_decrypts_within_limit() {
    let data = PlayerLoadData {
        player_id: 1,
        level: 5,
        player_name: "Warrior".into(),
        extra_name: "The Brave".into(),
        alignment: 2,
        archetype: 2,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        exp: 500,
        naquadah: 100,
        known_stargates: vec![1, 2],
        abilities: vec![10, 20],
        training_points: 3,
        applied_science_points: 1,
        blueprint_ids: vec![],
        first_login: 0,
        access_level: 0,
        skin_color_id: 5,
        ability_tree: archetype_ability_tree(2),
        items: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
    };
    let entry = WorldEntryInfo { player_entity_id: 100, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02, world_stargates: vec![] };
    let (packets, _seqs) = build_map_loaded(&TEST_KEY, 5, &[], 100, &data, &entry);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    // Mercury MAX_BODY_LENGTH is 1411 bytes
    const MAX_PLAINTEXT: usize = 1411;
    for (i, pkt) in packets.iter().enumerate() {
        let pt = enc.decrypt(pkt).unwrap();
        assert!(pt.len() <= MAX_PLAINTEXT,
            "packet {} plaintext {} bytes exceeds {} limit", i, pt.len(), MAX_PLAINTEXT);
    }
}

#[test]
fn build_map_loaded_contains_setup_world_params_and_player_data_loaded() {
    let data = PlayerLoadData {
        player_id: 1,
        level: 5,
        player_name: "Warrior".into(),
        extra_name: "The Brave".into(),
        alignment: 2,
        archetype: 2,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        exp: 500,
        naquadah: 100,
        known_stargates: vec![1, 2],
        abilities: vec![10, 20],
        training_points: 3,
        applied_science_points: 1,
        blueprint_ids: vec![],
        first_login: 0,
        access_level: 0,
        skin_color_id: 5,
        ability_tree: archetype_ability_tree(2),
        items: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
    };
    let entry = WorldEntryInfo { player_entity_id: 100, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02, world_stargates: vec![] };
    let (packets, _seqs) = build_map_loaded(&TEST_KEY, 5, &[], 100, &data, &entry);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);

    // Collect all decrypted plaintext across all packets
    let mut all_bytes = Vec::new();
    for pkt in &packets {
        let pt = enc.decrypt(pkt).unwrap();
        // Include everything after the flags byte (body + footers).
        // We're just checking for presence of specific marker bytes.
        all_bytes.extend_from_slice(&pt[1..]);
    }
    // setupWorldParameters = 0xFA should be present
    assert!(all_bytes.contains(&0xFA), "should contain setupWorldParameters (0xFA)");
    // onPlayerDataLoaded = 0xF3 should be present
    assert!(all_bytes.contains(&0xF3), "should contain onPlayerDataLoaded (0xF3)");
    // onAbilityTreeInfo uses extended 0xBD
    assert!(all_bytes.contains(&0xBD), "should contain extended encoding marker (0xBD)");
}

#[test]
fn build_map_loaded_uses_mercury_fragmentation() {
    use cimmeria_mercury::packet::{FLAG_FRAGMENTED, FLAG_HAS_SEQUENCE};

    let data = PlayerLoadData {
        player_id: 1,
        level: 5,
        player_name: "Warrior".into(),
        extra_name: "The Brave".into(),
        alignment: 2,
        archetype: 2,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec!["head_test".into(), "torso_test".into()],
        exp: 500,
        naquadah: 100,
        known_stargates: vec![1, 2],
        abilities: vec![10, 20, 30],
        training_points: 3,
        applied_science_points: 1,
        blueprint_ids: vec![],
        first_login: 0,
        access_level: 0,
        skin_color_id: 5,
        ability_tree: archetype_ability_tree(2),
        items: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
    };
    let entry = WorldEntryInfo {
        player_entity_id: 100, space_id: 65552,
        pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02,
        world_stargates: vec![],
    };
    let (packets, seqs) = build_map_loaded(&TEST_KEY, 10, &[], 100, &data, &entry);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);

    // Must produce multiple packets (body > 1300 bytes)
    assert!(packets.len() > 1,
        "Expected multiple fragments, got {} packet(s)", packets.len());
    assert_eq!(seqs as usize, packets.len());

    for (i, pkt) in packets.iter().enumerate() {
        let pt = enc.decrypt(pkt).unwrap();
        let flags = pt[0];

        // Every fragment must have FLAG_FRAGMENTED (0x20) and FLAG_HAS_SEQUENCE (0x40)
        assert!(flags & FLAG_FRAGMENTED != 0,
            "Packet {} flags=0x{:02X} missing FLAG_FRAGMENTED (0x20)", i, flags);
        assert!(flags & FLAG_HAS_SEQUENCE != 0,
            "Packet {} flags=0x{:02X} missing FLAG_HAS_SEQUENCE (0x40)", i, flags);

        // Parse footers from the end: seq_id (4), frag_end (4), frag_begin (4)
        let len = pt.len();
        let seq_id = u32::from_le_bytes(pt[len-4..len].try_into().unwrap());
        let frag_end = u32::from_le_bytes(pt[len-8..len-4].try_into().unwrap());
        let frag_begin = u32::from_le_bytes(pt[len-12..len-8].try_into().unwrap());

        // frag_begin should be base_seq (10)
        assert_eq!(frag_begin, 10,
            "Packet {} frag_begin={} expected 10", i, frag_begin);
        // frag_end should be base_seq + num_frags - 1
        let expected_frag_end = 10 + packets.len() as u32 - 1;
        assert_eq!(frag_end, expected_frag_end,
            "Packet {} frag_end={} expected {}", i, frag_end, expected_frag_end);
        // seq_id should be base_seq + i
        assert_eq!(seq_id, 10 + i as u32,
            "Packet {} seq_id={} expected {}", i, seq_id, 10 + i as u32);

        // Body chunk size: for non-last fragments, should be FRAGMENT_BODY_SIZE (1300)
        let body_len = len - 1 - 12; // flags(1) + footers(12) subtracted
        if i < packets.len() - 1 {
            assert_eq!(body_len, 1300,
                "Packet {} body_len={} expected 1300 (FRAGMENT_BODY_SIZE)", i, body_len);
        }

        eprintln!("Fragment {}: plaintext_len={} flags=0x{:02X} body={} seq={} frag=[{}-{}]",
            i, pt.len(), flags, body_len, seq_id, frag_begin, frag_end);
    }
}

#[test]
fn level_exp_boundaries() {
    assert_eq!(super::stats::level_exp(0), 0);
    assert_eq!(super::stats::level_exp(1), 100);
    assert_eq!(super::stats::level_exp(10), 9000);
    assert_eq!(super::stats::level_exp(20), 400000);
    assert_eq!(super::stats::level_exp(99), 400000); // clamped
}

#[test]
fn archetype_stats_commando_differs() {
    let soldier = archetype_stats(1);
    let commando = archetype_stats(2);
    assert_eq!(soldier.coordination, 5);
    assert_eq!(commando.coordination, 4);
    assert_eq!(commando.perception, 5);
}

/// Verify BeingAppearance wire encoding matches C++ reference:
///   msg_id=0x9A (index 26, direct: 26|0x80)
///   word_len=u16 LE (entity_id + bodyset WSTRING + array WSTRING)
///   entity_id=u32 LE
///   bodyset: [u32 char_count][UTF-16LE data]
///   components: [u32 element_count][WSTRING element]*
#[test]
fn being_appearance_wire_encoding() {
    let entity_id: u32 = 42;
    let bodyset = "BS_HumanMale.BS_HumanMale";
    let components = vec![
        "BS_HumanMale.BS_HM_Head_00".to_string(),
        "BS_HumanMale.BS_HM_Torso_00".to_string(),
        "BS_HumanMale.BS_HM_Legs_00".to_string(),
        "AR_Global.Prisoner_Torso".to_string(),
    ];

    let mut args = Vec::new();
    write_wstring(&mut args, bodyset);
    args.extend_from_slice(&(components.len() as u32).to_le_bytes());
    for comp in &components {
        write_wstring(&mut args, comp);
    }

    let mut body = Vec::new();
    append_entity_method(&mut body, method_idx::BEING_APPEARANCE, entity_id, &args);

    // method_index=26 < 61 -> direct: msg_id = 26 | 0x80 = 0x9A
    assert_eq!(body[0], 0x9A, "msg_id should be 0x9A for BeingAppearance (index 26)");

    // word_len (u16 LE) at offset 1-2
    let word_len = u16::from_le_bytes([body[1], body[2]]);
    let expected_payload = 4 + args.len(); // entity_id + args
    assert_eq!(word_len as usize, expected_payload, "word_len mismatch");

    // entity_id (u32 LE) at offset 3-6
    let eid = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
    assert_eq!(eid, entity_id, "entity_id mismatch");

    // bodyset WSTRING at offset 7
    let off = 7;
    let bs_char_count = u32::from_le_bytes([body[off], body[off+1], body[off+2], body[off+3]]);
    assert_eq!(bs_char_count as usize, bodyset.len(), "bodyset char_count should match");

    // Verify bodyset UTF-16LE chars
    let bs_data_start = off + 4;
    for (i, ch) in bodyset.encode_utf16().enumerate() {
        let wire_ch = u16::from_le_bytes([
            body[bs_data_start + i * 2],
            body[bs_data_start + i * 2 + 1],
        ]);
        assert_eq!(wire_ch, ch, "bodyset char {i} mismatch");
    }

    // Component array count
    let comp_off = bs_data_start + bodyset.len() * 2;
    let comp_count = u32::from_le_bytes([
        body[comp_off], body[comp_off+1], body[comp_off+2], body[comp_off+3],
    ]);
    assert_eq!(comp_count, components.len() as u32, "component count mismatch");

    // Verify each component WSTRING
    let mut cursor = comp_off + 4;
    for (idx, comp) in components.iter().enumerate() {
        let cc = u32::from_le_bytes([
            body[cursor], body[cursor+1], body[cursor+2], body[cursor+3],
        ]);
        assert_eq!(cc as usize, comp.len(), "component {idx} char_count mismatch");
        cursor += 4;
        for (i, ch) in comp.encode_utf16().enumerate() {
            let wire_ch = u16::from_le_bytes([
                body[cursor + i * 2],
                body[cursor + i * 2 + 1],
            ]);
            assert_eq!(wire_ch, ch, "component {idx} char {i} mismatch");
        }
        cursor += comp.len() * 2;
    }

    // Verify total length
    assert_eq!(cursor, body.len(), "total body length should match parsed position");
}

/// Verify onEntityTint wire encoding: 3x u32 LE
#[test]
fn entity_tint_wire_encoding() {
    let entity_id: u32 = 42;
    let primary: u32 = 0;
    let secondary: u32 = 0;
    let skin: u32 = SKIN_TINTS[5]; // skin_color_id = 5

    let mut args = Vec::with_capacity(12);
    args.extend_from_slice(&primary.to_le_bytes());
    args.extend_from_slice(&secondary.to_le_bytes());
    args.extend_from_slice(&skin.to_le_bytes());

    let mut body = Vec::new();
    append_entity_method(&mut body, method_idx::ON_ENTITY_TINT, entity_id, &args);

    // method_index=10 < 61 -> direct: msg_id = 10 | 0x80 = 0x8A
    assert_eq!(body[0], 0x8A, "msg_id should be 0x8A for onEntityTint (index 10)");

    let word_len = u16::from_le_bytes([body[1], body[2]]);
    assert_eq!(word_len, 16, "word_len should be 4 (entity_id) + 12 (3x u32)");

    let eid = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
    assert_eq!(eid, entity_id);

    let p = u32::from_le_bytes([body[7], body[8], body[9], body[10]]);
    assert_eq!(p, 0, "primaryColorId should be 0");

    let s = u32::from_le_bytes([body[11], body[12], body[13], body[14]]);
    assert_eq!(s, 0, "secondaryColorId should be 0");

    let sk = u32::from_le_bytes([body[15], body[16], body[17], body[18]]);
    assert_eq!(sk, SKIN_TINTS[5], "skinTint should match SKIN_TINTS[5]");
}

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
            (0, BandolierItem {
                item_id: 100, clip_size: 15, default_ammo_type: 1,
                current_ammo: 8, cur_ammo_type: 1,
            }),
            (2, BandolierItem {
                item_id: 101, clip_size: 12, default_ammo_type: 7,
                current_ammo: 12, cur_ammo_type: 7,
            }),
        ],
    };
    let entry = WorldEntryInfo {
        player_entity_id: 100, space_id: 65552,
        pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02,
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
        49, 0, 0, 0,   // stat_id = 49 (AMMO_SLOT_1)
         0, 0, 0, 0,   // min = 0
         8, 0, 0, 0,   // cur = 8
        15, 0, 0, 0,   // max = 15
    ];
    let ammo_slot_3_tuple: [u8; 16] = [
        51, 0, 0, 0,   // stat_id = 51 (AMMO_SLOT_3 — slot index 2)
         0, 0, 0, 0,   // min = 0
        12, 0, 0, 0,   // cur = 12
        12, 0, 0, 0,   // max = 12
    ];

    let contains = |needle: &[u8]| {
        all_bytes.windows(needle.len()).any(|w| w == needle)
    };

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
    let stale_slot_2: [u8; 16] = [
        50, 0, 0, 0,
         0, 0, 0, 0,
        15, 0, 0, 0,
        15, 0, 0, 0,
    ];
    assert!(
        !contains(&stale_slot_2),
        "empty slot 1 should not have stale clip_size in its AmmoSlot stat",
    );
}
