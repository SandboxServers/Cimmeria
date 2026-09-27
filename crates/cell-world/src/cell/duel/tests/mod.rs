//! SS-D1 duel tests.
//!
//! - [`registry`]: the pure state machine on an injected clock.
//! - [`challenge`]: the cell-side challenge checks (audit CAT-M-12) and the
//!   byte-exact prompt.
//! - [`response`]: `sendDuelResponse` (CAT-M-13) and the accept.
//! - [`tick`]: expiry, the countdown end and cooldown pruning.
//! - [`outbound`]: the send-failure row names its recipient.
//!
//! Every handler test drains through [`drain`], which fails on any
//! `onDuelEntitiesSet` [151] or `Clear` [153] (D-SS25).

mod challenge;
mod outbound;
mod registry;
mod response;
mod tick;

use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::messages::{CellToBaseMsg, DuelBaseToCell};
use crate::cell::space_manager::SpaceManager;

pub(super) const SPACES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Harset" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
</Spaces>"#;

pub(super) const CELL_SPACES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Harset" />
</Spaces>"#;

/// Challenger: entity 10, player 1000, account 500.
pub(super) const A_EID: u32 = 10;
pub(super) const A_PID: i32 = 1000;
/// Target: entity 20, player 2000, account 600.
pub(super) const B_EID: u32 = 20;
pub(super) const B_PID: i32 = 2000;
/// A third player: entity 30, player 3000.
pub(super) const C_EID: u32 = 30;
pub(super) const C_PID: i32 = 3000;

/// Challenger at the origin, target 5 units away, and a third player 5
/// units the other way, all in Agnos.
pub(super) fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(SPACES_XML).unwrap();
    mgr.create_startup_spaces(CELL_SPACES_XML).unwrap();
    add_player(&mut mgr, A_EID, A_PID, 500, "Agnos", [0.0, 0.0, 0.0]);
    add_player(&mut mgr, B_EID, B_PID, 600, "Agnos", [5.0, 0.0, 0.0]);
    add_player(&mut mgr, C_EID, C_PID, 700, "Agnos", [0.0, 0.0, 5.0]);
    mgr
}

pub(super) fn add_player(
    mgr: &mut SpaceManager,
    eid: u32,
    pid: i32,
    account: u32,
    world: &str,
    pos: [f32; 3],
) {
    mgr.create_entity(eid, world, pos, [0.0; 3]).unwrap();
    mgr.connect_entity(eid);
    let e = mgr.get_entity_mut(eid).unwrap();
    e.player_id = Some(pid);
    e.account_id = Some(account);
}

pub(super) fn challenge_msg(from: (u32, i32), to: (u32, i32)) -> DuelBaseToCell {
    DuelBaseToCell::Challenge {
        player_id: from.1,
        entity_id: from.0,
        account_id: 500,
        target_player_id: to.1,
        target_entity_id: to.0,
    }
}

/// Run a challenge from `from` to `to` at `now`.
pub(super) async fn challenge(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    from: (u32, i32),
    to: (u32, i32),
    now: Instant,
) {
    super::challenge::handle_at(challenge_msg(from, to), tx, mgr, now).await;
}

/// One client method the handlers queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Sent {
    pub entity_id: u32,
    pub method_index: u16,
    pub args: Vec<u8>,
}

/// Drain the channel. Panics on any `onDuelEntitiesSet` / `Clear`.
pub(super) fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<Sent> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert!(
                    method_index != 151 && method_index != 153,
                    "SS-D1 must never send onDuelEntitiesSet/Clear (D-SS25)"
                );
                out.push(Sent {
                    entity_id,
                    method_index,
                    args,
                });
            }
            other => panic!("unexpected message {other:?}"),
        }
    }
    out
}

/// The text of an `onPlayerCommunication` [28] send, checking it is a
/// feedback line (channel 9).
pub(super) fn feedback_text(sent: &Sent) -> String {
    assert_eq!(sent.method_index, 28, "not a feedback line: {sent:?}");
    let a = &sent.args;
    let speaker_len = u32::from_le_bytes(a[0..4].try_into().unwrap()) as usize;
    let mut off = 4 + speaker_len * 2;
    assert_eq!(a[off + 1], 9, "feedback rides channel 9");
    off += 2;
    let n = u32::from_le_bytes(a[off..off + 4].try_into().unwrap()) as usize;
    off += 4;
    let units: Vec<u16> = (0..n)
        .map(|i| u16::from_le_bytes([a[off + 2 * i], a[off + 2 * i + 1]]))
        .collect();
    String::from_utf16(&units).unwrap()
}

/// Every feedback line sent to `entity_id`, in order.
pub(super) fn lines_to(sent: &[Sent], entity_id: u32) -> Vec<String> {
    sent.iter()
        .filter(|s| s.entity_id == entity_id && s.method_index == 28)
        .map(feedback_text)
        .collect()
}
