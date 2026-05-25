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
        weapon_visual: None,
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
        auto_reload: true,
        reload_on_activate: false,
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
/// The fixture loads the **full inventory caps** from
/// `python/common/Constants.py`: 40 main-bag + 100 mission-bag + 100
/// crafting + 4 bandolier + 11 equipment slots
/// (head/face/neck/chest/hands/waist/back/legs/feet/artifact1/artifact2).
/// The original fixture used `items: vec![]`, which let an inventory
/// regression — a future +5kB body from filling these bags — silently
/// slip past this guard until a real player logged in.
///
/// Adding new entity-method records to the bundle is fine; pushing the
/// fragment count above the ceiling is a wire-format regression that
/// would re-introduce the bug the split-counter design was meant to
/// fix from the other side (this one tightens the reliable side; the
/// tickSync split tightens the unreliable side).
#[test]
fn build_map_loaded_fragment_count_fits_within_reliable_tx_window() {
    use cimmeria_entity::inventory::{
        InvItem, INV_ARTIFACT1, INV_ARTIFACT2, INV_BACK, INV_BANDOLIER, INV_CHEST, INV_CRAFTING,
        INV_FACE, INV_FEET, INV_HANDS, INV_HEAD, INV_LEGS, INV_MAIN, INV_MISSION, INV_NECK,
        INV_WAIST,
    };
    use cimmeria_mercury::consts::TX_WINDOW_SIZE;

    // Build a full-capacity inventory fixture. Item shape mirrors the
    // serialiser in `crates/entity/src/inventory.rs::InvItem::serialize`
    // (~37 B per item with an empty ammo_types). The 11 equipment slots
    // (head/face/neck/chest/hands/waist/back/legs/feet/artifact1/artifact2)
    // and the four bandolier slots are pre-equipped to mimic a
    // max-decorated player.
    fn fill_bag(items: &mut Vec<InvItem>, container_id: i32, slots: i32) {
        for slot in 0..slots {
            items.push(InvItem {
                id: items.len() as i32 + 1,
                dbid: 5000 + slot,
                stack_size: 1,
                slot_id: slot,
                container_id,
                is_bound: false,
                durability: 100,
                ammo_types: vec![],
                cur_ammo_type: 0,
                charges: 0,
            });
        }
    }
    let mut items = Vec::new();
    fill_bag(&mut items, INV_MAIN, 40);
    fill_bag(&mut items, INV_MISSION, 100);
    fill_bag(&mut items, INV_CRAFTING, 100);
    fill_bag(&mut items, INV_BANDOLIER, 4);
    for equip_slot in [
        INV_HEAD,
        INV_FACE,
        INV_NECK,
        INV_CHEST,
        INV_HANDS,
        INV_WAIST,
        INV_BACK,
        INV_LEGS,
        INV_FEET,
        INV_ARTIFACT1,
        INV_ARTIFACT2,
    ] {
        fill_bag(&mut items, equip_slot, 1);
    }

    // Worst-case mapLoaded: a level-20 player with all archetype slots
    // populated, a non-trivial component list, a full ability tree, every
    // stargate already discovered, and the full bag-of-bags above.
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
        weapon_visual: None,
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
        items,
        active_bandolier_slot: 0,
        bandolier_items: vec![],
        auto_reload: true,
        reload_on_activate: false,
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
    assert_eq!(
        seqs as usize,
        packets.len(),
        "seq count must match fragment count"
    );
    assert!(
        packets.len() < TX_WINDOW_SIZE,
        "mapLoaded emitted {} fragments with a full-inventory fixture — meets or \
         exceeds the {}-slot reliable TX window cap (must be strictly less than {} \
         to leave headroom for other in-flight packets). Adding a new entity-method \
         record or growing an existing record (e.g. wire format bloat in onUpdateItem) \
         pushed the bundle to or past the wire-format ceiling; either split the new \
         record off into a separate phase or shrink an existing one.",
        packets.len(),
        TX_WINDOW_SIZE,
        TX_WINDOW_SIZE
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
        weapon_visual: None,
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
        auto_reload: true,
        reload_on_activate: false,
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
        weapon_visual: None,
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
        auto_reload: true,
        reload_on_activate: false,
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
        weapon_visual: None,
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
        auto_reload: true,
        reload_on_activate: false,
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
/// Regression guard: `onChatJoined` and `onPlayerCommunication` MUST NOT
/// appear in the mapLoaded entity-method bundle. Both used to live there
/// and padded ~311 B onto the worst-case fragment burst — roughly 156 B
/// for the 8 × `onChatJoined` plus 155 B for the welcome
/// `onPlayerCommunication`. They're now fired from
/// `handle_on_client_ready`, matching the original
/// `python/base/SGWPlayer.py` flow where `onClientReady` calls into
/// `ChannelManager.playerLoggedIn`.
///
/// Structural check: walking entity-method records over the body must not
/// yield indices `method_idx::ON_PLAYER_COMMUNICATION` or
/// `method_idx::ON_CHAT_JOINED`. Reverting the fix (re-adding
/// `append_method!(method_idx::ON_CHAT_JOINED, ...)` or
/// `append_method!(method_idx::ON_PLAYER_COMMUNICATION, ...)` to
/// `build_map_loaded_body_inner`) must break this test.
#[test]
fn build_map_loaded_omits_chat_joined_and_player_communication_from_bundle() {
    use crate::mercury::method_idx;

    let data = sample_player_load_data();
    let entry = sample_world_entry();

    let body = build_map_loaded_body(42, &data, &entry);
    let records = walk_entity_method_records(&body);
    let method_indices: Vec<u16> = records.iter().map(|(idx, _)| *idx).collect();

    assert!(
        !records
            .iter()
            .any(|(idx, _)| *idx == method_idx::ON_CHAT_JOINED),
        "onChatJoined (method {}) must not appear in mapLoaded bundle; \
         it is deferred to handle_on_client_ready to match the original \
         SGWPlayer.py onClientReady → ChannelManager.playerLoggedIn flow. \
         Indices found: {method_indices:?}",
        method_idx::ON_CHAT_JOINED,
    );
    assert!(
        !records
            .iter()
            .any(|(idx, _)| *idx == method_idx::ON_PLAYER_COMMUNICATION),
        "onPlayerCommunication (method {}) must not appear in mapLoaded \
         bundle; the welcome message is deferred to handle_on_client_ready. \
         Indices found: {method_indices:?}",
        method_idx::ON_PLAYER_COMMUNICATION,
    );
}

/// Encode `s` the same way `write_wstring` does (length-prefixed UTF-16LE)
/// and return the **payload bytes only** — the UTF-16LE code units, no
/// length prefix. That's the substring a reader needs to grep for inside
/// a serialised `BeingAppearance` arg block to prove a given component
/// string is present (or absent) without re-parsing the WSTRING ARRAY
/// framing.
fn utf16le_bytes(s: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(s.len() * 2);
    for ch in s.encode_utf16() {
        buf.extend_from_slice(&ch.to_le_bytes());
    }
    buf
}

/// Return `true` if `needle` appears as a contiguous subsequence of
/// `haystack`. Tiny linear scan is fine — these test bodies are short.
fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Wire-format regression guard for the spawn-holstered invariant:
/// `build_map_loaded_body` must filter the active bandolier weapon out of
/// the on-wire `BeingAppearance.ComponentList`. Two failure modes the
/// guard catches:
///
///   1. A caller accidentally passes `holstered = false` to
///      `appearance_components` (or reads `data.components` directly).
///      The weapon string then leaks onto the wire and the client
///      renders the armed pose at spawn instead of the holstered pose.
///   2. A future refactor of `appearance_components` drops the filter
///      step (e.g., a `cloned()` that bypasses the `.filter()` predicate).
///
/// The test:
///   - non-weapon components ("torso", "head") MUST appear in the
///     serialised body — proves the test setup actually emitted a
///     BeingAppearance with a non-empty ComponentList.
///   - the weapon visual ("BS_Gun.Pistol") MUST NOT appear anywhere in
///     the serialised body — its presence would indicate the filter
///     was bypassed.
#[test]
fn build_map_loaded_filters_weapon_visual_from_being_appearance() {
    let mut data = sample_player_load_data();
    data.components = vec![
        "BS_HumanMale.Torso".into(),
        "BS_Gun.Pistol".into(),
        "BS_HumanMale.Head".into(),
    ];
    data.weapon_visual = Some("BS_Gun.Pistol".into());

    let entry = sample_world_entry();
    let body = build_map_loaded_body(entry.player_entity_id, &data, &entry);

    let torso = utf16le_bytes("BS_HumanMale.Torso");
    let head = utf16le_bytes("BS_HumanMale.Head");
    let pistol = utf16le_bytes("BS_Gun.Pistol");

    assert!(
        contains_subslice(&body, &torso),
        "non-weapon component 'BS_HumanMale.Torso' must appear in the wire body — \
         test setup must actually emit a BeingAppearance with a populated ComponentList \
         or the holster-filter assertion below is vacuous"
    );
    assert!(
        contains_subslice(&body, &head),
        "non-weapon component 'BS_HumanMale.Head' must appear in the wire body"
    );
    assert!(
        !contains_subslice(&body, &pistol),
        "weapon visual 'BS_Gun.Pistol' must NOT appear in the wire body — \
         BeingAppearance.ComponentList must filter the active bandolier weapon \
         at spawn so the client's appearance compositor selects the holstered pose. \
         If this triggers, an emit site is reading data.components directly or \
         passing holstered=false to appearance_components."
    );
}
