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

use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::mercury::compose_forced_position_body;

use super::super::helpers::send_bundle_to_witness_reliable;
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
#[tracing::instrument(
    name = "world_entry.teleport_player",
    level = "info",
    skip_all,
    fields(entity_id, space_id)
)]
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

    // Bundle the engine-level snap + streaming-load hint into one frame.
    //
    // **Transaction-state audit**: both messages target the player's own
    // entity, long since created. `FORCED_POSITION` (0x31) is a property-
    // update on an already-live entity — NOT a creation — so it doesn't
    // enter a CREATE_ENTITY transaction. Same-entity `onPlayerTeleport`
    // (method 116) following it in the same bundle binds to the already-
    // live entity and is NOT HOLD-FOR-TRANSACTION dropped. See
    // [docs/architecture/mercury-bundle.md] safe-combine catalogue.
    //
    // Pre-bundle: 2 reliable packets (FORCED_POSITION snap + onPlayerTeleport
    // hint). Post-bundle: 1 reliable packet (~70 B body, well under
    // FRAGMENT_BODY_SIZE=1300). Pinned by
    // `teleport_bundles_forced_position_and_player_teleport_to_single_packet`.
    let bundle = build_teleport_bundle(entity_id, space_id, position, prev_pos);
    send_bundle_to_witness_reliable(transport, connected, entity_to_addr, entity_id, bundle).await;

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

/// Compose the teleport handshake into a single Mercury bundle.
///
/// Order matches the pre-bundle dispatch sequence:
///   1. `FORCED_POSITION (0x31)` — engine-level snap; the avatar moves here.
///   2. `onPlayerTeleport (method 116)` — streaming-load waiting flag;
///      kicks the client into terrain-chunk loading at the new position.
///
/// `FORCED_POSITION` is appended via [`ChannelBundle::append_raw_message`]
/// because it's a Mercury base message (0x31), not an entity-method call.
/// The 50-byte body is composed by [`compose_forced_position_body`].
///
/// Extracted as a pure builder so the burst-shape regression guard
/// [`tests::teleport_bundles_forced_position_and_player_teleport_to_single_packet`]
/// pins the same composition the handler actually emits.
fn build_teleport_bundle(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    prev_pos: [f32; 3],
) -> ChannelBundle {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_raw_message(&compose_forced_position_body(
        entity_id, space_id, position, prev_pos,
    ));
    // Direction is zeroed — we don't currently rotate the avatar on ring
    // travel. The wire args are 24 bytes: 12 for position, 12 for the
    // zero direction.
    let mut args = Vec::with_capacity(24);
    for &c in &position {
        args.extend_from_slice(&c.to_le_bytes());
    }
    args.extend_from_slice(&[0u8; 12]);
    bundle.append_entity_method(
        crate::cell::client_methods::player::ON_PLAYER_TELEPORT,
        IDBASE_SGW_PLAYER,
        entity_id,
        &args,
    );
    bundle
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
    /// player by emitting **one bundled packet** to the player's own addr —
    /// FORCED_POSITION + onPlayerTeleport concatenated into a single Mercury
    /// frame — with **zero** witness fan-out (teleport is owner-only).
    ///
    /// Pre-bundle this fired 2 reliable packets; after the issue #360
    /// migration the same two records land in one fragment. The fan-out
    /// shape (owner-only routing) and message ordering inside the bundle
    /// are preserved.
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
            1,
            "post-bundle: FORCED_POSITION + onPlayerTeleport ride one Mercury \
             frame to the owner addr (was 2 packets pre-bundle)"
        );
        assert_eq!(
            sent[0].0, player_addr,
            "bundled teleport handshake goes to the player's own addr only"
        );

        // The bundled packet's decrypted body must equal the concatenation of
        // the standalone FORCED_POSITION body and the standalone
        // onPlayerTeleport entity-method body — the two records the client
        // processes after fragment reassembly. Decrypting the full packet
        // and inspecting body bytes lets the test fire on any wire-format
        // drift inside either composer.
        use cimmeria_mercury::encryption::MercuryEncryption;
        let key = [0u8; 32];
        let enc = MercuryEncryption::from_session_key(key);
        let pt = enc.decrypt(&sent[0].1).expect("decrypt bundled packet");
        // pt[0] = flags byte; body starts at pt[1]; suffix is the seq footer
        // (4 bytes for FLAG_HAS_SEQUENCE-bearing packets).
        let bundled_body = &pt[1..pt.len() - 4];

        let expected_forced = compose_forced_position_body(entity_id, space_id, position, prev_pos);
        let mut expected_method = Vec::new();
        let mut args = Vec::with_capacity(24);
        for &c in &position {
            args.extend_from_slice(&c.to_le_bytes());
        }
        args.extend_from_slice(&[0u8; 12]);
        crate::mercury::append_entity_method(
            &mut expected_method,
            crate::cell::client_methods::player::ON_PLAYER_TELEPORT,
            IDBASE_SGW_PLAYER,
            entity_id,
            &args,
        );
        let mut expected_body = Vec::new();
        expected_body.extend_from_slice(&expected_forced);
        expected_body.extend_from_slice(&expected_method);

        assert_eq!(
            bundled_body,
            expected_body.as_slice(),
            "bundled body must equal FORCED_POSITION body || onPlayerTeleport body \
             — a regression here means either compose_forced_position_body or \
             append_entity_method drifted on the bundle path"
        );
    }

    /// Burst-shape regression guard for the issue #360 teleport bundle
    /// migration. Pin two invariants the migration depends on:
    ///   - `num_messages == 2` — exactly FORCED_POSITION + onPlayerTeleport.
    ///   - `estimated_packet_count() == 1` — both messages comfortably fit
    ///     one fragment (~70 B total body). A regression that grows either
    ///     composer past the fragment cutoff fires here.
    #[test]
    fn teleport_bundles_forced_position_and_player_teleport_to_single_packet() {
        let bundle = build_teleport_bundle(0xDEAD, 5, [10.0, 20.0, 30.0], [1.0, 2.0, 3.0]);
        assert_eq!(
            bundle.num_messages(),
            2,
            "teleport handshake must contain exactly FORCED_POSITION + onPlayerTeleport"
        );
        assert_eq!(
            bundle.estimated_packet_count(),
            1,
            "teleport handshake bundle must collapse to 1 reliable packet \
             (was 2 pre-bundle)"
        );
    }
}
