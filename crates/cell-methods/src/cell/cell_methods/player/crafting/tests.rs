//! Crafting cell methods: argument round trips and the forward to the base.

use super::*;
use crate::cell::messages::CraftRequest;
use crate::test_support::make_space_manager_with_player;
use cimmeria_content_engine::chain::ChainEngine;

const ENTITY: u32 = 1;
const PLAYER_ID: i32 = 4711;

fn i32_bytes(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn array_bytes(out: &mut Vec<u8>, values: &[i32]) {
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for &v in values {
        i32_bytes(out, v);
    }
}

/// The client's argument bytes for `verb`, laid out per `SGWPlayer.def`.
/// Written independently of the parser, so a parser that reads a field in
/// the wrong order or width does not round-trip.
fn encode(verb: &CraftVerb) -> (u16, Vec<u8>) {
    let mut out = Vec::new();
    let index = match verb {
        CraftVerb::Spend { discipline_id } => {
            i32_bytes(&mut out, *discipline_id);
            SPEND_APPLIED_SCIENCE_POINTS
        }
        CraftVerb::Craft {
            blueprint_id,
            items,
            quantity,
        } => {
            i32_bytes(&mut out, *blueprint_id);
            array_bytes(&mut out, items);
            i32_bytes(&mut out, *quantity);
            CRAFT
        }
        CraftVerb::Research { item_id, kickers } => {
            i32_bytes(&mut out, *item_id);
            array_bytes(&mut out, kickers);
            RESEARCH
        }
        CraftVerb::ReverseEngineer { item_id } => {
            i32_bytes(&mut out, *item_id);
            REVERSE_ENGINEER
        }
        CraftVerb::Alloy {
            blueprint_id,
            current_tier_item_id,
            lower_tier_items,
        } => {
            i32_bytes(&mut out, *blueprint_id);
            i32_bytes(&mut out, *current_tier_item_id);
            array_bytes(&mut out, lower_tier_items);
            ALLOYING
        }
        CraftVerb::Respec => RESPEC_CRAFTING,
    };
    (index, out)
}

/// One sample per verb, with distinct values in every field and arrays of
/// several elements, so a swapped or dropped field changes the result.
fn samples() -> Vec<CraftVerb> {
    vec![
        CraftVerb::Spend { discipline_id: 21 },
        CraftVerb::Craft {
            blueprint_id: 412,
            items: vec![0x0102_0304, 0x7000_0001, -5],
            quantity: 3,
        },
        CraftVerb::Research {
            item_id: 5481,
            kickers: vec![5668, 5670],
        },
        CraftVerb::ReverseEngineer { item_id: 777 },
        CraftVerb::Alloy {
            blueprint_id: 42,
            current_tier_item_id: 9001,
            lower_tier_items: vec![11, 12, 13, 14],
        },
        CraftVerb::Respec,
    ]
}

#[test]
fn every_verb_round_trips() {
    for verb in samples() {
        let (index, args) = encode(&verb);
        assert_eq!(parse_verb(index, &args), Ok(verb.clone()), "{verb:?}");
    }
}

#[test]
fn empty_arrays_round_trip() {
    for verb in [
        CraftVerb::Craft {
            blueprint_id: 1,
            items: vec![],
            quantity: 1,
        },
        CraftVerb::Research {
            item_id: 2,
            kickers: vec![],
        },
        CraftVerb::Alloy {
            blueprint_id: 3,
            current_tier_item_id: 4,
            lower_tier_items: vec![],
        },
    ] {
        let (index, args) = encode(&verb);
        assert_eq!(parse_verb(index, &args), Ok(verb));
    }
}

/// Every strict prefix of every sample fails to parse. Catches a parser that
/// stops early (e.g. the old stub that read only `aCraftId`).
#[test]
fn every_truncation_is_rejected() {
    for verb in samples() {
        let (index, args) = encode(&verb);
        for len in 0..args.len() {
            assert!(
                matches!(
                    parse_verb(index, &args[..len]),
                    Err(ArgsError::Truncated { .. })
                ),
                "{verb:?} truncated to {len} bytes must not parse"
            );
        }
    }
}

#[test]
fn trailing_bytes_are_rejected() {
    for verb in samples() {
        let (index, mut args) = encode(&verb);
        args.push(0);
        assert_eq!(
            parse_verb(index, &args),
            Err(ArgsError::TrailingBytes { extra: 1 }),
            "{verb:?}"
        );
    }
}

#[test]
fn forged_array_count_is_rejected_without_allocating() {
    let mut args = Vec::new();
    i32_bytes(&mut args, 5481);
    args.extend_from_slice(&u32::MAX.to_le_bytes());
    args.extend_from_slice(&[0; 8]);
    assert_eq!(
        parse_verb(RESEARCH, &args),
        Err(ArgsError::Truncated { offset: 4 })
    );
}

#[test]
fn non_crafting_index_is_not_parsed() {
    assert_eq!(parse_verb(94, &[]), Err(ArgsError::NotACraftingMethod));
}

fn player_space() -> SpaceManager {
    let mut mgr = make_space_manager_with_player(ENTITY);
    mgr.get_entity_mut(ENTITY).unwrap().player_id = Some(PLAYER_ID);
    mgr
}

/// Driving the real player dispatcher with each verb's bytes forwards exactly
/// one `CellToBaseMsg::Crafting` carrying every argument, the entity, the
/// player id and `allowed = 0`. This pins the 95-100 routing and the forward
/// together: a verb that stops forwarding, drops a field, or reaches another
/// handler fails here.
#[tokio::test]
async fn every_verb_forwards_every_argument_through_player_dispatch() {
    let engine = ChainEngine::new();
    for verb in samples() {
        let mut mgr = player_space();
        let (tx, mut rx) = mpsc::channel(8);
        let (index, args) = encode(&verb);

        let handled = super::super::dispatch(ENTITY, index, &args, &tx, &mut mgr, &engine).await;
        assert!(handled, "{verb:?} must be handled");

        match rx.try_recv() {
            Ok(CellToBaseMsg::Crafting(request)) => assert_eq!(
                request,
                CraftRequest {
                    entity_id: ENTITY,
                    player_id: PLAYER_ID,
                    verb: verb.clone(),
                    allowed: 0,
                }
            ),
            other => panic!("{verb:?}: expected one Crafting message, got {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "{verb:?}: exactly one message");
    }
}

#[tokio::test]
async fn malformed_request_is_handled_but_not_forwarded() {
    let mut mgr = player_space();
    let (tx, mut rx) = mpsc::channel(8);
    // `craft` with only its first INT32: the shape the old stub accepted.
    let handled = dispatch(ENTITY, CRAFT, &412i32.to_le_bytes(), &tx, &mut mgr).await;
    assert!(handled);
    assert!(rx.try_recv().is_err(), "a malformed request is dropped");
}

#[tokio::test]
async fn request_from_entity_without_player_id_is_not_forwarded() {
    let mut mgr = make_space_manager_with_player(ENTITY);
    let (tx, mut rx) = mpsc::channel(8);
    let handled = dispatch(ENTITY, RESPEC_CRAFTING, &[], &tx, &mut mgr).await;
    assert!(handled);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn out_of_range_index_is_not_handled() {
    let mut mgr = player_space();
    let (tx, _rx) = mpsc::channel(8);
    assert!(!dispatch(ENTITY, TRADE_REQUEST, &[], &tx, &mut mgr).await);
}

/// `send_on_update_discipline` enqueues an `EntityMethodCall` with
/// method index 136 and the 8-byte payload `[disciplineId LE][expertise LE]`.
///
/// Bug shape this catches: an off-by-one in method_index (e.g., 135 or
/// 137), a swap of disciplineId/expertise in the wire bytes, or a
/// regression that changes the args length.
#[tokio::test]
async fn send_on_update_discipline_emits_correct_message() {
    let (tx, mut rx) = mpsc::channel(8);

    send_on_update_discipline(42, 7, 50, &tx).await;

    let msg = rx
        .recv()
        .await
        .expect("send_on_update_discipline must enqueue exactly one CellToBaseMsg");

    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 42);
            assert_eq!(
                method_index, 136,
                "onUpdateDiscipline method index per docs/protocol/client-method-dispatch-table.md \
                 is 136 — a change here desyncs the client's crafting UI",
            );
            assert_eq!(
                args,
                vec![0x07, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00],
                "wire payload: disciplineId=7 LE, expertise=50 LE (0x32)",
            );
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// CR-05: `allowed` is the station mask at request time. A craft+research
/// station 3 units away grants bits 1 and 2; once the player walks 20 units
/// off, the same request carries 0. With the forward hard-wired to 0 (the
/// CR-01 placeholder), the first assertion fails.
#[tokio::test]
async fn forward_carries_the_station_mask_at_request_time() {
    use cimmeria_cell_catalog::crafting::{ENTITYFLAG_CRAFT_CRAFT, ENTITYFLAG_CRAFT_RESEARCH};

    const STATION: u32 = 100_050;
    let mut mgr = player_space();
    let world = mgr.get_entity_world_name(ENTITY).unwrap();
    let p = mgr.get_entity(ENTITY).unwrap().position;
    mgr.spawn_npc(STATION, &world, [p.x + 3.0, p.y, p.z], [0.0; 3])
        .unwrap();
    let station = mgr.get_entity_mut(STATION).unwrap();
    station.is_player = false;
    station.entity_flags = (ENTITYFLAG_CRAFT_CRAFT | ENTITYFLAG_CRAFT_RESEARCH) as u64;

    let (tx, mut rx) = mpsc::channel(8);
    let (index, args) = encode(&CraftVerb::ReverseEngineer { item_id: 20_001 });
    assert!(dispatch(ENTITY, index, &args, &tx, &mut mgr).await);
    match rx.try_recv() {
        Ok(CellToBaseMsg::Crafting(request)) => assert_eq!(request.allowed, 0x03),
        other => panic!("expected one Crafting message, got {other:?}"),
    }

    mgr.update_entity_position(ENTITY, [p.x + 23.0, p.y, p.z], [0; 3], [0.0; 3]);
    assert!(dispatch(ENTITY, index, &args, &tx, &mut mgr).await);
    match rx.try_recv() {
        Ok(CellToBaseMsg::Crafting(request)) => assert_eq!(request.allowed, 0),
        other => panic!("expected one Crafting message, got {other:?}"),
    }
}
