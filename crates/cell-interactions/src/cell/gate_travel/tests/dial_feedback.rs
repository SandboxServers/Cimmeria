//! #727: every refused dial tells the player why — see
//! [`super::super::dial_feedback`].
//!
//! One byte-exact wire test for the line itself, then one test per refusal
//! branch of `handle_dial_gate`, each asserting exactly one feedback line
//! with that branch's text. `handle_dial_gate`'s own `get_entity_world_name`
//! miss sends the same `NotInWorld` line but cannot be reached once the
//! address-book gate has found the entity, so the no-entity test covers the
//! text. The unrecoverable-arrival branch is asserted in
//! [`super::arrival`], beside the fixture it needs. Deleting any one
//! `send_dial_refusal` call fails its branch's test; deleting the helper's
//! send fails all of them.

use super::super::dial_feedback::{refusal_messages, DialRefusal};
use super::*;
use tokio::sync::mpsc;

/// Drain every message queued so far.
pub(super) fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// Read one `WSTRING` (u32 UTF-16 unit count, then the units LE) at `*at`.
fn read_wstring(args: &[u8], at: &mut usize) -> String {
    let n = u32::from_le_bytes(args[*at..*at + 4].try_into().unwrap()) as usize;
    *at += 4;
    let units: Vec<u16> = args[*at..*at + n * 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    *at += n * 2;
    String::from_utf16(&units).unwrap()
}

/// The text of every `onPlayerCommunication` (method 28) in `sent`.
pub(super) fn feedback_lines(sent: &[CellToBaseMsg]) -> Vec<&'static str> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                method_index: 28,
                args,
                ..
            } => {
                let mut at = 0;
                let _speaker = read_wstring(args, &mut at);
                at += 2; // SpeakerFlags, Channel
                let text = read_wstring(args, &mut at);
                // Map back to the static text so tests compare against
                // `DialRefusal::text()` directly.
                [
                    DialRefusal::UnknownAddress,
                    DialRefusal::NotInWorld,
                    DialRefusal::AlreadyOnDestination,
                    DialRefusal::NoSafeArrival,
                ]
                .into_iter()
                .map(DialRefusal::text)
                .find(|t| *t == text)
                .or(Some("<unexpected feedback text>"))
            }
            _ => None,
        })
        .collect()
}

/// The (method, args) pairs of every client-method call in `sent`.
fn method_calls(sent: &[CellToBaseMsg]) -> Vec<(u16, Vec<u8>)> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => Some((*method_index, args.clone())),
            _ => None,
        })
        .collect()
}

fn wstring(s: &str) -> Vec<u8> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let mut out = (units.len() as u32).to_le_bytes().to_vec();
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

/// Byte-exact wire check against `entities/defs/interfaces/Communicator.def`
/// `onPlayerCommunication(WSTRING Speaker, UINT8 SpeakerFlags, UINT8 Channel,
/// WSTRING Text)`, client method 28: speaker `SYSTEM`, flags 0,
/// `CHAN_feedback` = 9 (`enumerations.xml`), then the text. Literals, not the
/// serializer, so a change to the shared serializer or the channel constant
/// shows up here.
#[test]
fn a_refusal_line_is_on_player_communication_on_the_feedback_channel() {
    let text = "Failed to dial: you are already on that world";
    let mut want = Vec::new();
    // "SYSTEM": 6 UTF-16 units.
    want.extend_from_slice(&[6, 0, 0, 0]);
    for b in b"SYSTEM" {
        want.extend_from_slice(&[*b, 0]);
    }
    want.push(0); // SpeakerFlags
    want.push(9); // Channel: CHAN_feedback
    want.extend_from_slice(&wstring(text));

    assert_eq!(
        refusal_messages(DialRefusal::AlreadyOnDestination),
        vec![(28, want)],
        "one onPlayerCommunication, nothing else, for a same-world dial"
    );
}

/// The unknown-address refusal is the text line followed by the address-book
/// `onErrorCode` (121): `UINT8 SystemID = 0, INT32 InstanceID = 0,
/// UINT16 ErrorCodeID = 180`.
#[test]
fn the_unknown_address_refusal_is_the_line_then_error_code_180() {
    let msgs = refusal_messages(DialRefusal::UnknownAddress);
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].0, 28);
    assert!(msgs[0]
        .1
        .ends_with(&wstring("Failed to dial: not a known stargate address")));
    assert_eq!(msgs[1], (121, vec![0u8, 0, 0, 0, 0, 180, 0]));
}

/// Address-book branch: a real gate the player does not hold.
#[tokio::test]
async fn an_address_the_player_does_not_hold_says_not_a_known_address() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    let (tx, mut rx) = mpsc::channel(16);

    assert!(!handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await);
    assert_eq!(
        feedback_lines(&drain(&mut rx)),
        vec![DialRefusal::UnknownAddress.text()]
    );
}

/// `stargates` branch: the address is in the book but no such gate exists.
#[tokio::test]
async fn a_nonexistent_address_says_not_a_known_address() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr.get_entity_mut(1).unwrap().known_stargates = vec![999];
    let (tx, mut rx) = mpsc::channel(16);

    assert!(!handle_dial_gate(1, 999, 0, &tx, &mut mgr, &engine()).await);
    assert_eq!(
        feedback_lines(&drain(&mut rx)),
        vec![DialRefusal::UnknownAddress.text()]
    );
}

/// The existence-oracle rule: a nonexistent address and an address the
/// player does not hold send byte-identical client traffic, so a client
/// probing the id space learns nothing from the answer.
#[tokio::test]
async fn unknown_and_nonexistent_addresses_are_indistinguishable() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    let (tx, mut rx) = mpsc::channel(16);

    // Not held: gate 2 exists.
    mgr.get_entity_mut(1).unwrap().known_stargates = vec![];
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;
    let not_held = method_calls(&drain(&mut rx));

    // Nonexistent: held (so the book gate passes), but no gate 999.
    mgr.get_entity_mut(1).unwrap().known_stargates = vec![999];
    handle_dial_gate(1, 999, 0, &tx, &mut mgr, &engine()).await;
    let nonexistent = method_calls(&drain(&mut rx));

    assert!(!not_held.is_empty(), "the refusal must send something");
    assert_eq!(
        not_held, nonexistent,
        "an address that does not exist must look exactly like one the \
         player does not hold"
    );
}

/// No cell entity for the caller (the `player_knows_stargate` front-runner).
#[tokio::test]
async fn a_dial_with_no_cell_entity_says_not_in_a_world() {
    let mut mgr = make_manager_with_stargates();
    let (tx, mut rx) = mpsc::channel(16);

    assert!(!handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await);
    assert_eq!(
        feedback_lines(&drain(&mut rx)),
        vec![DialRefusal::NotInWorld.text()]
    );
}

/// Dialling the world you are standing on.
#[tokio::test]
async fn dialling_your_own_world_says_already_there() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(&mut mgr, 1);
    mgr.connect_entity(1);
    let (tx, mut rx) = mpsc::channel(16);

    assert!(!handle_dial_gate(1, 1, 0, &tx, &mut mgr, &engine()).await);
    assert_eq!(
        feedback_lines(&drain(&mut rx)),
        vec![DialRefusal::AlreadyOnDestination.text()]
    );
}

/// An accepted dial and a cancel send no refusal line: the feedback is for
/// refusals only.
#[tokio::test]
async fn an_accepted_dial_and_a_cancel_send_no_refusal() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(&mut mgr, 1);
    mgr.connect_entity(1);
    let (tx, mut rx) = mpsc::channel(16);

    assert!(handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await);
    assert!(!handle_dial_gate(1, -1, 0, &tx, &mut mgr, &engine()).await);
    assert!(feedback_lines(&drain(&mut rx)).is_empty());
}
