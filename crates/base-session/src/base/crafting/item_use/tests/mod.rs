//! Crafting item use: the pure rule ([`rule`]), every outcome against the
//! seeded database ([`live_db`]), failures ([`failures`]), replays and races
//! ([`races`]), and the seed guard ([`seed`]).
//!
//! Live-DB sentinels, in the crafting `0x7000_Cxxx` block:
//! `0x7000_CF80..=0x7000_CFBF`, sixteen 4-id slots (account, player, two
//! item instances). The account id doubles as the entity id, so each test's
//! outbox rows are removed by exact entity. Clear of persistence and the GM
//! grants (`0x7000_C000..0x7000_CC0F`), sync, `.allcraft`, tools and spend
//! (`0x7000_CD00..0x7000_CE1F`) and the world-entry test (`0x7000_CF00`,
//! `0x7000_CF01`).

use std::sync::Arc;

use cimmeria_mercury::encryption::EncryptionVersion;
use sqlx::PgPool;

use super::*;
use crate::base::crafting::feedback::feedback_text_args;
use crate::base::crafting::persistence::load_crafting_state;
use crate::base::crafting::test_players::{
    cleanup as cleanup_player, insert_player, OneSession, SESSION_ACCOUNT_ID,
};
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{Captured, LogCaptureGuard};

mod failures;
mod live_db;
mod races;
mod rule;
mod seed;

const TEST_BASE: i32 = 0x7000_CF80;

/// Seed items used by the tests.
/// "Blueprint: Steel Plating (Materials Subcombine A)", teaches 25.
const STEEL_PLATING: i32 = 6483;
/// "Blueprint: Health Antidote", teaches 367 and 369.
const HEALTH_ANTIDOTE: i32 = 8882;
/// "Racial Paradigm Guide: Goa'uld", raises paradigm 3.
const GOAULD_GUIDE: i32 = 7808;
const GOAULD: i32 = 3;

/// One test's ids: slot `n` of the sentinel block.
#[derive(Debug, Clone, Copy)]
struct Slot {
    account_id: i32,
    player_id: i32,
    item: i32,
    other_item: i32,
}

impl Slot {
    fn new(n: i32) -> Self {
        let base = TEST_BASE + 4 * n;
        Slot {
            account_id: base,
            player_id: base + 1,
            item: base + 2,
            other_item: base + 3,
        }
    }

    fn entity(self) -> u32 {
        self.account_id as u32
    }
}

/// Remove everything a slot may have written, by exact id.
async fn cleanup(pool: &PgPool, slot: Slot) {
    for id in [slot.item, slot.other_item] {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
            .bind(id)
            .execute(pool)
            .await;
    }
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(slot.account_id)
        .execute(pool)
        .await;
    cleanup_player(pool, slot.account_id, slot.player_id).await;
}

/// A fresh player for `slot` knowing `blueprints`, with `paradigm` (id,
/// level) stored over the starting levels when given.
async fn player(pool: &PgPool, slot: Slot, blueprints: &[i32], paradigm: Option<(i32, i32)>) {
    cleanup(pool, slot).await;
    insert_player(pool, slot.account_id, slot.player_id).await;
    let mut levels = vec![5, 1, 1, 1, 1];
    if let Some((id, level)) = paradigm {
        levels[(id - 1) as usize] = level;
    }
    sqlx::query(
        "UPDATE sgw_player SET blueprint_ids = $2, racial_paradigm_levels = $3 \
         WHERE player_id = $1",
    )
    .bind(slot.player_id)
    .bind(blueprints)
    .bind(&levels)
    .execute(pool)
    .await
    .expect("seed crafting columns");
}

/// Put instance `item_id` of design `type_id` in `container_id` for
/// `player_id`, with `stack`.
async fn give(
    pool: &PgPool,
    item_id: i32,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    stack: i32,
) {
    sqlx::query(
        "INSERT INTO sgw_inventory (item_id, stack_size, container_id, slot_id, type_id, character_id) \
         VALUES ($1, $2, $3, 0, $4, $5)",
    )
    .bind(item_id)
    .bind(stack)
    .bind(container_id)
    .bind(type_id)
    .bind(player_id)
    .execute(pool)
    .await
    .expect("insert inventory row");
}

/// `(owner, stack_size)` of an instance, `None` once it is gone.
async fn instance(pool: &PgPool, item_id: i32) -> Option<(i32, i32)> {
    sqlx::query_as("SELECT character_id, stack_size FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .expect("read instance")
}

/// The known blueprints and the paradigm levels by id.
async fn crafting(pool: &PgPool, player_id: i32) -> (Vec<i32>, Vec<(i32, i8)>) {
    let state = load_crafting_state(pool, player_id).await.expect("load");
    let mut levels: Vec<(i32, i8)> = state.racial_paradigm_levels.into_iter().collect();
    levels.sort_unstable();
    (state.blueprint_ids, levels)
}

/// The outbox rows for a slot's entity, as event types.
async fn outbox_rows(pool: &PgPool, slot: Slot) -> Vec<String> {
    sqlx::query_scalar("SELECT event_type FROM cell_event_outbox WHERE entity_id = $1 ORDER BY id")
        .bind(slot.account_id)
        .fetch_all(pool)
        .await
        .expect("read outbox")
}

/// Run one use through the real handler with a live pool (or none).
async fn use_item(
    pool: Option<&PgPool>,
    session: &OneSession,
    slot: Slot,
    item_id: i32,
) -> Option<ConsumedItem> {
    let db_pool = pool.map(|p| Arc::new(p.clone()));
    let ctx = CraftCtx {
        db_pool: &db_pool,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    };
    handle_crafting_item_use(slot.entity(), slot.player_id, item_id, &ctx).await
}

fn packet(entity: u32, seq: u32, method: u16, args: &[u8]) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        seq,
        &[],
        entity,
        method,
        args,
        EncryptionVersion::V1,
    )
}

/// The one text line a refusal sends, at sequence `seq`.
fn refusal(entity: u32, seq: u32, text: &str) -> Vec<u8> {
    packet(
        entity,
        seq,
        method_idx::ON_PLAYER_COMMUNICATION,
        &feedback_text_args(text),
    )
}

/// The one `crafting` event named `event` for `player_id`, with the full
/// identity on it.
fn event(capture: &LogCaptureGuard, name: &str, slot: Slot) -> Captured {
    let found = capture
        .all()
        .into_iter()
        .find(|c| {
            c.target == "crafting"
                && c.has_field("event", name)
                && c.has_field("player_id", &slot.player_id.to_string())
        })
        .unwrap_or_else(|| panic!("no {name} event for {}", slot.player_id));
    for (k, v) in [
        ("verb", VERB.to_string()),
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("entity_id", slot.entity().to_string()),
    ] {
        assert!(found.has_field(k, &v), "{name}: {k}={v}: {found:#?}");
    }
    found
}

fn assert_fields(event: &Captured, fields: &[(&str, &str)]) {
    for &(k, v) in fields {
        assert!(event.has_field(k, v), "{k}={v}: {event:#?}");
    }
}
