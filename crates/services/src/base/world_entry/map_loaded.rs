//! `mapLoaded` -> enter-world step.
//!
//! After the create-player step, the client loads terrain geometry and then
//! sends `mapLoaded` (cell method 25). This handler emits the VIEWPORT +
//! CELL_PLAYER + FORCED_POSITION standalone packet followed by a separate
//! fragmented bundle of entity methods (BeingAppearance, etc.).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{
    build_enter_world, build_map_loaded_body, fragment_count, fragment_map_loaded,
};

use super::super::world_entry_appearance::{build_appearance_args, build_tint_args};
use super::super::{ConnectedClientState, PendingClientReadyInfo};
use super::methods::default_player_load_data;

/// Enter world: send VIEWPORT + CELL_PLAYER + FORCED_POSITION + entity data.
///
/// Called when the client sends `mapLoaded` after receiving `onClientMapLoad`
/// in the create-player step. The client has finished loading terrain
/// geometry and is ready to receive entity placement and data.
///
/// In C++, this is triggered by the CellApp's `onCellPlayerCreateAck` callback
/// (which itself fires after `connected()` sends `onClientMapLoad`) and the
/// Python `onClientReady()` -> `mapLoaded()` callback chain.
pub(crate) async fn handle_map_loaded(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    _db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Take the pending data (consumes it -- enter-world only runs once per mapLoaded)
    let (entry_info, player_data) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        let entry = c
            .pending_map_loaded
            .take()
            .ok_or("handle_map_loaded: no pending world entry")?;
        let data = c
            .pending_player_load_data
            .take()
            .unwrap_or_else(default_player_load_data);
        (entry, data)
    };

    tracing::info!(
        %addr,
        player_entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        "Enter world: client map loaded -- sending VIEWPORT + CELL + POSITION + entity data"
    );

    // Send enter-world as TWO separate bundles, matching the C++ server:
    //
    // 1. VIEWPORT + CELL_PLAYER + FORCED_POSITION -- standalone 99-byte packet.
    //    This creates the cell entity, puts it in the world, and the entity enters
    //    a brief "transaction" state during creation.
    //
    // 2. Entity methods (mapLoaded body) -- separate fragmented bundle.
    //    By arriving in a new bundle, these are processed after the entity's
    //    creation transaction completes, so BeingAppearance hits the
    //    "SCHEDULING JOB" path instead of "HOLD FOR TRANSACTION".
    //
    // Previously we combined everything into one fragmented bundle, which caused
    // BeingAppearance to be silently dropped (HOLD FOR TRANSACTION) because the
    // entity was still in its creation transaction during bundle processing.
    let map_body = build_map_loaded_body(entry_info.player_entity_id, &player_data, &entry_info);

    let map_frags = fragment_count(map_body.len());
    // Reserve 1 seq for the standalone enter-world packet + N seqs for map fragments.
    let total_seqs = 1 + map_frags;

    let (acks, base_seq) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
        let seq = c.next_seq.fetch_add(total_seqs, Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK;
        (acks, seq)
    };

    // Packet 1: VIEWPORT + (BeingAppearance + onEntityTint) + CELL_PLAYER + FORCED_POSITION.
    // The appearance methods sit before createCellPlayer so the client's
    // cell-entity-creation handler picks up the bodyset during its internal
    // appearance evaluation, eliminating the dev-cube placeholder flash.
    let enter_world_pkt = build_enter_world(&key, base_seq, &acks, &entry_info, Some(&player_data));
    tracing::debug!(%addr, len = enter_world_pkt.len(), seq = base_seq,
        "UDP_OUT enter world: VIEWPORT+CELL+FORCED (standalone)");
    transport.send_to(&enter_world_pkt, addr).await?;
    // Register this reliable send with the per-session Channel's TX
    // window. ACK consumption + RTO sampling are live, and the
    // retransmit driver in `tick_sync.rs` will resend the cached bytes
    // if the RTO fires before the client acks.
    super::super::helpers::shadow_register_reliable_send(
        connected,
        addr,
        base_seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&enter_world_pkt),
    );

    // Packet 2+: Entity methods (mapLoaded body, possibly fragmented).
    // Mask `map_base_seq` and each derived seq to the 28-bit space —
    // `base_seq + 1` (or `base_seq + i`) can land on `NULL_SEQUENCE`
    // when `base_seq` is near `SEQUENCE_MASK`, which would be rejected
    // by the peer's parser and break ACK draining.
    let map_base_seq = base_seq.wrapping_add(1) & cimmeria_mercury::packet::SEQUENCE_MASK;
    let (map_packets, map_seqs) = fragment_map_loaded(&key, map_base_seq, &[], &map_body);
    debug_assert_eq!(map_seqs, map_frags);
    tracing::info!(
        %addr,
        enter_world_seq = base_seq,
        map_base_seq,
        map_fragments = map_packets.len(),
        map_body_len = map_body.len(),
        "mapLoaded: split send (standalone VIEWPORT+CELL + separate entity methods)"
    );
    for (i, pkt_data) in map_packets.iter().enumerate() {
        let frag_seq =
            map_base_seq.wrapping_add(i as u32) & cimmeria_mercury::packet::SEQUENCE_MASK;
        tracing::debug!(%addr, len = pkt_data.len(), seq = frag_seq,
            part = i + 1, total = map_packets.len(), "UDP_OUT mapLoaded entity data");
        if let Err(e) = transport.send_to(pkt_data, addr).await {
            // a single failed fragment leaves the client with
            // a partial enter-world bundle — invisible NPCs, missing
            // appearance, or stuck on the load screen. error! so a
            // partial world-entry is greppable per fragment.
            tracing::error!(
                %addr,
                fragment_idx = i + 1,
                total_fragments = map_packets.len(),
                fragment_seq = frag_seq,
                map_body_len = map_body.len(),
                "map_loaded: fragment send failed -- client will have a half-loaded world: {e}"
            );
            return Err(e.into());
        }
        super::super::helpers::shadow_register_reliable_send(
            connected,
            addr,
            frag_seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(pkt_data),
        );
    }

    let total_bytes: usize =
        enter_world_pkt.len() + map_packets.iter().map(|p| p.len()).sum::<usize>();
    let pkt_count = 1 + map_packets.len();
    tracing::info!(%addr, player = %player_data.player_name,
        level = player_data.level, archetype = player_data.archetype,
        packets = pkt_count,
        "World entry complete ({} bytes across {} packets)", total_bytes, pkt_count);

    // first_login DB clear is deferred to `handle_on_client_ready` so the
    // flag only clears after we've actually fired `onPlayMovie` — if the
    // client disconnects between mapLoaded and onClientReady, they correctly
    // see the cinematic again on their next attempt.
    //
    // The previous implementation also spawned a 10-second timer that fired
    // a synthetic `cancelMovie` to resend BeingAppearance, working around a
    // dev-cube flash on first-login cinematic exit. That hack is gone: the
    // root cause was firing onPlayMovie inside the mapLoaded bundle (before
    // the model is bound to a possessed pawn), which let the cinematic-exit
    // CollectGarbage reclaim the in-flight appearance asset. The cinematic
    // is now fired from `handle_on_client_ready` after the appearance is
    // rooted to a live actor. See issue #288.

    // Register entity_id -> addr before the final onClientReady gate so any
    // resource responses and future client-targeted traffic can resolve the
    // transport, but defer CellService player initialization until the client
    // explicitly signals readiness (matches C++ SGWPlayer.onClientReady).
    entity_to_addr
        .lock()
        .unwrap()
        .insert(entry_info.player_entity_id, addr);

    // Cache BeingAppearance + onEntityTint args for resend after onClientReady.
    // The first copy in the mapLoaded bundle may be dropped because the entity is
    // still in a "transaction" during bundle processing (all messages in a reassembled
    // bundle are processed in one frame). The C++ server sends BeingAppearance 3-5
    // times via createCacheStamp replays; this second send mimics that.
    // Player spawns weapon-holstered. The wire `ComponentList` therefore
    // omits the active bandolier weapon visual; the client's appearance
    // compositor falls back to `WEAP_Melee = 4` for the animation pose
    // key at `entity+0x3D2` (`ghidra://SGW.exe@0x00ec0840`).
    let appearance_args = build_appearance_args(
        &player_data.bodyset,
        &player_data.appearance_components(true),
    );
    let tint_args = build_tint_args(player_data.skin_color_id);

    // The C++ server waits for the exposed SGWPlayer base method
    // `onClientReady` (msg_id 0xD8) before calling into the cell-side
    // post-load logic that eventually fires `player.loaded`.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        // Cache appearance data for resend after cinematic (cancelMovie).
        // PendingClientReadyInfo is consumed by onClientReady, but cancelMovie
        // may arrive later (after the cinematic ends).
        c.cached_appearance_args = Some(appearance_args.clone());
        c.cached_tint_args = Some(tint_args.clone());
        c.pending_client_ready = Some(PendingClientReadyInfo {
            entity_id: entry_info.player_entity_id,
            player_id: player_data.player_id,
            world_name: entry_info.world_name.clone(),
            appearance_args,
            tint_args,
            first_login: player_data.first_login,
        });
    }

    tracing::info!(%addr, "World entry complete -- waiting for SGWPlayer.onClientReady");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestTransport;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn map_loaded_errors_when_no_pending_entry() {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new({
                let mut m = HashMap::new();
                m.insert(
                    addr,
                    crate::test_support::test_default_connected_client_state(),
                );
                m
            }));
        let key = [0u8; 32];

        // Client has no pending_map_loaded — must return an error.
        let result = handle_map_loaded(
            &transport,
            addr,
            key,
            &connected,
            &None,
            &Arc::new(Mutex::new(HashMap::new())),
            &None,
        )
        .await;
        let err = result.expect_err("must fail when pending_map_loaded is missing");
        assert!(
            err.to_string().contains("no pending world entry"),
            "unexpected error: {err}"
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Negative-logging regression guard.
    //
    // Per-fragment `transport.send_to(...).await?;` propagation with
    // no log left a partial enter-world bundle as a silent player-
    // visible bug (invisible NPCs, missing appearance, stuck-on-load).
    // The handler now emits an ERROR per failing fragment with
    // fragment_idx + total + body_len. The guard pins:
    //   1. Result is Err (preserved propagation).
    //   2. ERROR fires naming "fragment send failed".
    //   3. fragment_idx field carries the failing fragment index (1).
    //   4. No fragments past the failing one were attempted.
    // ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn map_loaded_fragment_send_failure_errors_and_logs() {
        use crate::test_support::LogCapture;
        use async_trait::async_trait;
        use cimmeria_mercury::transport::Transport as TransportTrait;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tracing::Level;

        /// Succeed for `fail_after` sends, then fail every subsequent
        /// send_to. Mirrors the FailingTransport pattern in
        /// `cell_dispatch/tests.rs`.
        struct FailAfter {
            sent: AtomicUsize,
            fail_after: usize,
            local: SocketAddr,
        }

        #[async_trait]
        impl TransportTrait for FailAfter {
            async fn send_to(&self, bytes: &[u8], _addr: SocketAddr) -> std::io::Result<usize> {
                let n = self.sent.fetch_add(1, Ordering::SeqCst);
                if n < self.fail_after {
                    Ok(bytes.len())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "synthetic-test-failure",
                    ))
                }
            }
            fn local_addr(&self) -> std::io::Result<SocketAddr> {
                Ok(self.local)
            }
        }

        let capture = LogCapture::install();

        // Succeed the standalone enter-world send (index 0), fail every
        // fragment send after that.
        let typed = Arc::new(FailAfter {
            sent: AtomicUsize::new(0),
            fail_after: 1,
            local: "127.0.0.1:0".parse().unwrap(),
        });
        let transport: Arc<dyn Transport> = typed.clone();

        let addr: SocketAddr = "127.0.0.1:55502".parse().unwrap();
        let mut client = crate::test_support::test_default_connected_client_state();
        client.pending_map_loaded = Some(crate::mercury::types::WorldEntryInfo {
            player_entity_id: 7777,
            space_id: 0x0001_0042,
            pos: [0.0; 3],
            rot: [0.0; 3],
            world_name: "Agnos".to_string(),
            class_id: 2,
            world_stargates: Vec::new(),
        });
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new({
                let mut m = HashMap::new();
                m.insert(addr, client);
                m
            }));
        let key = [0u8; 32];

        let result = handle_map_loaded(
            &transport,
            addr,
            key,
            &connected,
            &None,
            &Arc::new(Mutex::new(HashMap::new())),
            &None,
        )
        .await;

        assert!(
            result.is_err(),
            "fragment send failure must propagate as Err"
        );
        let frag_err = capture
            .find_message(Level::ERROR, "map_loaded: fragment send failed")
            .expect(
                "negative-logging convention: per-fragment ERROR must fire — captured: see all()",
            );
        // Sanity-check the structured fields carry the failing fragment
        // identity (idx + total) so an ops grep can pin down which
        // fragment was lost.
        assert!(
            frag_err.has_field("fragment_idx", "1"),
            "fragment_idx field must point at the failing fragment (1-indexed): {:#?}",
            frag_err
        );
        // Send-attempt count: 1 (standalone success) + 1 (first
        // fragment failure) — no fragments past the failing one were
        // attempted. Same abort-on-first-failure shape as the bundle
        // guard in `cell_dispatch/tests.rs`.
        assert_eq!(
            typed.sent.load(Ordering::SeqCst),
            2,
            "abort-on-first-failure: standalone enter-world + 1 fragment = 2 sends"
        );
    }
}
