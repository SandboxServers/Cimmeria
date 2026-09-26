//! Test helpers for this crate's tests.
//!
//! The generic helpers live in `cimmeria-test-support` and are re-exported
//! here, so tests keep importing them from `crate::test_support`:
//!
//! - **Live-DB**: [`require_db_or_skip!`] opens a pool against
//!   `DATABASE_URL`. It skips when the variable is unset and **fails** when it
//!   is set but unreachable (#615). See
//!   `docs/architecture/integration-test-infra.md`.
//! - **Log capture**: [`LogCapture`] for negative-logging regression guards.
//! - **Transport fake**: [`TestTransport`], the recording UDP fake behind the
//!   **fan-out byte test** type in `TESTING.md`. See
//!   `docs/architecture/transport-trait.md`.
//!
//! The domain fixtures below (`make_space_manager*`, `seed_ability_defs`,
//! `test_default_connected_client_state`) stay here, next to the types they
//! build. See `docs/architecture/services-crate-split.md` §3.

pub(crate) use cimmeria_test_support::*;

// ── SpaceManager test fixtures ────────────────────────────────────────

use crate::cell::space_manager::SpaceManager;

/// Standard test space setup: SpaceManager with a single Agnos space.
///
/// The same ~5-line block was duplicated across dispatch, interaction,
/// and vendor test modules. Extracted here so every cell test shares
/// the same default world geometry.
pub(crate) fn make_space_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    mgr
}

/// Same as [`make_space_manager`], but also creates a player entity at
/// the origin so tests that need an avatar don't repeat the entity
/// creation boilerplate.
pub(crate) fn make_space_manager_with_player(entity_id: u32) -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(entity_id, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr
}

/// Register minimal `AbilityDef`s so the trainability predicate's
/// "ability exists" gate passes for these ids.
pub(crate) fn seed_ability_defs(mgr: &mut SpaceManager, ability_ids: &[i32]) {
    for &ability_id in ability_ids {
        mgr.ability_defs.insert(
            ability_id,
            cimmeria_entity::abilities::AbilityDef {
                ability_id,
                name: format!("TestAbility{ability_id}"),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
    }
}

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
pub(crate) fn test_default_connected_client_state() -> ConnectedClientState {
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
