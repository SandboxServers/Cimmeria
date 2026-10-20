//! Tests for the pre-game dialog (MG-21). Each test passes `start_dialog`
//! explicitly, never the global switch.

use super::state::drain_refusal;
use super::*;
use std::time::{Duration, Instant};

use cimmeria_wire::cell::client_methods::minigame::{
    ON_START_MINIGAME_DIALOG, ON_START_MINIGAME_DIALOG_CLOSE,
};
use cimmeria_wire::state_field::{BSF_DEAD, BSF_PLAYING_MINIGAME};

fn mgr_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(1).unwrap().is_player = true;
    mgr.get_entity_mut(1).unwrap().player_id = Some(7);
    mgr
}

fn request() -> MinigameLaunchRequest {
    MinigameLaunchRequest {
        entity_id: 1,
        player_id: 7,
        game_name: "Livewire".into(),
        difficulty: 3,
        on_victory_chains: vec![1001],
        on_failure_chains: vec![1002],
        source: "content_chain",
    }
}

/// Every message queued on the channel, in order.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn start_minigame_count(sent: &[CellToBaseMsg]) -> usize {
    sent.iter()
        .filter(|m| matches!(m, CellToBaseMsg::StartMinigame { .. }))
        .count()
}

/// A refused answer: no `StartMinigame`, and client 54 closes the dialog.
fn assert_refused(sent: &[CellToBaseMsg]) {
    assert_eq!(start_minigame_count(sent), 0, "{sent:?}");
    assert!(sent.iter().any(|m| matches!(
        m,
        CellToBaseMsg::EntityMethodCall { method_index, .. }
            if *method_index == ON_START_MINIGAME_DIALOG_CLOSE
    )));
}

/// A wide string as the client reads it: `u32` length, then UTF-16LE.
fn wstr(s: &str) -> Vec<u8> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let mut v = (units.len() as u32).to_le_bytes().to_vec();
    for u in units {
        v.extend(u.to_le_bytes());
    }
    v
}

/// The client 53 arguments, spelt out field by field.
fn expected_dialog_args() -> Vec<u8> {
    let mut v = wstr("Livewire");
    v.extend(wstr("Difficulty 3"));
    v.extend(1i32.to_le_bytes());
    v.extend(wstr("Livewire"));
    v.extend(0xFFFFi32.to_le_bytes());
    v.extend([1u8, 0, 0]);
    v
}

/// Off: the launch is direct. One `StartMinigame` is sent and no dialog (53).
/// Revert: a `start_or_offer` that always shows the dialog fails this.
#[tokio::test]
async fn off_sends_start_minigame_and_no_dialog() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);

    assert!(start_or_offer(request(), false, &tx, &mut mgr).await);

    let sent = drain(&mut rx);
    assert_eq!(start_minigame_count(&sent), 1);
    assert!(!sent.iter().any(|m| matches!(
        m,
        CellToBaseMsg::EntityMethodCall { method_index, .. }
            if *method_index == ON_START_MINIGAME_DIALOG
    )));
}

/// On: client 53 with the exact argument bytes, and no `StartMinigame`, with
/// the hold still clear. Play launches and sets the hold. A second Play sends
/// only 54.
/// Revert: an `expected_dialog_args` mismatch or a direct launch fails this.
#[tokio::test]
async fn on_shows_dialog_then_play_launches_and_a_second_play_closes() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);

    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1, "only the dialog is sent: {sent:?}");
    match &sent[0] {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(*entity_id, 1);
            assert_eq!(*method_index, ON_START_MINIGAME_DIALOG);
            assert_eq!(*args, expected_dialog_args());
        }
        other => panic!("expected the start dialog, got {other:?}"),
    }
    assert_eq!(
        mgr.get_entity(1).unwrap().state_field & BSF_PLAYING_MINIGAME,
        0
    );

    assert!(accept_offer(1, Instant::now(), &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(start_minigame_count(&sent), 1);
    assert_ne!(
        mgr.get_entity(1).unwrap().state_field & BSF_PLAYING_MINIGAME,
        0
    );

    assert!(!accept_offer(1, Instant::now(), &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 1, "only the close is sent: {sent:?}");
    match &sent[0] {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(*method_index, ON_START_MINIGAME_DIALOG_CLOSE);
            assert!(args.is_empty());
        }
        other => panic!("expected the dialog close, got {other:?}"),
    }
}

/// Cancel after an offer removes it and sends nothing. With no offer it
/// returns false and sends nothing.
/// Revert: a `cancel_offer` that leaves the offer in place fails the
/// `offers` assert, and one that sends fails the empty-channel assert.
#[tokio::test]
async fn cancel_removes_the_offer_and_sends_nothing() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    assert_eq!(drain(&mut rx).len(), 1);

    assert!(cancel_offer(1, &mut mgr));
    assert!(mgr.minigame_launch.offers.is_empty());
    assert!(drain(&mut rx).is_empty());

    assert!(!cancel_offer(1, &mut mgr));
    assert!(drain(&mut rx).is_empty());
}

/// Play 61 s after the offer (one second past its expiry) sends 54 and no
/// `StartMinigame`, and the offer is gone.
/// Revert: dropping the expiry check launches the expired offer and fails.
#[tokio::test]
async fn play_after_the_offer_expires_closes_the_dialog() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    assert_eq!(drain(&mut rx).len(), 1);
    let late = mgr.minigame_launch.offers[&1].expires_at + Duration::from_secs(1);

    assert!(!accept_offer(1, late, &tx, &mut mgr).await);

    let sent = drain(&mut rx);
    assert_eq!(start_minigame_count(&sent), 0);
    assert!(sent.iter().any(|m| matches!(
        m,
        CellToBaseMsg::EntityMethodCall { method_index, .. }
            if *method_index == ON_START_MINIGAME_DIALOG_CLOSE
    )));
    assert!(mgr.minigame_launch.offers.is_empty());
}

/// Play while the player is already in a minigame sends 54 and no
/// `StartMinigame`, and the offer is gone.
/// Revert: dropping the `already_playing` branch sends the `StartMinigame` and fails this.
#[tokio::test]
async fn accept_while_already_playing_closes_the_dialog() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    drain(&mut rx);
    mgr.get_entity_mut(1).unwrap().state_field |= BSF_PLAYING_MINIGAME;

    assert!(!accept_offer(1, Instant::now(), &tx, &mut mgr).await);

    let sent = drain(&mut rx);
    assert_refused(&sent);
    assert!(mgr.minigame_launch.offers.is_empty());
}

/// Tearing the entity down drops its offer, so a recycled id cannot inherit it.
/// Revert: removing the cleanup line in `destroy_entity` leaves the offer and fails this.
#[tokio::test]
async fn destroy_entity_drops_the_pending_offer() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    drain(&mut rx);
    assert!(mgr.minigame_launch.offers.contains_key(&1));

    mgr.destroy_entity(1);

    assert!(mgr.minigame_launch.offers.is_empty());
}

/// An id recycled to another player after a teardown gets no launch: the offer
/// is gone at destroy, and a Play on the new entity sends only 54.
/// Revert: keeping the offer through `destroy_entity` fails the empty assert,
/// and a launch on the new entity fails `assert_refused`.
#[tokio::test]
async fn play_on_a_recycled_entity_id_is_refused() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    drain(&mut rx);

    mgr.destroy_entity(1);
    assert!(mgr.minigame_launch.offers.is_empty());
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let newcomer = mgr.get_entity_mut(1).unwrap();
    newcomer.is_player = true;
    newcomer.player_id = Some(200);

    assert!(!accept_offer(1, Instant::now(), &tx, &mut mgr).await);
    assert_refused(&drain(&mut rx));
}

/// The offer belongs to the player it was made for: the same entity answered
/// by another player's session sends only 54.
/// Revert: dropping the `player_mismatch` check launches for player 7 on player 200's entity.
#[tokio::test]
async fn accept_refuses_an_offer_for_another_player() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    drain(&mut rx);
    mgr.get_entity_mut(1).unwrap().player_id = Some(200);

    assert!(!accept_offer(1, Instant::now(), &tx, &mut mgr).await);

    assert_refused(&drain(&mut rx));
    assert!(mgr.minigame_launch.offers.is_empty());
}

/// A player who died while the dialog was open gets no launch.
/// Revert: dropping the `dead` check launches a minigame for a dead player.
#[tokio::test]
async fn accept_refuses_a_dead_entity() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    drain(&mut rx);
    mgr.get_entity_mut(1).unwrap().state_field |= BSF_DEAD;

    assert!(!accept_offer(1, Instant::now(), &tx, &mut mgr).await);

    assert_refused(&drain(&mut rx));
}

/// A player who left the offer's space while the dialog was open gets no launch.
/// The offer records the space it was made in; repointing that record to a
/// space the entity is not in stands for the move. The entity itself stays put,
/// so `get_entity` still finds it and only the space check can refuse.
/// Revert: dropping the `moved` check launches into the space the player left.
#[tokio::test]
async fn accept_refuses_an_entity_that_left_its_space() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    drain(&mut rx);
    mgr.minigame_launch.offers.get_mut(&1).unwrap().space_id = 999;

    assert!(!accept_offer(1, Instant::now(), &tx, &mut mgr).await);

    assert_refused(&drain(&mut rx));
}

/// MG-24: two launches queued by effect scripts start in queue order with the
/// switch off, and the queue is empty after the drain.
/// Revert: a drain that leaves the queue in place, or sends nothing, fails this.
#[tokio::test]
async fn drain_starts_queued_launches_in_order_and_empties_the_queue() {
    let mut mgr = mgr_with_player();
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(2).unwrap().player_id = Some(8);
    let (tx, mut rx) = mpsc::channel(8);
    mgr.minigame_launch.queue.push(request());
    mgr.minigame_launch.queue.push(MinigameLaunchRequest {
        entity_id: 2,
        player_id: 8,
        difficulty: 4,
        ..request()
    });

    drain_launch_queue(&tx, &mut mgr).await;

    let sent = drain(&mut rx);
    let difficulties: Vec<u32> = sent
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::StartMinigame { difficulty, .. } => Some(*difficulty),
            _ => None,
        })
        .collect();
    assert_eq!(difficulties, vec![3, 4], "{sent:?}");
    assert!(mgr.minigame_launch.queue.is_empty());
}

/// MG-24 review F2: two launches queued for one entity start one game. The
/// first takes the hold, so the second is refused at drain time.
/// Revert: without the `already_playing` drain check both sends go out.
#[tokio::test]
async fn two_queued_launches_for_one_entity_start_one_game() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    mgr.minigame_launch.queue.push(request());
    mgr.minigame_launch.queue.push(MinigameLaunchRequest {
        difficulty: 4,
        ..request()
    });

    drain_launch_queue(&tx, &mut mgr).await;

    assert_eq!(start_minigame_count(&drain(&mut rx)), 1);
    assert!(mgr.minigame_launch.queue.is_empty());
}

/// MG-24 review F2: a launch whose entity already has an open offer waits,
/// so a second offer never replaces the first.
/// Revert: without the `offer_pending` check the second offer overwrites it.
#[test]
fn drain_refusal_names_an_open_offer() {
    let mut mgr = mgr_with_player();
    mgr.minigame_launch.offers.insert(
        1,
        PendingOffer {
            req: request(),
            space_id: 0,
            expires_at: Instant::now() + OFFER_TTL,
        },
    );

    assert_eq!(drain_refusal(&request(), None, &mgr), Some("offer_pending"));
}

/// MG-24 review F1: the drain names a recycled entity id before any dialog is
/// offered to the new occupant. The launch path refuses too, but with the
/// dialog on only this check stands between the new occupant and the offer.
/// Revert: without the `player_mismatch` check the drain returns no refusal.
#[test]
fn drain_refusal_names_a_recycled_entity_id() {
    let mut mgr = mgr_with_player();
    mgr.get_entity_mut(1).unwrap().player_id = Some(9);

    assert_eq!(
        drain_refusal(&request(), None, &mgr),
        Some("player_mismatch")
    );
}

/// MG-24 review F3: a caster killed after the cast does not start a game.
/// Revert: without the `caster_dead` check the launch starts and holds.
#[tokio::test]
async fn drain_refuses_a_caster_that_died_after_the_cast() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let space = *mgr.entity_space.get(&1).unwrap();
    mgr.minigame_launch.enqueue(request(), space).unwrap();
    mgr.get_entity_mut(1).unwrap().state_field |= BSF_DEAD;

    drain_launch_queue(&tx, &mut mgr).await;

    assert_eq!(start_minigame_count(&drain(&mut rx)), 0);
    assert!(mgr.minigame_launch.queue.is_empty());
}

/// MG-24 review F3: a caster who left the space it queued in does not start a
/// game in the new one. The queued space is one the entity is no longer in
/// (its live space is the fixture's). `drain_refusal` is called directly: a
/// moved entity moved between real spaces, and the fixture has one.
/// Revert: without the `stale_queue` check the drain names no refusal.
#[test]
fn drain_refuses_a_caster_that_left_its_queued_space() {
    let mgr = mgr_with_player();
    let live = *mgr.entity_space.get(&1).unwrap();

    assert_eq!(
        drain_refusal(&request(), Some(live + 1), &mgr),
        Some("stale_queue")
    );
    assert_eq!(drain_refusal(&request(), Some(live), &mgr), None);
}

/// MG-24 review F1: a queued launch for a player id the entity no longer
/// holds (its id was recycled) does not start.
/// Revert: without the `player_mismatch` check the new occupant is launched.
#[tokio::test]
async fn drain_refuses_a_recycled_entity_id() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    mgr.minigame_launch.queue.push(request());
    mgr.get_entity_mut(1).unwrap().player_id = Some(9);

    drain_launch_queue(&tx, &mut mgr).await;

    assert_eq!(start_minigame_count(&drain(&mut rx)), 0);
}

/// MG-24 review F3: the queue is bounded. A full queue refuses the next
/// launch, and the refused one is not queued.
/// Revert: an `enqueue` without the bound grows the queue past it.
#[test]
fn enqueue_bounds_the_queue() {
    let mut mgr = mgr_with_player();
    let state = &mut mgr.minigame_launch;
    for id in 0..MAX_QUEUED_LAUNCHES as u32 {
        let req = MinigameLaunchRequest {
            entity_id: 100 + id,
            ..request()
        };
        assert_eq!(state.enqueue(req, 0), Ok(()));
    }
    let extra = MinigameLaunchRequest {
        entity_id: 999,
        ..request()
    };
    assert_eq!(state.enqueue(extra, 0), Err("queue_full"));
    assert_eq!(state.queue.len(), MAX_QUEUED_LAUNCHES);
}

/// MG-24: a drain with nothing queued sends nothing.
#[tokio::test]
async fn drain_of_an_empty_queue_sends_nothing() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);

    drain_launch_queue(&tx, &mut mgr).await;

    assert!(drain(&mut rx).is_empty());
}
