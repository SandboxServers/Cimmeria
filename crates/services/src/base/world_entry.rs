use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg, MailOp};
use crate::mercury::{
    archetype_ability_tree, build_avatar_update, build_char_list, build_create_entity,
    build_entity_leave, build_entity_method_packet, build_map_loaded,
    build_reset_entities, build_world_entry_phase_a, build_world_entry_phase_b,
    PlayerLoadData, WorldEntryInfo, DEFAULT_SPACE_ID,
};

use super::ConnectedClientState;
use super::character::query_character_list;
use super::helpers::{drain_acks_and_seq, get_account_entity_id, send_to_witness};

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
fn resolve_space_id_fallback(world_name: &str) -> u32 {
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

/// Send the Phase 5 world entry packet when the client calls `playCharacter`.
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

    tracing::info!(
        %addr,
        player_id,
        entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        pos = ?entry_info.pos,
        "Phase 5a: sending RESET_ENTITIES (entity teardown)"
    );

    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };

    // Phase 5a: Send ONLY RESET_ENTITIES.
    // The C++ server sends RESET_ENTITIES in its own flushed bundle. The client
    // tears down all entities, then sends ENABLE_ENTITIES, which triggers
    // Phase 5b (CREATE_BASE_PLAYER + viewport + cell + forced position).
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT RESET_ENTITIES (Phase 5a)");
    socket.send_to(&pkt, addr).await?;

    // Store the world entry info and player load data for Phase 5b.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entry_info.player_entity_id);
            c.player_entity_id = Some(entry_info.player_entity_id);
            c.pending_world_entry = Some(entry_info);
            c.player_name = Some(player_load_data.player_name.clone());
            c.pending_player_load_data = Some(player_load_data);
        }
    }

    tracing::info!(%addr, "Phase 5a sent -- waiting for ENABLE_ENTITIES from client");

    Ok(())
}

/// Phase 5b-B: send VIEWPORT + CELL_PLAYER + FORCED_POSITION + entity data.
///
/// Called when the client sends its first cell or base method after receiving
/// `onClientMapLoad` in Phase 5b-A. The client has finished loading terrain
/// geometry and is ready to receive entity placement and data.
///
/// In C++, this is triggered by the CellApp's `onCellPlayerCreateAck` callback
/// (which itself fires after `connected()` sends `onClientMapLoad`) and the
/// Python `onClientReady()` -> `mapLoaded()` callback chain.
pub(crate) async fn handle_map_loaded_phase_b(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Take the pending data (consumes it -- Phase 5b-B only runs once)
    let (entry_info, player_data) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
        let entry = c.pending_world_entry_phase_b.take()
            .ok_or("Phase 5b-B: no pending world entry")?;
        let data = c.pending_player_load_data.take()
            .unwrap_or_else(default_player_load_data);
        (entry, data)
    };

    tracing::info!(
        %addr,
        player_entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        "Phase 5b-B: client ready -- sending VIEWPORT + CELL + POSITION + entity data"
    );

    // Packet 1: VIEWPORT + CELL_PLAYER + FORCED_POSITION
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_world_entry_phase_b(&key, seq, &acks, &entry_info);
    socket.send_to(&pkt, addr).await?;

    // Packets 2+: mapLoaded entity data sequence
    let (acks2, seq2) = drain_acks_and_seq(connected, addr)?;
    let map_packets = build_map_loaded(
        &key, seq2, &acks2, entry_info.player_entity_id, &player_data, &entry_info,
    );
    let pkt_count = map_packets.len() as u32;
    for (i, pkt_data) in map_packets.iter().enumerate() {
        tracing::debug!(%addr, len = pkt_data.len(), seq = seq2 + i as u32,
            part = i + 1, total = pkt_count, "UDP_OUT mapLoaded");
        socket.send_to(pkt_data, addr).await?;
    }
    if pkt_count > 1 {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get(&addr) {
            c.next_seq.fetch_add(pkt_count - 1, Ordering::Relaxed);
        }
    }

    let total_bytes: usize = map_packets.iter().map(|p| p.len()).sum();
    tracing::info!(%addr, player = %player_data.player_name,
        level = player_data.level, archetype = player_data.archetype,
        packets = pkt_count,
        "Phase 5b-B: mapLoaded complete ({} bytes across {} packets)", total_bytes, pkt_count);

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
    }

    // Register entity_id -> addr BEFORE notifying CellService, so any response
    // messages (content engine actions, AoI events) can find the client socket.
    entity_to_addr.lock().unwrap().insert(entry_info.player_entity_id, addr);

    // Notify CellService that this entity now has a client controller
    if let Some(ref tx) = cell_tx {
        let _ = tx.send(BaseToCellMsg::ConnectEntity {
            entity_id: entry_info.player_entity_id,
        }).await;

        // Send InitPlayerState so CellService can fire content engine events
        let _ = tx.send(BaseToCellMsg::InitPlayerState {
            entity_id: entry_info.player_entity_id,
            player_id: player_data.player_id,
            world_name: entry_info.world_name.clone(),
        }).await;
    }

    tracing::info!(%addr, "Phase 5b complete -- player in world");
    Ok(())
}

/// Handle `ENABLE_ENTITIES` (0x08) -- dispatches Phase 4 or Phase 5b-A.
///
/// - **Phase 4** (no `pending_player_entity_id`): First ENABLE_ENTITIES after connect.
///   Creates the Account entity and sends the character list, then starts tick-sync.
/// - **Phase 5b-A** (has `pending_player_entity_id`): After world entry RESET_ENTITIES.
///   Sends `CREATE_BASE_PLAYER` + `onClientMapLoad`. The client loads terrain and
///   then sends a cell/base method, which triggers Phase 5b-B.
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
    // Check if we have a pending world entry (Phase 5b-A).
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
        // ── Phase 5b-A: CREATE_BASE_PLAYER + onClientMapLoad ──
        // Send only the base entity and terrain load notification. The client
        // will load geometry and respond with `mapLoaded` (cell method index 25,
        // msg_id 0x99). Phase 5b-B (viewport + cell + position + entity data)
        // is sent in response to that message.
        let (acks, seq) = drain_acks_and_seq(connected, addr)?;

        tracing::info!(
            %addr,
            player_entity_id = entry_info.player_entity_id,
            space_id = entry_info.space_id,
            seq,
            "Phase 5b-A: sending CREATE_BASE_PLAYER + onClientMapLoad (waiting for mapLoaded)"
        );

        let pkt = build_world_entry_phase_a(&key, seq, &acks, &entry_info);
        socket.send_to(&pkt, addr).await?;

        // Store world entry + player data for Phase 5b-B (triggered by mapLoaded)
        {
            let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            if let Some(c) = clients.get_mut(&addr) {
                c.pending_world_entry_phase_b = Some(entry_info);
                c.pending_player_load_data = pending_load;
            }
        }

        tracing::info!(%addr, "Phase 5b-A complete -- waiting for client mapLoaded");
        return Ok(());
    }

    // ── Phase 4: initial entity creation -- send Account entity + char list ──
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

/// Handle a gate travel request from CellService.
///
/// This re-uses the Phase 5a/5b world entry flow:
/// 1. Send RESET_ENTITIES to tear down the client entity system.
/// 2. Set up pending world entry for the new world (reusing same entity_id).
/// 3. Client responds with ENABLE_ENTITIES -> Phase 5b sends the new world packets.
///
/// The CellService has already removed the entity from its old space.
/// We tell it to create the entity in the new space, then send the client
/// the full world-entry + mapLoaded sequence for the destination.
pub(crate) async fn handle_gate_travel(
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
    let (key, account_id, pending_acks_arc, next_seq) = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get(&addr).ok_or("Gate travel: client state not found")?;
        (c.key, c.account_id, Arc::clone(&c.pending_acks), Arc::clone(&c.next_seq))
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

    // Phase 5a: Send RESET_ENTITIES
    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    socket.send_to(&pkt, addr).await?;

    // Store pending world entry for Phase 5b (ENABLE_ENTITIES handler)
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entity_id);
            c.pending_world_entry = Some(entry_info);
            c.pending_player_load_data = Some(player_load_data);
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

/// Handle a message from CellService -- dispatches AoI packets to witness clients.
pub(crate) async fn handle_cell_message(
    msg: CellToBaseMsg,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    match msg {
        CellToBaseMsg::SpaceData { space_id, world_name } => {
            register_space(world_name, space_id);
        }
        CellToBaseMsg::EntityCreated { entity_id, space_id, position } => {
            tracing::debug!(entity_id, space_id, ?position, "CellService: entity created");
        }
        CellToBaseMsg::EnteredAoI { witness_id, entity_id, space_id: _, class_id, position, direction } => {
            tracing::debug!(witness_id, entity_id, class_id, "AoI: entity entered witness range");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_create_entity(
                        key, seq, acks, entity_id,
                        class_id, position, direction,
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
        CellToBaseMsg::GrantItem { entity_id: _, player_id, item_id, container_id, count } => {
            handle_grant_item(player_id, item_id, container_id, count, db_pool).await;
        }
    }
}

/// Persist a mission state change to the database.
async fn handle_mission_update(
    player_id: i32,
    mission_id: i32,
    status: i8,
    current_step_id: Option<i32>,
    completed_step_ids: &[i32],
    completed_objective_ids: &[i32],
    active_objective_ids: &[i32],
    failed_objective_ids: &[i32],
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, mission_id, "MissionUpdate: no DB pool");
            return;
        }
    };

    let result = sqlx::query(
        "INSERT INTO sgw_mission (player_id, mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (player_id, mission_id) DO UPDATE SET \
         status = EXCLUDED.status, \
         current_step_id = EXCLUDED.current_step_id, \
         completed_step_ids = EXCLUDED.completed_step_ids, \
         completed_objective_ids = EXCLUDED.completed_objective_ids, \
         active_objective_ids = EXCLUDED.active_objective_ids, \
         failed_objective_ids = EXCLUDED.failed_objective_ids",
    )
    .bind(player_id)
    .bind(mission_id)
    .bind(status as i16)
    .bind(current_step_id)
    .bind(completed_step_ids)
    .bind(completed_objective_ids)
    .bind(active_objective_ids)
    .bind(failed_objective_ids)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(player_id, mission_id, status, "Mission state persisted"),
        Err(e) => tracing::error!(player_id, mission_id, "Failed to persist mission: {e}"),
    }
}

/// Persist a granted item to the inventory database.
async fn handle_grant_item(
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "GrantItem: no DB pool");
            return;
        }
    };

    // Find the next available slot in this container
    let next_slot: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(slot_id), -1) + 1 FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2",
    )
    .bind(player_id)
    .bind(container_id)
    .fetch_one(pool.as_ref())
    .await
    .unwrap_or(0);

    let result = sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
         bound, durability, charges) VALUES ($1, $2, $3, $4, $5, false, 100, 0)",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(count)
    .bind(next_slot)
    .bind(container_id)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(player_id, item_id, container_id, slot = next_slot, "Item persisted to inventory"),
        Err(e) => tracing::error!(player_id, item_id, "Failed to persist item: {e}"),
    }
}

/// Handle a mail request from CellService by querying the DB and sending results to the client.
pub(crate) async fn handle_mail_request(
    entity_id: u32,
    op: MailOp,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    use crate::cell::mail;

    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(entity_id, "Mail request: no DB pool available");
            return;
        }
    };

    // Get the player's character_id (player_id) from their account via entity lookup
    let account_id = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => { tracing::warn!(entity_id, "Mail: no client addr"); return; }
        };
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => c.account_id,
            None => { tracing::warn!(entity_id, "Mail: client not found"); return; }
        }
    };

    // Resolve player_id from account_id
    let player_id = match sqlx::query_scalar::<_, i32>(
        "SELECT player_id FROM sgw_player WHERE account_id = $1 ORDER BY player_id LIMIT 1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(pid)) => pid,
        _ => {
            tracing::warn!(entity_id, account_id, "Mail: could not resolve player_id");
            return;
        }
    };

    // Get player name for mail read responses
    let player_name = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => { tracing::warn!(entity_id, "Mail: no addr for player name lookup"); return; }
        };
        let clients = connected.lock().unwrap();
        clients.get(&addr)
            .and_then(|c| c.player_name.clone())
            .unwrap_or_default()
    };

    match op {
        MailOp::RequestHeaders { b_archive } => {
            tracing::debug!(entity_id, player_id, b_archive, "Mail: querying headers");

            #[derive(sqlx::FromRow)]
            struct MailRow {
                mail_id: i32,
                sender_name: String,
                sender_id: Option<i32>,
                subject: String,
                cash: i64,
                sent_time: i32,
                read_time: i32,
                flags: i32,
            }

            let rows = sqlx::query_as::<_, MailRow>(
                "SELECT mail_id, sender_name, sender_id, subject, cash, sent_time, read_time, flags \
                 FROM sgw_gate_mail WHERE character_id = $1 ORDER BY mail_id DESC",
            )
            .bind(player_id)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();

            let headers: Vec<mail::MailHeader> = rows.iter().map(|r| mail::MailHeader {
                id: r.mail_id,
                from_text: r.sender_name.clone(),
                from_id: r.sender_id.unwrap_or(0),
                subject_text: r.subject.clone(),
                cash: r.cash as i32,
                sent_time: r.sent_time as f32,
                read_time: r.read_time as f32,
                flags: r.flags,
            }).collect();

            tracing::debug!(entity_id, count = headers.len(), "Mail: sending headers to client");

            let args = mail::serialize_on_mail_header_info(b_archive, &headers);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury::method_idx::ON_MAIL_HEADER_INFO, &args)
                },
            ).await;
        }

        MailOp::RequestBody { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: querying body");

            #[derive(sqlx::FromRow)]
            struct BodyRow {
                message: String,
            }

            match sqlx::query_as::<_, BodyRow>(
                "SELECT message FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .fetch_optional(pool.as_ref())
            .await
            {
                Ok(Some(row)) => {
                    // Mark as read (set read_time if not already set)
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i32;
                    let _ = sqlx::query(
                        "UPDATE sgw_gate_mail SET read_time = $1 WHERE mail_id = $2 AND read_time = 0",
                    )
                    .bind(now)
                    .bind(mail_id)
                    .execute(pool.as_ref())
                    .await;

                    let args = mail::serialize_on_mail_read(mail_id, &row.message, &player_name);
                    send_to_witness(
                        socket, connected, entity_to_addr, entity_id,
                        |key, seq, acks| {
                            build_entity_method_packet(key, seq, acks, entity_id,
                                crate::mercury::method_idx::ON_MAIL_READ, &args)
                        },
                    ).await;
                }
                _ => {
                    tracing::warn!(entity_id, mail_id, "Mail body not found");
                }
            }
        }

        MailOp::Delete { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: deleting");
            let _ = sqlx::query(
                "DELETE FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .execute(pool.as_ref())
            .await;

            // Notify client to remove the header
            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury::method_idx::ON_MAIL_HEADER_REMOVE, &args)
                },
            ).await;
        }

        MailOp::Archive { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: archiving");
            // Set the MAIL_Archive flag (bit 0)
            let _ = sqlx::query(
                "UPDATE sgw_gate_mail SET flags = flags | 1 WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .execute(pool.as_ref())
            .await;

            // Notify client to remove the header from inbox
            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury::method_idx::ON_MAIL_HEADER_REMOVE, &args)
                },
            ).await;
        }
    }
}

/// Query player load data using just the account_id (for gate travel where we
/// don't have the player_id readily available in ConnectedClientState).
async fn query_player_load_data_by_account(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
) -> PlayerLoadData {
    let pool = match db_pool {
        Some(p) => p,
        None => return default_player_load_data(),
    };

    #[derive(sqlx::FromRow)]
    struct PlayerRow {
        player_id: i32,
    }

    // Find the most recently used character for this account
    match sqlx::query_as::<_, PlayerRow>(
        "SELECT player_id FROM sgw_player WHERE account_id = $1 ORDER BY player_id LIMIT 1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => query_player_load_data(db_pool, account_id, row.player_id).await,
        _ => default_player_load_data(),
    }
}

/// Query the character's world entry data from the database and allocate a player entity ID.
///
/// If a CellService channel is available, sends `CreateEntity` to resolve the space_id
/// dynamically. Otherwise falls back to the hardcoded space ID table.
pub(crate) async fn query_world_entry(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    player_id: i32,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> WorldEntryInfo {
    let player_eid = entity_manager.lock().unwrap().create_entity("SGWPlayer").0 as u32;

    let default_entry = || WorldEntryInfo {
        player_entity_id: player_eid,
        space_id: DEFAULT_SPACE_ID,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".to_string(),
    };

    let pool = match db_pool {
        Some(p) => p,
        None => return default_entry(),
    };

    #[derive(sqlx::FromRow)]
    struct EntryRow {
        world_location: String,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
    }

    match sqlx::query_as::<_, EntryRow>(
        "SELECT world_location, pos_x, pos_y, pos_z \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => {
            let pos = [row.pos_x, row.pos_y, row.pos_z];

            // Resolve space_id via CellService if available, else fall back
            // to the legacy hardcoded table.
            let space_id = if let Some(tx) = cell_tx {
                // Send CreateEntity with a oneshot so we can await the
                // resolved space_id before building the wire packet.
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx.send(BaseToCellMsg::CreateEntity {
                    entity_id: player_eid,
                    world_name: row.world_location.clone(),
                    position: pos,
                    rotation: [0.0; 3],
                    reply_tx,
                }).await.is_ok() {
                    match reply_rx.await {
                        Ok(sid) => sid,
                        Err(_) => {
                            tracing::warn!(world = %row.world_location, "CellService oneshot dropped -- using fallback");
                            resolve_space_id_fallback(&row.world_location)
                        }
                    }
                } else {
                    resolve_space_id_fallback(&row.world_location)
                }
            } else {
                resolve_space_id_fallback(&row.world_location)
            };

            WorldEntryInfo {
                player_entity_id: player_eid,
                space_id,
                pos,
                rot: [0.0; 3],
                world_name: row.world_location.clone(),
            }
        }
        Ok(None) => {
            tracing::warn!(player_id, account_id, "Character not found for world entry");
            default_entry()
        }
        Err(e) => {
            tracing::error!("Failed to query world entry: {e}");
            default_entry()
        }
    }
}

// ── Player load data query ───────────────────────────────────────────────────

/// Query full player data from the database for the mapLoaded sequence.
///
/// Returns all fields needed by [`build_map_loaded`]: level, name, archetype,
/// appearance, abilities, inventory stubs, experience, etc.
pub(crate) async fn query_player_load_data(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    player_id: i32,
) -> PlayerLoadData {
    let pool = match db_pool {
        Some(p) => p,
        None => return default_player_load_data(),
    };

    #[derive(sqlx::FromRow)]
    struct PlayerRow {
        level: i32,
        player_name: String,
        extra_name: String,
        alignment: i32,
        archetype: i32,
        gender: i32,
        bodyset: String,
        components: Vec<String>,
        exp: i32,
        naquadah: i32,
        known_stargates: Vec<i32>,
        abilities: Vec<i32>,
        training_points: i32,
        applied_science_points: i32,
        blueprint_ids: Vec<i32>,
        first_login: i32,
        access_level: i32,
        skin_color_id: i32,
        bandolier_slot: i32,
    }

    match sqlx::query_as::<_, PlayerRow>(
        "SELECT level, player_name, extra_name, alignment, archetype, gender, \
         bodyset, components, exp, naquadah, known_stargates, abilities, \
         training_points, applied_science_points, blueprint_ids, first_login, \
         access_level, skin_color_id, bandolier_slot \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => {
            tracing::debug!(
                player_id, level = row.level, archetype = row.archetype,
                name = %row.player_name, "Loaded player data for mapLoaded"
            );
            // Query inventory items for this character
            let items = query_inventory_items(pool.as_ref(), player_id).await;
            tracing::debug!(player_id, item_count = items.len(), "Loaded inventory items");

            // Merge equipped item visual components into body components
            // (matches requestCharacterVisuals / Inventory.py:462-465)
            let mut components = row.components;
            let item_visuals: Vec<String> = sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.character_id = $1 \
                   AND ri.visual_component IS NOT NULL \
                   AND ( \
                     (inv.container_id IN (3,4,5,6,7,8,9,10,11,12,13,14) AND inv.slot_id = 0) \
                     OR (inv.container_id = 3 AND inv.slot_id = $2) \
                   )",
            )
            .bind(player_id)
            .bind(row.bandolier_slot)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();
            if !item_visuals.is_empty() {
                tracing::debug!(player_id, visuals = ?item_visuals, "Equipped item visual components");
            }
            components.extend(item_visuals);

            PlayerLoadData {
                player_id,
                level: row.level,
                player_name: row.player_name,
                extra_name: row.extra_name,
                alignment: row.alignment,
                archetype: row.archetype,
                gender: row.gender,
                bodyset: row.bodyset,
                components,
                exp: row.exp,
                naquadah: row.naquadah,
                known_stargates: row.known_stargates,
                abilities: row.abilities,
                training_points: row.training_points,
                applied_science_points: row.applied_science_points,
                blueprint_ids: row.blueprint_ids,
                first_login: row.first_login,
                access_level: row.access_level,
                skin_color_id: row.skin_color_id,
                ability_tree: archetype_ability_tree(row.archetype),
                items,
            }
        }
        Ok(None) => {
            tracing::warn!(player_id, account_id, "Player not found for mapLoaded");
            default_player_load_data()
        }
        Err(e) => {
            tracing::error!(player_id, "Failed to query player load data: {e}");
            default_player_load_data()
        }
    }
}

/// Query inventory items from `sgw_inventory` for a character.
///
/// Returns `InvItem` structs ready for wire serialization via `onUpdateItem`.
/// Note: `slot_id` is stored 0-indexed in DB but sent 1-indexed on the wire
/// (Python: `'slotID': self.slotId + 1`).
async fn query_inventory_items(pool: &PgPool, player_id: i32) -> Vec<cimmeria_entity::inventory::InvItem> {
    #[derive(sqlx::FromRow)]
    struct InvRow {
        item_id: i32,
        type_id: i32,
        stack_size: i32,
        slot_id: i32,
        container_id: i32,
        bound: bool,
        durability: i32,
        charges: i32,
    }

    match sqlx::query_as::<_, InvRow>(
        "SELECT item_id, type_id, stack_size, slot_id, container_id, bound, durability, charges \
         FROM sgw_inventory WHERE character_id = $1 ORDER BY container_id, slot_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|r| cimmeria_entity::inventory::InvItem {
                id: r.item_id,
                dbid: r.type_id,
                stack_size: r.stack_size,
                slot_id: r.slot_id + 1, // DB is 0-indexed, wire is 1-indexed
                container_id: r.container_id,
                is_bound: r.bound,
                durability: r.durability,
                ammo_types: vec![], // TODO: parse EAmmoType[] from DB
                cur_ammo_type: 0,   // TODO: parse EAmmoType enum from DB
                charges: r.charges,
            })
            .collect(),
        Err(e) => {
            tracing::error!(player_id, "Failed to query inventory items: {e}");
            vec![]
        }
    }
}

/// Default player load data when the DB is unavailable.
pub(crate) fn default_player_load_data() -> PlayerLoadData {
    PlayerLoadData {
        player_id: 0,
        level: 1,
        player_name: "Unknown".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 1,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 1,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: archetype_ability_tree(1),
        items: vec![],
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
