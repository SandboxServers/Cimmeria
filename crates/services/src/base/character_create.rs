//! Character creation handler — parse args, validate visuals, INSERT into DB.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::mercury::read_wstring;

use super::character::{query_character_list, send_char_create_failed};
use super::chardef::chardef_lookup;
use super::helpers::{drain_acks_and_seq, get_access_level, get_account_entity_id};
use super::resources::{bag_min_slot, pick_first_open_bag, BAG_FILL_ORDER};
use super::ConnectedClientState;

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
        tracing::info!(%addr, skin_tint_color_id, "createCharacter: invalid skin tint");
        send_char_create_failed(transport, addr, key, connected, 2).await?;
        return Ok(());
    }

    // Derive alignment, archetype, gender, bodyset, starting position from CharDefId.
    let (alignment, archetype, gender, bodyset, world_location, start_x, start_y, start_z) =
        match chardef_lookup(char_def_id) {
            Some(info) => info,
            None => {
                tracing::warn!(%addr, char_def_id, "createCharacter: unknown CharDefId");
                send_char_create_failed(transport, addr, key, connected, 2).await?;
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
    let mut resolved: HashMap<i32, ResolvedChoice> = HashMap::new();

    // Client choices must target VIS_Optional groups only
    for &(vg_id, choice_id) in &visual_choices {
        let group = match visgroups.get(&vg_id) {
            Some(g) => g,
            None => {
                tracing::warn!(%addr, vg_id, char_def_id, "Invalid visual group");
                send_char_create_failed(transport, addr, key, connected, 10003).await?;
                return Ok(());
            }
        };
        if group.vis_type != "VIS_Optional" {
            tracing::warn!(%addr, vg_id, "Choice not allowed for forced visual group");
            send_char_create_failed(transport, addr, key, connected, 10003).await?;
            return Ok(());
        }
        let choice = match group.choices.get(&choice_id) {
            Some(c) => c,
            None => {
                tracing::warn!(%addr, vg_id, choice_id, "Invalid choice for visual group");
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
        if let std::collections::hash_map::Entry::Vacant(e) = resolved.entry(vg_id) {
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
                tracing::warn!(%addr, vg_id, char_def_id, "Missing choice for optional visual group");
                send_char_create_failed(transport, addr, key, connected, 10000).await?;
                return Ok(());
            }
        }
    }

    // ── Separate body components from item components (Account.py:156-161) ───

    let mut body_components: Vec<String> = Vec::new();
    struct ItemChoice {
        item_id: i32,
        item_bound: bool,
        item_durability: i32,
    }
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

    let world_id: Option<i32> =
        match sqlx::query_scalar("SELECT world_id FROM resources.worlds WHERE world = $1")
            .bind(world_location)
            .fetch_optional(pool.as_ref())
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    world_location,
                    "character_create: world_id lookup failed: {e}"
                );
                None
            }
        };

    // ── Look up starting abilities (Account.py:166) ───

    let abilities: Vec<i32> = match sqlx::query_scalar(
        "SELECT ability_id FROM resources.char_creation_abilities WHERE char_def_id = $1",
    )
    .bind(char_def_id)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                char_def_id,
                "character_create: starting abilities lookup failed: {e}"
            );
            Vec::new()
        }
    };

    tracing::debug!(
        %addr, char_def_id,
        components = ?body_components,
        item_count = item_choices.len(),
        world_id = ?world_id,
        ability_count = abilities.len(),
        "Resolved character creation visuals"
    );

    // ── INSERT into sgw_player with components, world_id, abilities ───

    // KI-11 fix: stamp the new character's `access_level` from the
    // account's session level (loaded from `account.accesslevel` at login),
    // mirroring the C++ server which passed the account access level into
    // the character INSERT. Without this, a GM account created normal
    // (access_level 0) characters, so the character never carried GM
    // authority into the cell entity or the client's AccessLevel property.
    let access_level = get_access_level(connected, addr) as i32;

    let result = sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_player \
         (account_id, player_name, extra_name, alignment, archetype, gender, \
          world_location, bodyset, level, title, pos_x, pos_y, pos_z, \
          skin_color_id, components, world_id, abilities, access_level) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 0, $9, $10, $11, $12, $13, $14, $15, $16) \
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

                // Pick the first bag that's both valid for this item AND
                // still has room. Pre-fix this picked the first valid bag
                // unconditionally and `continue`d if it was full — so an
                // item that could overflow to a later bag was silently
                // dropped (live observation 2026-06-02: item 4343 lost
                // at character create because its primary bag filled up
                // first while a later valid bag still had room).
                let bag_id = match pick_first_open_bag(&container_sets, &slot_indices) {
                    Some(bag) => bag,
                    None => {
                        // Either the item has no valid container (content
                        // gap) or every valid container is genuinely full.
                        // Both are operator-actionable — surface the
                        // distinction so a real content gap doesn't get
                        // confused with "too many starter items."
                        let any_valid_container =
                            BAG_FILL_ORDER.iter().any(|b| container_sets.contains(b));
                        if any_valid_container {
                            tracing::warn!(
                                %addr,
                                item_id = item.item_id,
                                "All valid containers full — starter item dropped"
                            );
                        } else {
                            tracing::warn!(
                                %addr,
                                item_id = item.item_id,
                                "No valid container for starter item"
                            );
                        }
                        continue;
                    }
                };

                let entry = slot_indices
                    .entry(bag_id)
                    .or_insert_with(|| bag_min_slot(bag_id));
                let current_slot = *entry;
                *entry += 1;

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
            let pkt =
                crate::mercury::build_on_character_list(&key, seq, &acks, &characters, account_eid);
            tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list");
            transport.send_to(&pkt, addr).await?;
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
            send_char_create_failed(transport, addr, key, connected, error_code).await?;
        }
    }

    Ok(())
}

/// Validate a character name for length, format, and whitespace rules.
///
/// Allowed characters: ASCII letters, digits, spaces, hyphens, apostrophes.
/// Rejects: leading/trailing whitespace, consecutive spaces, control chars,
/// HTML/script injection, zero-width characters, and names outside 3-20 chars.
///
/// Returns `Ok(())` if valid, or `Err(reason)` with a human-readable rejection reason.
fn validate_character_name(name: &str) -> Result<(), &'static str> {
    if name.len() < 3 {
        return Err("too short (min 3)");
    }
    if name.len() > 20 {
        return Err("too long (max 20)");
    }
    if name != name.trim() {
        return Err("leading or trailing whitespace");
    }
    if name.contains("  ") {
        return Err("consecutive spaces");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '\'')
    {
        return Err("invalid characters (only letters, digits, spaces, hyphens, apostrophes)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_valid() {
        assert!(validate_character_name("John").is_ok());
        assert!(validate_character_name("Sam Carter").is_ok());
        assert!(validate_character_name("O'Neill").is_ok());
        assert!(validate_character_name("Teal-c").is_ok());
        assert!(validate_character_name("abc").is_ok()); // min length
        assert!(validate_character_name("12345678901234567890").is_ok()); // max length (20)
    }

    #[test]
    fn name_too_short() {
        assert!(validate_character_name("AB").is_err());
        assert!(validate_character_name("A").is_err());
        assert!(validate_character_name("").is_err());
    }

    #[test]
    fn name_too_long() {
        assert!(validate_character_name("123456789012345678901").is_err()); // 21 chars
        assert!(validate_character_name("AAAAAAAAAAAAAAAAAAAAA").is_err());
    }

    #[test]
    fn name_rejects_html() {
        assert!(validate_character_name("<script>").is_err());
        assert!(validate_character_name("a]>b").is_err());
    }

    #[test]
    fn name_rejects_control_chars() {
        assert!(validate_character_name("abc\0def").is_err());
        assert!(validate_character_name("abc\ndef").is_err());
        assert!(validate_character_name("abc\tdef").is_err());
    }

    #[test]
    fn name_rejects_bad_whitespace() {
        assert!(validate_character_name(" Leading").is_err());
        assert!(validate_character_name("Trailing ").is_err());
        assert!(validate_character_name("Two  Spaces").is_err());
    }

    #[test]
    fn name_rejects_non_ascii() {
        assert!(validate_character_name("Ünïcödé").is_err());
        assert!(validate_character_name("名前").is_err());
    }

    #[test]
    fn skin_tint_valid_range() {
        for i in 0..=15i32 {
            assert!((0..=15).contains(&i));
        }
        assert!(!(0..=15).contains(&-1i32));
        assert!(!(0..=15).contains(&16i32));
    }
}
