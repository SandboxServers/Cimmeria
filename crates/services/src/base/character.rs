//! Character list queries, delete, visuals, and shared helpers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use crate::mercury::{
    build_char_create_failed, build_character_visuals, build_on_character_list, CharacterInfo,
    SKIN_TINTS,
};

use super::helpers::{drain_acks_and_seq, get_account_entity_id};
use super::ConnectedClientState;

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
            tracing::info!(
                account_id,
                count = rows.len(),
                "Character list query result"
            );
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

    let result = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2")
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

    let characters = query_character_list(db_pool, account_id).await;
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_on_character_list(&key, seq, &acks, &characters, account_eid);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list after delete");
    socket.send_to(&pkt, addr).await?;

    Ok(())
}

/// Handle `requestCharacterVisuals` (0xC6).
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

    let account_id = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .get(&addr)
            .ok_or("addr not in connected map")?
            .account_id
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
            // Equipment containers + bandolier are defined alongside the
            // identical query in player_load/core.rs (CONTAINER_BANDOLIER and
            // EQUIPMENT_CONTAINERS). Bind them via ANY/parameter so the two
            // sites stay in sync without depending on string-literal identity.
            use super::world_entry::methods::player_load::core::{
                CONTAINER_BANDOLIER, EQUIPMENT_CONTAINERS,
            };
            let item_visuals: Vec<String> = match sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.container_id = ANY($1) \
                   AND inv.character_id = $2 \
                   AND ri.visual_component IS NOT NULL \
                   AND ( \
                     (inv.container_id <> $3 AND inv.slot_id = 0) \
                     OR (inv.container_id = $3 AND inv.slot_id = $4) \
                   )",
            )
            .bind({
                let mut all: Vec<i32> = EQUIPMENT_CONTAINERS.to_vec();
                all.push(CONTAINER_BANDOLIER);
                all
            })
            .bind(player_id)
            .bind(CONTAINER_BANDOLIER)
            .bind(bandolier_slot)
            .fetch_all(pool.as_ref())
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(
                        player_id,
                        "character visuals query failed (skipping appearance overlay): {e}"
                    );
                    Vec::new()
                }
            };

            components.extend(item_visuals);

            tracing::debug!(
                %addr, player_id, %bodyset,
                component_count = components.len(),
                skin_color_id,
                "Sending character visuals"
            );

            let skin_tint = SKIN_TINTS
                .get(skin_color_id as usize)
                .copied()
                .unwrap_or(0x2F1308FF);
            let account_eid = get_account_entity_id(connected, addr)?;
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_character_visuals(
                &key,
                seq,
                &acks,
                player_id,
                &bodyset,
                &components,
                0xFF,
                0xFF,
                skin_tint,
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

#[cfg(test)]
mod query_character_list_tests {
    //! Live-DB integration tests for query_character_list.
    //!
    //! Skip cleanly when DATABASE_URL is unset; against the bundled
    //! local Postgres they exercise the no-pool short-circuit, the
    //! happy-path round-trip with multiple characters, and the
    //! account-isolation guarantee.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for character-list tests. Distinct from prior
    /// live-DB sentinels (outbox 0x000 / grant_cash +0x100 /
    /// move +0x200 / grant_item +0x300 / missions +0x400 / mail +0x500 /
    /// vendor/repair +0x600 / paid_repair +0x700 / sell +0x800 /
    /// buyback +0x900 / purchase +0x0A00 / ammo +0x0B00 /
    /// vendor_data +0x0C00 / player_load_meta +0x0D00 /
    /// vendor_helpers +0x0E00 / player_load_core +0x0F00).
    const TEST_BASE: i32 = 0x7000_1000;

    async fn cleanup(pool: &PgPool, account_ids: &[i32]) {
        for account_id in account_ids {
            let _ = sqlx::query("DELETE FROM sgw_player WHERE account_id = $1")
                .bind(account_id)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
                .bind(account_id)
                .execute(pool)
                .await;
        }
    }

    async fn insert_account(pool: &PgPool, account_id: i32) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("char-list-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    }

    async fn insert_character(
        pool: &PgPool,
        account_id: i32,
        player_id: i32,
        name: &str,
        archetype: i32,
        level: i32,
    ) {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, $3, 1, $4, 1, $5, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(level)
        .bind(archetype)
        .bind(name)
        .execute(pool)
        .await
        .expect("insert character");
    }

    /// No DB pool short-circuits to an empty Vec. Important because
    /// the offline path in connect_loop relies on this returning
    /// empty rather than erroring.
    #[tokio::test]
    async fn no_pool_returns_empty() {
        // No `require_db_or_skip!()` — exercises the None branch.
        let chars = query_character_list(&None, 0).await;
        assert!(chars.is_empty());
    }

    /// Happy path: two characters under one account come back ordered
    /// by `player_id`, with name/level/archetype round-tripped. Bug
    /// shape: a regression to ORDER BY player_name (or any unstable
    /// ordering) breaks the client-side "first character" pick on the
    /// character-select screen.
    #[tokio::test]
    async fn returns_account_characters_ordered_by_player_id() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_a = TEST_BASE + 2;
        let player_b = TEST_BASE + 1;
        cleanup(&pool, &[account_id]).await;
        insert_account(&pool, account_id).await;
        // Insert in reverse player_id order so a "rows return in
        // insertion order" regression would put zelda first.
        insert_character(&pool, account_id, player_a, "zelda", 3, 5).await;
        insert_character(&pool, account_id, player_b, "alpha", 1, 7).await;

        let db_pool = Some(Arc::new(pool.clone()));
        let chars = query_character_list(&db_pool, account_id as u32).await;

        assert_eq!(chars.len(), 2);
        assert_eq!(
            chars[0].player_id, player_b,
            "lowest player_id (alpha) must come first — locks ORDER BY player_id",
        );
        assert_eq!(chars[0].name, "alpha");
        assert_eq!(chars[0].level, 7);
        assert_eq!(chars[0].archetype, 1);
        assert_eq!(chars[1].player_id, player_a);
        assert_eq!(chars[1].name, "zelda");

        cleanup(&pool, &[account_id]).await;
    }

    /// Account isolation: characters under a different account_id MUST
    /// NOT leak through. Bug shape: a refactor that drops the WHERE
    /// account_id predicate (or passes the wrong bind position) would
    /// turn the character-select screen into "every character on the
    /// shard" — a session-confusion + privacy vector.
    #[tokio::test]
    async fn other_accounts_characters_are_isolated() {
        let pool = require_db_or_skip!();
        let account_mine = TEST_BASE + 100;
        let account_other = TEST_BASE + 101;
        let player_mine = TEST_BASE + 102;
        let player_other = TEST_BASE + 103;
        cleanup(&pool, &[account_mine, account_other]).await;
        insert_account(&pool, account_mine).await;
        insert_account(&pool, account_other).await;
        insert_character(&pool, account_mine, player_mine, "mine", 1, 1).await;
        insert_character(&pool, account_other, player_other, "other", 2, 2).await;

        let db_pool = Some(Arc::new(pool.clone()));
        let chars = query_character_list(&db_pool, account_mine as u32).await;

        assert_eq!(chars.len(), 1);
        assert_eq!(
            chars[0].player_id, player_mine,
            "only the queried account's character may be returned",
        );

        cleanup(&pool, &[account_mine, account_other]).await;
    }
}
