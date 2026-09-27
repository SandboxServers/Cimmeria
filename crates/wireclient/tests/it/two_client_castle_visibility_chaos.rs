//! Two real wire clients, one shared Castle world, under real-network
//! conditions (NA37 round 2).
//!
//! `two_client_castle_visibility.rs` and NA34's in-process test both run
//! over lossless localhost with sub-millisecond RTT and zero reordering.
//! The owner plays over the internet from the colo. This file wraps the
//! real `BaseService` UDP socket in `cimmeria_mercury::lossy_transport`
//! (TESTING.md type 10 / `docs/architecture/network-chaos-testing.md`) so
//! the same two-client Castle scenario runs under packet loss, jitter,
//! elevated latency, and a deterministic burst drop with retransmit --
//! the conditions that were never exercised before this round.
//! `LossyTransport`'s reorder-buffer primitive (also added this round) is
//! unit-tested in `crates/mercury/src/lossy_transport.rs` but deliberately
//! **not** exercised end to end here -- see the doc comment on
//! `lossy_network_both_directions_still_converge` for why it breaks the
//! Mercury phase-3 handshake's positional parsing.
//!
//! Shared setup/driver helpers live in `tests/it/support/mod.rs`.
//!
//! Run locally (same DB setup as the lossless variant):
//! ```text
//! /c/Users/Steve/AppData/Local/Temp/cimmeria-castle/reload-db.sh
//! DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/<db> \
//!   cargo test -p cimmeria-wireclient --test it two_client_castle_visibility_chaos -- --test-threads=1
//! ```
//! Requires the `chaos-testing` feature on the `cimmeria-services`
//! dev-dependency (already enabled in `Cargo.toml`) for
//! `BaseService::set_transport_override`.

use std::sync::Arc;
use std::time::Duration;

use cimmeria_mercury::lossy_transport::{LossyConfig, LossyTransport};
use cimmeria_mercury::transport::BidirectionalTransport;
use cimmeria_wireclient::bundle::S2CMessage;

use crate::support::{
    bind_base_socket, credentials_for, enter_castle_with_timeout, insert_castle_character,
    insert_sentinel_account, live_db_pool_or_skip, start_server_with_base_transport, wait_for,
    wait_for_recording, CASTLE_BASE_POS,
};

/// Generous per-step recv timeout for scenarios running under injected
/// loss/latency -- retransmit needs RTO cycles to recover, and jitter can
/// stack across several packets in one world-entry step.
const CHAOS_RECV_TIMEOUT: Duration = Duration::from_secs(20);
/// Bound for "does the introduction eventually complete" assertions.
const CONVERGENCE_TIMEOUT: Duration = Duration::from_secs(25);

/// Scenario 1: ~7% packet loss plus jitter on the server→client direction
/// during world entry. Both clients enter Castle (in the "walk-up" order
/// -- A already in world, B connecting into a populated space, the harder
/// direction per NA34/NA37-round-1) and both must still end up with the
/// other's `CREATE_ENTITY` + `BEING_APPEARANCE` within
/// [`CONVERGENCE_TIMEOUT`]. Client→server stays lossless -- the scenario
/// targets what the owner's *client* would actually experience (packets
/// from the server arriving damaged), not a symmetric assumption that
/// would also make outbound acks unreliable and confound the result.
///
/// **Deliberately excludes `LossyConfig::with_reorder_buffer`.** The
/// Mercury phase-3 handshake (`baseAppLogin` reply + time-sync) is exactly
/// two raw, unencrypted-then-encrypted datagrams that `GameSession`
/// (`crates/wireclient/src/session.rs`) reads *positionally* off the
/// socket before any `Channel`/reassembly machinery exists to reorder
/// them back -- there is no sequence number to key on yet at that point
/// in the protocol. A reorder buffer sized ≥2 on a fresh connection's
/// destination bucket holds (or swaps) exactly those first two packets,
/// which breaks handshake parsing outright (confirmed empirically: this
/// test failed with `"unexpected flags 0x48 (want 0x58)"` when a
/// `with_reorder_buffer(3)` config was tried here). This isn't a server
/// bug -- the real client's own phase-3 handshake is presumably just as
/// positionally rigid, predating the Channel's general sequencing. See
/// the burst-drop scenario below for reorder-adjacent (retransmit
/// ordering) coverage instead.
#[tokio::test]
async fn lossy_network_both_directions_still_converge() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "cimmeria_services=info,cimmeria_auth=info".to_string()),
        )
        .with_test_writer()
        .try_init();
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };

    let (raw_transport, base_port) = bind_base_socket().await;
    // Send (server->client): ~7% loss, 5ms base latency + 30ms jitter.
    // Recv (client->server): lossless -- see the doc comment above.
    let send_cfg = LossyConfig::new(Duration::from_millis(5), 70, 0, 0xCA57_7E01)
        .with_jitter(Duration::from_millis(30));
    let recv_cfg = LossyConfig::new(Duration::ZERO, 0, 0, 0xCA57_7E02);
    let lossy = Arc::new(LossyTransport::new_asymmetric(
        raw_transport,
        send_cfg,
        recv_cfg,
    ));
    let server = start_server_with_base_transport(
        &std::env::var("DATABASE_URL").unwrap(),
        base_port,
        lossy.clone() as Arc<dyn BidirectionalTransport>,
    )
    .await;

    const SENTINEL_ACCOUNT: i32 = 900_301;
    const A_PLAYER_ID: i32 = 900_302;
    const B_PLAYER_ID: i32 = 900_303;
    insert_sentinel_account(&pool, SENTINEL_ACCOUNT, "na37_chaos_b").await;
    let pos_a = CASTLE_BASE_POS;
    let pos_b = [
        CASTLE_BASE_POS[0] + 50.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];
    insert_castle_character(&pool, 2, A_PLAYER_ID, "NA37ChaosA", pos_a).await;
    insert_castle_character(&pool, SENTINEL_ACCOUNT, B_PLAYER_ID, "NA37ChaosB", pos_b).await;

    let gm_creds = credentials_for("test");
    let b_creds = credentials_for("na37_chaos_b");

    // A enters first and is fully ready before B connects -- the
    // "walk-up" order.
    let a = enter_castle_with_timeout(
        &server.auth_url,
        &gm_creds,
        A_PLAYER_ID,
        1,
        CHAOS_RECV_TIMEOUT,
    )
    .await;
    let b = enter_castle_with_timeout(
        &server.auth_url,
        &b_creds,
        B_PLAYER_ID,
        2,
        CHAOS_RECV_TIMEOUT,
    )
    .await;
    let a_id = a.player_entity_id.unwrap();
    let b_id = b.player_entity_id.unwrap();

    // Direction 1: A sees B, despite loss/reorder/jitter on everything the
    // server sent A from the moment A connected.
    wait_for(&a, CONVERGENCE_TIMEOUT, |m| {
        m.is_create() && m.entity_id == Some(b_id)
    })
    .await
    .unwrap_or_else(|| {
        panic!("lossy network: A never saw B's create within {CONVERGENCE_TIMEOUT:?}")
    });
    wait_for(&a, CONVERGENCE_TIMEOUT, |m| {
        m.method_index == Some(26) && m.entity_id == Some(b_id)
    })
    .await
    .unwrap_or_else(|| {
        panic!("lossy network: A never saw B's BEING_APPEARANCE within {CONVERGENCE_TIMEOUT:?}")
    });

    // Direction 2: B sees A.
    wait_for(&b, CONVERGENCE_TIMEOUT, |m| {
        m.is_create() && m.entity_id == Some(a_id)
    })
    .await
    .unwrap_or_else(|| {
        panic!("lossy network: B never saw A's create within {CONVERGENCE_TIMEOUT:?}")
    });
    wait_for(&b, CONVERGENCE_TIMEOUT, |m| {
        m.method_index == Some(26) && m.entity_id == Some(a_id)
    })
    .await
    .unwrap_or_else(|| {
        panic!("lossy network: B never saw A's BEING_APPEARANCE within {CONVERGENCE_TIMEOUT:?}")
    });

    // Flush anything still held in the reorder buffer before shutdown so a
    // partial buffer doesn't mask an in-flight send during teardown.
    let _ = lossy.flush_reorder_buffer().await;

    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = ANY($1)")
        .bind([A_PLAYER_ID, B_PLAYER_ID].as_slice())
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(SENTINEL_ACCOUNT)
        .execute(&pool)
        .await;
    server.orchestrator.stop_all().await;
}

/// Scenario 2: burst-drop the exact reliable packet carrying the peer's
/// introduction, and check three things:
///
/// 1. Is it retransmitted? (Mercury's Channel-level RTO/TX-window
///    mechanism should fire regardless of payload -- `CREATE_ENTITY` gets
///    no special treatment.)
/// 2. Does the eventual delivery still land within a bounded time?
/// 3. **Ordering**: does any *reliable-stream* message referencing the
///    peer's entity id (the appearance/stat cascade, method calls) reach
///    the witness's message handlers *before* the retransmitted
///    `CREATE_ENTITY`? The witness's `GameSession` now runs the same
///    in-order receive gate as the SGW client
///    (`Channel::receive_parsed`, modelled on `queueAckForPacket` at
///    `ghidra://SGW.exe@0x0158cba0`), so the cascade must wait behind the
///    lost create exactly as it does on a real client.
///
/// Unreliable position relays (`UPDATE_AVATAR`, msg `0x10`) are *expected*
/// to arrive first: they ride the separate unreliable stream, which the
/// client delivers on arrival. That is harmless on the real client:
/// `EntityManager::onEntityMoveWithError` (`ghidra://SGW.exe@0x00dd1650`)
/// stores the latest position for an id it does not know yet in the
/// pending-entity map at `EntityManager+0x30`, and the create handler
/// (`ghidra://SGW.exe@0x00dd2270`) consumes that record when it builds the
/// entity. So the test requires the early messages to be position relays
/// only, and requires at least one, which proves the drop really landed on
/// B's create rather than on some other packet.
///
/// The drop targets B's introduction specifically (not A's own world-entry
/// traffic) via [`LossyTransport::drop_next_sends_to`], armed on A's
/// address only after A is fully settled and before B connects, so the
/// very next thing the server sends to A's address is deterministically
/// the AoI-tick-driven `CREATE_ENTITY` for B.
///
/// History: NA37 round 2 ran this with a harness that handed bundles over
/// in raw arrival order and saw the whole cascade land before the create.
/// NA38 found the real client orders its reliable stream itself, so that
/// was a harness gap, not a server bug. With the gate wired into the
/// harness this passes on an unchanged server. Reverting the gate (every
/// reliable packet delivered on arrival) fails it again. See
/// `docs/analysis/npc-ai-restoration/work-packets.md` NA37 and NA38.
#[tokio::test]
async fn burst_drop_of_peer_create_entity_recovers_via_retransmit() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "cimmeria_services=info,cimmeria_auth=info".to_string()),
        )
        .with_test_writer()
        .try_init();
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };

    let (raw_transport, base_port) = bind_base_socket().await;
    // Lossless baseline -- the ONLY loss in this scenario is the
    // deterministic targeted drop armed below, so the test isolates the
    // burst-drop-and-retransmit behavior from any probabilistic noise.
    let lossy = Arc::new(LossyTransport::new_symmetric(
        raw_transport,
        LossyConfig::new(Duration::ZERO, 0, 0, 0),
    ));
    let server = start_server_with_base_transport(
        &std::env::var("DATABASE_URL").unwrap(),
        base_port,
        lossy.clone() as Arc<dyn BidirectionalTransport>,
    )
    .await;

    const SENTINEL_ACCOUNT: i32 = 900_401;
    const A_PLAYER_ID: i32 = 900_402;
    const B_PLAYER_ID: i32 = 900_403;
    insert_sentinel_account(&pool, SENTINEL_ACCOUNT, "na37_burst_b").await;
    let pos_a = CASTLE_BASE_POS;
    let pos_b = [
        CASTLE_BASE_POS[0] + 30.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];
    insert_castle_character(&pool, 2, A_PLAYER_ID, "NA37BurstA", pos_a).await;
    insert_castle_character(&pool, SENTINEL_ACCOUNT, B_PLAYER_ID, "NA37BurstB", pos_b).await;

    let gm_creds = credentials_for("test");
    let b_creds = credentials_for("na37_burst_b");

    // A settles first -- fully onClientReady'd. `enter_castle_with_timeout`
    // returns as soon as A's *own* onClientReady bytes are sent, not once
    // the server has finished reacting to it -- A's own post-ready burst
    // bundle (BeingAppearance resend + chat-joined + welcome) and its own
    // AoI-tick discovery of Castle's existing NPCs both still land
    // asynchronously afterward. Empirically (`aoi.create_emit` /
    // `mercury.lossy_transport` tracing), arming the drop immediately after
    // `enter_castle_with_timeout` returns races that settling traffic --
    // the drop can catch A's *own* burst bundle instead of B's later
    // introduction. A short quiescence wait avoids the race; everything A
    // triggers on its own is on a lossless transport in this test, so it
    // completes in well under this window on localhost.
    let a = enter_castle_with_timeout(
        &server.auth_url,
        &gm_creds,
        A_PLAYER_ID,
        1,
        CHAOS_RECV_TIMEOUT,
    )
    .await;
    let a_addr = a.local_addr().expect("A's local UDP addr");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Arm the targeted drop on A's address now that A has fully settled.
    // B's own world-entry traffic goes to B's address and is untouched.
    // `min_len = 50` skips A's own small periodic tickSync/keepalive
    // traffic (observed on the wire at ~32 bytes) so the drop lands on the
    // next *substantial* send to A instead -- deterministically the
    // AoI-tick's CREATE_ENTITY introducing B once B reaches onClientReady
    // (observed on the wire at 64 bytes; confirmed via
    // `RUST_LOG=mercury.lossy_transport=debug,aoi.create_emit=debug`
    // while developing this test).
    lossy.drop_next_sends_to(1, a_addr, 50);

    let b = enter_castle_with_timeout(
        &server.auth_url,
        &b_creds,
        B_PLAYER_ID,
        2,
        CHAOS_RECV_TIMEOUT,
    )
    .await;
    let b_id = b.player_entity_id.unwrap();

    // Record everything A receives on the way to seeing B's create, so we
    // can inspect what (if anything) arrived first.
    let (create_hit, seen) = wait_for_recording(&a, CONVERGENCE_TIMEOUT, |m| {
        m.is_create() && m.entity_id == Some(b_id)
    })
    .await;
    let create_msg = create_hit.unwrap_or_else(|| {
        panic!(
            "burst-drop: A never saw B's (retransmitted) create within {CONVERGENCE_TIMEOUT:?}; \
             saw {} other messages first: {seen:?}",
            seen.len()
        )
    });
    let create_index = seen
        .iter()
        .position(|m| {
            m.msg_id == create_msg.msg_id
                && m.entity_id == create_msg.entity_id
                && m.class_id == create_msg.class_id
        })
        .expect("the matching message must be present in its own recording");

    let (early_positions, early_reliable): (Vec<&S2CMessage>, Vec<&S2CMessage>) = seen
        [..create_index]
        .iter()
        .filter(|m| m.entity_id == Some(b_id))
        .partition(|m| m.is_position_update());

    assert!(
        early_reliable.is_empty(),
        "ordering hazard: {} reliable-stream message(s) for B's entity (id {b_id}) reached A's \
         handlers before B's CREATE_ENTITY: {early_reliable:?}. The client's queueAckForPacket \
         holds these behind the lost create; the harness gate must too (NA38).",
        early_reliable.len()
    );
    assert!(
        !early_positions.is_empty(),
        "no position relay for B arrived ahead of B's create, so the targeted drop did not land \
         on the create and this run tested nothing"
    );

    // The cascade the gate held back must still follow the create.
    wait_for(&a, CONVERGENCE_TIMEOUT, |m| {
        m.method_index == Some(26) && m.entity_id == Some(b_id)
    })
    .await
    .unwrap_or_else(|| {
        panic!("burst-drop: A never saw B's BEING_APPEARANCE after the retransmitted create")
    });

    let _ = lossy.flush_reorder_buffer().await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = ANY($1)")
        .bind([A_PLAYER_ID, B_PLAYER_ID].as_slice())
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(SENTINEL_ACCOUNT)
        .execute(&pool)
        .await;
    server.orchestrator.stop_all().await;
}

/// Scenario 3: elevated latency + jitter approximating a "high-latency
/// client" (Mobile-profile-like: ~80ms base + jitter, no loss). Applied
/// symmetrically to the whole transport rather than to just one
/// destination -- `LossyTransport` chaos-wraps the entire `BaseService`
/// socket (shared across every connected client), and there is no
/// per-destination `LossyConfig` seam today to single out one peer's
/// latency independently of the other's. Testing "both clients slow" is
/// at least as hard as "one client slow" for the property this checks
/// (does introduction still complete within a bound proportional to the
/// latency), so this is a conservative stand-in; a genuine per-peer
/// latency profile is a follow-up to `LossyTransport` if a test ever
/// needs to assert asymmetric behavior specifically.
#[tokio::test]
async fn high_latency_clients_still_converge() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };

    let (raw_transport, base_port) = bind_base_socket().await;
    let cfg = LossyConfig::new(Duration::from_millis(80), 0, 0, 0x4A7E_0003)
        .with_jitter(Duration::from_millis(40));
    let lossy = Arc::new(LossyTransport::new_symmetric(raw_transport, cfg));
    let server = start_server_with_base_transport(
        &std::env::var("DATABASE_URL").unwrap(),
        base_port,
        lossy.clone() as Arc<dyn BidirectionalTransport>,
    )
    .await;

    const SENTINEL_ACCOUNT: i32 = 900_501;
    const A_PLAYER_ID: i32 = 900_502;
    const B_PLAYER_ID: i32 = 900_503;
    insert_sentinel_account(&pool, SENTINEL_ACCOUNT, "na37_latency_b").await;
    let pos_a = CASTLE_BASE_POS;
    let pos_b = [
        CASTLE_BASE_POS[0] + 20.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];
    insert_castle_character(&pool, 2, A_PLAYER_ID, "NA37LatencyA", pos_a).await;
    insert_castle_character(&pool, SENTINEL_ACCOUNT, B_PLAYER_ID, "NA37LatencyB", pos_b).await;

    let gm_creds = credentials_for("test");
    let b_creds = credentials_for("na37_latency_b");

    let a = enter_castle_with_timeout(
        &server.auth_url,
        &gm_creds,
        A_PLAYER_ID,
        1,
        CHAOS_RECV_TIMEOUT,
    )
    .await;
    let b = enter_castle_with_timeout(
        &server.auth_url,
        &b_creds,
        B_PLAYER_ID,
        2,
        CHAOS_RECV_TIMEOUT,
    )
    .await;
    let a_id = a.player_entity_id.unwrap();
    let b_id = b.player_entity_id.unwrap();

    wait_for(&a, CONVERGENCE_TIMEOUT, |m| {
        m.is_create() && m.entity_id == Some(b_id)
    })
    .await
    .unwrap_or_else(|| {
        panic!("high latency: A never saw B's create within {CONVERGENCE_TIMEOUT:?}")
    });
    wait_for(&b, CONVERGENCE_TIMEOUT, |m| {
        m.is_create() && m.entity_id == Some(a_id)
    })
    .await
    .unwrap_or_else(|| {
        panic!("high latency: B never saw A's create within {CONVERGENCE_TIMEOUT:?}")
    });

    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = ANY($1)")
        .bind([A_PLAYER_ID, B_PLAYER_ID].as_slice())
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(SENTINEL_ACCOUNT)
        .execute(&pool)
        .await;
    server.orchestrator.stop_all().await;
}
