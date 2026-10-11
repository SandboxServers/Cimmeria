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

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::player_load::core::query_player_load_data;
use crate::base::session_identity::{identity_for_addr, identity_for_entity};
use crate::base::{helpers, world_entry_appearance, ConnectedClientState};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// How long a refresh waits before its one retry of a failed player load
/// (D-TG11). Long enough to ride out a pool hiccup, short enough that the
/// player still sees the gear change land promptly when the retry succeeds.
const LOAD_RETRY_DELAY: Duration = Duration::from_millis(200);

/// Re-run the player's appearance assembly and broadcast `BEING_APPEARANCE`.
///
/// Returns without sending when the entity has no connected client (e.g.
/// logging out, not yet connected) — the assembly will run again on the next
/// `world_entry` and the witness packet would have nowhere to land in the
/// meantime. Also returns without sending or caching when the player load
/// fails twice (TG-ITM-14), so a database error never replaces the player's
/// model with the default body.
pub async fn refresh_player_appearance(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    // Each lock is taken in its own statement so the guard is released
    // before `log_skip` (which takes the same locks) runs.
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied();
    let Some(addr) = addr else {
        log_skip(
            connected,
            entity_to_addr,
            entity_id,
            player_id,
            "entity_to_addr_miss",
        );
        return;
    };
    let client = connected
        .lock()
        .unwrap()
        .get(&addr)
        .map(|c| (c.account_id, c.weapon_holstered));
    let Some((account_id, holstered)) = client else {
        log_skip(
            connected,
            entity_to_addr,
            entity_id,
            player_id,
            "client_state_missing",
        );
        return;
    };

    let mut player_data = query_player_load_data(db_pool, account_id, player_id).await;
    // The loader never fails outright: on any error it returns the default
    // payload (player_id 0, the "naked human male" body). With a live pool
    // that payload is a failed load, not an appearance, and sending it would
    // strip the player's model on every client in range. D-TG11: retry once
    // after a short delay, then give up and leave the last good appearance in
    // place — on the witnesses' screens and in `cached_appearance_args`.
    if db_pool.is_some() && player_data.player_id != player_id {
        tokio::time::sleep(LOAD_RETRY_DELAY).await;
        player_data = query_player_load_data(db_pool, account_id, player_id).await;
        if player_data.player_id != player_id {
            let identity = identity_for_addr(connected, addr);
            // The load error itself is on the loader's own ERROR
            // `player_load_degraded` row for each attempt; this row says what
            // the refresh did about it.
            tracing::warn!(
                event = "appearance_refresh_aborted",
                cause = "player_load_failed",
                attempts = 2u32,
                entity_id,
                // The player's own entity: its name is the player's.
                entity_name = known_names::player_name(player_id),
                account_id = identity.account_id,
                account_name = identity.account_name,
                player_id,
                player_name = known_names::player_name(player_id),
                "refresh_player_appearance: player load failed twice, keeping the last appearance"
            );
            return;
        }
    }
    let degraded = player_data.player_id != player_id;
    if degraded {
        let identity = identity_for_addr(connected, addr);
        // Only offline mode (`db_pool = None`) gets here: the default payload
        // is the only appearance there is, and world entry sent the same one,
        // so it is still sent. The loader already logged its own DEBUG
        // `no_db_pool` row.
        tracing::debug!(
            event = "appearance_refresh_degraded",
            cause = "player_load_failed",
            entity_id,
            // The player's own entity: its name is the player's.
            entity_name = known_names::player_name(player_id),
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name = known_names::player_name(player_id),
            "refresh_player_appearance: offline, sending default appearance"
        );
    }
    // Honors the live `weapon_holstered` mirror from `ConnectedClientState`,
    // which is set in lockstep with `CellEntity::weapon_holstered` on the
    // cell side. Inventory moves don't change holster state — they re-emit
    // appearance with whatever holster the player currently has, so a
    // mid-combat equip doesn't accidentally re-holster the weapon.
    let components = player_data.appearance_components(holstered);
    let appearance_args =
        world_entry_appearance::build_appearance_args(&player_data.bodyset, &components);

    // Cache the freshly-built args so any subsequent witness-join (entering
    // AoI, world_entry replay) ships the same appearance the existing
    // witnesses just received — without this the cached entry can lag
    // behind the broadcast and a player who fires a new witness ends up
    // seeing the pre-refresh weapon visible on the model.
    {
        let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied();
        let Some(addr) = addr else {
            log_skip(
                connected,
                entity_to_addr,
                entity_id,
                player_id,
                "entity_missing",
            );
            return;
        };
        let mut clients = connected.lock().unwrap();
        if let Some(c) = clients.get_mut(&addr) {
            c.cached_appearance_args = Some(appearance_args.clone());
        }
    }

    helpers::send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::BEING_APPEARANCE,
                &appearance_args,
                version,
            )
        },
    )
    .await;

    // The send above reaches only the player's own client. Everyone else who
    // can see this player learns about the new gear / drawn weapon here;
    // players who arrive later get it from `cached_appearance_args` via the
    // player-ghost cascade.
    helpers::broadcast_to_witnesses(
        cell_tx,
        entity_id,
        method_idx::BEING_APPEARANCE,
        appearance_args,
    )
    .await;

    tracing::debug!(
        event = "appearance_refreshed",
        entity_id,
        entity_name = known_names::player_name(player_id),
        account_id,
        account_name = known_names::account_name(account_id),
        player_id,
        player_name = known_names::player_name(player_id),
        holstered,
        component_count = components.len(),
        bodyset = player_data.bodyset.as_str(),
        degraded,
        "refresh_player_appearance: appearance refreshed"
    );
}

/// Log an appearance refresh that stopped before the broadcast. An entity
/// whose witness session ended moments ago is a normal race (DEBUG); any
/// other miss is an expectation failure (WARN). Takes the `connected` and
/// `entity_to_addr` locks, so the caller must hold neither.
fn log_skip(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_id: u32,
    player_id: i32,
    reason: &'static str,
) {
    let identity = identity_for_entity(connected, entity_to_addr, entity_id);
    let player_name = known_names::player_name(player_id);
    if helpers::witness_recently_departed(entity_id) {
        tracing::debug!(
            event = "appearance_refresh_skipped",
            reason = "witness_session_ended",
            entity_id,
            entity_name = identity.player_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name,
            "refresh_player_appearance: skipped"
        );
    } else {
        tracing::warn!(
            event = "appearance_refresh_skipped",
            reason,
            entity_id,
            entity_name = identity.player_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name,
            "refresh_player_appearance: skipped"
        );
    }
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
