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

use crate::cell::messages::BaseToCellMsg;

use super::super::helpers::send_bundle_to_witness_reliable;
use super::super::world_entry::handle_map_loaded;
use super::super::world_entry_chat::build_welcome_message_args;
use super::super::ConnectedClientState;
use super::builders::build_on_client_ready_burst_bundle;
use super::cinematic::send_cinematic;
use super::cinematic_aoi_hold;

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
pub(crate) async fn handle_on_client_ready(
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Cross-world transition fallback: the SGW client sends `mapLoaded`
    // (cell method 0x99) on initial logins and same-instance respawns,
    // but skips it on cross-world transitions via `gate_travel`
    // (RESET_ENTITIES → ENABLE_ENTITIES → CREATE_BASE_PLAYER + onClientMapLoad).
    // The client jumps straight to `onClientReady` instead. If the
    // transition is mid-flight (`pending_map_loaded` still set, meaning
    // the `mapLoaded` cell-method handler never ran), drive the
    // enter-world bundle from here so the destination world finishes
    // setup. After this synth call, `pending_client_ready` will be
    // populated by `handle_map_loaded` and the rest of this function
    // proceeds normally.
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

    // Take the pending finalization AND copy out the player_name in the
    // same lock so the welcome-message send below doesn't need a second
    // round-trip. `player_name` is set during `playCharacter` and stays
    // for the session, so reading it here is safe.
    //
    // The first-login cinematic hold starts in this same critical section.
    // Taking `pending_client_ready` opens the pre-ready AoI gate, and the
    // cell answers `ConnectEntity` with `EnteredAoI` while this function is
    // still awaiting DB reads — a hold armed any later (say, next to
    // `send_cinematic`) lets those creates reach a client that is about to
    // play a fullscreen movie. See `cinematic_aoi_hold`.
    let (pending, player_name, access_level, account_id, aoi_hold) = {
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
                let aoi_hold = pending
                    .as_ref()
                    .filter(|p| p.first_login != 0)
                    .map(|_| cinematic_aoi_hold::begin(c));
                (
                    pending,
                    c.player_name.clone(),
                    c.access_level,
                    c.account_id,
                    aoi_hold,
                )
            }
            // No session for this addr: `pending` is `None` so we bail below
            // before `account_id` is ever read. The 0 is unreachable filler,
            // not a sentinel any log will carry.
            None => (None, None, 0, 0, None),
        }
    };

    let Some(pending) = pending else {
        tracing::debug!(%addr, "SGWPlayer.onClientReady received with no pending world-entry finalization");
        return Ok(());
    };

    let entity_id = pending.entity_id;

    tracing::info!(
        %addr,
        entity_id,
        account_id,
        player_id = pending.player_id,
        world = %pending.world_name,
        "SGWPlayer.onClientReady received -- finalizing world entry"
    );

    // Query saved missions from DB before sending InitPlayerState
    let saved_missions =
        super::super::world_entry::methods::query_saved_missions(db_pool, pending.player_id).await;

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
    let (active_bandolier_slot, bandolier_items, system_options, state_field, known_stargates) =
        if let Some(pool) = db_pool {
            #[derive(sqlx::FromRow)]
            struct PlayerInitRow {
                bandolier_slot: i32,
                auto_reload: bool,
                reload_on_activate: bool,
                state_field: i32,
                known_stargates: Vec<i32>,
            }
            let row: Option<PlayerInitRow> = match sqlx::query_as::<_, PlayerInitRow>(
                "SELECT bandolier_slot, auto_reload, reload_on_activate, state_field, \
                    known_stargates \
                 FROM sgw_player WHERE player_id = $1",
            )
            .bind(pending.player_id)
            .fetch_optional(pool.as_ref())
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(
                    player_id = pending.player_id,
                    "Player init read failed; defaulting to XML defaults but logging error: {e}"
                );
                    None
                }
            };
            let (slot, opts, state_field, known) = match row {
                Some(r) => (
                    r.bandolier_slot,
                    cimmeria_entity::cell_entity::SystemOptions {
                        auto_reload: r.auto_reload,
                        reload_on_activate: r.reload_on_activate,
                    },
                    // Stored masked (PERSISTED_STATE_FIELD_MASK + the
                    // schema's non-negative CHECK), so the lossless cast
                    // back to the in-memory u32 bitmask is safe.
                    r.state_field as u32,
                    r.known_stargates,
                ),
                None => {
                    // `PlayerLoadData::default()` carries `player_id: 0`, so a
                    // transient failure earlier in world entry reaches here with
                    // a key matching nothing. Now that the address book rides
                    // this row, the visible consequence is a player who can walk
                    // and fight but whose every dial is refused.
                    tracing::warn!(
                        player_id = pending.player_id,
                        reason = "player_init_row_missing",
                        "Player init read matched no row -- bandolier, options and the \
                     stargate address book all default to empty; every dial is refused"
                    );
                    (
                        0,
                        cimmeria_entity::cell_entity::SystemOptions::default(),
                        0,
                        Vec::new(),
                    )
                }
            };

            let items =
                super::super::world_entry::methods::player_load::meta::query_bandolier_items(
                    db_pool,
                    pending.player_id,
                )
                .await;

            (slot, items, opts, state_field, known)
        } else {
            (
                0,
                Vec::new(),
                cimmeria_entity::cell_entity::SystemOptions::default(),
                0,
                Vec::new(),
            )
        };

    // Cross-world ring transport carry-through: take the pending ring id
    // BEFORE we drop the connected lock so a concurrent disconnect can't
    // strand it. Set in `gate_travel::handle_gate_travel` only when
    // `Effect::TeleportCrossWorld` produced this hop — stargate dial
    // travel leaves it None.
    let advance_ring_destination_id: Option<i32> = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .get_mut(&addr)
            .and_then(|c| c.pending_destination_ring_id.take())
    };

    if let Some(ref tx) = cell_tx {
        if let Err(e) = tx.send(BaseToCellMsg::ConnectEntity { entity_id }).await {
            // ConnectEntity drop leaves the cell with no
            // record of the new player — every subsequent AoI / method
            // call for this entity would silently drop. error! because
            // recovery requires the player to log out and back in.
            tracing::error!(
                entity_id,
                account_id,
                player_id = pending.player_id,
                "ConnectEntity: base→cell send failed -- cell will not see this player, all AoI traffic will drop: {e}"
            );
        }

        if let Err(e) = tx
            .send(BaseToCellMsg::InitPlayerState {
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
                state_field,
                access_level,
                known_stargates,
                character_name: player_name.clone(),
            })
            .await
        {
            // same shape as the ConnectEntity error above —
            // missing InitPlayerState leaves the cell with a connected
            // entity but no mission/ability/bandolier state. Player
            // will appear loaded but quests / hotbar will be empty.
            tracing::error!(
                entity_id,
                account_id,
                player_id = pending.player_id,
                world_name = %pending.world_name,
                "InitPlayerState: base→cell send failed -- player loaded with empty mission/ability state: {e}"
            );
        }

        if let Some(region_id) = advance_ring_destination_id {
            // Wake the destination ring's FSM. Sent AFTER InitPlayerState
            // so the cell-side handler runs against the fully-initialised
            // entity (player_id, missions, etc.) — `mark_player_loaded`
            // doesn't need that state directly, but downstream chain
            // events fired by the unlock cascade do.
            if let Err(e) = tx
                .send(BaseToCellMsg::AdvanceRingDestination {
                    entity_id,
                    region_id,
                })
                .await
            {
                // ring-destination drop strands the player
                // mid-transport — they arrive at the destination but
                // the ring FSM stays in RemoteLoadWait forever.
                tracing::error!(
                    entity_id,
                    region_id,
                    "AdvanceRingDestination: base→cell send failed -- ring FSM stuck, player invisible to other ring riders: {e}"
                );
            }
        }
    }

    // Bundle the post-onClientReady burst: BeingAppearance resend +
    // onEntityTint resend + 8× onChatJoined + onPlayerCommunication welcome.
    //
    // **Transaction-state audit** (see
    // [docs/architecture/mercury-bundle.md](../../../docs/architecture/mercury-bundle.md)):
    // every message below targets the player's own `entity_id`, which was
    // created in `handle_map_loaded`'s prior bundle. The CREATE_BASE_PLAYER
    // transaction released at that bundle's end-of-frame, so same-entity
    // messages in THIS bundle bind to the now-live entity and are NOT
    // HOLD-FOR-TRANSACTION dropped. No CREATE_ENTITY / CELL_PLAYER fires
    // inside this handler, so the bundle is exclusively post-transaction
    // property/method updates — the canonical "safe to combine" case.
    //
    // Pre-bundle: 11 reliable packets (1 appearance + 1 tint + 8 chat-joined
    // + 1 welcome), each consuming a TX-window slot. Post-bundle: 1 reliable
    // packet (the burst body is ~700 B, well under FRAGMENT_BODY_SIZE=1300),
    // pinned by [`super::builders::tests::on_client_ready_burst_bundles_to_single_packet`].
    //
    // `speaker` resolution mirrors the pre-bundle path: prefer the session's
    // `player_name`, fall back to "Server" (a real DEFAULT_CHAT_CHANNELS
    // entry) with a WARN so the unexpected missing-name case stays visible.
    let appearance_args = pending.appearance_args;
    let tint_args = pending.tint_args;
    let speaker = player_name.as_deref().unwrap_or_else(|| {
        tracing::warn!(
            %addr,
            entity_id,
            "onClientReady: player_name not set on connected state — \
             sending welcome with generic Server speaker"
        );
        "Server"
    });
    let welcome_args = build_welcome_message_args(speaker, entity_id);

    let bundle =
        build_on_client_ready_burst_bundle(entity_id, &appearance_args, &tint_args, &welcome_args);
    send_bundle_to_witness_reliable(transport, connected, entity_to_addr, entity_id, bundle).await;

    // Push contact lists (Friends / Ignore + any custom lists) to the client.
    // Runs after the burst bundle so the entity is fully live on the client
    // before we start sending list-method packets. Also creates the system
    // lists on first login (idempotent via ensure_system_lists).
    super::super::contact_list::handlers::push_contact_lists_on_login(
        entity_id,
        pending.player_id,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;

    // Fan out online status (CM 89, eventId=LoggedInStatus, data=1) to all
    // online players who have this character in any of their contact lists.
    // Runs after the list-push so the player is fully set up before watchers
    // are notified. Fire-and-forget via tokio::spawn — the login burst is
    // already complete and we must not block it on per-watcher DB + sends.
    if let Some(ref name) = player_name {
        let name_owned = name.clone();
        let pool_c = db_pool.clone();
        let transport_c = transport.clone();
        let connected_c = connected.clone();
        let entity_to_addr_c = entity_to_addr.clone();
        tokio::spawn(async move {
            super::super::contact_list::handlers::fanout_login_status(
                &name_owned,
                true, // online
                &pool_c,
                &transport_c,
                &connected_c,
                &entity_to_addr_c,
            )
            .await;
        });
    }

    // First-login cinematic — fires AFTER appearance is bound to the now-live
    // possessed pawn. Sending it inside the mapLoaded bundle (before this
    // gate) lets the cinematic-exit CollectGarbage reclaim the in-flight
    // appearance asset and produces a "dev cube" flash. Issue #288.
    if pending.first_login != 0 {
        send_cinematic(
            transport,
            addr,
            entity_id,
            "Cine-SGWLogo.SGWLogo",
            true, // fullscreen
            connected,
            entity_to_addr,
        )
        .await;

        if let Some(pool) = db_pool {
            match sqlx::query("UPDATE sgw_player SET first_login = 0 WHERE player_id = $1")
                .bind(pending.player_id)
                .execute(pool.as_ref())
                .await
            {
                Ok(r) if r.rows_affected() == 0 => {
                    // silent rows_affected==0 was the
                    // original-Python ghost bug — UPDATE succeeds but
                    // touches nothing, flag stays set, cinematic
                    // re-fires every login. error! so a single ops query
                    // (rows_affected != expected) surfaces it.
                    tracing::error!(
                        player_id = pending.player_id,
                        rows_affected = 0,
                        expected = 1,
                        "first_login flag NOT cleared — cinematic will re-fire on next login (no matching player row?)"
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        player_id = pending.player_id,
                        error = %e,
                        "Failed to clear first_login flag after cinematic dispatch; player will see intro again next login",
                    );
                }
            }
        }

        tracing::info!(%addr, entity_id, "First-login cinematic dispatched after onClientReady gate");
    }

    // Flush any AoI messages the cell tried to dispatch while the client
    // was still loading terrain. The client is now ready to receive
    // entity-state traffic; the Channel's deferred-send queue absorbs
    // any overflow past the 32-slot TX window.
    //
    // On first login only the player-self method calls go now. Entity
    // introductions stay buffered until the movie is cancelled or runs out.
    if let Some(hold) = aoi_hold {
        super::super::world_entry::cell_dispatch::flush_deferred_self_methods(
            entity_id,
            addr,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        cinematic_aoi_hold::arm_timeout(
            hold,
            entity_id,
            addr,
            transport,
            connected,
            entity_to_addr,
        );
    } else {
        super::super::world_entry::cell_dispatch::flush_deferred_aoi(
            entity_id,
            addr,
            "on_client_ready",
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    tracing::info!(%addr, entity_id, "World entry finalized (BeingAppearance resent)");
    Ok(())
}

#[cfg(test)]
mod tests;
