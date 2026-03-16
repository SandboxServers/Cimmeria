//! World entry builders and data: map loading, world parameters, archetype stats,
//! ability trees, and the `mapLoaded()` multi-packet sequence.

// ── Submodules ───────────────────────────────────────────────────────────────

mod map_loaded;
mod phases;
mod stats;

// ── Re-exports ───────────────────────────────────────────────────────────────
// Matches the public API that mercury/mod.rs imports from `world_data::*`.

pub use phases::{
    build_create_player, build_enter_world, build_enter_world_body,
    build_on_player_data_loaded, build_setup_world_parameters,
};

pub use map_loaded::{
    build_map_loaded, build_map_loaded_body, fragment_map_loaded, fragment_count,
};

pub use stats::{archetype_stats, archetype_ability_tree};

// ── Shared imports from parent (mercury) ─────────────────────────────────────
// Used by submodules via `super::`.

pub(crate) use super::{
    encrypt_packet, write_wstring, append_entity_method, method_idx,
    REPLY_FLAGS, BASEMSG_CREATE_BASE_PLAYER, BASEMSG_SPACE_VIEWPORT_INFO,
    BASEMSG_CREATE_CELL_PLAYER, BASEMSG_FORCED_POSITION,
    SKIN_TINTS,
};
pub(crate) use super::types::{ArchetypeStats, PlayerLoadData, WorldEntryInfo};

// ── Data lookup functions ────────────────────────────────────────────────────
// Used by both phases.rs and map_loaded.rs, so they live here.

/// Look up the world_id for a world name (from db/resources/Worlds/Seed/worlds.sql).
pub(crate) fn world_id_for_name(world_name: &str) -> i32 {
    match world_name {
        "CombatSim" => 1,
        "SandBox" => 2,
        "Tol-Alpha-00" => 3,
        "Tol-Alpha-01" => 4,
        "Tol-Alpha-02" => 5,
        "Ca-Alpha-00" => 6,
        "Ca-Alpha-01" => 7,
        "Castle" => 8,
        "Tol-POI-06" => 9,
        "Agnos" => 10,
        "Anima_Vitrus" => 11,
        "Castle_CellBlock" => 12,
        "Hebridan" => 13,
        "Kheb" => 14,
        "Lucia" => 15,
        "Naitac" => 16,
        "PrimHatak" => 17,
        "Omega_Site" => 18,
        "Tollana" => 19,
        "Agnos_Library" => 20,
        "Playground" => 21,
        "TestSGC1" => 22,
        "Beta_Site_Evo_1" => 23,
        "Dakara" => 24,
        "Harset" => 57,
        "SGC_W1" => 58,
        "Harset_CmdCenter" => 68,
        "Menfa_Dark" => 77,
        "Omega_Site_CmdCenter" => 80,
        "Pertho" => 83,
        "SGC" => 86,
        "SGC_W2" => 87,
        "Tollana_Curia" => 88,
        "Temple" => 89,
        "Yotunheim" => 90,
        "Holding_Area" => 91,
        "Vitrus" => 92,
        _ => {
            tracing::warn!(world = %world_name, "Unknown world_id — using 1 (CombatSim)");
            1
        }
    }
}

/// Look up the client terrain path for a world name (client_map from worlds.sql).
/// Most worlds use the same name; a few differ.
pub(crate) fn client_map_for_world(world_name: &str) -> &str {
    match world_name {
        "CombatSim" => "Combat_Terrain_Test",
        "SandBox" => "Harset_CmdCenter",
        "Tol-Alpha-00" => "Tol-Alpha_Pocket_00",
        "Tol-Alpha-01" => "Tol-Alpha_Pocket_01",
        "Tol-Alpha-02" => "Tol-Alpha_Pocket_02",
        "Ca-Alpha-00" => "Ca-Alpha_Pocket_00",
        "Ca-Alpha-01" => "Ca-Alpha_Pocket_01",
        "Tol-POI-06" => "Tol_POI_Test06",
        _ => world_name, // Most worlds: client_map == world_name
    }
}

/// Serialize setupWorldParameters argument payload (22 args from World.py defaults).
pub(crate) fn build_world_params_args(world_name: &str) -> Vec<u8> {
    let mut args = Vec::with_capacity(88);
    args.extend_from_slice(&world_id_for_name(world_name).to_le_bytes()); // worldId
    args.extend_from_slice(&0i32.to_le_bytes());       // weatherSetId
    args.extend_from_slice(&1i32.to_le_bytes());       // minToRealMinutes
    args.extend_from_slice(&1440i32.to_le_bytes());    // minutesPerDay
    args.extend_from_slice(&100000i32.to_le_bytes());  // currentTimeInSeconds
    args.extend_from_slice(&(-9.8f32).to_le_bytes());  // gravity
    args.extend_from_slice(&6.0f32.to_le_bytes());     // runSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes());     // sidewaysRunSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes());     // backwardsRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // walkSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // sidewaysWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes());     // backwardsWalkSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes());     // crouchRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // sidewaysCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // backwardsCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // crouchWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes());     // sidewaysCrouchWalkSpeed
    args.extend_from_slice(&0.75f32.to_le_bytes());    // backwardsCrouchWalkSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes());     // swimSpeed
    args.extend_from_slice(&2.5f32.to_le_bytes());     // sidewaysSwimSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // backwardsSwimSpeed
    args.extend_from_slice(&8.0f32.to_le_bytes());     // jumpSpeed
    args
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
        };
        let entry = WorldEntryInfo { player_entity_id: 42, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02 };
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
        };
        let entry = WorldEntryInfo { player_entity_id: 100, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02 };
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
        };
        let entry = WorldEntryInfo { player_entity_id: 100, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02 };
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
        };
        let entry = WorldEntryInfo {
            player_entity_id: 100, space_id: 65552,
            pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(), class_id: 0x02,
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
        assert_eq!(stats::level_exp(0), 0);
        assert_eq!(stats::level_exp(1), 100);
        assert_eq!(stats::level_exp(10), 9000);
        assert_eq!(stats::level_exp(20), 400000);
        assert_eq!(stats::level_exp(99), 400000); // clamped
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
        use super::*;

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
        use super::*;

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
}
