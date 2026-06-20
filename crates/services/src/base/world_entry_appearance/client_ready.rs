//! `SGWPlayer.onClientReady` finalization handler.
//!
//! Extracted from `world_entry_appearance.rs`. This is the world-entry
//! finalization step: it drives the cross-world synth-mapLoaded fallback,
//! queries the player's persisted state for the cell `InitPlayerState`
//! handoff, dispatches the post-onClientReady burst bundle, fires the
//! first-login cinematic, and flushes deferred AoI.

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
    let (pending, player_name, access_level) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let entry = clients.get_mut(&addr);
        match entry {
            // `access_level` is the authoritative GM/admin level for this
            // session (from `account.accesslevel` at login). Carried into
            // the cell on `InitPlayerState` so the cell-method GM gate can
            // reject `gm*`/debug methods from non-privileged callers
            // without trusting any client byte (#475 / CAT-N-03).
            Some(c) => (
                c.pending_client_ready.take(),
                c.player_name.clone(),
                c.access_level,
            ),
            None => (None, None, 0),
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
    let (active_bandolier_slot, bandolier_items, system_options, state_field) = if let Some(pool) =
        db_pool
    {
        #[derive(sqlx::FromRow)]
        struct PlayerInitRow {
            bandolier_slot: i32,
            auto_reload: bool,
            reload_on_activate: bool,
            state_field: i32,
        }
        let row: Option<PlayerInitRow> = match sqlx::query_as::<_, PlayerInitRow>(
            "SELECT bandolier_slot, auto_reload, reload_on_activate, state_field \
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
        let (slot, opts, state_field) = match row {
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
            ),
            None => (0, cimmeria_entity::cell_entity::SystemOptions::default(), 0),
        };

        let items = super::super::world_entry::methods::player_load::meta::query_bandolier_items(
            db_pool,
            pending.player_id,
        )
        .await;

        (slot, items, opts, state_field)
    } else {
        (
            0,
            Vec::new(),
            cimmeria_entity::cell_entity::SystemOptions::default(),
            0,
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
                "ConnectEntity: base→cell send failed -- cell will not see this player, all AoI traffic will drop: {e}"
            );
        }

        if let Err(e) = tx
            .send(BaseToCellMsg::InitPlayerState {
                entity_id,
                player_id: pending.player_id,
                world_name: pending.world_name.clone(),
                archetype_id,
                saved_missions,
                abilities,
                active_bandolier_slot,
                bandolier_items,
                system_options,
                state_field,
                access_level,
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
    super::super::world_entry::cell_dispatch::flush_deferred_aoi(
        entity_id,
        addr,
        transport,
        connected,
        entity_to_addr,
    )
    .await;

    tracing::info!(%addr, entity_id, "World entry finalized (BeingAppearance resent)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────────────────────────────────────────────────────────
    // Negative-logging regression guards: onClientReady
    // cell_tx sends. ConnectEntity / InitPlayerState / AdvanceRing-
    // Destination are critical handoffs from base to cell — dropping
    // any of them strands the player in a half-loaded state. The
    // guards drive `handle_on_client_ready` with a closed cell→base
    // channel and assert each of the three ERROR logs fires
    // independently so a partial revert (only one of three) is also
    // caught.
    // ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn on_client_ready_errors_each_cell_tx_send_independently_when_closed() {
        use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
        use tracing::Level;

        let capture = LogCapture::install();
        let addr: SocketAddr = "127.0.0.1:55600".parse().unwrap();
        let entity_id: u32 = 8888;
        let key = [0u8; 32];

        // Pre-stage pending_client_ready + pending_destination_ring_id
        // so the function reaches all three cell_tx send sites.
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(entity_id);
        state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
            entity_id,
            player_id: 42,
            world_name: "Agnos".to_string(),
            appearance_args: vec![0xAB],
            tint_args: vec![0xCD],
            first_login: 0, // skip the cinematic + DB UPDATE branch
        });
        // Non-None forces the AdvanceRingDestination send to fire.
        state.pending_destination_ring_id = Some(17);
        state.player_name = Some("Tester".to_string());

        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new({
                let mut m = HashMap::new();
                m.insert(addr, state);
                m
            }));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

        // Closed cell→base channel: all three sends will SendError.
        let (tx, rx) = mpsc::channel::<BaseToCellMsg>(8);
        drop(rx);
        let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);

        // db_pool=None so DB-querying branches return empty/default
        // without needing a live Postgres.
        let _ = handle_on_client_ready(
            addr,
            key,
            &connected,
            &cell_tx,
            &transport,
            &entity_to_addr,
            &None,
        )
        .await;

        // All three ERRORs must fire — assert each independently so a
        // partial revert (e.g. only one of three reverted to `let _`)
        // is also caught.
        assert!(
            capture
                .find_message(Level::ERROR, "ConnectEntity: base→cell send failed")
                .is_some(),
            "negative-logging convention: ConnectEntity ERROR missing. Captured: {:#?}",
            capture.all()
        );
        assert!(
            capture
                .find_message(Level::ERROR, "InitPlayerState: base→cell send failed")
                .is_some(),
            "negative-logging convention: InitPlayerState ERROR missing"
        );
        assert!(
            capture
                .find_message(
                    Level::ERROR,
                    "AdvanceRingDestination: base→cell send failed"
                )
                .is_some(),
            "negative-logging convention: AdvanceRingDestination ERROR missing (requires \
             pending_destination_ring_id to be Some; check test staging)"
        );
    }

    /// Live-DB regression guard for the `first_login` UPDATE's
    /// `rows_affected == 0` branch. Stages `pending.first_login = 1`
    /// with `pending.player_id` pointing at a `sgw_player` row that
    /// does NOT exist; the UPDATE succeeds (no SQL error) but
    /// touches zero rows. Pre-#304 this was silent — the flag stayed
    /// set in some other table or the cinematic just re-fired every
    /// login with no signal. The handler now emits an ERROR naming
    /// `rows_affected=0` + `expected=1` so a single ops query
    /// catches it.
    ///
    /// Sentinel ID is well below `i32::MAX` and outside the live-DB
    /// fixture base ranges; nothing to clean up because the test
    /// does NOT insert the row.
    #[tokio::test]
    async fn first_login_update_errors_when_player_row_missing() {
        use crate::test_support::{
            require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
        };
        use tracing::Level;

        let pool = require_db_or_skip!();
        let capture = LogCapture::install();

        // Sentinel — must NOT exist in sgw_player. Picked outside
        // every other test base (TEST_BASE families peak around
        // 0x7000_0FFF).
        const MISSING_PLAYER_ID: i32 = 0x7FFE_FF99;

        let addr: SocketAddr = "127.0.0.1:55700".parse().unwrap();
        let entity_id: u32 = 9999;
        let key = [0u8; 32];

        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(entity_id);
        state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
            entity_id,
            player_id: MISSING_PLAYER_ID,
            world_name: "Agnos".to_string(),
            appearance_args: vec![0xAB],
            tint_args: vec![0xCD],
            first_login: 1, // forces the cinematic + UPDATE branch
        });
        state.player_name = Some("Tester".to_string());

        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new({
                let mut m = HashMap::new();
                m.insert(addr, state);
                m
            }));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

        // Open cell→base channel; we don't care about its sends for this test.
        let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(32);
        let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);
        let db_pool = Some(Arc::new(pool));

        let _ = handle_on_client_ready(
            addr,
            key,
            &connected,
            &cell_tx,
            &transport,
            &entity_to_addr,
            &db_pool,
        )
        .await;

        let event = capture
            .find_message(Level::ERROR, "first_login flag NOT cleared")
            .expect(
                "negative-logging convention: missing player row must emit \
                 ERROR with rows_affected=0",
            );
        assert!(
            event.has_field("rows_affected", "0"),
            "rows_affected field must be 0 — pin the structured shape so \
             ops can query (rows_affected != expected): {event:#?}"
        );
        assert!(
            event.has_field("expected", "1"),
            "expected field must be 1 — pin the paired structured field: {event:#?}"
        );
        assert!(
            event.has_field("player_id", &MISSING_PLAYER_ID.to_string()),
            "player_id field must carry the missing id for ops triage: {event:#?}"
        );
    }
}
