//! The dial-side address-book gate (CAT-O-01) — see
//! [`super::super::address_book`].
//!
//! Four shapes: the refusal itself, its byte-exact wire form, the
//! pre-`InitPlayerState` window, and the dial-in-flight cancellation that
//! makes a rejected re-dial safe.

use super::*;

/// CAT-O-01: `target_address_id` arrives as a raw client integer. A player
/// who does not hold the address must not travel, must not arm a dial, must
/// stay in their space, and must be told why.
///
/// Deleting the `player_knows_stargate` call from `handle_dial_gate` fails
/// this on all four counts.
#[tokio::test]
async fn dial_gate_to_an_address_the_player_does_not_know_is_refused() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    // Holds an unrelated address — proves the check is per-address, not
    // "has any address at all".
    mgr.get_entity_mut(1).unwrap().known_stargates = vec![15];
    let space_before = mgr.get_entity_space_id(1);

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    assert!(
        !handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await,
        "a refused dial must report failure so gmDHD's feedback is honest"
    );

    assert!(
        mgr.get_entity(1).is_some(),
        "a refused dial must not tear the traveller out of their space"
    );
    assert_eq!(mgr.get_entity_space_id(1), space_before);
    assert!(
        mgr.gate_dial(1).is_none(),
        "the refusal must land before anything is armed — an armed dial \
         would open on its timer and become crossable"
    );

    let mut saw_error = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::GateTravel { .. } => {
                panic!("a refused dial must never enqueue a GateTravel")
            }
            CellToBaseMsg::EntityMethodCall { method_index, .. }
                if method_index == crate::cell::client_methods::player::ON_ERROR_CODE =>
            {
                saw_error = true
            }
            _ => {}
        }
    }
    assert!(saw_error, "the client must be told the dial was refused");
}

/// Byte-exact wire check on the refusal, against
/// `entities/defs/SGWPlayer.def:1240-1244`:
/// `UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID`, little-endian,
/// seven bytes. `InstanceID` is 0 and not the stargate id: `SystemID = 0`
/// is `ERRORCODE_SYSTEM_Ability`, the only token the enum defines, and the
/// client reads `InstanceID` as an ability id under it.
#[tokio::test]
async fn the_refusal_emits_the_stargate_address_feedback_code() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;

    let args = loop {
        match rx.try_recv() {
            // 121 literal, not the `ON_ERROR_CODE` constant: this test is
            // the byte-exact wire check, so it pins the index the client
            // dispatches on rather than the name the server calls it.
            Ok(CellToBaseMsg::EntityMethodCall {
                method_index: 121,
                args,
                ..
            }) => break args,
            Ok(_) => continue,
            Err(_) => panic!("expected an onErrorCode (121) for the refused dial"),
        }
    };
    assert_eq!(
        args,
        vec![0u8, 0, 0, 0, 0, 180, 0],
        "SystemID=0 (ERRORCODE_SYSTEM_Ability), InstanceID=0, \
         ErrorCodeID=180 (CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress)"
    );
}

/// The window between `CreateEntity` and `InitPlayerState`: the entity
/// exists but its address book has not arrived. Refusing is correct —
/// pinned here so a future "I dialled right after loading and it was
/// refused" report is recognised as designed behaviour, not a regression.
#[tokio::test]
async fn a_dial_before_init_player_state_is_refused() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    assert!(
        mgr.get_entity(1).unwrap().known_stargates.is_empty(),
        "a freshly created cell entity starts with an empty address book"
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;

    assert!(mgr.get_entity(1).is_some());
    assert!(mgr.gate_dial(1).is_none());
    while let Ok(msg) = rx.try_recv() {
        assert!(
            !matches!(msg, CellToBaseMsg::GateTravel { .. }),
            "no travel may happen before the address book has loaded"
        );
    }
}

/// `SGWPlayer.py:2061` calls `cancelDialing()` before it warns, exactly as
/// the other two reject branches do. The bug shape without it: dial a gate
/// you hold, then dial one you do not. The first dial stays armed, opens on
/// its timer, and the player walks into the gate and is sent to a world
/// they were refused.
///
/// Dropping the `space_mgr.cancel_gate_dial(entity_id)` beside the gate
/// fails this.
#[tokio::test]
async fn a_dial_refused_for_the_address_book_cancels_the_dial_in_flight() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    // Holds Castle's address (2) but not the Agnos-side 15.
    mgr.get_entity_mut(1).unwrap().known_stargates = vec![2];

    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;
    assert!(mgr.gate_dial(1).is_some(), "precondition: a dial is armed");

    // Re-dial an address the player does not hold. Gate 15 is a real row,
    // so this can only be the address-book refusal.
    mgr.get_entity_mut(1).unwrap().known_stargates = vec![];
    handle_dial_gate(1, 15, 0, &tx, &mut mgr, &engine()).await;
    assert!(
        mgr.gate_dial(1).is_none(),
        "an address-book refusal must cancel the dial in flight, not leave \
         the previous destination armed and crossable"
    );
}
