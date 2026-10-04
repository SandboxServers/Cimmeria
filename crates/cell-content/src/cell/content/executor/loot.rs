//! `Action::OpenLoot` — a loot window on a live container, without killing
//! it. Decision (@Cadacious, 2026-09-28).
//!
//! Corpse loot is rolled on death into `CellEntity::loot` and shared. A
//! container rolls per looter, on open, into `CellEntity::container_loot`
//! keyed by `player_id`, so two players never share or take each other's
//! roll. The window is the corpse window: the same `onLootDisplay` bytes
//! (`cimmeria_wire::cell::loot`), and `lootItem` / Loot All take from the
//! looter's roll through `SpaceManager::loot_list_mut`.
//!
//! The gates, in order, each with visible feedback and a `reason=`:
//!
//! 1. The container is the chain's interact target, in range and in the
//!    player's space (`no_container`, `out_of_range`).
//! 2. A pending roll for this player reopens as it is, never re-rolled, when
//!    the action is once-per-character or reopen-only (`pending_reopened`).
//! 3. Reopen-only (`loot_table_id: None`) stops here (`nothing_pending`,
//!    `already_looted`).
//! 4. Once-per-character: the container key is already in
//!    `CellEntity::looted_containers` (`already_looted`).
//! 5. The table rolls; an empty roll opens nothing and sets no flag
//!    (`empty_roll`).
//!
//! Then the roll is stored, the once flag is set and persisted
//! (`CellToBaseMsg::ContainerLooted`), and the window opens.
//!
//! Loot left in the window when the player closes it stays pending for that
//! character for as long as the container entity lives (the client sends
//! nothing on close), so the next press reopens it. A server restart or a
//! fresh instance loses it; the once flag does not come back.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_entity::cell_entity::LootItem;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::loot::serialize_on_loot_display;
use tokio::sync::mpsc;

use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{interact_range, SpaceManager};

/// What the player reads when a press opens nothing.
pub(super) const ALREADY_LOOTED_TEXT: &str = "You have already taken everything from this.";
pub(super) const NOTHING_HERE_TEXT: &str = "There is nothing here for you right now.";
pub(super) const EMPTY_ROLL_TEXT: &str = "You search it, but find nothing of use.";
pub(super) const OUT_OF_RANGE_TEXT: &str = "You are too far away to search that.";

/// `Action::OpenLoot`.
#[tracing::instrument(
    name = "loot.container_open",
    level = "info",
    skip_all,
    fields(entity_id, player_id, chain_id, loot_table_id = tracing::field::Empty)
)]
pub(super) async fn open_loot(
    action: Action,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    params: &HashMap<String, serde_json::Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Action::OpenLoot {
        loot_table_id,
        once_per_character,
        container_key,
    } = action
    else {
        return;
    };
    tracing::Span::current().record("loot_table_id", tracing::field::debug(loot_table_id));
    let id = space_mgr.player_identity(entity_id);
    let refuse = |space_mgr: &SpaceManager,
                  reason: &'static str,
                  container: Option<u32>,
                  key: Option<&str>| {
        let names = cimmeria_names::book();
        tracing::info!(
            target: "loot",
            event = "loot.container_refused",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id,
            player_name = id.player_name,
            container_entity_id = container,
            container_entity_name = container.and_then(|c| space_mgr.entity_names(c).entity_name),
            container_key = key,
            loot_table_id,
            loot_table_name = loot_table_id.and_then(|t| names.loot_table(t)),
            chain_id,
            chain_name = names.chain(chain_id),
            reason,
            "open_loot: nothing rolled"
        );
    };

    // 1. The container: the interact target this chain fired on, else the
    // player's interaction pin (a follow-up chain).
    let container = params
        .get("target_entity_id")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .or_else(|| {
            space_mgr
                .get_entity(entity_id)
                .and_then(|e| e.last_interaction_target)
        });
    let Some(container) = container.filter(|&c| c != entity_id) else {
        refuse(space_mgr, "no_container", None, None);
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            reason = "no_container",
            "open_loot fired with no interact target -- bind it to an interact_tag chain"
        );
        return;
    };
    if let Err(fail) = interact_range(entity_id, container, space_mgr) {
        refuse(space_mgr, "out_of_range", Some(container), None);
        tracing::debug!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            container,
            container_name = space_mgr.entity_names(container).entity_name,
            ?fail,
            "open_loot: container out of range"
        );
        feedback(entity_id, OUT_OF_RANGE_TEXT, tx, space_mgr).await;
        return;
    }
    let key = container_key.or_else(|| space_mgr.get_entity(container).and_then(|c| c.tag.clone()));
    let key_ref = key.as_deref();
    if once_per_character && key.is_none() {
        refuse(space_mgr, "no_container_key", Some(container), None);
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            container,
            container_name = space_mgr.entity_names(container).entity_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            reason = "no_container_key",
            "open_loot: once_per_character needs a container_key or a tagged container"
        );
        feedback(entity_id, NOTHING_HERE_TEXT, tx, space_mgr).await;
        return;
    }
    let already = key_ref.is_some_and(|k| {
        space_mgr
            .get_entity(entity_id)
            .is_some_and(|p| p.looted_containers.contains(k))
    });

    // 2. A pending roll reopens untouched.
    let pending = space_mgr.loot_view(container, Some(player_id));
    if !pending.is_empty() && (once_per_character || loot_table_id.is_none()) {
        tracing::info!(
            target: "loot",
            event = "loot.container_reopened",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id,
            player_name = id.player_name,
            container_entity_id = container,
            container_entity_name = space_mgr.entity_names(container).entity_name,
            container_key = key_ref,
            item_count = pending.len(),
            reason = "pending_reopened",
            "open_loot: reopening this player's pending roll"
        );
        show(entity_id, container, &pending, tx, space_mgr).await;
        return;
    }

    // 3. Reopen-only.
    let Some(table_id) = loot_table_id else {
        let (reason, text) = if already {
            ("already_looted", ALREADY_LOOTED_TEXT)
        } else {
            ("nothing_pending", NOTHING_HERE_TEXT)
        };
        refuse(space_mgr, reason, Some(container), key_ref);
        feedback(entity_id, text, tx, space_mgr).await;
        return;
    };

    // 4. Once per character.
    if once_per_character && already {
        refuse(space_mgr, "already_looted", Some(container), key_ref);
        feedback(entity_id, ALREADY_LOOTED_TEXT, tx, space_mgr).await;
        return;
    }

    // 5. Roll.
    let Some(entries) = space_mgr.loot_tables.get(&table_id).cloned() else {
        refuse(space_mgr, "unknown_loot_table", Some(container), key_ref);
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            loot_table_id = table_id,
            loot_table_name = cimmeria_names::book().loot_table(table_id),
            reason = "unknown_loot_table",
            "open_loot: the loot table has no rows -- check db/resources/Loot/Seed"
        );
        feedback(entity_id, EMPTY_ROLL_TEXT, tx, space_mgr).await;
        return;
    };
    let rolled = crate::cell::abilities::roll_loot_entries(&entries, container, table_id);
    if rolled.is_empty() {
        refuse(space_mgr, "empty_roll", Some(container), key_ref);
        feedback(entity_id, EMPTY_ROLL_TEXT, tx, space_mgr).await;
        return;
    }

    let items: Vec<LootItem> = {
        let Some(c) = space_mgr.get_entity_mut(container) else {
            return;
        };
        let items: Vec<LootItem> = rolled
            .into_iter()
            .map(|(design_id, quantity)| {
                let index = c.next_loot_index;
                c.next_loot_index += 1;
                LootItem {
                    design_id,
                    quantity,
                    index,
                }
            })
            .collect();
        c.is_loot_container = true;
        c.container_loot.insert(player_id, items.clone());
        items
    };

    if once_per_character {
        if let Some(k) = key.clone() {
            if let Some(p) = space_mgr.get_entity_mut(entity_id) {
                p.looted_containers.insert(k.clone());
            }
            if tx
                .send(CellToBaseMsg::ContainerLooted {
                    entity_id,
                    player_id,
                    container_key: k,
                })
                .await
                .is_err()
            {
                tracing::warn!(
                    target: "loot",
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    player_id,
                    player_name = id.player_name,
                    container_key = key_ref,
                    reason = "base_channel_closed",
                    "open_loot: ContainerLooted not sent -- the flag holds for this session only"
                );
            }
        }
    }

    tracing::info!(
        target: "loot",
        event = "loot.container_opened",
        entity_id,
        entity_name = space_mgr.entity_names(entity_id).entity_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id,
        player_name = id.player_name,
        container_entity_id = container,
        container_entity_name = space_mgr.entity_names(container).entity_name,
        container_key = key_ref,
        loot_table_id = table_id,
        loot_table_name = cimmeria_names::book().loot_table(table_id),
        item_count = items.len(),
        once_per_character,
        chain_id,
        chain_name = cimmeria_names::book().chain(chain_id),
        "open_loot: rolled and opened the loot window"
    );
    show(entity_id, container, &items, tx, space_mgr).await;
}

/// Pin the container as the player's loot target and open the window.
async fn show(
    entity_id: u32,
    container: u32,
    items: &[LootItem],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if let Some(p) = space_mgr.get_entity_mut(entity_id) {
        p.looting_entity = Some(container);
    }
    let sent = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_LOOT_DISPLAY,
            args: serialize_on_loot_display(container as i32, items, 1),
        })
        .await;
    if sent.is_err() {
        tracing::warn!(
            target: "loot",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            container,
            container_name = space_mgr.entity_names(container).entity_name,
            reason = "base_channel_closed",
            "open_loot: onLootDisplay not sent"
        );
    }
}

/// One feedback line, so no press is silent.
async fn feedback(
    entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let sent = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
        })
        .await;
    if sent.is_err() {
        tracing::warn!(
            target: "loot",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            reason = "base_channel_closed",
            "open_loot: feedback line not sent"
        );
    }
}
