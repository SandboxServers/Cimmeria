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

use sqlx::PgPool;
use tokio::net::UdpSocket;

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
    socket: &Arc<UdpSocket>,
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

    // 1. Engine-level snap.
    send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| build_forced_position(key, seq, acks, entity_id, space_id, position),
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
        socket,
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
    use std::collections::HashMap;
    use std::io::ErrorKind;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use tokio::net::UdpSocket;

    fn assert_no_udp_packet(receiver: &UdpSocket) {
        let mut buf = [0u8; 2048];
        let err = receiver
            .try_recv_from(&mut buf)
            .expect_err("early return must not send UDP");
        assert_eq!(err.kind(), ErrorKind::WouldBlock);
    }

    #[tokio::test]
    async fn teleport_early_returns_when_entity_not_in_addr_map() {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));

        handle_teleport_player(
            999,
            65536,
            [10.0, 20.0, 30.0],
            &socket,
            &connected,
            &entity_to_addr,
            &None,
        )
        .await;
        assert!(entity_to_addr.lock().unwrap().is_empty());
        assert!(connected.lock().unwrap().is_empty());
        assert_no_udp_packet(&receiver);
    }

    #[tokio::test]
    async fn teleport_early_returns_when_client_state_missing() {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let fake_addr = receiver.local_addr().unwrap();
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
            &socket,
            &connected,
            &entity_to_addr,
            &None,
        )
        .await;
        assert_eq!(entity_to_addr.lock().unwrap().get(&1), Some(&fake_addr));
        assert!(connected.lock().unwrap().is_empty());
        assert_no_udp_packet(&receiver);
    }
}
