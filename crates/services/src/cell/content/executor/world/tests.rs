//! Unit tests for the world-mutation action handlers. These call each
//! `pub(super)` fn directly (no `execute_actions` dispatch wrapper) so
//! the channel fan-out, entity mutation, and missing-tag paths are
//! pinned at the lowest level that produces the signal.
//!
//! Fan-out shape (the load-bearing invariant): `set_interaction_type`
//! must broadcast to the **target's** witnesses, with `witness_id =
//! player_eid` and `entity_id = target_eid` on every message. A bug
//! that flipped these — e.g. iterating `entity_id`'s witnesses, or
//! swapping the witness/entity fields on the payload — would route
//! packets to the wrong recipients and is what these tests guard.

use super::*;
use cimmeria_common::EntityId;

/// Build a fresh `SpaceManager` with the `Agnos` startup space
/// pre-created. All tests below stage entities into that one space so
/// `find_entity_by_tag` (space-scoped) resolves consistently.
fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Stage the canonical "player triggers chain, drone is the tagged
/// target" fixture: `player_id` witnessing `npc_id`, `npc_id` tagged
/// `Drone` with `interaction_type_flags = initial_flags`. Returns once
/// `find_entity_by_tag(player_id, "Drone")` resolves to `npc_id` and
/// `get_witnesses_of(npc_id)` returns `[player_id]`.
fn stage_drone_with_witness(
    mgr: &mut SpaceManager,
    player_id: u32,
    npc_id: u32,
    initial_flags: i64,
) {
    mgr.create_entity(npc_id, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.tag = Some("Drone".to_string());
        npc.interaction_type_flags = initial_flags;
    }
    mgr.create_entity(player_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.is_player = true;
        p.player_id = Some(42);
        p.witnesses.insert(EntityId(npc_id as i32));
    }
    mgr.connect_entity(player_id);
}

/// `"add"` ORs the mask into the existing flags and broadcasts the
/// post-update value to each witness. The payload is the new
/// `interaction_type_flags` as little-endian `u64`. Bug shapes guarded:
/// - field mutation: `target.interaction_type_flags |= mask`
/// - fan-out cardinality: exactly one message per witness, no more
/// - wire routing: `witness_id = player_eid`, `entity_id = npc_eid`
/// - payload shape: 8-byte LE encoding of the post-OR flags
#[tokio::test]
async fn set_interaction_type_add_op_or_masks_flags_and_broadcasts_to_each_witness() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, /* player */ 1, /* npc */ 101, 0x01);
    // Second witness — exercises the per-witness loop.
    mgr.create_entity(2, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(2) {
        p.is_player = true;
        p.player_id = Some(43);
        p.witnesses.insert(EntityId(101));
    }
    mgr.connect_entity(2);

    let (tx, mut rx) = mpsc::channel(16);
    set_interaction_type(
        "Drone".to_string(),
        "add".to_string(),
        0x04,
        /* entity_id (source) */ 1,
        /* chain_id */ 1032,
        &tx,
        &mut mgr,
    )
    .await;

    // Field mutation: 0x01 | 0x04 == 0x05.
    assert_eq!(
        mgr.get_entity(101).unwrap().interaction_type_flags,
        0x05,
        "add op must OR the mask into existing flags"
    );

    // Drain channel into a Vec — iteration order over the player set
    // isn't stable, so collect-then-sort by witness_id before asserting.
    let mut emitted: Vec<(u32, u32, u16, Vec<u8>)> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
            } => emitted.push((witness_id, entity_id, method_index, args)),
            other => panic!("expected WitnessEntityMethod, got {other:?}"),
        }
    }
    emitted.sort_by_key(|(w, _, _, _)| *w);

    assert_eq!(
        emitted.len(),
        2,
        "fan-out cardinality: exactly one WitnessEntityMethod per witness — \
         a regression that iterated source `entity_id`'s witnesses, or \
         double-sent to the owner, would change this count"
    );
    let expected_payload = (0x05_u64).to_le_bytes().to_vec();
    for (i, expected_witness) in [1u32, 2u32].iter().enumerate() {
        let (witness_id, entity_id, method_index, args) = &emitted[i];
        assert_eq!(
            witness_id, expected_witness,
            "WitnessEntityMethod.witness_id must be the player_eid — \
             a swap with entity_id would route the packet to the NPC"
        );
        assert_eq!(
            *entity_id, 101,
            "WitnessEntityMethod.entity_id must be the target NPC's eid \
             (the ghost on which the method is called), not the chain source"
        );
        assert_eq!(
            *method_index,
            crate::mercury::method_idx::INTERACTION_TYPE,
            "method_index must be the INTERACTION_TYPE constant — \
             wire-format regression guard"
        );
        assert_eq!(
            args, &expected_payload,
            "payload must be exactly 8-byte LE u64 of the new flags (0x05) — \
             byte-exact wire format guard"
        );
    }
}

/// `"|"` is the legacy script alias for `"add"`. Pin that it ORs flags
/// identically — a refactor that handled only `"add"` and fell through
/// `"|"` to the warn default would silently break every chain authored
/// against the legacy operator string.
#[tokio::test]
async fn set_interaction_type_pipe_alias_matches_add_op() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0x02);

    let (tx, _rx) = mpsc::channel(8);
    set_interaction_type(
        "Drone".to_string(),
        "|".to_string(),
        0x10,
        1,
        1032,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(101).unwrap().interaction_type_flags,
        0x12,
        "\"|\" alias must OR identically to \"add\": 0x02 | 0x10 == 0x12"
    );
}

/// `"remove"` clears the masked bits (AND NOT). Pre-fix mistake shape:
/// using `&= mask` (keep only those bits) instead of `&= !mask` (clear
/// those bits) would leave the entity stuck on the masked-off
/// interaction type with everything else gone.
#[tokio::test]
async fn set_interaction_type_remove_op_clears_bit_in_flags() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0x0F);

    let (tx, mut rx) = mpsc::channel(8);
    set_interaction_type(
        "Drone".to_string(),
        "remove".to_string(),
        0x04,
        1,
        1032,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(101).unwrap().interaction_type_flags,
        0x0B,
        "remove op must clear the masked bits: 0x0F & !0x04 == 0x0B"
    );
    // And the broadcast carries the post-clear value.
    let msg = rx.try_recv().expect("expected WitnessEntityMethod");
    match msg {
        CellToBaseMsg::WitnessEntityMethod { args, .. } => {
            assert_eq!(
                args,
                (0x0B_u64).to_le_bytes().to_vec(),
                "remove op payload must be the post-clear flags as LE u64"
            );
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
}

/// `"~"` is the legacy script alias for `"remove"` (AND NOT). Pin it to
/// the same shape so a refactor that dropped the alias arm would trip
/// here, not in production.
#[tokio::test]
async fn set_interaction_type_tilde_alias_matches_remove_op() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0x07);

    let (tx, _rx) = mpsc::channel(8);
    set_interaction_type(
        "Drone".to_string(),
        "~".to_string(),
        0x02,
        1,
        1032,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(101).unwrap().interaction_type_flags,
        0x05,
        "\"~\" alias must clear identically to \"remove\": 0x07 & !0x02 == 0x05"
    );
}

/// `"set"` replaces the flags entirely (assignment, not OR).
/// Distinguishable from `"add"` only when the existing value has bits
/// the mask doesn't — pin `existing = 0xFF, mask = 0x01, result = 0x01`
/// (not `0xFF`), the exact shape that catches an accidental `|=`.
#[tokio::test]
async fn set_interaction_type_set_op_replaces_flags_entirely() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0xFF);

    let (tx, _rx) = mpsc::channel(8);
    set_interaction_type(
        "Drone".to_string(),
        "set".to_string(),
        0x01,
        1,
        1032,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(101).unwrap().interaction_type_flags,
        0x01,
        "set op must REPLACE flags, not OR — a regression to `|=` would yield 0xFF"
    );
}

/// Unknown operation strings hit the default arm with `warn!` and leave
/// the flags untouched. The broadcast still fires (the code path is:
/// warn → fall through → broadcast the unchanged value) — pin both:
/// WARN was emitted AND flags didn't change.
#[tokio::test]
async fn set_interaction_type_unknown_op_warns_and_does_not_mutate_flags() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0x42);

    let (tx, _rx) = mpsc::channel(8);
    set_interaction_type(
        "Drone".to_string(),
        "frobnicate".to_string(),
        0x04,
        1,
        1032,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(101).unwrap().interaction_type_flags,
        0x42,
        "unknown op must NOT mutate flags — default arm only warns"
    );
    assert!(
        capture
            .find_message(Level::WARN, "Unknown interaction type operation")
            .is_some(),
        "default arm must WARN — catches chain-authoring typos. Captured: {:#?}",
        capture.all()
    );
}

/// Missing tag (find_entity_by_tag returns None) takes the outer-else
/// branch: debug-log only, no mutation, no channel send. Pin the "no
/// message on channel" shape — a regression that fell through to
/// broadcast on `target_id = 0` (or panicked on the `get_entity_mut`
/// unwrap path) would trip here.
#[tokio::test]
async fn set_interaction_type_missing_tag_emits_no_message() {
    let mut mgr = make_space_mgr();
    // Player exists, but no entity tagged "GhostTag".
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(8);
    set_interaction_type(
        "GhostTag".to_string(),
        "add".to_string(),
        0x04,
        1,
        1032,
        &tx,
        &mut mgr,
    )
    .await;

    assert!(
        rx.try_recv().is_err(),
        "missing-tag path must not emit any WitnessEntityMethod"
    );
}

/// Fan-out routing: when the source entity (chain trigger) has its own
/// witness set populated, the broadcast loop must iterate the
/// **target**'s witnesses, not the source's. Bug shape: a copy-paste
/// `get_witnesses_of(entity_id)` instead of `get_witnesses_of(target_id)`
/// would emit packets to whoever could see the source player, not
/// whoever could see the NPC — silently routing the interaction-type
/// change to the wrong AoI cohort.
#[tokio::test]
async fn set_interaction_type_broadcasts_to_target_witnesses_not_source_witnesses() {
    let mut mgr = make_space_mgr();
    // Drone (target) is witnessed only by player 1.
    stage_drone_with_witness(&mut mgr, /* player */ 1, /* npc */ 101, 0x00);
    // Player 2 exists and witnesses the *source* player (1), but NOT
    // the drone. A bug that iterated `get_witnesses_of(entity_id=1)`
    // would emit to player 2; the correct loop on `target_id=101`
    // emits only to player 1.
    mgr.create_entity(2, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(2) {
        p.is_player = true;
        p.player_id = Some(43);
        p.witnesses.insert(EntityId(1)); // sees the source, not the drone
    }
    mgr.connect_entity(2);

    let (tx, mut rx) = mpsc::channel(8);
    set_interaction_type(
        "Drone".to_string(),
        "add".to_string(),
        0x08,
        /* source */ 1,
        1032,
        &tx,
        &mut mgr,
    )
    .await;

    let mut witnesses_received: Vec<u32> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::WitnessEntityMethod { witness_id, .. } = msg {
            witnesses_received.push(witness_id);
        }
    }
    assert_eq!(
        witnesses_received,
        vec![1],
        "broadcast must reach only player 1 (who witnesses the drone); \
         player 2 witnesses the source player but NOT the drone — \
         routing-by-source-witnesses would put 2 in this set"
    );
}

/// `set_visible` emits exactly one `EntityMethodCall` (not
/// `WitnessEntityMethod` — this method is dispatched at the entity-id
/// level, base routes it). Pin both true/false to catch a regression
/// that hard-coded `1` regardless of the `visible` argument.
#[tokio::test]
async fn set_visible_emits_on_visible_with_correct_bool_byte() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0x00);

    // visible=false → byte 0
    let (tx, mut rx) = mpsc::channel(8);
    set_visible("Drone".to_string(), false, 1, 1032, &tx, &mgr).await;
    let msg = rx.try_recv().expect("expected EntityMethodCall");
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 101, "entity_id must be the target NPC's eid");
            assert_eq!(method_index, crate::mercury::method_idx::ON_VISIBLE);
            assert_eq!(args, vec![0u8], "visible=false must encode as byte 0");
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "exactly one message expected");

    // visible=true → byte 1
    let (tx, mut rx) = mpsc::channel(8);
    set_visible("Drone".to_string(), true, 1, 1032, &tx, &mgr).await;
    let msg = rx.try_recv().expect("expected EntityMethodCall");
    match msg {
        CellToBaseMsg::EntityMethodCall { args, .. } => {
            assert_eq!(args, vec![1u8], "visible=true must encode as byte 1");
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// `destroy_tagged_entity` removes the target from the space —
/// subsequent `get_entity` returns None. Pin the by-tag lookup
/// resolved correctly (target gone, source still present).
#[tokio::test]
async fn destroy_tagged_entity_removes_target_from_space() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0x00);
    assert!(
        mgr.get_entity(101).is_some(),
        "fixture sanity: NPC exists pre-destroy"
    );

    destroy_tagged_entity("Drone".to_string(), 1, 1032, &mut mgr);

    assert!(
        mgr.get_entity(101).is_none(),
        "destroy_tagged_entity must remove the target NPC from the space"
    );
    assert!(
        mgr.get_entity(1).is_some(),
        "source entity (the player) must NOT be touched — \
         a regression that destroyed `entity_id` instead of `target_id` would trip here"
    );
}

/// `move_waypoint` updates the target's position. Pin the exact
/// coordinates — destination is propagated verbatim, not
/// clamped/zeroed/swapped. Bug shape: a regression that called
/// `update_entity_position(entity_id, ...)` instead of
/// `(target_id, ...)` would move the source player instead of the NPC.
#[tokio::test]
async fn move_waypoint_updates_target_position_to_destination() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101, 0x00);
    let player_pos_before = mgr.get_entity(1).unwrap().position;

    move_waypoint("Drone".to_string(), [50.0, 1.5, 75.0], 1, 1032, &mut mgr);

    let drone = mgr.get_entity(101).unwrap();
    assert_eq!(
        (drone.position.x, drone.position.y, drone.position.z),
        (50.0, 1.5, 75.0),
        "target NPC must be moved to the destination verbatim"
    );
    let player_pos_after = mgr.get_entity(1).unwrap().position;
    assert_eq!(
        player_pos_before, player_pos_after,
        "source player must NOT be moved — \
         a swap of target_id ↔ entity_id in the update_entity_position \
         call would trip here"
    );
}

/// `set_aggression` with a missing tag must take the outer-None branch
/// silently — no panic, no mutation. The happy-path (`level=1` writes
/// the field) is already covered by
/// `set_aggression_level_one_writes_entity_field` in `executor::tests`;
/// this is the complementary negative path so a regression that
/// unwrapped `find_entity_by_tag` would crash here.
#[test]
fn set_aggression_missing_tag_leaves_unrelated_entities_untouched() {
    let mut mgr = make_space_mgr();
    // Player exists. NPC with a DIFFERENT tag exists.
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }
    mgr.create_entity(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(n) = mgr.get_entity_mut(101) {
        n.tag = Some("NotDrone".to_string());
        assert_eq!(n.aggression, 0, "fixture sanity: NPC starts passive");
    }

    set_aggression("Drone".to_string(), 1, 1, 1032, &mut mgr);

    assert_eq!(
        mgr.get_entity(101).unwrap().aggression,
        0,
        "missing-tag path must not touch any entity's aggression — \
         a regression that fell through to mutating an arbitrary entity \
         would trip here"
    );
}
