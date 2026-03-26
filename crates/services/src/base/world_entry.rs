//! World entry orchestration -- play_character (entity teardown), ENABLE_ENTITIES
//! (create player), mapLoaded (enter world), gate travel, and CellToBase message dispatch.
//!
//! Sub-concerns are split into sibling modules:
//! - `world_entry_player` -- DB queries (world entry, player load, inventory, XP, missions, mail)
//! - `world_entry_appearance` -- BeingAppearance/onEntityTint assembly and visual resend helpers

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::mercury::{
    build_avatar_update, build_char_list,
    build_create_entity_base, build_create_entity_cascade,
    build_entity_leave, build_entity_method_packet,
    build_map_loaded_body, fragment_map_loaded, fragment_count,
    build_reset_entities, build_create_player,
    build_enter_world,
    WorldEntryInfo,
    DEFAULT_SPACE_ID, SGWPLAYER_CLASS_ID,
};

use super::{ConnectedClientState, PendingClientReadyInfo};
use super::character::query_character_list;
use super::helpers::{drain_acks_and_seq, get_account_entity_id, send_to_witness};

// Re-exports from sibling modules so connect_loop.rs imports stay unchanged.
pub(crate) use super::world_entry_appearance::{handle_cancel_movie, handle_on_client_ready};
use super::world_entry_appearance::{build_appearance_args, build_tint_args};
use super::world_entry_player::{
    default_player_load_data, query_player_load_data,
    query_player_load_data_by_account, query_world_entry,
    handle_grant_xp, handle_grant_item, handle_grant_cash, handle_mission_update, handle_mail_request,
};

// ── Space registry (populated from CellService SpaceData messages) ───────────

/// Thread-safe space registry mapping world_name -> space_id.
/// Populated at startup when CellService sends SpaceData for each space.
static SPACE_REGISTRY: std::sync::LazyLock<Mutex<HashMap<String, u32>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Register a space in the global registry (called from CellToBase message handler).
fn register_space(world_name: String, space_id: u32) {
    tracing::debug!(world = %world_name, space_id, "Registered space in BaseApp registry");
    SPACE_REGISTRY.lock().unwrap().insert(world_name, space_id);
}

/// Hardcoded space ID fallback (used when CellService oneshot fails or is unavailable).
pub(crate) fn resolve_space_id_fallback(world_name: &str) -> u32 {
    match world_name {
        "Castle_CellBlock" => DEFAULT_SPACE_ID,     // 65552
        "SGC_W1"           => DEFAULT_SPACE_ID + 1, // 65553
        "CombatSim"        => DEFAULT_SPACE_ID + 2, // 65554
        _ => {
            tracing::warn!("Unknown world_location: {world_name}, defaulting to Castle_CellBlock");
            DEFAULT_SPACE_ID
        }
    }
}

// ── Reset step: playCharacter ────────────────────────────────────────────────

/// Send RESET_ENTITIES when the client calls `playCharacter` to begin world entry.
pub(crate) async fn handle_play_character(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Guard: only send once per connection.
    let arcs = {
        let mut clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            if !c.world_entry_sent {
                c.world_entry_sent = true;
                Some((Arc::clone(&c.pending_acks), Arc::clone(&c.next_seq)))
            } else {
                None
            }
        } else {
            tracing::warn!(%addr, "play_character: addr not in connected map");
            None
        }
    };

    let (pending_acks_arc, next_seq) = match arcs {
        Some(a) => a,
        None => return Ok(()),
    };

    // Query character data from DB and resolve space via CellService
    let entry_info = query_world_entry(db_pool, account_id, player_id, entity_manager, cell_tx).await;

    // Also query the full player data needed for mapLoaded
    let player_load_data = query_player_load_data(db_pool, account_id, player_id).await;

    // NOTE: C++ Account.py:293-296 uses SGWGmPlayer (0x03) for access_level > 0,
    // but SGWGmPlayer adds 6 ClientMethods and 80+ CellMethods that shift ALL
    // flattened method indices. Our hardcoded method_idx constants (BeingAppearance=26,
    // etc.) only work for SGWPlayer. Until we build a separate SGWGmPlayer index
    // table, always use SGWPlayer (0x02) regardless of access_level.
    // TODO: Build SGWGmPlayer method index table to enable GM entity type.

    tracing::info!(
        %addr,
        player_id,
        entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        pos = ?entry_info.pos,
        class_id = entry_info.class_id,
        "World entry: sending RESET_ENTITIES (entity teardown)"
    );

    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };

    // Entity teardown: Send ONLY RESET_ENTITIES.
    // The C++ server sends RESET_ENTITIES in its own flushed bundle. The client
    // tears down all entities, then sends ENABLE_ENTITIES, which triggers
    // the create-player step (CREATE_BASE_PLAYER + viewport + cell + forced position).
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT RESET_ENTITIES (entity teardown)");
    socket.send_to(&pkt, addr).await?;

    // Store the world entry info and player load data for the create-player step.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entry_info.player_entity_id);
            c.player_entity_id = Some(entry_info.player_entity_id);
            c.player_name = Some(player_load_data.player_name.clone());
            c.player_level = Some(player_load_data.level);
            c.player_archetype = Some(player_load_data.archetype);
            c.world_name = Some(entry_info.world_name.clone());
            c.player_xp = Some(player_load_data.exp as u64);
            c.player_training_points = Some(player_load_data.training_points as u32);
            c.pending_world_entry = Some(entry_info);
            c.pending_player_load_data = Some(player_load_data);
            c.pending_client_ready = None;
        }
    }

    tracing::info!(%addr, "Entity teardown sent -- waiting for ENABLE_ENTITIES from client");

    Ok(())
}

// ── Enter world: mapLoaded ───────────────────────────────────────────────────

/// Enter world: send VIEWPORT + CELL_PLAYER + FORCED_POSITION + entity data.
///
/// Called when the client sends `mapLoaded` after receiving `onClientMapLoad`
/// in the create-player step. The client has finished loading terrain
/// geometry and is ready to receive entity placement and data.
///
/// In C++, this is triggered by the CellApp's `onCellPlayerCreateAck` callback
/// (which itself fires after `connected()` sends `onClientMapLoad`) and the
/// Python `onClientReady()` -> `mapLoaded()` callback chain.
pub(crate) async fn handle_map_loaded(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Take the pending data (consumes it -- enter-world only runs once per mapLoaded)
    let (entry_info, player_data) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        let entry = c.pending_map_loaded.take()
            .ok_or("handle_map_loaded: no pending world entry")?;
        let data = c.pending_player_load_data.take()
            .unwrap_or_else(default_player_load_data);
        (entry, data)
    };

    tracing::info!(
        %addr,
        player_entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        "Enter world: client map loaded -- sending VIEWPORT + CELL + POSITION + entity data"
    );

    // Send enter-world as TWO separate bundles, matching the C++ server:
    //
    // 1. VIEWPORT + CELL_PLAYER + FORCED_POSITION -- standalone 99-byte packet.
    //    This creates the cell entity, puts it in the world, and the entity enters
    //    a brief "transaction" state during creation.
    //
    // 2. Entity methods (mapLoaded body) -- separate fragmented bundle.
    //    By arriving in a new bundle, these are processed after the entity's
    //    creation transaction completes, so BeingAppearance hits the
    //    "SCHEDULING JOB" path instead of "HOLD FOR TRANSACTION".
    //
    // Previously we combined everything into one fragmented bundle, which caused
    // BeingAppearance to be silently dropped (HOLD FOR TRANSACTION) because the
    // entity was still in its creation transaction during bundle processing.
    let map_body = build_map_loaded_body(
        entry_info.player_entity_id, &player_data, &entry_info,
    );

    let map_frags = fragment_count(map_body.len());
    // Reserve 1 seq for the standalone enter-world packet + N seqs for map fragments.
    let total_seqs = 1 + map_frags;

    let (acks, base_seq) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
        let seq = c.next_seq.fetch_add(total_seqs, Ordering::Relaxed);
        (acks, seq)
    };

    // Packet 1: VIEWPORT + CELL_PLAYER + FORCED_POSITION (standalone, ~99 bytes)
    let enter_world_pkt = build_enter_world(&key, base_seq, &acks, &entry_info);
    tracing::debug!(%addr, len = enter_world_pkt.len(), seq = base_seq,
        "UDP_OUT enter world: VIEWPORT+CELL+FORCED (standalone)");
    socket.send_to(&enter_world_pkt, addr).await?;

    // Packet 2+: Entity methods (mapLoaded body, possibly fragmented)
    let map_base_seq = base_seq + 1;
    let (map_packets, map_seqs) = fragment_map_loaded(&key, map_base_seq, &[], &map_body);
    debug_assert_eq!(map_seqs, map_frags);
    tracing::info!(
        %addr,
        enter_world_seq = base_seq,
        map_base_seq,
        map_fragments = map_packets.len(),
        map_body_len = map_body.len(),
        "mapLoaded: split send (standalone VIEWPORT+CELL + separate entity methods)"
    );
    for (i, pkt_data) in map_packets.iter().enumerate() {
        tracing::debug!(%addr, len = pkt_data.len(), seq = map_base_seq + i as u32,
            part = i + 1, total = map_packets.len(), "UDP_OUT mapLoaded entity data");
        socket.send_to(pkt_data, addr).await?;
    }

    let total_bytes: usize = enter_world_pkt.len() + map_packets.iter().map(|p| p.len()).sum::<usize>();
    let pkt_count = 1 + map_packets.len();
    tracing::info!(%addr, player = %player_data.player_name,
        level = player_data.level, archetype = player_data.archetype,
        packets = pkt_count,
        "World entry complete ({} bytes across {} packets)", total_bytes, pkt_count);

    // Clear first_login flag in DB after sending the intro movie
    if player_data.first_login != 0 {
        if let Some(ref pool) = db_pool {
            let _ = sqlx::query(
                "UPDATE sgw_player SET first_login = 0 WHERE player_id = $1",
            )
            .bind(player_data.player_id)
            .execute(pool.as_ref())
            .await;
        }

        // The first-login cinematic (onPlayMovie) blocks the client from
        // processing BeingAppearance. cancelMovie fires if the player presses
        // Escape, but NOT if the cinematic plays to completion.
        // Spawn a delayed resend to cover the natural-end case.
        // Duplicates with cancelMovie are harmless -- client just re-applies.
        let delay_socket = Arc::clone(socket);
        let delay_connected = Arc::clone(connected);
        let delay_entity_to_addr = Arc::clone(entity_to_addr);
        let delay_entity_id = entry_info.player_entity_id;
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            tracing::info!(entity_id = delay_entity_id,
                "Cinematic timer: resending BeingAppearance after 10s delay");
            handle_cancel_movie(
                &delay_socket,
                // Look up addr from entity_to_addr since it's stable
                {
                    let map = delay_entity_to_addr.lock().unwrap();
                    match map.get(&delay_entity_id).copied() {
                        Some(a) => a,
                        None => return,
                    }
                },
                delay_entity_id,
                &delay_connected,
                &delay_entity_to_addr,
            ).await;
        });
    }

    // Register entity_id -> addr before the final onClientReady gate so any
    // resource responses and future client-targeted traffic can resolve the
    // socket, but defer CellService player initialization until the client
    // explicitly signals readiness (matches C++ SGWPlayer.onClientReady).
    entity_to_addr.lock().unwrap().insert(entry_info.player_entity_id, addr);

    // Cache BeingAppearance + onEntityTint args for resend after onClientReady.
    // The first copy in the mapLoaded bundle may be dropped because the entity is
    // still in a "transaction" during bundle processing (all messages in a reassembled
    // bundle are processed in one frame). The C++ server sends BeingAppearance 3-5
    // times via createCacheStamp replays; this second send mimics that.
    let appearance_args = build_appearance_args(&player_data.bodyset, &player_data.components);
    let tint_args = build_tint_args(player_data.skin_color_id);

    // The C++ server waits for the exposed SGWPlayer base method
    // `onClientReady` (msg_id 0xD8) before calling into the cell-side
    // post-load logic that eventually fires `player.loaded`.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        // Cache appearance data for resend after cinematic (cancelMovie).
        // PendingClientReadyInfo is consumed by onClientReady, but cancelMovie
        // may arrive later (after the cinematic ends).
        c.cached_appearance_args = Some(appearance_args.clone());
        c.cached_tint_args = Some(tint_args.clone());
        c.pending_client_ready = Some(PendingClientReadyInfo {
            entity_id: entry_info.player_entity_id,
            player_id: player_data.player_id,
            world_name: entry_info.world_name.clone(),
            appearance_args,
            tint_args,
        });
    }

    tracing::info!(%addr, "World entry complete -- waiting for SGWPlayer.onClientReady");
    Ok(())
}

// ── ENABLE_ENTITIES dispatch ────────────────────────────────────────────────

/// Handle `ENABLE_ENTITIES` (0x08) -- dispatches char list or create-player step.
///
/// - **Char list** (no `pending_player_entity_id`): First ENABLE_ENTITIES after connect.
///   Creates the Account entity and sends the character list, then starts tick-sync.
/// - **Create player** (has `pending_player_entity_id`): After world entry RESET_ENTITIES.
///   Sends `CREATE_BASE_PLAYER` + `onClientMapLoad`. The client loads terrain and
///   then sends `mapLoaded`, which triggers the enter-world step.
pub(crate) async fn handle_enable_entities(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    _entity_manager: &Arc<Mutex<EntityManager>>,
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    _entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check if we have a pending world entry (create-player step).
    let pending = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            match (c.pending_player_entity_id.take(), c.pending_world_entry.take()) {
                (Some(eid), Some(entry)) => Some((eid, entry)),
                _ => None,
            }
        } else {
            None
        }
    };

    // Also retrieve the pending player load data
    let pending_load = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_load_data.take()
        } else {
            None
        }
    };

    if let Some((_eid, entry_info)) = pending {
        // -- Create player step: CREATE_BASE_PLAYER + onClientMapLoad --
        // Send only the base entity and terrain load notification. The client
        // will load geometry and respond with `mapLoaded` (cell method index 25,
        // msg_id 0x99). The enter-world step (viewport + cell + position + entity data)
        // is sent in response to that message.
        let (acks, seq) = drain_acks_and_seq(connected, addr)?;

        tracing::info!(
            %addr,
            player_entity_id = entry_info.player_entity_id,
            space_id = entry_info.space_id,
            seq,
            "Create player: sending CREATE_BASE_PLAYER + onClientMapLoad (waiting for mapLoaded)"
        );

        let pkt = build_create_player(&key, seq, &acks, &entry_info);
        socket.send_to(&pkt, addr).await?;

        // Store world entry + player data for the enter-world step (triggered by mapLoaded)
        {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_map_loaded = Some(entry_info);
            c.pending_player_load_data = pending_load;
            c.pending_client_ready = None;
        }
    }

        tracing::info!(%addr, "Create player complete -- waiting for client mapLoaded");
        return Ok(());
    }

    // -- Phase 4: initial entity creation -- send Account entity + char list --
    // Account entity was already allocated in Phase 3 (handle_login).

    // Guard: only send once per connection.
    {
        let mut clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            if c.char_list_sent {
                return Ok(()); // already sent
            }
            c.char_list_sent = true;
        } else {
            tracing::warn!(%addr, "enable_entities: addr not in connected map");
            return Ok(());
        }
    }

    // Query characters from DB
    let characters = query_character_list(db_pool, account_id).await;
    let account_eid = get_account_entity_id(connected, addr)?;

    tracing::info!(
        %addr,
        account_entity_id = account_eid,
        count = characters.len(),
        "Phase 4: sending character list ({})",
        if characters.is_empty() { "creation screen" } else { "select screen" }
    );

    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_char_list(&key, seq, &acks, &characters, account_eid);
    tracing::trace!(%addr, len = pkt.len(), seq, hex = %super::helpers::to_hex(&pkt), "UDP_OUT char_list");
    socket.send_to(&pkt, addr).await?;

    tracing::info!(%addr, "Phase 4 complete -- char list sent");

    Ok(())
}

// ── Gate travel ─────────────────────────────────────────────────────────────

/// Handle a gate travel request from CellService.
///
/// This re-uses the world entry flow (teardown -> create player -> enter world):
/// 1. Send RESET_ENTITIES to tear down the client entity system.
/// 2. Set up pending world entry for the new world (reusing same entity_id).
/// 3. Client responds with ENABLE_ENTITIES -> create-player + enter-world steps send the new world packets.
///
/// The CellService has already removed the entity from its old space.
/// We tell it to create the entity in the new space, then send the client
/// the full world-entry + mapLoaded sequence for the destination.
async fn handle_gate_travel(
    entity_id: u32,
    target_world_name: &str,
    position: [f32; 3],
    rotation: [f32; 3],
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Look up client socket from entity_id
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied()
        .ok_or("Gate travel: no client addr for entity")?;

    // Get client state
    let (key, account_id, _access_level, pending_acks_arc, next_seq) = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get(&addr).ok_or("Gate travel: client state not found")?;
        (c.key, c.account_id, c.access_level, Arc::clone(&c.pending_acks), Arc::clone(&c.next_seq))
    };

    tracing::info!(
        entity_id, %addr, world = %target_world_name,
        "Gate travel: sending RESET_ENTITIES for world transition"
    );

    // Tell CellService to create the entity in the new space and await the
    // resolved space_id via oneshot (needed for the world-entry wire packet).
    let space_id = if let Some(tx) = cell_tx {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        if tx.send(BaseToCellMsg::CreateEntity {
            entity_id,
            world_name: target_world_name.to_string(),
            position,
            rotation,
            reply_tx,
        }).await.is_ok() {
            match reply_rx.await {
                Ok(sid) => sid,
                Err(_) => {
                    tracing::warn!(world = %target_world_name, "Gate travel: CellService oneshot dropped -- using fallback");
                    resolve_space_id_fallback(target_world_name)
                }
            }
        } else {
            resolve_space_id_fallback(target_world_name)
        }
    } else {
        resolve_space_id_fallback(target_world_name)
    };

    // Build the world entry info for the new destination
    let entry_info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: rotation,
        world_name: target_world_name.to_string(),
        class_id: SGWPLAYER_CLASS_ID, // See NOTE above -- SGWGmPlayer shifts method indices
    };

    // Query player load data from DB (same player, different world)
    let player_id = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get(&addr).ok_or("client not found")?;
        // We stored account_id but need player_id for the DB query.
        // The player_id isn't stored in ConnectedClientState, so we query by
        // account_id alone (the DB query uses account_id to find the active char).
        c.account_id
    };
    let player_load_data = query_player_load_data_by_account(db_pool, account_id).await;

    // Entity teardown: Send RESET_ENTITIES
    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    socket.send_to(&pkt, addr).await?;

    // Store pending world entry for the create-player step (ENABLE_ENTITIES handler)
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entity_id);
            c.pending_world_entry = Some(entry_info);
            c.pending_player_load_data = Some(player_load_data);
            c.pending_client_ready = None;
            // world_entry_sent stays true -- we don't reset it, since
            // handle_enable_entities checks pending_player_entity_id
        }
    }

    tracing::info!(
        entity_id, %addr, world = %target_world_name,
        "Gate travel: RESET_ENTITIES sent -- awaiting ENABLE_ENTITIES"
    );

    let _ = player_id; // account_id used above
    Ok(())
}

// ── CellToBase message dispatch ─────────────────────────────────────────────

/// Handle a message from CellService -- dispatches AoI packets to witness clients.
pub(crate) async fn handle_cell_message(
    msg: CellToBaseMsg,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
    minigame_registry: &Option<crate::minigame::SessionRegistry>,
    minigame_external_host: &str,
    minigame_external_port: u16,
) {
    match msg {
        CellToBaseMsg::SpaceData { space_id, world_name } => {
            register_space(world_name, space_id);
        }
        CellToBaseMsg::EntityCreated { entity_id, space_id, position } => {
            tracing::debug!(entity_id, space_id, ?position, "CellService: entity created");
        }
        CellToBaseMsg::EnteredAoI { witness_id, entity_id, space_id: _, class_id, position, direction, level, npc_data } => {
            tracing::debug!(witness_id, entity_id, class_id, level, "AoI: entity entered witness range");
            // Packet 1: CREATE_ENTITY + UPDATE_AVATAR (BaseApp immediate)
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_create_entity_base(
                        key, seq, acks, entity_id,
                        class_id, position, direction,
                    )
                },
            ).await;
            // Packet 2: createOnClient() property cascade (CellApp round-trip)
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_create_entity_cascade(
                        key, seq, acks, entity_id,
                        class_id, level, npc_data.as_ref(),
                    )
                },
            ).await;
        }
        CellToBaseMsg::LeftAoI { witness_id, entity_id } => {
            tracing::debug!(witness_id, entity_id, "AoI: entity left witness range");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_entity_leave(key, seq, acks, entity_id)
                },
            ).await;
        }
        CellToBaseMsg::EntityMoved { witness_id, entity_id, space_id: _, position, direction, velocity } => {
            tracing::trace!(witness_id, entity_id, "AoI: entity position update");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_avatar_update(
                        key, seq, acks, entity_id,
                        position, velocity, direction,
                    )
                },
            ).await;
        }
        CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } => {
            tracing::debug!(entity_id, method_index, args_len = args.len(), "CellService->client entity method call");
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method_index, &args)
                },
            ).await;
        }
        CellToBaseMsg::GateTravel { entity_id, target_world_name, position, rotation } => {
            if let Err(e) = handle_gate_travel(
                entity_id, &target_world_name, position, rotation,
                socket, connected, entity_to_addr, cell_tx, db_pool,
            ).await {
                tracing::error!(entity_id, world = %target_world_name, "Gate travel failed: {e}");
            }
        }
        CellToBaseMsg::MailRequest { entity_id, op } => {
            handle_mail_request(entity_id, op, socket, connected, entity_to_addr, db_pool).await;
        }
        CellToBaseMsg::MissionUpdate { player_id, mission_id, status, current_step_id,
                                        completed_step_ids, completed_objective_ids, active_objective_ids,
                                        failed_objective_ids } => {
            handle_mission_update(
                player_id, mission_id, status, current_step_id,
                &completed_step_ids, &completed_objective_ids, &active_objective_ids,
                &failed_objective_ids, db_pool,
            ).await;
        }
        CellToBaseMsg::GrantXP { entity_id, xp_amount } => {
            handle_grant_xp(entity_id, xp_amount, socket, connected, entity_to_addr).await;
        }
        CellToBaseMsg::GrantItem { entity_id, player_id, item_id, container_id, count } => {
            handle_grant_item(
                entity_id, player_id, item_id, container_id, count,
                db_pool, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::GrantCash { entity_id, amount } => {
            handle_grant_cash(entity_id, amount, db_pool, socket, connected, entity_to_addr).await;
        }
        CellToBaseMsg::WitnessEntityMethod { witness_id, entity_id, method_index, args } => {
            tracing::debug!(witness_id, entity_id, method_index, "Broadcast entity method to witness");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method_index, &args)
                },
            ).await;
        }
        CellToBaseMsg::StartMinigame { entity_id, player_id, game_name, difficulty, on_victory_chains } => {
            tracing::info!(entity_id, player_id, %game_name, difficulty, "Starting minigame session");
            if let Some(registry) = minigame_registry {
                let seed = rand::random::<u32>();
                let ticket = registry.register(
                    entity_id, player_id, game_name.clone(), difficulty,
                    1, // tech_competency — TODO: read from player entity
                    seed, 0, 0, 1, // abilities, intelligence, player_level
                    on_victory_chains,
                ).await;

                if let Some(ticket) = ticket {
                    // Build URL: http://unused/{ip}/{port}/{gameName}/{entityId}/{ticket}
                    let url = format!(
                        "http://unused/{}/{}/{}/{}/{}",
                        minigame_external_host, minigame_external_port,
                        game_name, entity_id, ticket
                    );
                    tracing::info!(entity_id, %url, "Sending onStartMinigame to client");

                    // onStartMinigame(URL: WSTRING) — MinigamePlayer client method
                    // Method index for onStartMinigame in the SGWPlayer flat dispatch table
                    let url_utf16: Vec<u16> = url.encode_utf16().collect();
                    let mut args = Vec::with_capacity(4 + url_utf16.len() * 2);
                    args.extend_from_slice(&(url_utf16.len() as u32).to_le_bytes());
                    for ch in &url_utf16 {
                        args.extend_from_slice(&ch.to_le_bytes());
                    }
                    let method = crate::cell::dispatch::CLIENT_MG_ON_START_MINIGAME;
                    send_to_witness(
                        socket, connected, entity_to_addr, entity_id,
                        |key, seq, acks| {
                            build_entity_method_packet(key, seq, acks, entity_id, method, &args)
                        },
                    ).await;
                } else {
                    tracing::warn!(entity_id, "Failed to register minigame session (duplicate?)");
                }
            }
        }
        CellToBaseMsg::MinigameResult { entity_id, result_code, on_victory_chains } => {
            tracing::info!(entity_id, result_code, "Minigame result received");
            // Send onEndMinigame to client
            let method = crate::cell::dispatch::CLIENT_MG_ON_END_MINIGAME;
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method, &[])
                },
            ).await;
            // Forward to CellApp for victory chain processing
            if let Some(cell_tx) = cell_tx {
                let _ = cell_tx.send(BaseToCellMsg::MinigameResult {
                    entity_id,
                    result_code,
                    on_victory_chains,
                }).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn space_id_mapping_known_worlds() {
        // Verify the three known space IDs are distinct and have high 16 bits == 1.
        let castle_cellblock: u32 = 65552; // (1 << 16) | 16
        let sgc_w1: u32 = 65553;           // (1 << 16) | 17
        let combat_sim: u32 = 65554;       // (1 << 16) | 18

        // All distinct
        assert_ne!(castle_cellblock, sgc_w1);
        assert_ne!(sgc_w1, combat_sim);
        assert_ne!(castle_cellblock, combat_sim);

        // High 16 bits == 1 for all three
        assert_eq!(castle_cellblock >> 16, 1);
        assert_eq!(sgc_w1 >> 16, 1);
        assert_eq!(combat_sim >> 16, 1);
    }
}
