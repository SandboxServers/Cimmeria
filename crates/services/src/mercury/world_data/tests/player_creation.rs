//! Tests covering `build_enter_world_body` and `build_create_player` —
//! the player-creation packet ordering and appearance-injection invariants.
//!
//! Fragmentation-level `mapLoaded` tests live in `map_loaded.rs`.

use super::super::*;
use super::{sample_player_load_data, sample_world_entry, walk_entity_method_records, TEST_KEY};
use cimmeria_mercury::encryption::MercuryEncryption;

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
        "record 1 must be BeingAppearance — must land in the first fragment alongside \
         createCellPlayer or the player entity renders without a body model for the \
         time it takes the rest of the bundle to arrive"
    );
    assert_eq!(
        records[2].0,
        method_idx::ON_ENTITY_TINT,
        "record 2 must be onEntityTint — pairs with BeingAppearance so the model \
         loads with the correct tint instead of defaulting then re-tinting"
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
