//! H02 -- the source/destination cross-link guard.
//!
//! A ring pair is linked by `remote_region_id` on both ends. Advancing a
//! destination on its *state* alone is not enough: a pad legitimately
//! reserved by another source is also in `RecvWait`.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{spawn_player, state_of, three_ring_mgr, FakeClock};
use crate::cell::ring_transport::{handle_region_trigger, State};

/// `kick_off_warmup` advances the destination only when its back-pointer
/// names this source. Without that check it advanced on `state == RecvWait`
/// alone, so a pad reserved by a *different* source (whose own `RecvWait`
/// bound has not yet expired) would be dragged into this trip and would then
/// receive passengers it was holding no slot for. The
/// `RECV_WAIT_TIMEOUT > SEND_WAIT_TIMEOUT` margin is the cheap guard; this is
/// the structural one.
#[tokio::test]
async fn a_destination_reserved_by_another_source_is_not_advanced() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Ring 2 is in RecvWait but reserved for ring 3, not ring 1.
    let now = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(3, now);
    }
    // Ring 1 believes it is sending to 2 and its passenger steps on the pad.
    {
        let src = mgr.ring_transporters.get_mut(1).unwrap();
        src.enter_send_wait(2, 42, now);
    }
    handle_region_trigger(2001, true, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        state_of(&mgr, 1),
        State::SendWarmup,
        "the source still sends"
    );
    assert_eq!(
        state_of(&mgr, 2),
        State::RecvWait,
        "the destination belongs to ring 3's trip and must not be advanced"
    );
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().remote_region_id,
        Some(3),
        "the destination's reservation must be left pointing at its real source"
    );
}
