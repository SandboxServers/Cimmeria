//! Tests covering the multi-packet `mapLoaded()` builder, the individual
//! phase builders (`onPlayerDataLoaded`, `setupWorldParameters`), and the
//! Mercury fragmentation framing they ride on.

use super::super::*;
use super::{sample_player_load_data, sample_world_entry, walk_entity_method_records, TEST_KEY};
use cimmeria_mercury::encryption::MercuryEncryption;

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
    let entry = WorldEntryInfo {
        player_entity_id: 42,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
    let (packets, seqs) = build_map_loaded(&TEST_KEY, 5, &[], 42, &data, &entry);
    assert!(
        !packets.is_empty(),
        "mapLoaded should produce at least one packet"
    );
    assert_eq!(
        seqs as usize,
        packets.len(),
        "seqs_consumed should match packet count"
    );
    for (i, pkt) in packets.iter().enumerate() {
        assert!(!pkt.is_empty(), "packet {} should not be empty", i);
    }
}

/// Mercury's 32-bit per-session ACK bitmap limits the reliable TX window
/// to 32 in-flight packets. The world-entry burst is the densest
/// reliable emission the server does — `build_map_loaded` alone returns
/// multiple fragments, and the surrounding flow adds more (charList,
/// versionInfo, resourceFragments, createBasePlayer). If the burst ever
/// exceeds 32 packets the receiver back-pressures and tickSync ACKs
/// stall until the window drains.
///
/// This guard pins `build_map_loaded` itself — by far the dominant
/// fragment source in the burst — at well under the 32-packet ceiling.
/// Adding new entity-method records to the bundle is fine; pushing the
/// fragment count above the ceiling is a wire-format regression that
/// would re-introduce the bug the split-counter design was meant to
/// fix from the other side (this one tightens the reliable side; the
/// tickSync split tightens the unreliable side).
#[test]
fn build_map_loaded_fragment_count_fits_within_reliable_tx_window() {
    use cimmeria_mercury::consts::TX_WINDOW_SIZE;

    // Worst-case mapLoaded: a level-20 player with all archetype slots
    // populated, a non-trivial component list, a full ability tree, and
    // every stargate already discovered. If a fresh-character fixture
    // fits, a more decorated one must too, but explicitly biasing toward
    // worst-case makes the guard meaningful.
    let data = PlayerLoadData {
        player_id: 1,
        level: 20,
        player_name: "FragmentStressTester".into(),
        extra_name: "Twenty-Char-Extra-Nm".into(),
        alignment: 2,
        archetype: 2,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![
            "BS_HumanMale.Head".into(),
            "BS_HumanMale.Torso".into(),
            "BS_HumanMale.Arms".into(),
            "BS_HumanMale.Legs".into(),
            "BS_HumanMale.Boots".into(),
        ],
        exp: 999_999,
        naquadah: 99_999,
        known_stargates: (1..=64).collect(),
        abilities: (1..=64).collect(),
        training_points: 40,
        applied_science_points: 20,
        blueprint_ids: vec![],
        first_login: 0,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: archetype_ability_tree(2),
        items: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
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

    let (packets, seqs) = build_map_loaded(&TEST_KEY, 5, &[], 100, &data, &entry);
    assert_eq!(seqs as usize, packets.len(), "seq count must match fragment count");
    assert!(
        packets.len() < TX_WINDOW_SIZE,
        "mapLoaded emitted {} fragments — exceeds the 32-slot reliable TX \
         window cap. Adding a new entity-method record bumped the bundle past \
         the wire-format ceiling; either split the new record off into a \
         separate phase or shrink an existing one.",
        packets.len()
    );
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
    let entry = WorldEntryInfo {
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
    let (packets, _seqs) = build_map_loaded(&TEST_KEY, 5, &[], 100, &data, &entry);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    // Mercury MAX_BODY_LENGTH is 1411 bytes
    const MAX_PLAINTEXT: usize = 1411;
    for (i, pkt) in packets.iter().enumerate() {
        let pt = enc.decrypt(pkt).unwrap();
        assert!(
            pt.len() <= MAX_PLAINTEXT,
            "packet {} plaintext {} bytes exceeds {} limit",
            i,
            pt.len(),
            MAX_PLAINTEXT
        );
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
    let entry = WorldEntryInfo {
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
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
    assert!(
        all_bytes.contains(&0xFA),
        "should contain setupWorldParameters (0xFA)"
    );
    // onPlayerDataLoaded = 0xF3 should be present
    assert!(
        all_bytes.contains(&0xF3),
        "should contain onPlayerDataLoaded (0xF3)"
    );
    // onAbilityTreeInfo uses extended 0xBD
    assert!(
        all_bytes.contains(&0xBD),
        "should contain extended encoding marker (0xBD)"
    );
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
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
    let (packets, seqs) = build_map_loaded(&TEST_KEY, 10, &[], 100, &data, &entry);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);

    // Must produce multiple packets (body > 1300 bytes)
    assert!(
        packets.len() > 1,
        "Expected multiple fragments, got {} packet(s)",
        packets.len()
    );
    assert_eq!(seqs as usize, packets.len());

    for (i, pkt) in packets.iter().enumerate() {
        let pt = enc.decrypt(pkt).unwrap();
        let flags = pt[0];

        // Every fragment must have FLAG_FRAGMENTED (0x20) and FLAG_HAS_SEQUENCE (0x40)
        assert!(
            flags & FLAG_FRAGMENTED != 0,
            "Packet {} flags=0x{:02X} missing FLAG_FRAGMENTED (0x20)",
            i,
            flags
        );
        assert!(
            flags & FLAG_HAS_SEQUENCE != 0,
            "Packet {} flags=0x{:02X} missing FLAG_HAS_SEQUENCE (0x40)",
            i,
            flags
        );

        // Parse footers from the end: seq_id (4), frag_end (4), frag_begin (4)
        let len = pt.len();
        let seq_id = u32::from_le_bytes(pt[len - 4..len].try_into().unwrap());
        let frag_end = u32::from_le_bytes(pt[len - 8..len - 4].try_into().unwrap());
        let frag_begin = u32::from_le_bytes(pt[len - 12..len - 8].try_into().unwrap());

        // frag_begin should be base_seq (10)
        assert_eq!(
            frag_begin, 10,
            "Packet {} frag_begin={} expected 10",
            i, frag_begin
        );
        // frag_end should be base_seq + num_frags - 1
        let expected_frag_end = 10 + packets.len() as u32 - 1;
        assert_eq!(
            frag_end, expected_frag_end,
            "Packet {} frag_end={} expected {}",
            i, frag_end, expected_frag_end
        );
        // seq_id should be base_seq + i
        assert_eq!(
            seq_id,
            10 + i as u32,
            "Packet {} seq_id={} expected {}",
            i,
            seq_id,
            10 + i as u32
        );

        // Body chunk size: for non-last fragments, should be FRAGMENT_BODY_SIZE (1300)
        let body_len = len - 1 - 12; // flags(1) + footers(12) subtracted
        if i < packets.len() - 1 {
            assert_eq!(
                body_len, 1300,
                "Packet {} body_len={} expected 1300 (FRAGMENT_BODY_SIZE)",
                i, body_len
            );
        }

        eprintln!(
            "Fragment {}: plaintext_len={} flags=0x{:02X} body={} seq={} frag=[{}-{}]",
            i,
            pt.len(),
            flags,
            body_len,
            seq_id,
            frag_begin,
            frag_end
        );
    }
}

/// Issue #288 regression guard: `onPlayMovie` (the first-login intro
/// cinematic, method index 155) MUST NOT appear in the mapLoaded entity
/// method bundle, even when `first_login = 1`. It is deferred to
/// `handle_on_client_ready` so the cinematic plays after BeingAppearance is
/// rooted to a live possessed pawn — firing it inside this bundle (the
/// original behavior) let the cinematic-exit `CollectGarbage` reclaim the
/// in-flight appearance asset and produced a one-frame dev-cube flash on
/// the first post-cinematic render.
///
/// Two assertions, redundant on purpose:
/// 1. Structural: walking entity-method records over the body must not
///    yield method index 155.
/// 2. Byte-pattern: the UTF-16LE-encoded cinematic asset name must not
///    appear anywhere in the body — guards against the structural walker
///    breaking and silently passing if a future change confuses it.
///
/// Reverting the fix (re-adding the `onPlayMovie` step to
/// `build_map_loaded_body_inner`) must break this test.
#[test]
fn build_map_loaded_omits_first_login_cinematic_from_bundle() {
    let mut data = sample_player_load_data();
    data.first_login = 1; // would-be cinematic trigger pre-#288 fix
    let entry = sample_world_entry();

    let body = build_map_loaded_body(42, &data, &entry);

    // (1) Structural — no record with method_index 155 (ON_PLAY_MOVIE).
    let records = walk_entity_method_records(&body);
    assert!(
        !records.iter().any(|(idx, _)| *idx == 155),
        "onPlayMovie (method 155) must not appear in mapLoaded bundle; \
         it is deferred to handle_on_client_ready. Records found: {:?}",
        records.iter().map(|(idx, _)| *idx).collect::<Vec<_>>()
    );

    // (2) Byte-pattern — "Cine-SGWLogo.SGWLogo" UTF-16LE must not be present.
    let cinematic_name: Vec<u8> = "Cine-SGWLogo.SGWLogo"
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    assert!(
        !body
            .windows(cinematic_name.len())
            .any(|w| w == cinematic_name),
        "Cinematic asset name 'Cine-SGWLogo.SGWLogo' (UTF-16LE) must not appear \
         in mapLoaded bundle bytes"
    );
}
