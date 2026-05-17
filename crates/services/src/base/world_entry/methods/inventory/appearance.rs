//! Player appearance refresh — re-runs the player-load appearance assembly
//! and re-broadcasts `BEING_APPEARANCE` to witnesses.
//!
//! Used whenever the *visible* component set on the player changes without
//! a fresh login: equipped item granted, bandolier active slot changed,
//! item moved into/out of an equipment slot. The appearance query at
//! [`super::super::player_load::core`] filters bandolier visual components
//! by the persisted `bandolier_slot` column, so the right value just has
//! to be in the DB by the time this helper runs — callers must commit any
//! `bandolier_slot` mutation before invoking.
//!
//! Today this loads the full player profile (abilities, missions, weapon
//! stats, …) just to grab `bodyset` + `components`. The same TODO that's
//! lived on the inline copy in `grant.rs` applies here: a narrow
//! `query_player_appearance_data` would be a real optimisation. Keeping
//! the wide query for now so behaviour is byte-identical to the grant
//! path.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use super::super::player_load::core::query_player_load_data;
use crate::base::{helpers, world_entry_appearance, ConnectedClientState};
use crate::mercury::{build_entity_method_packet, method_idx};

/// Re-run the player's appearance assembly and broadcast `BEING_APPEARANCE`.
///
/// Returns silently when the entity has no connected client (e.g. logging
/// out, not yet connected) — the assembly will run again on the next
/// `world_entry` and the witness packet would have nowhere to land in the
/// meantime.
pub async fn refresh_player_appearance(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let account_id = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => {
                tracing::debug!(
                    entity_id,
                    player_id,
                    "refresh_player_appearance: entity has no socket addr (disconnected?), skipping"
                );
                return;
            }
        };
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => c.account_id,
            None => {
                tracing::debug!(
                    entity_id,
                    player_id,
                    "refresh_player_appearance: client state missing for addr, skipping"
                );
                return;
            }
        }
    };

    let player_data = query_player_load_data(db_pool, account_id, player_id).await;
    let appearance_args = world_entry_appearance::build_appearance_args(
        &player_data.bodyset,
        &player_data.components,
    );

    // Cache the freshly-built args so any subsequent witness-join (entering
    // AoI, world_entry replay) ships the same appearance the existing
    // witnesses just received — without this the cached entry can lag
    // behind the broadcast and a player who fires a new witness ends up
    // seeing the pre-refresh weapon visible on the model.
    {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => return,
        };
        let mut clients = connected.lock().unwrap();
        if let Some(c) = clients.get_mut(&addr) {
            c.cached_appearance_args = Some(appearance_args.clone());
        }
    }

    helpers::send_to_witness_reliable(
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
                method_idx::BEING_APPEARANCE,
                &appearance_args,
            )
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_mercury::encryption::MercuryEncryption;
    use std::sync::atomic::{AtomicBool, AtomicU32};
    use std::time::Instant;

    fn make_connected_state(account_id: u32) -> ConnectedClientState {
        ConnectedClientState {
            enc: MercuryEncryption::from_session_key([0u8; 32]),
            key: [0u8; 32],
            account_id,
            access_level: 0,
            char_list_sent: false,
            world_entry_sent: false,
            pending_player_entity_id: None,
            player_entity_id: None,
            next_seq: Arc::new(AtomicU32::new(1)),
            pending_acks: Arc::new(Mutex::new(Vec::new())),
            last_recv: Arc::new(Mutex::new(Instant::now())),
            account_entity_id: 1,
            next_data_id: 0,
            pending_world_entry: None,
            pending_player_load_data: None,
            pending_map_loaded: None,
            pending_client_ready: None,
            cached_appearance_args: None,
            cached_tint_args: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            cinematic_spam_cancel: Arc::new(AtomicBool::new(false)),
            player_name: None,
            player_level: None,
            player_archetype: None,
            world_name: None,
            player_xp: None,
            player_training_points: None,
            active_player_id: None,
            pending_destination_ring_id: None,
            channel: Mutex::new(cimmeria_mercury::channel::Channel::new(
                "127.0.0.1:9999".parse().unwrap(),
            )),
        }
    }

    async fn make_socket() -> Arc<UdpSocket> {
        Arc::new(UdpSocket::bind("127.0.0.1:0").await.expect("bind UDP"))
    }

    /// Regression for the "F1-F4 changes the active slot but the player
    /// keeps holding the old weapon" bug: the appearance refresh path
    /// MUST populate `cached_appearance_args` on the connected client
    /// state so that the next AoI re-broadcast (and any subsequent
    /// world_entry replay) ships the new appearance.
    ///
    /// Runs against `db_pool = None` so it doesn't need DATABASE_URL —
    /// `query_player_load_data` returns the default-sentinel payload in
    /// that mode, which is enough to exercise the cache-set path.
    #[tokio::test]
    async fn refresh_player_appearance_populates_cached_args() {
        let socket = make_socket().await;
        let addr: SocketAddr = "127.0.0.1:55700".parse().unwrap();

        let entity_to_addr = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(42u32, addr);
            m
        }));
        let connected = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(addr, make_connected_state(0x1234));
            m
        }));

        // Pre-condition: cached args are empty so the assertion below is
        // genuinely checking the helper wrote them.
        {
            let clients = connected.lock().unwrap();
            assert!(
                clients.get(&addr).unwrap().cached_appearance_args.is_none(),
                "test setup invariant: cache must start empty",
            );
        }

        refresh_player_appearance(42, 100, &None, &socket, &connected, &entity_to_addr).await;

        let clients = connected.lock().unwrap();
        let state = clients.get(&addr).expect("connected state must exist");
        assert!(
            state.cached_appearance_args.is_some(),
            "refresh_player_appearance must populate cached_appearance_args; \
             without this, a later world_entry/AoI replay would still ship \
             the pre-refresh weapon visual",
        );
    }

    /// When the entity has no addr mapping (already disconnected, never
    /// connected), the helper must return cleanly without panicking and
    /// without modifying any shared state. This is the realistic shape
    /// of a logout race: the cell sends `ActiveSlotUpdate`, the addr
    /// is already torn down by the time the base processes it.
    #[tokio::test]
    async fn refresh_player_appearance_no_addr_is_noop() {
        let socket = make_socket().await;
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Should not panic; should not insert anything into `connected`.
        refresh_player_appearance(42, 100, &None, &socket, &connected, &entity_to_addr).await;

        assert!(
            connected.lock().unwrap().is_empty(),
            "no-addr path must not synthesize a client state entry",
        );
    }
}
