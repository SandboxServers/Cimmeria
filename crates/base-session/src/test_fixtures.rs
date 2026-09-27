//! Test fixtures for the session types, behind the `test-support` feature so
//! the higher crates' tests can build them too (cimmeria-services enables it in
//! its [dev-dependencies]). Not for production paths.

// ── ConnectedClientState fixture ──────────────────────────────────────
//
// `ConnectedClientState` is built up across the Phase 3 handshake
// (login.rs) and there is no production constructor for "an empty one"
// — every field has to be threaded through. Several unit tests just
// want a structurally-valid placeholder so they can populate the one
// or two fields the test actually cares about. Centralise that here.

use crate::base::ConnectedClientState;
use cimmeria_mercury::encryption::MercuryEncryption;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Build a `ConnectedClientState` with all fields zeroed/None and
/// fresh `Arc`s — a structurally-valid placeholder for tests that
/// only mutate a couple of fields. Not for production paths.
pub fn test_default_connected_client_state() -> ConnectedClientState {
    let key = [0u8; 32];
    ConnectedClientState {
        enc: MercuryEncryption::from_session_key(key),
        key,
        enc_version: cimmeria_mercury::encryption::EncryptionVersion::V1,
        account_id: 0,
        account_name: None,
        access_level: 0,
        dnd_message: None,
        char_list_sent: false,
        world_entry_sent: false,
        pending_player_entity_id: None,
        player_entity_id: None,
        next_seq: Arc::new(AtomicU32::new(0)),
        next_seq_unreliable: Arc::new(AtomicU32::new(0)),
        pending_acks: Arc::new(Mutex::new(Vec::new())),
        last_recv: Arc::new(Mutex::new(Instant::now())),
        connected_at: Instant::now(),
        account_entity_id: 0,
        next_data_id: 0,
        pending_world_entry: None,
        pending_player_load_data: None,
        pending_map_loaded: None,
        pending_client_ready: None,
        deferred_aoi_msgs: Vec::new(),
        cached_appearance_args: None,
        cached_tint_args: None,
        weapon_holstered: true,
        cancelled: Arc::new(AtomicBool::new(false)),
        cinematic_spam_cancel: Arc::new(AtomicBool::new(false)),
        cinematic_aoi_hold: None,
        listed_online: false,
        rate_limits: Default::default(),
        player_name: None,
        player_level: None,
        player_archetype: None,
        player_alignment: None,
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
