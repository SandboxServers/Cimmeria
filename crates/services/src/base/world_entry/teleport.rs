//! Authoritative same-world teleport handler.
//!
//! Snaps the player's avatar to a new position via `FORCED_POSITION`,
//! coordinates streaming chunk loading via `onPlayerTeleport`, and persists
//! the new position so a relog mid-ceremony doesn't snap back.
//!
//! Extracted from `cell_dispatch.rs` to keep the dispatcher focused on
//! routing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::mercury::{build_entity_method_packet, build_forced_position};

use super::super::helpers::send_to_witness_reliable;
use super::super::ConnectedClientState;

/// Authoritative same-world teleport: snap the player's avatar to `position`.
///
/// Sends three things, in order:
/// 1. `FORCED_POSITION` (0x31) — the engine-level snap. Without this the
///    avatar does not move (the client keeps sending `AVATAR_UPDATE_EXPLICIT`
///    from the source pad). See `build_forced_position` for wire details.
/// 2. `onPlayerTeleport` (method 116) — flags the client into streaming-load
///    waiting state with the new position so terrain chunks load cleanly.
///    See SGWPlayer.def's comment on this method.
/// 3. Persist new pos to `sgw_player` so a relog mid-ceremony doesn't
///    teleport the player back to the source pad. We fail closed on missing
///    `active_player_id` for the same reason as `gate_travel.rs`.
pub(super) async fn handle_teleport_player(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    prev_pos: [f32; 3],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    // The cell owns authoritative `space_id` and passes it through. We only
    // need the connection state for account_id/active_player_id (DB persist).
    let (account_id, active_player_id) = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => {
                tracing::warn!(entity_id, "TeleportPlayer: no client addr for entity");
                return;
            }
        };
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => (c.account_id, c.active_player_id),
            None => {
                tracing::warn!(entity_id, %addr, "TeleportPlayer: client state not found");
                return;
            }
        }
    };

    tracing::info!(
        entity_id,
        ?position,
        space_id,
        "TeleportPlayer: snapping avatar"
    );

    // 1. Engine-level snap. `prev_pos` is the entity's last-known position
    //    before the teleport — see `build_forced_position` and
    //    `spec.protocol.mercury` §1.10.6 for why this is not zero.
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_forced_position(key, seq, acks, entity_id, space_id, position, prev_pos)
        },
    )
    .await;

    // 2. Streaming-load waiting flag (method 116). Direction is zeroed —
    //    we don't currently rotate the avatar on ring travel.
    let mut args = Vec::with_capacity(24);
    for &c in &position {
        args.extend_from_slice(&c.to_le_bytes());
    }
    args.extend_from_slice(&[0u8; 12]); // direction = 0,0,0
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                crate::cell::client_methods::player::ON_PLAYER_TELEPORT,
                &args,
            )
        },
    )
    .await;

    // 3. Persist. Mirrors gate_travel's fail-closed on missing active_player_id.
    if let Some(pool) = db_pool {
        let pid = match active_player_id {
            Some(p) => p,
            None => {
                tracing::error!(
                    entity_id,
                    account_id,
                    "TeleportPlayer: no active_player_id cached — refusing to persist"
                );
                return;
            }
        };
        let res = sqlx::query(
            "UPDATE sgw_player SET pos_x = $1, pos_y = $2, pos_z = $3 \
             WHERE player_id = $4 AND account_id = $5",
        )
        .bind(position[0])
        .bind(position[1])
        .bind(position[2])
        .bind(pid)
        .bind(account_id as i32)
        .execute(pool.as_ref())
        .await;
        match res {
            Ok(r) if r.rows_affected() == 0 => {
                tracing::warn!(
                    entity_id,
                    pid,
                    account_id,
                    "TeleportPlayer: persistence UPDATE matched 0 rows"
                );
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(entity_id, pid, account_id, error = %e,
                    "TeleportPlayer: failed to persist position");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestTransport;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn teleport_early_returns_when_entity_not_in_addr_map() {
        // Typed handle for the no-send assertion; dyn handle for the call.
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));

        handle_teleport_player(
            999,
            65536,
            [10.0, 20.0, 30.0],
            [5.0, 20.0, 30.0],
            &dyn_transport,
            &connected,
            &entity_to_addr,
            &None,
        )
        .await;
        assert!(entity_to_addr.lock().unwrap().is_empty());
        assert!(connected.lock().unwrap().is_empty());
        assert!(transport.is_empty(), "early return must not send UDP");
    }

    #[tokio::test]
    async fn teleport_early_returns_when_client_state_missing() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(1, fake_addr);
            m
        }));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));

        handle_teleport_player(
            1,
            65536,
            [10.0, 20.0, 30.0],
            [5.0, 20.0, 30.0],
            &dyn_transport,
            &connected,
            &entity_to_addr,
            &None,
        )
        .await;
        assert_eq!(entity_to_addr.lock().unwrap().get(&1), Some(&fake_addr));
        assert!(connected.lock().unwrap().is_empty());
        assert!(transport.is_empty(), "early return must not send UDP");
    }

    /// Domain B (fan-out byte test): a valid same-world teleport snaps the
    /// player by emitting exactly two packets to the player's own addr, in
    /// order — `FORCED_POSITION` (seq 0) then `onPlayerTeleport` (seq 1) —
    /// byte-exact, with **zero** witness fan-out (teleport is owner-only).
    /// Catches a regression that drops the engine-level snap, reorders the
    /// burst, or leaks the snap to other clients.
    #[tokio::test]
    async fn teleport_emits_forced_position_then_player_teleport_to_owner_only() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();

        let entity_id = 0x1234u32;
        let space_id = 5u32;
        let position = [10.0f32, 20.0, 30.0];
        let prev_pos = [1.0f32, 2.0, 3.0];
        let player_addr: SocketAddr = "127.0.0.1:40100".parse().unwrap();

        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::from([(entity_id, player_addr)])));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::from([(
                player_addr,
                crate::test_support::test_default_connected_client_state(),
            )])));

        // db_pool = None → no persistence, no extra emit.
        handle_teleport_player(
            entity_id,
            space_id,
            position,
            prev_pos,
            &dyn_transport,
            &connected,
            &entity_to_addr,
            &None,
        )
        .await;

        let sent = transport.drain();
        assert_eq!(
            sent.len(),
            2,
            "exactly FORCED_POSITION + onPlayerTeleport, no witness fan-out"
        );
        assert_eq!(sent[0].0, player_addr, "snap goes to the player's own addr");
        assert_eq!(
            sent[1].0, player_addr,
            "teleport hint goes to the same addr"
        );

        // test_default_connected_client_state starts next_seq at 0, key all-zero.
        let key = [0u8; 32];
        assert_eq!(
            sent[0].1,
            build_forced_position(&key, 0, &[], entity_id, space_id, position, prev_pos),
            "FORCED_POSITION wire bytes (seq 0)"
        );
        let mut args = Vec::with_capacity(24);
        for &c in &position {
            args.extend_from_slice(&c.to_le_bytes());
        }
        args.extend_from_slice(&[0u8; 12]); // direction = 0,0,0
        assert_eq!(
            sent[1].1,
            build_entity_method_packet(
                &key,
                1,
                &[],
                entity_id,
                crate::cell::client_methods::player::ON_PLAYER_TELEPORT,
                &args,
            ),
            "onPlayerTeleport wire bytes (seq 1)"
        );
    }
}
