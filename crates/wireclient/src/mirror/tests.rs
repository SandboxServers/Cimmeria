//! Mirror tests. Bodies come from the server's own producers
//! (`compose_create_entity_base_body`, `compose_create_entity_cascade_body`,
//! `append_entity_method`), so the mirror is checked against the real layouts.

use bytes::Bytes;
use cimmeria_mercury::channel_bundle::{IDBASE_NPC_DEFAULT, IDBASE_SGW_PLAYER};
use cimmeria_wire::cell::messages::NpcAoIData;
use cimmeria_wire::mercury::aoi::{
    compose_create_entity_base_body, compose_create_entity_cascade_body,
};
use cimmeria_wire::mercury::append_entity_method;

use super::*;
use crate::bundle::decode_bundle_strict;
use crate::semantic::SemanticError;

const FROST_POS: [f32; 3] = [-322.0, 73.6, -211.0];
const FROST_MESH: &str = "CA-Props.CA-PrisonerCorpse00";

/// Frame one WORD_LENGTH message: the id byte, a `u16` length, the payload.
/// Entity methods and `createBasePlayer` are framed this way on the wire.
fn word_msg(msg_id: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = vec![msg_id];
    v.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// `createBasePlayer` (0x05): entity id, class id, property count.
fn create_base_player(id: u32, class: u8) -> Vec<u8> {
    let mut payload = id.to_le_bytes().to_vec();
    payload.push(class);
    payload.push(0);
    word_msg(0x05, &payload)
}

/// `leaveAoI` (0x0C): entity id only.
fn leave(id: u32) -> Vec<u8> {
    word_msg(0x0C, &id.to_le_bytes())
}

/// Decode a body the way the session does, failing the test on bad framing.
fn messages(body: Vec<u8>) -> Vec<S2CMessage> {
    decode_bundle_strict(&Bytes::from(body)).expect("bundle decodes cleanly")
}

/// Decode a body and apply every message in order, failing on any error.
fn feed(mirror: &mut EntityMirror, body: Vec<u8>) {
    for msg in messages(body) {
        mirror.apply(&msg).expect("apply succeeds");
    }
}

/// An NPC cascade with the appearance and name the frost corpse carries.
fn frost_npc() -> NpcAoIData {
    NpcAoIData {
        static_mesh: Some(FROST_MESH.to_owned()),
        name_id: Some(7031),
        ..NpcAoIData::default()
    }
}

/// One `InvItem` in wire layout: five i32, the bound flag, durability, an
/// empty ammo array, the current ammo type and the charges.
fn inv_item_bytes(id: i32, slot: i32) -> Vec<u8> {
    let mut b = Vec::new();
    for v in [id, 0x55, 1, slot, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.push(1); // is_bound
    b.extend_from_slice(&0i32.to_le_bytes()); // durability
    b.extend_from_slice(&0u32.to_le_bytes()); // ammo_types count
    b.extend_from_slice(&0i32.to_le_bytes()); // cur_ammo_type
    b.extend_from_slice(&0i32.to_le_bytes()); // charges
    b
}

#[test]
fn create_then_cascade_identifies_frosts_corpse() {
    let mut mirror = EntityMirror::new();
    feed(
        &mut mirror,
        compose_create_entity_base_body(100751, 1, FROST_POS, [0.0; 3]),
    );
    feed(
        &mut mirror,
        compose_create_entity_cascade_body(100751, 1, 0, Some(&frost_npc())),
    );

    let query = EntityQuery::static_mesh(FROST_MESH).name_id(7031);
    let found = mirror.find_one(&query).expect("exactly one corpse");
    assert_eq!(found.entity_id, 100751);
    assert_eq!(found.position, Some(FROST_POS));
}

#[test]
fn leave_removes_and_reintroduction_replaces() {
    let mut mirror = EntityMirror::new();
    let first = [1.0, 2.0, 3.0];
    let second = [4.0, 5.0, 6.0];

    feed(
        &mut mirror,
        compose_create_entity_base_body(5, 1, first, [0.0; 3]),
    );
    feed(
        &mut mirror,
        compose_create_entity_cascade_body(5, 1, 0, Some(&frost_npc())),
    );
    assert_eq!(mirror.get(5).map(|e| e.introduced), Some(0));

    // A second CREATE_ENTITY for the same id, with no leave in between, must
    // drop the mesh and name the cascade gave the first introduction.
    feed(
        &mut mirror,
        compose_create_entity_base_body(5, 1, second, [0.0; 3]),
    );
    let replaced = mirror.get(5).expect("re-introduced entity is held");
    assert_eq!(replaced.introduced, 1);
    assert_eq!(replaced.static_mesh, None);
    assert_eq!(replaced.name_id, None);
    assert_eq!(replaced.position, Some(second));
    assert_eq!(mirror.len(), 1);

    feed(&mut mirror, leave(5));
    assert!(mirror.get(5).is_none());
    assert!(mirror.is_empty());

    feed(
        &mut mirror,
        compose_create_entity_base_body(5, 1, first, [0.0; 3]),
    );
    assert_eq!(mirror.get(5).map(|e| e.introduced), Some(2));
}

#[test]
fn an_npc_extended_method_uses_idbase_62() {
    let mut mirror = EntityMirror::new();
    feed(&mut mirror, create_base_player(10, 2));
    feed(
        &mut mirror,
        compose_create_entity_base_body(200, 1, [0.0; 3], [0.0; 3]),
    );

    // 0xBD, sub-slot 0, to the NPC: index 62 under its idbase, not 61.
    // No NPC decoder sits at 61 or above yet, so `apply` returns Ok(None)
    // under either idbase and the index is checked through the helper `apply`
    // uses. The Player half is guarded through `apply` by
    // `inventory_tracks_update_and_remove_on_the_own_entity` (0xBD sub 11).
    // TODO(WC-11): assert through `apply` once an NPC decoder at 61+ exists.
    let npc_payload = [200u32.to_le_bytes().as_slice(), &[0u8, 0, 0, 0]].concat();
    let npc_msg = &messages(word_msg(0xBD, &npc_payload))[0];
    assert_eq!(method_index(npc_msg, TargetKind::Npc), Some(62));
    assert_eq!(npc_msg.method_index_for(IDBASE_NPC_DEFAULT), Some(62));
    assert_eq!(mirror.apply(npc_msg), Ok(None));

    // The same bytes to the own player resolve to 61.
    let own_payload = [10u32.to_le_bytes().as_slice(), &[0u8, 0, 0, 0]].concat();
    let own_msg = &messages(word_msg(0xBD, &own_payload))[0];
    assert_eq!(method_index(own_msg, TargetKind::Player), Some(61));
    assert_eq!(own_msg.method_index_for(IDBASE_SGW_PLAYER), Some(61));
    assert_eq!(mirror.apply(own_msg), Ok(None));
    assert!(mirror.orphans().is_empty());
}

#[test]
fn a_method_for_an_unknown_entity_is_an_orphan() {
    let mut mirror = EntityMirror::new();
    // Direct encoding, method 12, to an entity the mirror never introduced.
    let payload = [999u32.to_le_bytes().as_slice(), b"hi"].concat();
    let msg = messages(word_msg(0x80 | 12, &payload)).remove(0);

    assert_eq!(mirror.apply(&msg), Ok(None));
    assert_eq!(mirror.orphans(), std::slice::from_ref(&msg));
    assert!(mirror.is_empty());
}

#[test]
fn interactable_finds_the_body_after_interaction_type_changes() {
    let mut mirror = EntityMirror::new();
    for id in [1, 2] {
        feed(
            &mut mirror,
            compose_create_entity_base_body(id, 1, [0.0; 3], [0.0; 3]),
        );
        feed(
            &mut mirror,
            compose_create_entity_cascade_body(id, 1, 0, Some(&NpcAoIData::default())),
        );
    }
    assert!(mirror
        .find(&EntityQuery::default().interactable())
        .is_empty());

    let mut body = Vec::new();
    append_entity_method(
        &mut body,
        3, // InteractionType
        IDBASE_NPC_DEFAULT,
        1,
        &0x4000_0000u64.to_le_bytes(),
    );
    feed(&mut mirror, body);

    let found = mirror.find(&EntityQuery::default().interactable());
    let ids: Vec<u32> = found.iter().map(|e| e.entity_id).collect();
    assert_eq!(ids, vec![1]);
}

#[test]
fn find_one_reports_ambiguity_with_ids() {
    let mut mirror = EntityMirror::new();
    for id in [1, 2] {
        feed(
            &mut mirror,
            compose_create_entity_base_body(id, 1, [0.0; 3], [0.0; 3]),
        );
        feed(
            &mut mirror,
            compose_create_entity_cascade_body(id, 1, 0, Some(&frost_npc())),
        );
    }

    let query = EntityQuery::static_mesh(FROST_MESH).name_id(7031);
    let err = mirror.find_one(&query).expect_err("two corpses match");
    assert_eq!(
        err,
        MirrorError::Ambiguous {
            query: query.to_string(),
            ids: vec![1, 2],
        }
    );
}

#[test]
fn inventory_tracks_update_and_remove_on_the_own_entity() {
    let mut mirror = EntityMirror::new();
    feed(&mut mirror, create_base_player(7, 2));

    // UpdateItem (72): u32 count, then the items.
    let mut args = 2u32.to_le_bytes().to_vec();
    args.extend(inv_item_bytes(501, 0));
    args.extend(inv_item_bytes(502, 1));
    let mut body = Vec::new();
    append_entity_method(&mut body, 72, IDBASE_SGW_PLAYER, 7, &args);
    feed(&mut mirror, body);
    assert_eq!(mirror.inventory().len(), 2);
    assert_eq!(mirror.inventory()[&501].slot_id, 0);

    // The same id again upserts rather than duplicates.
    let mut args = 1u32.to_le_bytes().to_vec();
    args.extend(inv_item_bytes(501, 3));
    let mut body = Vec::new();
    append_entity_method(&mut body, 72, IDBASE_SGW_PLAYER, 7, &args);
    feed(&mut mirror, body);
    assert_eq!(mirror.inventory().len(), 2);
    assert_eq!(mirror.inventory()[&501].slot_id, 3);

    // RemoveItem (71): u32 count, then the i32 item ids.
    let mut args = 1u32.to_le_bytes().to_vec();
    args.extend_from_slice(&502i32.to_le_bytes());
    let mut body = Vec::new();
    append_entity_method(&mut body, 71, IDBASE_SGW_PLAYER, 7, &args);
    feed(&mut mirror, body);
    let ids: Vec<i32> = mirror.inventory().keys().copied().collect();
    assert_eq!(ids, vec![501]);
}

#[test]
fn a_bad_known_method_is_an_error() {
    let mut mirror = EntityMirror::new();
    feed(&mut mirror, create_base_player(7, 2));

    let err = mirror
        .apply(&short_dialog_to(7))
        .expect_err("short dialog body");
    assert!(is_short_dialog_error(&err, 7));
    assert!(mirror.orphans().is_empty());
}

/// `onDialogDisplay` (105) is 17 bytes of arguments; this sends 16, encoded
/// against the player table (0xBD sub-slot 44).
fn short_dialog_to(target: u32) -> S2CMessage {
    let mut args = Vec::new();
    for v in [7i32, 9, 0] {
        args.extend_from_slice(&v.to_le_bytes());
    }
    args.push(1); // is_immediate
    args.extend_from_slice(&3i32.to_le_bytes()); // mission_id
    args.pop();
    assert_eq!(args.len(), 16);
    let mut body = Vec::new();
    append_entity_method(&mut body, 105, IDBASE_SGW_PLAYER, target, &args);
    messages(body).remove(0)
}

fn is_short_dialog_error(err: &MirrorError, target: u32) -> bool {
    matches!(
        err,
        MirrorError::Semantic {
            entity_id,
            method_index: 105,
            source: SemanticError::Short { .. },
        } if *entity_id == target
    )
}

/// `onUpdateItem` (72) with one item, encoded against the player table.
fn update_item_to(target: u32, item_id: i32) -> S2CMessage {
    let mut args = 1u32.to_le_bytes().to_vec();
    args.extend(inv_item_bytes(item_id, 0));
    let mut body = Vec::new();
    append_entity_method(&mut body, 72, IDBASE_SGW_PLAYER, target, &args);
    messages(body).remove(0)
}

/// `CREATE_ENTITY` alone, with no avatar update after it: an entity the
/// client holds but has no position for.
fn create_entity_only(id: u32, class: u8) -> Vec<u8> {
    let mut payload = id.to_le_bytes().to_vec();
    payload.extend_from_slice(&[0xFF, class, 0, 0]);
    word_msg(0x09, &payload)
}

#[test]
fn a_player_class_entity_that_is_not_us_decodes_with_the_player_table() {
    let mut mirror = EntityMirror::new();
    feed(&mut mirror, create_base_player(7, 2));
    // Another player in GM class (3) and a mob (4), both seen through AoI.
    feed(
        &mut mirror,
        compose_create_entity_base_body(30, 3, [0.0; 3], [0.0; 3]),
    );
    feed(
        &mut mirror,
        compose_create_entity_base_body(40, 4, [0.0; 3], [0.0; 3]),
    );

    // The class-3 entity reads 0xBD sub 11 as onUpdateItem (72) and still
    // leaves our inventory alone; the mob reads the same bytes as 73.
    let event = mirror.apply(&update_item_to(30, 900)).expect("decodes");
    assert!(matches!(event, Some(S2CEvent::UpdateItem { .. })));
    assert_eq!(mirror.apply(&update_item_to(40, 901)), Ok(None));
    assert!(mirror.inventory().is_empty(), "only the own entity's items");

    let err = mirror
        .apply(&short_dialog_to(30))
        .expect_err("player-table decoder runs for class 3");
    assert!(is_short_dialog_error(&err, 30));
    assert_eq!(mirror.apply(&short_dialog_to(40)), Ok(None));
}

#[test]
fn the_own_entity_uses_the_player_table_whatever_its_class() {
    let mut mirror = EntityMirror::new();
    // Own id with a non-player class: the own-id clause alone must pick Player.
    feed(&mut mirror, create_base_player(8, 1));
    let err = mirror
        .apply(&short_dialog_to(8))
        .expect_err("own entity decodes as Player");
    assert!(is_short_dialog_error(&err, 8));
}

#[test]
fn near_and_class_match_only_entities_that_fit() {
    let mut mirror = EntityMirror::new();
    feed(
        &mut mirror,
        compose_create_entity_base_body(1, 1, [10.0, 0.0, 0.0], [0.0; 3]),
    );
    feed(&mut mirror, create_entity_only(2, 4));
    assert_eq!(mirror.get(2).map(|e| e.position), Some(None));

    let ids =
        |q: EntityQuery| -> Vec<u32> { mirror.find(&q).iter().map(|e| e.entity_id).collect() };
    assert_eq!(
        ids(EntityQuery::default().near([10.0, 0.0, 1.0], 1.0)),
        vec![1]
    );
    assert!(ids(EntityQuery::default().near([10.0, 0.0, 1.5], 1.0)).is_empty());
    // No position never matches `near`, however wide.
    assert_eq!(ids(EntityQuery::default().near([0.0; 3], 1.0e9)), vec![1]);
    assert_eq!(ids(EntityQuery::default().class_id(4)), vec![2]);
    assert_eq!(ids(EntityQuery::default()), vec![1, 2]);
}

#[test]
fn invisible_hides_and_reintroduction_clears_it() {
    let mut mirror = EntityMirror::new();
    feed(
        &mut mirror,
        compose_create_entity_base_body(5, 1, [0.0; 3], [0.0; 3]),
    );
    // entityInvisible is constant-length: entity id, then the alias byte
    // (`build_entity_invisible`).
    feed(
        &mut mirror,
        [&[0x0B], 5u32.to_le_bytes().as_slice(), &[0xFF]].concat(),
    );
    assert_eq!(mirror.get(5).map(|e| e.hidden), Some(true));

    feed(
        &mut mirror,
        compose_create_entity_base_body(5, 1, [0.0; 3], [0.0; 3]),
    );
    assert_eq!(mirror.get(5).map(|e| e.hidden), Some(false));
}

#[test]
fn query_display_names_every_set_field() {
    let q = EntityQuery::static_mesh("CA-Props.X")
        .name_id(7031)
        .class_id(1)
        .near([1.0, 2.5, -3.0], 4.0)
        .interactable();
    assert_eq!(
        q.to_string(),
        "mesh=CA-Props.X name_id=7031 class=1 near=(1,2.5,-3)±4 interactable"
    );
    assert_eq!(EntityQuery::default().to_string(), "any");
}
