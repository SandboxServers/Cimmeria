//! Organization router and squad handler tests.
//!
//! Every client-bound call is captured from the `CellToBaseMsg` channel as
//! `(recipient entity, method index, args)` and compared against the wire
//! builders, which are themselves pinned byte for byte in `cimmeria-wire`.
//! Rejections are negative-log tests (TESTING.md type 12): the event, its
//! level and `reason`, the feedback the player got, and unchanged state.

use tokio::sync::mpsc;
use tracing::Level;

use super::*;
use crate::test_support::{make_space_manager, LogCaptureGuard};

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::player::build_on_error_code;

mod org_creation;
mod router;
mod squad_invite;
mod squad_loot_entry;
mod squad_membership;
mod squad_ping_gm;
mod squad_telemetry;

/// One captured client-method call.
pub(super) type Sent = (u32, u16, Vec<u8>);

/// Drain everything the handlers queued for the base.
pub(super) fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<Sent> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => out.push((entity_id, method_index, args)),
            other => panic!("unexpected {other:?}"),
        }
    }
    out
}

/// The calls addressed to `entity_id`, in order.
pub(super) fn to(sent: &[Sent], entity_id: u32) -> Vec<(u16, Vec<u8>)> {
    sent.iter()
        .filter(|s| s.0 == entity_id)
        .map(|s| (s.1, s.2.clone()))
        .collect()
}

/// The rejection pair: `onErrorCode(0, instance, 0)` then `text`.
pub(super) fn rejection(instance: i32, text: &str) -> Vec<(u16, Vec<u8>)> {
    vec![(121, build_on_error_code(0, instance, 0)), (28, line(text))]
}

/// `text` from `SYSTEM` on the feedback channel.
pub(super) fn line(text: &str) -> Vec<u8> {
    serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text)
}

/// The account the fixture gives character `player_id`.
pub(super) fn account_of(player_id: i32) -> u32 {
    1000 + player_id as u32
}

/// A connected, initialised player: entity `entity_id`, character
/// `player_id`, named `name`.
pub(super) fn add_player(mgr: &mut SpaceManager, entity_id: u32, player_id: i32, name: &str) {
    mgr.create_entity(entity_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    let e = mgr.get_entity_mut(entity_id).unwrap();
    e.player_id = Some(player_id);
    e.account_id = Some(account_of(player_id));
    e.character_name = Some(name.to_string());
    e.level = 12;
    e.archetype_id = Some(3);
}

/// Entities 11.., characters 1.., named `names[i]`.
pub(super) fn world(names: &[&str]) -> SpaceManager {
    let mut mgr = make_space_manager();
    for (i, name) in names.iter().enumerate() {
        add_player(&mut mgr, 11 + i as u32, 1 + i as i32, name);
    }
    mgr
}

pub(super) fn channel() -> (mpsc::Sender<CellToBaseMsg>, mpsc::Receiver<CellToBaseMsg>) {
    mpsc::channel(256)
}

/// Invite the player at entity `invitee_entity` from the one at
/// `inviter_entity` and accept, returning the request id. Characters are
/// `entity - 10`.
pub(super) async fn invite_accept(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    inviter_entity: u32,
    invitee_entity: u32,
) -> i32 {
    let request_id = invite_only(mgr, tx, inviter_entity, invitee_entity).await;
    squad::respond(invitee_entity, request_id, true, tx, mgr).await;
    request_id
}

/// Issue an invite and return its request id.
pub(super) async fn invite_only(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    inviter_entity: u32,
    invitee_entity: u32,
) -> i32 {
    let invitee = invitee_entity as i32 - 10;
    let name = mgr
        .get_entity(invitee_entity)
        .unwrap()
        .character_name
        .clone()
        .unwrap();
    let now = std::time::Instant::now();
    let before = mgr.squads.pending_requests(invitee, now);
    squad::handle_invite(inviter_entity as i32 - 10, inviter_entity, &name, tx, mgr).await;
    let after = mgr.squads.pending_requests(invitee, now);
    *after
        .iter()
        .find(|id| !before.contains(id))
        .unwrap_or_else(|| panic!("invite from {inviter_entity} to {invitee_entity} not issued"))
}

/// Put the players at `members` into `leader`'s squad straight through the
/// registry, with timestamps a rate window in the past, so the handler
/// under test starts with a fresh invite budget. Stamps `squad_id` like
/// the join fanout does; sends nothing.
pub(super) fn seed_squad(mgr: &mut SpaceManager, leader: u32, members: &[u32]) -> i32 {
    let past = std::time::Instant::now()
        .checked_sub(crate::cell::squad::INVITE_RATE_WINDOW)
        .expect("uptime exceeds the rate window");
    let snap = |mgr: &SpaceManager, e: u32| squad::test_snapshot(mgr, e);
    let lead = snap(mgr, leader);
    let mut sid = 0;
    for &e in members {
        let m = snap(mgr, e);
        let issued = mgr
            .squads
            .invite(lead.player_id, &lead.name, m.player_id, past)
            .unwrap();
        let inv = mgr
            .squads
            .take_invite(m.player_id, issued.request_id, past)
            .unwrap();
        sid = mgr
            .squads
            .accept(&inv, m, Some(lead.clone()))
            .unwrap()
            .squad_id;
    }
    for &e in std::iter::once(&leader).chain(members) {
        mgr.get_entity_mut(e).unwrap().squad_id = Some(sid);
    }
    sid
}

/// A `rejected` outcome row named `event` at `level` with `reason`, on the
/// `squad` target.
pub(super) fn squad_event(
    capture: &LogCaptureGuard,
    level: Level,
    event: &str,
    reason: &str,
) -> bool {
    capture.all().iter().any(|c| {
        c.level == level
            && c.target == "squad"
            && c.has_field("event", event)
            && c.has_field("outcome", "rejected")
            && c.has_field("reason", reason)
    })
}
