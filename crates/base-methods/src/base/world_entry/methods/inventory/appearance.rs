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

/// A first failed load slower than this is a hung database, not a hiccup,
/// so the refresh aborts without retrying. The bandolier and equip callers
/// await the refresh on the base's single `cell_rx` loop, so every second
/// spent here is a second every player's cell-to-base traffic waits; a
/// retry against a hung pool would add up to another acquire timeout (30 s).
const FAST_FAILURE_LIMIT: Duration = Duration::from_secs(1);

/// Upper bound on the one retry's load, for the same reason: the cell queue
/// waits behind it.
const RETRY_LOAD_TIMEOUT: Duration = Duration::from_secs(2);

/// What [`load_with_bounded_retry`] got.
#[derive(Debug)]
enum BoundedLoad<T> {
    Loaded(T),
    Aborted {
        attempts: u32,
        /// Total time spent, both attempts and the retry delay included.
        elapsed_ms: u64,
        /// Why the retry did not produce a load: `skipped_slow_failure`
        /// (no retry), `failed` or `timed_out`.
        retry: &'static str,
    },
}

/// Run `load`, and retry it once (D-TG11) only when the first attempt failed
/// fast; the retry itself is capped at [`RETRY_LOAD_TIMEOUT`]. `loaded` says
/// whether a result is a real load. Generic over the loader so the retry
/// bounds can be tested without a database.
async fn load_with_bounded_retry<T, F, Fut>(
    mut load: F,
    loaded: impl Fn(&T) -> bool,
) -> BoundedLoad<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    // tokio's clock, so paused-time tests drive it.
    let started = tokio::time::Instant::now();
    let first = load().await;
    if loaded(&first) {
        return BoundedLoad::Loaded(first);
    }
    let elapsed_ms =
        |s: tokio::time::Instant| u64::try_from(s.elapsed().as_millis()).unwrap_or(u64::MAX);
    if started.elapsed() >= FAST_FAILURE_LIMIT {
        return BoundedLoad::Aborted {
            attempts: 1,
            elapsed_ms: elapsed_ms(started),
            retry: "skipped_slow_failure",
        };
    }
    tokio::time::sleep(LOAD_RETRY_DELAY).await;
    let retry = match tokio::time::timeout(RETRY_LOAD_TIMEOUT, load()).await {
        Ok(second) if loaded(&second) => return BoundedLoad::Loaded(second),
        Ok(_) => "failed",
        Err(_) => "timed_out",
    };
    BoundedLoad::Aborted {
        attempts: 2,
        elapsed_ms: elapsed_ms(started),
        retry,
    }
}

/// Re-run the player's appearance assembly and broadcast `BEING_APPEARANCE`.
///
/// Returns without sending when the entity has no connected client (e.g.
/// logging out, not yet connected) — the assembly will run again on the next
/// `world_entry` and the witness packet would have nowhere to land in the
/// meantime. Also returns without sending or caching when the player load
/// fails (TG-ITM-14), so a database error never replaces the player's model
/// with the default body. The retry is bounded (see [`FAST_FAILURE_LIMIT`]
/// and [`RETRY_LOAD_TIMEOUT`]) because callers await this on the base's
/// shared cell-to-base loop.
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

    // The loader never fails outright: on any error it returns the default
    // payload (player_id 0, the "naked human male" body). With a live pool
    // that payload is a failed load, not an appearance, and sending it would
    // strip the player's model on every client in range. D-TG11: retry once
    // after a short delay, then give up and leave the last good appearance in
    // place — on the witnesses' screens and in `cached_appearance_args`.
    //
    // Known gaps (TG-ITM-14):
    // - Only a failed *base row* load comes back as the default payload. When
    //   the equipment-visuals or weapon-visual sub-query fails, the loader
    //   returns the real `player_id` with those visuals stripped
    //   (`player_data.rs`, `equipment_visuals_query_failed` /
    //   `weapon_visual_query_failed`), which this check cannot tell from a
    //   good load, so that partial appearance is still cached and sent.
    // - The caller may already have changed state the aborted refresh was
    //   meant to show: `RefreshAppearance` sets the `weapon_holstered` mirror
    //   before calling here (`cell_dispatch/bandolier.rs`), so after an abort
    //   the mirror can disagree with what the clients show until the next
    //   successful refresh.
    let player_data = if db_pool.is_some() {
        let outcome = load_with_bounded_retry(
            || query_player_load_data(db_pool, account_id, player_id),
            |d| d.player_id == player_id,
        )
        .await;
        match outcome {
            BoundedLoad::Loaded(d) => d,
            BoundedLoad::Aborted {
                attempts,
                elapsed_ms,
                retry,
            } => {
                let identity = identity_for_addr(connected, addr);
                // The load error itself is on the loader's own ERROR
                // `player_load_degraded` row for each attempt; this row says
                // what the refresh did about it.
                tracing::warn!(
                    event = "appearance_refresh_aborted",
                    cause = "player_load_failed",
                    attempts,
                    elapsed_ms,
                    retry,
                    entity_id,
                    // The player's own entity: its name is the player's.
                    entity_name = known_names::player_name(player_id),
                    account_id = identity.account_id,
                    account_name = identity.account_name,
                    player_id,
                    player_name = known_names::player_name(player_id),
                    "refresh_player_appearance: player load failed, keeping the last appearance"
                );
                return;
            }
        }
    } else {
        query_player_load_data(db_pool, account_id, player_id).await
    };
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
