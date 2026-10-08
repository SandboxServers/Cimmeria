//! Character creation handler — parse args, validate visuals, INSERT into DB.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::mercury::read_wstring;

use super::character::{query_character_list, send_char_create_failed};
use super::chardef::{chardef_lookup, CharDefIdentity};
use super::helpers::{
    drain_acks_and_seq, get_access_level, get_account_entity_id, get_enc_version,
};
use super::ConnectedClientState;

mod name;
mod start_profile;
mod starter_kit;
use name::validate_character_name;
use start_profile::{resolve_start, starting_points};
use starter_kit::{
    describe_abilities, describe_items, has_bandolier_weapon, insert_starter_inventory,
    record_profile_grants, start_kit, StarterItem,
};

/// Handle `createCharacter` (0xC4) -- parse args and INSERT into sgw_player.
#[tracing::instrument(
    name = "character.create",
    level = "info",
    skip_all,
    fields(peer = %addr, account_id, payload_len = payload.len()),
)]
pub(crate) async fn handle_create_character(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    create_character(
        transport, addr, key, account_id, payload, connected, db_pool, false,
    )
    .await
}

/// [`handle_create_character`] with a test-only override: `force_debug_kit`
/// adds the debug kit whatever the profile says (the seed-drift test creates
/// a debug-kit char_def 3 this way). Never set from access level (L2).
#[allow(clippy::too_many_arguments)]
pub(super) async fn create_character(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    force_debug_kit: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, "createCharacter: no DB pool");
            send_char_create_failed(transport, addr, key, connected, 3).await?;
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
            send_char_create_failed(transport, addr, key, connected, 2).await?;
            return Ok(());
        }
    };
    off += consumed;

    // Name validation (matches Python Account.py:isCharacterNameAllowed).
    if let Err(reason) = validate_character_name(&name) {
        tracing::info!(%addr, %name, %reason, "createCharacter: name rejected");
        send_char_create_failed(transport, addr, key, connected, 2).await?;
        return Ok(());
    }

    let (extra_name, consumed) = match read_wstring(payload, off) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%addr, "createCharacter: failed to parse extraName: {e}");
            send_char_create_failed(transport, addr, key, connected, 2).await?;
            return Ok(());
        }
    };
    off += consumed;

    // Extra name validation — same format rules, but allowed to be empty.
    if !extra_name.is_empty() {
        if let Err(reason) = validate_character_name(&extra_name) {
            tracing::info!(%addr, %extra_name, %reason, "createCharacter: extra_name rejected");
            send_char_create_failed(transport, addr, key, connected, 2).await?;
            return Ok(());
        }
    }

    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for CharDefId");
        send_char_create_failed(transport, addr, key, connected, 2).await?;
        return Ok(());
    }
    let char_def_id = i32::from_le_bytes([
        payload[off],
        payload[off + 1],
        payload[off + 2],
        payload[off + 3],
    ]);
    off += 4;

    // Parse ARRAY<VisualChoices> -- count + entries
    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for visuals count");
        send_char_create_failed(transport, addr, key, connected, 2).await?;
        return Ok(());
    }
    let visual_count = u32::from_le_bytes([
        payload[off],
        payload[off + 1],
        payload[off + 2],
        payload[off + 3],
    ]) as usize;
    off += 4;
    // Each VisualChoices = { VisGroupId: INT32, ChoiceId: INT32 } = 8 bytes
    if off + visual_count * 8 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for visual choices");
        send_char_create_failed(transport, addr, key, connected, 2).await?;
        return Ok(());
    }
    let mut visual_choices: Vec<(i32, i32)> = Vec::with_capacity(visual_count);
    for _ in 0..visual_count {
        let vis_group_id = i32::from_le_bytes([
            payload[off],
            payload[off + 1],
            payload[off + 2],
            payload[off + 3],
        ]);
        off += 4;
        let choice_id = i32::from_le_bytes([
            payload[off],
            payload[off + 1],
            payload[off + 2],
            payload[off + 3],
        ]);
        off += 4;
        visual_choices.push((vis_group_id, choice_id));
    }

    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for SkinTintColorID");
        send_char_create_failed(transport, addr, key, connected, 2).await?;
        return Ok(());
    }
    let skin_tint_color_id = i32::from_le_bytes([
        payload[off],
        payload[off + 1],
        payload[off + 2],
        payload[off + 3],
    ]);

    // Skin tint validation (matches Python Account.py: ERROR_CharacterCreationInvalidSkinColor).
    if !(0..=15).contains(&skin_tint_color_id) {
        tracing::info!(
            %addr,
            skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
            "createCharacter: invalid skin tint"
        );
        send_char_create_failed(transport, addr, key, connected, 2).await?;
        return Ok(());
    }

    // Identity from the CharDef table; where it starts is the start profile.
    let CharDefIdentity {
        alignment,
        archetype,
        gender,
        bodyset,
    } = match chardef_lookup(char_def_id) {
        Some(info) => info,
        None => {
            tracing::warn!(
                %addr,
                char_def_id, // nt:id-only CharDef rows carry no name column to pair
                "createCharacter: unknown CharDefId"
            );
            send_char_create_failed(transport, addr, key, connected, 2).await?;
            return Ok(());
        }
    };

    tracing::info!(
        %addr,
        player_name = %name,
        extra_name = %extra_name,
        char_def_id, // nt:id-only CharDef rows carry no name column to pair
        alignment,
        archetype,
        archetype_name = cimmeria_names::archetype_name(archetype),
        gender,
        bodyset,
        visual_count = visual_choices.len(),
        skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
        "Creating character"
    );

    // ── Resolve visual choices (matches CharacterCreation.py:getAllChoices) ───

    // Query all visual groups and their choices for this char_def_id
    let vg_rows = sqlx::query_as::<
        _,
        (
            i32,
            String,
            Option<i32>,
            Option<String>,
            Option<i32>,
            Option<bool>,
            Option<i32>,
        ),
    >(
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
            send_char_create_failed(transport, addr, key, connected, 3).await?;
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
    let mut visgroups: std::collections::BTreeMap<i32, VisGroup> =
        std::collections::BTreeMap::new();
    for (vg_id, vis_type, choice_id, component, item_id, item_bound, item_durability) in &vg_rows {
        let group = visgroups.entry(*vg_id).or_insert_with(|| VisGroup {
            vis_type: vis_type.clone(),
            choices: std::collections::BTreeMap::new(),
        });
        if let (Some(cid), Some(comp)) = (choice_id, component) {
            group.choices.insert(
                *cid,
                ChoiceData {
                    component: comp.clone(),
                    item_id: *item_id,
                    item_bound: item_bound.unwrap_or(false),
                    item_durability: item_durability.unwrap_or(-1),
                },
            );
        }
    }

    // Validate client-provided choices and resolve forced groups
    struct ResolvedChoice {
        component: String,
        item_id: Option<i32>,
        item_bound: bool,
        item_durability: i32,
    }
    // Keyed in visual-group order (a BTreeMap, not a HashMap) so the
    // components array and the starter-item placement come out the same on
    // every run: two item choices can compete for one bag (glasses and an
    // accessory both want Face, 5), and which one wins must not depend on
    // hash order. The seeded characters copy this order.
    let mut resolved: std::collections::BTreeMap<i32, ResolvedChoice> =
        std::collections::BTreeMap::new();

    // Client choices must target VIS_Optional groups only
    for &(vg_id, choice_id) in &visual_choices {
        let group = match visgroups.get(&vg_id) {
            Some(g) => g,
            None => {
                tracing::warn!(
                    %addr,
                    vg_id, // nt:id-only visual groups carry no name column to pair
                    char_def_id, // nt:id-only CharDef rows carry no name column to pair
                    "Invalid visual group"
                );
                send_char_create_failed(transport, addr, key, connected, 10003).await?;
                return Ok(());
            }
        };
        if group.vis_type != "VIS_Optional" {
            tracing::warn!(
                %addr,
                vg_id, // nt:id-only visual groups carry no name column to pair
                "Choice not allowed for forced visual group"
            );
            send_char_create_failed(transport, addr, key, connected, 10003).await?;
            return Ok(());
        }
        let choice = match group.choices.get(&choice_id) {
            Some(c) => c,
            None => {
                tracing::warn!(
                    %addr,
                    vg_id, // nt:id-only visual groups carry no name column to pair
                    choice_id, // nt:id-only visual choices carry no name column to pair
                    "Invalid choice for visual group"
                );
                send_char_create_failed(transport, addr, key, connected, 10003).await?;
                return Ok(());
            }
        };
        resolved.insert(
            vg_id,
            ResolvedChoice {
                component: choice.component.clone(),
                item_id: choice.item_id,
                item_bound: choice.item_bound,
                item_durability: choice.item_durability,
            },
        );
    }

    // Auto-select forced groups; reject missing optional groups
    for (&vg_id, group) in &visgroups {
        if let std::collections::btree_map::Entry::Vacant(e) = resolved.entry(vg_id) {
            if group.vis_type == "VIS_Forced" {
                if let Some((_, choice)) = group.choices.iter().next() {
                    e.insert(ResolvedChoice {
                        component: choice.component.clone(),
                        item_id: choice.item_id,
                        item_bound: choice.item_bound,
                        item_durability: choice.item_durability,
                    });
                }
            } else {
                tracing::warn!(
                    %addr,
                    vg_id, // nt:id-only visual groups carry no name column to pair
                    char_def_id, // nt:id-only CharDef rows carry no name column to pair
                    "Missing choice for optional visual group"
                );
                send_char_create_failed(transport, addr, key, connected, 10000).await?;
                return Ok(());
            }
        }
    }

    // ── Separate body components from item components (Account.py:156-161) ───

    let mut body_components: Vec<String> = Vec::new();
    let mut item_choices: Vec<StarterItem> = Vec::new();

    for choice in resolved.values() {
        if let Some(item_id) = choice.item_id {
            item_choices.push(StarterItem {
                item_type_id: item_id,
                stack_size: 1,
                bound: Some(choice.item_bound),
                durability: choice.item_durability,
            });
        } else {
            body_components.push(choice.component.clone());
        }
    }

    // ── The start profile (Class Start v6 CS-02): world, point, level, kit ───

    let start = match resolve_start(pool.as_ref(), addr, char_def_id, force_debug_kit).await {
        Ok(start) => start,
        Err(code) => {
            send_char_create_failed(transport, addr, key, connected, code).await?;
            return Ok(());
        }
    };
    let world_location = start.profile.world.as_str();
    let world_id = Some(start.world_id);
    let [start_x, start_y, start_z] = start.profile.position;
    let start_level = start.profile.start_level;
    let (training_points, applied_science_points) = starting_points(start_level);
    let (starter_abilities, kit_items) = match start_kit(pool.as_ref(), &start).await {
        Ok(kit) => kit,
        Err(_) => {
            send_char_create_failed(transport, addr, key, connected, 3).await?;
            return Ok(());
        }
    };
    let abilities: Vec<i32> = starter_abilities.iter().map(|a| a.ability_id).collect();
    // The visual-choice items first (clothes onto the body), then the
    // profile's items and the debug kit (a weapon into the bandolier).
    let mut starter_items = item_choices;
    starter_items.extend(kit_items);

    tracing::debug!(
        %addr,
        char_def_id, // nt:id-only CharDef rows carry no name column to pair
        profile_id = %start.profile.profile_id, // nt:id-only profile key has no display name
        start_state = start.profile.start_state.as_str(),
        debug_kit = start.debug_kit.is_some(),
        components = ?body_components,
        item_count = starter_items.len(),
        world_id = ?world_id,
        world = world_location,
        ability_count = abilities.len(),
        "Resolved character creation visuals and start profile"
    );

    // ── INSERT into sgw_player with components, world_id, abilities ───

    // Stamp the new character's `access_level` from the account's session
    // level (loaded from `account.accesslevel` at login), mirroring the C++
    // server which passed the account access level into the character INSERT.
    // The persisted column is loaded at world entry (player_load) and sent to
    // the client as the `AccessLevel` entity property — propId 7 in the
    // mapLoaded block (`mercury::world_data::map_loaded`) — i.e. the
    // per-character marker the client uses for GM UI. (Server-side GM
    // authorization gates on the session level, not this column.) Without it,
    // a GM account's characters were created at access_level 0 and carried no
    // GM marker on every load.
    let access_level = get_access_level(connected, addr) as i32;

    // Account name is best-effort from the live session state; the creation
    // log lines and the Discord notification carry it.
    let account_name = connected
        .lock()
        .ok()
        .and_then(|c| c.get(&addr).and_then(|s| s.account_name.clone()));

    // The character row and its starter inventory commit together: a kit
    // that can't be placed rolls the character back and the client gets
    // the DB-error code, rather than a character that exists for good
    // without its pistol.
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                event = "character_create_failed",
                reason = "db_error",
                %addr,
                account_id,
                account_name = account_name.as_deref(),
                error = %e,
                "character_create: could not open the creation transaction"
            );
            send_char_create_failed(transport, addr, key, connected, 3).await?;
            return Ok(());
        }
    };

    let result = sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_player \
         (account_id, player_name, extra_name, alignment, archetype, gender, \
          world_location, bodyset, level, title, pos_x, pos_y, pos_z, \
          skin_color_id, components, world_id, abilities, access_level, training_points, \
          applied_science_points, debug_kit) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $20, 0, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19) \
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
    .bind(access_level)
    // v2 economy (D-AT02): one training point and one Applied Science Point
    // per level, so a level-1 start has 1 of each. The column defaults are 0,
    // so these binds are what give them.
    .bind(training_points)
    .bind(applied_science_points)
    // Lock L2: the GM / Debug NPC reset gives a debug-kit character its kit
    // back. From the profile (or the test override), never access level.
    .bind(start.debug_kit.is_some())
    // The profile's own level, never a mission's (CS-02).
    .bind(start_level)
    .fetch_one(&mut *tx)
    .await;

    match result {
        Ok(player_id) => {
            // ── Insert starter items into sgw_inventory (Account.py:182-207) ───

            let placed =
                match insert_starter_inventory(&mut tx, addr, player_id, &name, &starter_items)
                    .await
                {
                    Ok(placed) => placed,
                    Err(_) => {
                        // `insert_starter_inventory` logged the item and reason.
                        let _ = tx.rollback().await;
                        send_char_create_failed(transport, addr, key, connected, 3).await?;
                        return Ok(());
                    }
                };
            // Provenance for the profile's racial_core / signature / ...
            // grants (CS-01a table), in the same transaction.
            if record_profile_grants(&mut tx, addr, player_id, &start.profile)
                .await
                .is_err()
            {
                let _ = tx.rollback().await;
                send_char_create_failed(transport, addr, key, connected, 3).await?;
                return Ok(());
            }
            if let Err(e) = tx.commit().await {
                tracing::error!(
                    event = "character_create_failed",
                    reason = "db_error",
                    %addr,
                    account_id,
                    account_name = account_name.as_deref(),
                    player_id,
                    player_name = %name,
                    error = %e,
                    "character_create: commit failed"
                );
                send_char_create_failed(transport, addr, key, connected, 3).await?;
                return Ok(());
            }

            // One line with everything the character starts with, named
            // (Rule 6), so "why can't lomiada fire?" is one SigNoz query.
            tracing::info!(
                event = "character_created",
                %addr,
                account_id,
                account_name = account_name.as_deref(),
                player_id,
                player_name = %name,
                char_def_id, // nt:id-only char_def rows carry no name column
                archetype,
                archetype_name = cimmeria_names::archetype_name(archetype),
                world_id = ?world_id,
                world = world_location,
                profile_id = %start.profile.profile_id, // nt:id-only profile key has no display name
                start_state = start.profile.start_state.as_str(),
                debug_kit = start.debug_kit.is_some(),
                level = start_level,
                abilities = %describe_abilities(&starter_abilities),
                items = %describe_items(&placed),
                armed = has_bandolier_weapon(&placed),
                "Character created successfully"
            );

            // Discord gameplay-channel: a new character was created (on by
            // default — low volume / high signal).
            cimmeria_discord::emit_character_created(
                cimmeria_discord::Named::new(account_id, account_name),
                cimmeria_discord::Named::new(player_id, Some(name.clone())),
                cimmeria_discord::Named::new(
                    archetype,
                    cimmeria_names::archetype_name(archetype).map(str::to_string),
                ),
                cimmeria_discord::Named::from_parts(
                    world_id.map(i64::from),
                    Some(world_location.to_string()),
                ),
            );

            // Send updated character list (Account entity already exists)
            let characters = query_character_list(
                db_pool,
                account_id,
                super::session_identity::identity_for_addr(connected, addr).account_name,
            )
            .await;
            let account_eid = get_account_entity_id(connected, addr)?;
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let enc_version = get_enc_version(connected, addr);
            let pkt = crate::mercury::build_on_character_list(
                &key,
                seq,
                &acks,
                &characters,
                account_eid,
                enc_version,
            );
            tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list");
            transport.send_to(&pkt, addr).await?;
        }
        Err(e) => {
            let error_str = e.to_string();
            let error_code = if error_str.contains("sgw_player_player_name_key") {
                tracing::info!(%addr, player_name = %name, "Character name already taken");
                1 // name taken
            } else {
                tracing::error!(%addr, error = %e, "Character creation DB error");
                3 // DB error
            };
            send_char_create_failed(transport, addr, key, connected, error_code).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod live_db_tests;

#[cfg(test)]
mod seed_parity_live_db_tests;

#[cfg(test)]
mod kit_rollback_live_db_tests;

#[cfg(test)]
mod profile_live_db_tests;
