//! Tests covering the multi-packet `mapLoaded()` builder, the individual
//! phase builders (`onPlayerDataLoaded`, `setupWorldParameters`), and the
//! Mercury fragmentation framing they ride on.

use super::super::*;
use super::{sample_player_load_data, sample_world_entry, TEST_KEY};
use cimmeria_mercury::encryption::MercuryEncryption;

/// Walk an entity-method body and return the (method_index, byte_offset)
/// of each record in encounter order. Stops at the first non-record byte.
fn walk_entity_method_records(body: &[u8]) -> Vec<(u16, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let b = body[i];
        if !(0x80..=0xBD).contains(&b) {
            break;
        }
        if i + 3 > body.len() {
            break;
        }
        let word_len = u16::from_le_bytes([body[i + 1], body[i + 2]]) as usize;
        let record_end = i + 3 + word_len;
        if record_end > body.len() {
            break;
        }
        let method_index = if b == 0xBD {
            if i + 7 >= body.len() {
                break;
            }
            61 + body[i + 7] as u16
        } else {
            (b & 0x7F) as u16
        };
        out.push((method_index, i));
        i = record_end;
    }
    out
}

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

/// `build_enter_world_body`'s `forcedPosition` emit must place the spawn
/// position into the previous-position reference slot (offsets 24-35 of the
/// 49-byte `forcedPosition` payload, body offsets 74-85). Emitting zeros
/// there causes the client's camera-attach code to interpolate from world
/// origin (0,0,0) to the spawn position on the first frame, visibly
/// clipping through the world floor.
///
/// Reverting the fix (re-introducing a `[0.0f32, 0.0, 0.0]` literal in the
/// prev-position slot) must break this test. See `spec.protocol.mercury`
/// §1.10.6 + §1.16 Q3 closure (Ghidra `ProcessForcedEntityPosition` at
/// `0x00dd9ee0`).
#[test]
fn enter_world_body_forced_position_prev_pos_equals_spawn() {
    let info = WorldEntryInfo {
        player_entity_id: 100,
        space_id: 0x0001_0010,
        pos: [123.5, 456.25, 789.0],
        rot: [0.0, 0.0, 0.0],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
    // None for `load`: keep the original 99-byte VIEWPORT+CELL+FORCED layout
    // so this test pins the prev-pos slot independently of the appearance
    // insertion. The appearance-injection variant is covered separately.
    let body = build_enter_world_body(&info, None);
    // Sanity: total body length matches the documented 99-byte assembly
    // (spaceViewport 14 + createCellPlayer 35 + forcedPosition 50).
    assert_eq!(body.len(), 99, "enter-world body must be exactly 99 bytes");
    // forcedPosition starts at offset 49 (msg_id 0x31).
    assert_eq!(
        body[49],
        crate::mercury::BASEMSG_FORCED_POSITION,
        "forcedPosition record must start at body offset 49"
    );
    // pos at body offsets 62-73, prev_pos at body offsets 74-85.
    assert_eq!(&body[62..66], &123.5f32.to_le_bytes(), "pos.x");
    assert_eq!(&body[66..70], &456.25f32.to_le_bytes(), "pos.y");
    assert_eq!(&body[70..74], &789.0f32.to_le_bytes(), "pos.z");
    assert_eq!(
        &body[74..78],
        &123.5f32.to_le_bytes(),
        "prev_pos.x must equal spawn pos (zero would slide camera through floor)"
    );
    assert_eq!(
        &body[78..82],
        &456.25f32.to_le_bytes(),
        "prev_pos.y must equal spawn pos (zero would slide camera through floor)"
    );
    assert_eq!(
        &body[82..86],
        &789.0f32.to_le_bytes(),
        "prev_pos.z must equal spawn pos (zero would slide camera through floor)"
    );
}

/// `build_enter_world_body(info, Some(load))` must emit
/// `BeingAppearance` + `onEntityTint` BETWEEN `spaceViewportInfo` and
/// `createCellPlayer`. Live-debug evidence
/// (`logs/x64dbg-issue288-run3-analysis.txt`) showed the client's
/// `createCellPlayer` handler internally calls the appearance gate at
/// `retaddr=DD1DEF` — by setting the bodyset before that handler runs, the
/// internal gate evaluates against the bodyset and the renderable entity
/// comes up with the model on its first render frame.
#[test]
fn enter_world_body_with_load_inserts_appearance_before_cell_player() {
    use crate::mercury::method_idx;

    let info = sample_world_entry();
    let load = sample_player_load_data();
    let body = build_enter_world_body(&info, Some(&load));

    // spaceViewportInfo is the first 14 bytes (msg_id 0x08 + 4+4+4 ids + 1 viewport).
    assert_eq!(
        body[0],
        crate::mercury::BASEMSG_SPACE_VIEWPORT_INFO,
        "first record must be spaceViewportInfo"
    );
    let after_viewport = &body[14..];
    let records = walk_entity_method_records(after_viewport);
    assert!(
        records.len() >= 2,
        "expected at least 2 entity-method records between viewport and createCellPlayer, got {}",
        records.len()
    );
    assert_eq!(
        records[0].0,
        method_idx::BEING_APPEARANCE,
        "BeingAppearance must immediately follow spaceViewportInfo — \
         must be set on the entity BEFORE createCellPlayer so the cell-creation \
         handler's internal appearance gate (Ghidra retaddr 0xDD1DEF) picks up \
         the bodyset on the renderable entity's first frame"
    );
    assert_eq!(
        records[1].0,
        method_idx::ON_ENTITY_TINT,
        "onEntityTint must follow BeingAppearance and still precede createCellPlayer"
    );

    // After the two appearance records, the next byte must be the
    // createCellPlayer msg_id (0x06), confirming appearance sits BEFORE cell.
    let (_, tint_offset) = records[1];
    let tint_record = &after_viewport[tint_offset..];
    // Record header is 3 bytes (msg_id + u16 word_len) + word_len bytes of payload.
    let tint_payload_len = u16::from_le_bytes([tint_record[1], tint_record[2]]) as usize;
    let tint_record_end = 3 + tint_payload_len;
    let after_appearance = &tint_record[tint_record_end..];
    assert_eq!(
        after_appearance[0],
        crate::mercury::BASEMSG_CREATE_CELL_PLAYER,
        "createCellPlayer (msg 0x06) must immediately follow the appearance + tint pair"
    );
}

/// `build_create_player` must pre-warm the body-model assets by emitting
/// `BeingAppearance` + `onEntityTint` **between** `CREATE_BASE_PLAYER` and
/// `onClientMapLoad` when load data is supplied. Without the pre-warm the
/// client allocates the entity, starts terrain load, and only later (after
/// `mapLoaded` round-trip + entity-data bundle) receives the bodyset —
/// leaving a window where the dev-cube placeholder is rendered. Asserts
/// the records appear in exactly that order in the decrypted body.
#[test]
fn create_player_with_load_data_prewarms_bodyset_before_map_load() {
    use crate::mercury::method_idx;

    let info = sample_world_entry();
    let load = sample_player_load_data();
    let pkt = build_create_player(&TEST_KEY, 1, &[], &info, Some(&load));
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();
    // Body starts at offset 1 (offset 0 is flags). The first record must be
    // CREATE_BASE_PLAYER (0x05).
    assert_eq!(
        pt[1],
        crate::mercury::BASEMSG_CREATE_BASE_PLAYER,
        "first record must be CREATE_BASE_PLAYER"
    );
    // CREATE_BASE_PLAYER is 9 bytes total: [0x05][len:u16=6][entity:u32][class:u8][pad:u8].
    // So the next record begins at body offset 9.
    let after_create = &pt[1 + 9..];
    let records = walk_entity_method_records(after_create);
    assert!(
        records.len() >= 3,
        "expected ≥3 entity-method records after CREATE_BASE_PLAYER, got {}",
        records.len()
    );
    assert_eq!(
        records[0].0,
        method_idx::BEING_APPEARANCE,
        "BeingAppearance must be the FIRST entity-method after CREATE_BASE_PLAYER — \
         the client kicks off async model load on receipt; emitting it after \
         onClientMapLoad delays the load until the terrain round-trip finishes \
         and leaves a dev-cube placeholder window"
    );
    assert_eq!(
        records[1].0,
        method_idx::ON_ENTITY_TINT,
        "onEntityTint must follow BeingAppearance so the model loads with the \
         correct skin tint instead of defaulting and then re-tinting"
    );
    assert_eq!(
        records[2].0,
        method_idx::ON_CLIENT_MAP_LOAD,
        "onClientMapLoad must come LAST so the terrain load runs in parallel \
         with the already-started model load"
    );
}

/// Backwards-compat: passing `None` for `load` must keep the original
/// two-record body (CREATE_BASE_PLAYER + onClientMapLoad). Guards the
/// reanchor / GM-spawn / test paths where appearance data isn't available.
#[test]
fn create_player_without_load_data_emits_only_create_and_map_load() {
    use crate::mercury::method_idx;

    let info = sample_world_entry();
    let pkt = build_create_player(&TEST_KEY, 1, &[], &info, None);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();
    assert_eq!(pt[1], crate::mercury::BASEMSG_CREATE_BASE_PLAYER);
    let after_create = &pt[1 + 9..];
    let records = walk_entity_method_records(after_create);
    assert_eq!(
        records.len(),
        1,
        "no-load path must emit exactly one entity-method (onClientMapLoad), got {}",
        records.len()
    );
    assert_eq!(records[0].0, method_idx::ON_CLIENT_MAP_LOAD);
}

/// `BeingAppearance` (method 26) and `onEntityTint` (method 10) must land
/// in the first mapLoaded fragment so the player entity has a body model
/// from the moment `createCellPlayer` arrives. Moving them back to the
/// middle of the body (positions 15-16) puts them in fragment 4-5, leaving
/// a ~50-200 ms model-load gap.
///
/// Pin: the first three entity-method records emitted are
/// `setupWorldParameters`, `BeingAppearance`, `onEntityTint` (in that order),
/// and their byte offsets all sit comfortably inside `FRAGMENT_BODY_SIZE`.
#[test]
fn map_loaded_being_appearance_lands_in_first_fragment() {
    use crate::mercury::method_idx;
    use cimmeria_mercury::packet::FRAGMENT_BODY_SIZE;

    let data = sample_player_load_data();
    let entry = sample_world_entry();
    let body = build_map_loaded_body(100, &data, &entry);
    let records = walk_entity_method_records(&body);
    assert!(
        records.len() >= 3,
        "mapLoaded body must contain at least 3 entity-method records, got {}",
        records.len()
    );
    assert_eq!(
        records[0].0,
        method_idx::SETUP_WORLD_PARAMETERS,
        "record 0 must be setupWorldParameters"
    );
    assert_eq!(
        records[1].0,
        method_idx::BEING_APPEARANCE,
        "record 1 must be BeingAppearance — must land in the first fragment alongside createCellPlayer or the player entity renders without a body model for the time it takes the rest of the bundle to arrive"
    );
    assert_eq!(
        records[2].0,
        method_idx::ON_ENTITY_TINT,
        "record 2 must be onEntityTint — pairs with BeingAppearance so the model loads with the correct tint instead of defaulting then re-tinting"
    );
    let (_, being_offset) = records[1];
    let (_, tint_offset) = records[2];
    assert!(
        being_offset < FRAGMENT_BODY_SIZE,
        "BeingAppearance offset {} must fall inside FRAGMENT_BODY_SIZE ({})",
        being_offset,
        FRAGMENT_BODY_SIZE
    );
    assert!(
        tint_offset < FRAGMENT_BODY_SIZE,
        "onEntityTint offset {} must fall inside FRAGMENT_BODY_SIZE ({})",
        tint_offset,
        FRAGMENT_BODY_SIZE
    );
}
