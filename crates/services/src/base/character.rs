use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use crate::mercury::{
    build_char_create_failed, build_character_visuals, build_on_character_list,
    read_wstring, CharacterInfo, SKIN_TINTS,
};

use super::ConnectedClientState;
use super::chardef::chardef_lookup;
use super::helpers::{drain_acks_and_seq, get_account_entity_id};
use super::resources::{bag_max_slots, BAG_FILL_ORDER, INV_BANDOLIER};

/// Query the character list from the database.
pub(crate) async fn query_character_list(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
) -> Vec<CharacterInfo> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!("No DB pool -- returning empty character list");
            return Vec::new();
        }
    };

    #[derive(sqlx::FromRow)]
    struct CharRow {
        player_id: i32,
        player_name: String,
        extra_name: String,
        alignment: i32,
        level: i32,
        gender: i32,
        world_location: String,
        archetype: i32,
        title: i32,
    }

    tracing::debug!(account_id, "Querying sgw_player for character list");

    match sqlx::query_as::<_, CharRow>(
        "SELECT player_id, player_name, extra_name, alignment, level, gender, \
         world_location, archetype, title \
         FROM sgw_player WHERE account_id = $1 ORDER BY player_id",
    )
    .bind(account_id as i32)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => {
            tracing::info!(account_id, count = rows.len(), "Character list query result");
            rows.into_iter()
                .map(|r| CharacterInfo {
                    player_id: r.player_id,
                    name: r.player_name,
                    extra_name: r.extra_name,
                    alignment: r.alignment as u8,
                    level: r.level as u8,
                    gender: r.gender as u8,
                    world_location: r.world_location,
                    archetype: r.archetype as u8,
                    title: r.title as u8,
                    player_type: 1,
                    playable: 1,
                })
                .collect()
        }
        Err(e) => {
            tracing::error!(account_id, "Failed to query character list: {e}");
            Vec::new()
        }
    }
}

/// Handle `createCharacter` (0xC4) -- parse args and INSERT into sgw_player.
pub(crate) async fn handle_create_character(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, "createCharacter: no DB pool");
            send_char_create_failed(socket, addr, key, connected, 3).await?;
            return Ok(());
        }
    };

    // Parse createCharacter args (from Account.def):
    // [WSTRING Name][WSTRING ExtraName][INT32 CharDefId][ARRAY<VisualChoices> VisualChoiceList][INT32 SkinTintColorID]
    let mut off = 0;

    let (name, consumed) = match read_wstring(payload, off) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%addr, "createCharacter: failed to parse name: {e}");
            send_char_create_failed(socket, addr, key, connected, 2).await?;
            return Ok(());
        }
    };
    off += consumed;

    // Name length validation (matches C++ Account.py: rejects names < 3 chars).
    if name.len() < 3 {
        tracing::info!(%addr, name_len = name.len(), "createCharacter: name too short");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }

    let (extra_name, consumed) = match read_wstring(payload, off) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%addr, "createCharacter: failed to parse extraName: {e}");
            send_char_create_failed(socket, addr, key, connected, 2).await?;
            return Ok(());
        }
    };
    off += consumed;

    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for CharDefId");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let char_def_id = i32::from_le_bytes([
        payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
    ]);
    off += 4;

    // Parse ARRAY<VisualChoices> -- count + entries
    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for visuals count");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let visual_count = u32::from_le_bytes([
        payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
    ]) as usize;
    off += 4;
    // Each VisualChoices = { VisGroupId: INT32, ChoiceId: INT32 } = 8 bytes
    if off + visual_count * 8 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for visual choices");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let mut visual_choices: Vec<(i32, i32)> = Vec::with_capacity(visual_count);
    for _ in 0..visual_count {
        let vis_group_id = i32::from_le_bytes([
            payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
        ]);
        off += 4;
        let choice_id = i32::from_le_bytes([
            payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
        ]);
        off += 4;
        visual_choices.push((vis_group_id, choice_id));
    }

    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for SkinTintColorID");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let skin_tint_color_id = i32::from_le_bytes([
        payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
    ]);

    // Derive alignment, archetype, gender, bodyset, starting position from CharDefId.
    let (alignment, archetype, gender, bodyset, world_location, start_x, start_y, start_z) =
        match chardef_lookup(char_def_id) {
            Some(info) => info,
            None => {
                tracing::warn!(%addr, char_def_id, "createCharacter: unknown CharDefId");
                send_char_create_failed(socket, addr, key, connected, 2).await?;
                return Ok(());
            }
        };

    tracing::info!(
        %addr,
        name = %name,
        extra_name = %extra_name,
        char_def_id,
        alignment,
        archetype,
        gender,
        bodyset,
        visual_count = visual_choices.len(),
        skin_tint_color_id,
        "Creating character"
    );

    // ── Resolve visual choices (matches CharacterCreation.py:getAllChoices) ───

    // Query all visual groups and their choices for this char_def_id
    let vg_rows = sqlx::query_as::<_, (i32, String, Option<i32>, Option<String>, Option<i32>, Option<bool>, Option<i32>)>(
        "SELECT vg.vis_group_id, vg.vis_type::text, \
                c.choice_id, c.component, c.item_id, c.item_bound, c.item_durability \
         FROM resources.char_creation_visgroups vg \
         LEFT JOIN resources.char_creation_choices c ON c.vis_group_id = vg.vis_group_id \
         WHERE vg.char_def_id = $1 \
         ORDER BY vg.vis_group_id, c.choice_id",
    )
    .bind(char_def_id)
    .fetch_all(pool.as_ref())
    .await;

    let vg_rows = match vg_rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(%addr, error = %e, "createCharacter: failed to query visgroups");
            send_char_create_failed(socket, addr, key, connected, 3).await?;
            return Ok(());
        }
    };

    // Build visgroup map: vis_group_id -> (vis_type, choices ordered by choice_id)
    struct ChoiceData {
        component: String,
        item_id: Option<i32>,
        item_bound: bool,
        item_durability: i32,
    }
    struct VisGroup {
        vis_type: String,
        choices: std::collections::BTreeMap<i32, ChoiceData>,
    }
    let mut visgroups: std::collections::BTreeMap<i32, VisGroup> = std::collections::BTreeMap::new();
    for (vg_id, vis_type, choice_id, component, item_id, item_bound, item_durability) in &vg_rows {
        let group = visgroups.entry(*vg_id).or_insert_with(|| VisGroup {
            vis_type: vis_type.clone(),
            choices: std::collections::BTreeMap::new(),
        });
        if let (Some(cid), Some(comp)) = (choice_id, component) {
            group.choices.insert(*cid, ChoiceData {
                component: comp.clone(),
                item_id: *item_id,
                item_bound: item_bound.unwrap_or(false),
                item_durability: item_durability.unwrap_or(-1),
            });
        }
    }

    // Validate client-provided choices and resolve forced groups
    struct ResolvedChoice {
        component: String,
        item_id: Option<i32>,
        item_bound: bool,
        item_durability: i32,
    }
    let mut resolved: HashMap<i32, ResolvedChoice> = HashMap::new();

    // Client choices must target VIS_Optional groups only
    for &(vg_id, choice_id) in &visual_choices {
        let group = match visgroups.get(&vg_id) {
            Some(g) => g,
            None => {
                tracing::warn!(%addr, vg_id, char_def_id, "Invalid visual group");
                send_char_create_failed(socket, addr, key, connected, 10003).await?;
                return Ok(());
            }
        };
        if group.vis_type != "VIS_Optional" {
            tracing::warn!(%addr, vg_id, "Choice not allowed for forced visual group");
            send_char_create_failed(socket, addr, key, connected, 10003).await?;
            return Ok(());
        }
        let choice = match group.choices.get(&choice_id) {
            Some(c) => c,
            None => {
                tracing::warn!(%addr, vg_id, choice_id, "Invalid choice for visual group");
                send_char_create_failed(socket, addr, key, connected, 10003).await?;
                return Ok(());
            }
        };
        resolved.insert(vg_id, ResolvedChoice {
            component: choice.component.clone(),
            item_id: choice.item_id,
            item_bound: choice.item_bound,
            item_durability: choice.item_durability,
        });
    }

    // Auto-select forced groups; reject missing optional groups
    for (&vg_id, group) in &visgroups {
        if !resolved.contains_key(&vg_id) {
            if group.vis_type == "VIS_Forced" {
                if let Some((_, choice)) = group.choices.iter().next() {
                    resolved.insert(vg_id, ResolvedChoice {
                        component: choice.component.clone(),
                        item_id: choice.item_id,
                        item_bound: choice.item_bound,
                        item_durability: choice.item_durability,
                    });
                }
            } else {
                tracing::warn!(%addr, vg_id, char_def_id, "Missing choice for optional visual group");
                send_char_create_failed(socket, addr, key, connected, 10000).await?;
                return Ok(());
            }
        }
    }

    // ── Separate body components from item components (Account.py:156-161) ───

    let mut body_components: Vec<String> = Vec::new();
    struct ItemChoice { item_id: i32, item_bound: bool, item_durability: i32 }
    let mut item_choices: Vec<ItemChoice> = Vec::new();

    for choice in resolved.values() {
        if let Some(item_id) = choice.item_id {
            item_choices.push(ItemChoice {
                item_id,
                item_bound: choice.item_bound,
                item_durability: choice.item_durability,
            });
        } else {
            body_components.push(choice.component.clone());
        }
    }

    // ── Look up world_id (Account.py:163) ───

    let world_id: Option<i32> = sqlx::query_scalar(
        "SELECT world_id FROM resources.worlds WHERE world = $1",
    )
    .bind(world_location)
    .fetch_optional(pool.as_ref())
    .await
    .unwrap_or(None);

    // ── Look up starting abilities (Account.py:166) ───

    let abilities: Vec<i32> = sqlx::query_scalar(
        "SELECT ability_id FROM resources.char_creation_abilities WHERE char_def_id = $1",
    )
    .bind(char_def_id)
    .fetch_all(pool.as_ref())
    .await
    .unwrap_or_default();

    tracing::debug!(
        %addr, char_def_id,
        components = ?body_components,
        item_count = item_choices.len(),
        world_id = ?world_id,
        ability_count = abilities.len(),
        "Resolved character creation visuals"
    );

    // ── INSERT into sgw_player with components, world_id, abilities ───

    let result = sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_player \
         (account_id, player_name, extra_name, alignment, archetype, gender, \
          world_location, bodyset, level, title, pos_x, pos_y, pos_z, \
          skin_color_id, components, world_id, abilities) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 0, $9, $10, $11, $12, $13, $14, $15) \
         RETURNING player_id",
    )
    .bind(account_id as i32)
    .bind(&name)
    .bind(&extra_name)
    .bind(alignment)
    .bind(archetype)
    .bind(gender)
    .bind(world_location)
    .bind(bodyset)
    .bind(start_x)
    .bind(start_y)
    .bind(start_z)
    .bind(skin_tint_color_id)
    .bind(&body_components)
    .bind(world_id)
    .bind(&abilities)
    .fetch_one(pool.as_ref())
    .await;

    match result {
        Ok(player_id) => {
            // ── Insert starter items into sgw_inventory (Account.py:182-207) ───

            let mut slot_indices: HashMap<i32, i32> = HashMap::new();
            for item in &item_choices {
                // Look up which containers this item can go into
                let container_sets = sqlx::query_scalar::<_, Vec<i32>>(
                    "SELECT container_sets FROM resources.items WHERE item_id = $1",
                )
                .bind(item.item_id)
                .fetch_optional(pool.as_ref())
                .await
                .ok()
                .flatten()
                .unwrap_or_default();

                // Find the best container from BagFillOrder
                let bag_id = match BAG_FILL_ORDER.iter().find(|&&bag| container_sets.contains(&bag)) {
                    Some(&bag) => bag,
                    None => {
                        tracing::warn!(%addr, item_id = item.item_id, "No valid container for starter item");
                        continue;
                    }
                };

                let entry = slot_indices.entry(bag_id).or_insert_with(|| {
                    if bag_id == INV_BANDOLIER { 1 } else { 0 }
                });
                let current_slot = *entry;
                *entry += 1;

                if *entry > bag_max_slots(bag_id) {
                    tracing::warn!(%addr, bag_id, item_id = item.item_id, "Bag full, skipping starter item");
                    continue;
                }

                if let Err(e) = sqlx::query(
                    "INSERT INTO sgw_inventory \
                     (container_id, slot_id, type_id, character_id, durability, bound) \
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(bag_id)
                .bind(current_slot)
                .bind(item.item_id)
                .bind(player_id)
                .bind(item.item_durability)
                .bind(item.item_bound)
                .execute(pool.as_ref())
                .await
                {
                    tracing::error!(%addr, item_id = item.item_id, error = %e, "Failed to insert starter item");
                }
            }

            tracing::info!(%addr, player_id, name = %name, "Character created successfully");

            // Send updated character list (Account entity already exists)
            let characters = query_character_list(db_pool, account_id).await;
            let account_eid = get_account_entity_id(connected, addr)?;
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_on_character_list(&key, seq, &acks, &characters, account_eid);
            tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list");
            socket.send_to(&pkt, addr).await?;
        }
        Err(e) => {
            let error_str = e.to_string();
            let error_code = if error_str.contains("sgw_player_player_name_key") {
                tracing::info!(%addr, name = %name, "Character name already taken");
                1 // name taken
            } else {
                tracing::error!(%addr, error = %e, "Character creation DB error");
                3 // DB error
            };
            send_char_create_failed(socket, addr, key, connected, error_code).await?;
        }
    }

    Ok(())
}

/// Send `onCharacterCreateFailed`.
pub(crate) async fn send_char_create_failed(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    error_code: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_char_create_failed(&key, seq, &acks, error_code, account_eid);
    socket.send_to(&pkt, addr).await?;
    Ok(())
}

/// Handle `deleteCharacter` (0xC5) -- delete a character and send updated list.
pub(crate) async fn handle_delete_character(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, "deleteCharacter: no DB pool");
            return Ok(());
        }
    };

    // Delete the character (only if it belongs to this account).
    let result = sqlx::query(
        "DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) => {
            if r.rows_affected() > 0 {
                tracing::info!(%addr, player_id, account_id, "Character deleted");
            } else {
                tracing::warn!(%addr, player_id, account_id, "Character not found or not owned");
            }
        }
        Err(e) => {
            tracing::error!(%addr, player_id, "Failed to delete character: {e}");
            return Ok(());
        }
    }

    // Send updated character list (Account entity already exists).
    let characters = query_character_list(db_pool, account_id).await;
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_on_character_list(&key, seq, &acks, &characters, account_eid);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list after delete");
    socket.send_to(&pkt, addr).await?;

    Ok(())
}

/// Handle `requestCharacterVisuals` (0xC6).
///
/// Queries the DB for the player's bodyset, components, and skin color,
/// then sends `onCharacterVisuals` (0x84) so the client can render the
/// character model on the select screen.
pub(crate) async fn handle_request_character_visuals(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, player_id, "requestCharacterVisuals: no DB pool");
            return Ok(());
        }
    };

    // Ownership check: only return visuals for characters belonging to this account.
    let account_id = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients.get(&addr).ok_or("addr not in connected map")?.account_id
    };

    let row = sqlx::query_as::<_, (String, Vec<String>, i32, i32)>(
        "SELECT bodyset, components, skin_color_id, bandolier_slot \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await;

    match row {
        Ok(Some((bodyset, mut components, skin_color_id, bandolier_slot))) => {
            // Load item visual components from equipped inventory (Account.py:246-258)
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
            .bind(bandolier_slot)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();

            components.extend(item_visuals);

            tracing::debug!(
                %addr, player_id, %bodyset,
                component_count = components.len(),
                skin_color_id,
                "Sending character visuals"
            );

            let skin_tint = SKIN_TINTS.get(skin_color_id as usize).copied().unwrap_or(0x2F1308FF);
            let account_eid = get_account_entity_id(connected, addr)?;
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_character_visuals(
                &key, seq, &acks,
                player_id,
                &bodyset,
                &components,
                0xFF, // primaryTint (matches C++ Account.py)
                0xFF, // secondaryTint (matches C++ Account.py)
                skin_tint, // skinTint -- RGBA from SKIN_TINTS lookup
                account_eid,
            );
            tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT onCharacterVisuals");
            socket.send_to(&pkt, addr).await?;
        }
        Ok(None) => {
            tracing::warn!(%addr, player_id, "requestCharacterVisuals: player not found");
        }
        Err(e) => {
            tracing::error!(%addr, player_id, error = %e, "requestCharacterVisuals: DB error");
        }
    }

    Ok(())
}
