//! `SGWPlayer.onClientReady` finalization handler.
//!
//! Extracted from `world_entry_appearance.rs`. This is the world-entry
//! finalization step: it drives the cross-world synth-mapLoaded fallback,
//! queries the player's persisted state for the cell `InitPlayerState`
//! handoff, dispatches the post-onClientReady burst bundle, fires the
//! first-login cinematic, and flushes deferred AoI — or, on first login,
//! holds the entity introductions until the movie ends.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, WorldEntryCause};

use super::super::world_entry::handle_map_loaded;
use super::super::ConnectedClientState;
use super::cinematic_aoi_hold;

mod cell_handoff;
mod continuation;
mod mission_lock;
mod player_init_row;

/// Finalize world entry after the client sends `SGWPlayer.onClientReady`.
///
/// Also resends BeingAppearance + onEntityTint. The first copy was sent in the
/// mapLoaded bundle but may have been dropped because the entity was still in a
/// "transaction" during bundle processing. The C++ server sends BeingAppearance
/// 3-5 times via createCacheStamp replays; this second send mimics that.
#[tracing::instrument(
    name = "world_entry.on_client_ready",
    level = "info",
    skip_all,
    fields(peer = %addr),
)]
pub async fn handle_on_client_ready(
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Cross-world gate travel can skip mapLoaded. Synthesize it before
    // taking pending_client_ready so destination setup completes.
    let pending_map_loaded_present = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .get(&addr)
            .is_some_and(|c| c.pending_map_loaded.is_some())
    };
    if pending_map_loaded_present {
        tracing::info!(
            %addr,
            "onClientReady arrived with pending_map_loaded still set — \
             synthesising mapLoaded for cross-world transition"
        );
        if let Err(e) = handle_map_loaded(
            transport,
            addr,
            key,
            connected,
            cell_tx,
            entity_to_addr,
            db_pool,
        )
        .await
        {
            tracing::error!(%addr, error = %e, "Synth mapLoaded failed during cross-world onClientReady");
            // The synth `handle_map_loaded` consumed `pending_map_loaded`
            // before failing, so a retry would no longer enter this
            // branch and `pending_client_ready` will never be populated
            // by the normal map_loaded path. If we just returned here,
            // `pending_destination_ring_id` would be stranded on the
            // client state forever (disconnect cleanup doesn't clear it
            // either), and the destination ring's FSM would sit in
            // `RemoteLoadWait` indefinitely.
            //
            // Drop the pending ring id so state is consistent. The
            // destination ring still won't receive an
            // `AdvanceRingDestination` (no entity_id is reachable on
            // the dropped session anyway), but at least the
            // per-client state isn't lying about an in-flight transfer
            // that's never going to land.
            if let Ok(mut clients) = connected.lock() {
                if let Some(c) = clients.get_mut(&addr) {
                    if c.pending_destination_ring_id.take().is_some() {
                        tracing::warn!(
                            %addr,
                            "Cleared stranded pending_destination_ring_id after \
                             synth mapLoaded failure"
                        );
                    }
                }
            }
            return Ok(());
        }
    }

    // Arm the cinematic hold before DB reads open the pre-ready AoI gate.
    let (
        pending,
        player_name,
        access_level,
        account_id,
        aoi_hold,
        plugins,
        account_name,
        entry_cause,
        session_kind,
        session,
    ) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let entry = clients.get_mut(&addr);
        match entry {
            // `access_level` is the authoritative GM/admin level for this
            // session (from `account.accesslevel` at login). Carried into
            // the cell on `InitPlayerState` so the cell-method GM gate can
            // reject `gm*`/debug methods from non-privileged callers
            // without trusting any client byte (#475 / CAT-N-03).
            Some(c) => {
                let pending = c.pending_client_ready.take();
                if pending.is_some() {
                    let _ = cell_handoff::assign_entry_epoch(c);
                }
                let session = cell_handoff::SessionSnapshot::capture(c, pending.as_ref());
                // Read before the listing below sets the flag: a character
                // already listed is finishing a cross-world hop, not a login
                // (TG-NET-15). `logOff` and teardown clear it, so the next
                // `playCharacter` counts as a login again.
                let entry_cause = if c.listed_online {
                    WorldEntryCause::Travel
                } else {
                    WorldEntryCause::Login
                };
                // The client has created its player entity: list it in the
                // online name index (tells, duel challenges) until logOff or
                // teardown. Only a pending finalization lists, and only once:
                // gate travel re-runs this step for a character still listed.
                if pending.is_some() && !c.listed_online {
                    c.listed_online = true;
                    cimmeria_base_session::base::player_index::log_listed(addr, c, "world_entry");
                }
                // A lab chaos `Delay` (lab-chaos builds only) holds the same
                // way, from the same instant, so no live traffic slips ahead
                // of the delayed introductions.
                let aoi_hold = pending
                    .as_ref()
                    .filter(|p| {
                        p.first_login != 0
                            || cinematic_aoi_hold::chaos_wants_hold(addr, p.entity_id)
                    })
                    .map(|_| cinematic_aoi_hold::begin(c));
                (
                    pending,
                    c.player_name.clone(),
                    c.access_level,
                    c.account_id,
                    aoi_hold,
                    c.plugins.clone(),
                    // The login name for this function's lines (Rule 6).
                    super::super::session_identity::session_identity(c).account_name,
                    entry_cause,
                    // Lab or player, resolved at Phase 3 from the account
                    // flag (TG-NET-16); the cell stamps it on its rows.
                    c.session_kind,
                    session,
                )
            }
            // No session for this addr: `pending` is `None` so we bail below
            // before `account_id` is ever read. The 0 is unreachable filler,
            // not a sentinel any log will carry.
            None => (
                None,
                None,
                0,
                0,
                None,
                Default::default(),
                None,
                WorldEntryCause::Login,
                None,
                None,
            ),
        }
    };

    let Some(pending) = pending else {
        tracing::debug!(%addr, "SGWPlayer.onClientReady received with no pending world-entry finalization");
        return Ok(());
    };

    let entity_id = pending.entity_id;
    let name = player_name.as_deref();

    #[cfg(test)]
    session_token_tests::pause_after_pending(addr).await;

    tracing::info!(
        %addr,
        entity_id,
        entity_name = name,
        account_id,
        account_name,
        player_id = pending.player_id,
        player_name = name,
        world = %pending.world_name,
        "SGWPlayer.onClientReady received -- finalizing world entry"
    );

    // Query saved missions from DB before sending InitPlayerState. An
    // unreadable read locks the missions instead of loading none (TG-DB-12).
    let saved_missions = mission_lock::load_or_lock(
        db_pool,
        cell_tx,
        entity_to_addr,
        pending.player_id,
        entity_id,
    )
    .await;

    // Query player abilities from DB
    let abilities: Vec<i32> = if let Some(pool) = db_pool {
        sqlx::query_scalar("SELECT unnest(abilities) FROM sgw_player WHERE player_id = $1")
            .bind(pending.player_id)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Query archetype id from DB. Carried through to the cell so the
    // `Item_*` event-set lookup (reload-sequence path) can
    // resolve. Defaults to 0 on error / missing row — the cell
    // `archetype_item_event_set` falls through to "no animation" rather
    // than crashing, which matches the python behavior.
    let archetype_id: i32 = if let Some(pool) = db_pool {
        match sqlx::query_scalar::<_, Option<i32>>(
            "SELECT archetype FROM sgw_player WHERE player_id = $1",
        )
        .bind(pending.player_id)
        .fetch_optional(pool.as_ref())
        .await
        {
            Ok(Some(Some(a))) => a,
            Ok(Some(None)) | Ok(None) => 0,
            Err(e) => {
                tracing::error!(
                    player_id = pending.player_id,
                    player_name = name,
                    "Archetype read failed; defaulting to 0 but logging error: {e}"
                );
                0
            }
        }
    } else {
        0
    };

    // Query active bandolier slot, items, and server-synced system
    // options from DB — don't hardcode empty state, that previously
    // shipped two regressions where players spawned with bandolier
    // slot 0 + empty options regardless of their persisted values.
    // Distinguish DB error from "no row" so a connection blip doesn't
    // silently default a real player to empty bandolier state. The
    // two booleans come from the same SELECT so we make one
    // round-trip not three.
    //
    // `known_stargates` rides this same SELECT rather than a query of its
    // own: it is the cell's dial-gate input (CAT-O-01) and the client's
    // address book comes from the same column, so one read keeps the two
    // from straddling a concurrent unlock UPDATE and rendering an address
    // the client can click but the cell will refuse.
    let (
        active_bandolier_slot,
        bandolier_items,
        system_options,
        (known_stargates, tree_progress, level, looted_containers, shown_tutorials),
        body_set,
    ) = if let Some(pool) = db_pool {
        let row =
            match player_init_row::load_player_init_row(pool.as_ref(), pending.player_id).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(
                    player_id = pending.player_id,
                    player_name = name,
                    "Player init read failed; defaulting to XML defaults but logging error: {e}"
                );
                    None
                }
            };
        let (slot, opts, known, body_set) = match row {
            Some(r) => (
                r.bandolier_slot,
                cimmeria_entity::cell_entity::SystemOptions {
                    auto_reload: r.auto_reload,
                    reload_on_activate: r.reload_on_activate,
                },
                (
                    r.known_stargates,
                    cimmeria_entity::cell_entity::TreeProgress {
                        trained_abilities: r.trained_abilities,
                        tree_points_spent: r.tree_points_spent,
                        training_points: r.training_points,
                        credited_grants: r.credited_grants,
                    },
                    r.level,
                    r.looted_containers,
                    r.shown_tutorials,
                ),
                r.bodyset,
            ),
            None => {
                // `PlayerLoadData::default()` carries `player_id: 0`, so a
                // transient failure earlier in world entry reaches here with
                // a key matching nothing. Now that the address book rides
                // this row, the visible consequence is a player who can walk
                // and fight but whose every dial is refused.
                tracing::warn!(
                    player_id = pending.player_id,
                    player_name = name,
                    reason = "player_init_row_missing",
                    "Player init read matched no row -- bandolier, options and the \
                     stargate address book all default to empty; every dial is refused"
                );
                (
                    0,
                    cimmeria_entity::cell_entity::SystemOptions::default(),
                    (Vec::new(), Default::default(), 1, Vec::new(), Vec::new()),
                    None,
                )
            }
        };

        let items = super::super::world_entry::methods::player_load::meta::query_bandolier_items(
            db_pool,
            pending.player_id,
        )
        .await;

        (slot, items, opts, known, body_set)
    } else {
        (
            0,
            Vec::new(),
            cimmeria_entity::cell_entity::SystemOptions::default(),
            (Vec::new(), Default::default(), 1, Vec::new(), Vec::new()),
            None,
        )
    };

    // Carry the ring destination only for this still-current session.
    let advance_ring_destination_id: Option<i32> = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients.get_mut(&addr).and_then(|c| {
            session
                .as_ref()
                .is_some_and(|session| session.matches(c, entity_id, pending.player_id))
                .then(|| c.pending_destination_ring_id.take())
                .flatten()
        })
    };

    if let Some(ref tx) = cell_tx {
        let Some(ref session) = session else {
            tracing::warn!(%addr, "onClientReady pending finalization lacks a live player session");
            return Ok(());
        };
        if !cell_handoff::send_handoff(
            cell_handoff::HandoffContext {
                addr,
                connected,
                tx,
                session,
                entity_id,
                player_id: pending.player_id,
                account_id,
                account_name,
                player_name: name,
                world_name: &pending.world_name,
            },
            BaseToCellMsg::InitPlayerState {
                entity_id,
                player_id: pending.player_id,
                account_id,
                world_name: pending.world_name.clone(),
                archetype_id,
                saved_missions,
                abilities,
                active_bandolier_slot,
                bandolier_items,
                system_options,
                access_level,
                known_stargates,
                tree_progress,
                level,
                character_name: player_name.clone(),
                body_set,
                looted_containers,
                shown_tutorials,
                entry_cause,
                session_kind,
            },
            advance_ring_destination_id,
        )
        .await
        {
            return Ok(());
        }
    }
    let Some(session) = session
        .as_ref()
        .filter(|s| s.is_current(connected, addr, entity_id, pending.player_id))
    else {
        return Ok(());
    };
    continuation::run_continuation(continuation::ContinuationCtx {
        addr,
        entity_id,
        pending,
        player_name,
        account_id,
        aoi_hold,
        plugins: &plugins,
        transport,
        connected,
        entity_to_addr,
        db_pool,
        cell_tx,
        session,
    })
    .await
}

#[cfg(test)]
mod plugin_hook_tests;
#[cfg(test)]
mod session_token_tests;
#[cfg(test)]
mod tests;
