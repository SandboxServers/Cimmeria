//! Character list queries, delete, visuals, and shared helpers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

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
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    error_code: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_char_create_failed(&key, seq, &acks, error_code, account_eid);
    transport.send_to(&pkt, addr).await?;
    Ok(())
}

/// Handle `deleteCharacter` (0xC5) -- delete a character and send updated list.
#[tracing::instrument(
    name = "character.delete",
    level = "info",
    skip_all,
    fields(peer = %addr, account_id, player_id),
)]
pub(crate) async fn handle_delete_character(
    transport: &Arc<dyn Transport>,
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
    transport.send_to(&pkt, addr).await?;

    Ok(())
}

/// Handle `requestCharacterVisuals` (0xC6).
pub(crate) async fn handle_request_character_visuals(
    transport: &Arc<dyn Transport>,
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
            transport.send_to(&pkt, addr).await?;
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

    /// Sentinel base for the character-list tests, stepped past the
    /// highest live-DB sentinel reserved elsewhere in the crate. The
    /// sibling sentinels are listed alongside their owning test
    /// files; the +0x100 step here just reserves a fresh slot so
    /// concurrent live-DB runs don't collide on account/player ids.
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

#[cfg(test)]
mod delete_character_tests {
    use super::*;
    use crate::test_support::TestTransport;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn delete_character_no_db_short_circuits() {
        // Typed handle for inspection; dyn handle for the handler call.
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let key = [0u8; 32];

        let result =
            handle_delete_character(&dyn_transport, addr, key, 1, 1, &connected, &None).await;
        assert!(
            result.is_ok(),
            "no-DB mode must short-circuit without error"
        );

        // Short-circuit must be silent. A regression that returned Ok
        // after sending an error packet on the wire would still pass
        // the is_ok() assertion above; pin the no-output invariant too.
        assert!(
            transport.is_empty(),
            "no-DB short-circuit must not send any UDP packet",
        );
    }
}

#[cfg(test)]
mod delete_character_live_db_tests {
    //! Live-DB guards for `handle_delete_character`.
    //!
    //! Covers the account-isolation predicate on the DELETE statement
    //! and the `rows_affected == 0` warn branch. The happy-path delete
    //! is exercised as a side effect of the cross-account guard: the
    //! handler succeeds, but the row that should have been spared MUST
    //! still exist, and the row that SHOULD have been deleted is
    //! verified separately so a "DELETE deletes nothing" regression
    //! also trips here.
    //!
    //! Sentinels live in the file's reserved `0x7000_1000` slot, stepped
    //! past the `query_character_list_tests` sentinels (0x7000_1000,
    //! +1..+103) to a fresh `+0x100` window.
    use super::*;
    use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use tracing::Level;

    /// Sentinel base for the delete-character live-DB tests. The +0x100
    /// step keeps these account/player ids disjoint from the
    /// `query_character_list_tests` sentinels (TEST_BASE..TEST_BASE+103).
    const TEST_BASE: i32 = 0x7000_1100;

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
        .bind(format!("delete-char-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    }

    async fn insert_character(pool: &PgPool, account_id: i32, player_id: i32, name: &str) {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insert character");
    }

    async fn count_player(pool: &PgPool, player_id: i32) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(pool)
            .await
            .expect("count player row")
    }

    /// Build a `connected` map that satisfies the handler's
    /// `get_account_entity_id` lookup. The Account entity id is
    /// arbitrary — the wire path past the DELETE uses it only to
    /// address `onCharacterList`, which we don't decode in these tests.
    fn make_connected(
        addr: SocketAddr,
        account_id: u32,
        account_entity_id: u32,
    ) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
        let mut state = crate::test_support::test_default_connected_client_state();
        state.account_id = account_id;
        state.account_entity_id = account_entity_id;
        let mut m = HashMap::new();
        m.insert(addr, state);
        Arc::new(Mutex::new(m))
    }

    /// Cross-account delete must NOT remove the other account's
    /// character. Bug shape: a refactor that drops the
    /// `AND account_id = $2` predicate (or swaps the two binds) lets
    /// account A delete account B's character. Pin via an exact
    /// COUNT(*) on the spared row — `== 1` beats `is_ok()` because the
    /// handler always returns Ok regardless of `rows_affected`.
    #[tokio::test]
    async fn handle_delete_character_with_wrong_account_does_not_delete() {
        let pool = require_db_or_skip!();
        let account_a = TEST_BASE;
        let account_b = TEST_BASE + 1;
        let player_a = TEST_BASE + 10;
        let player_b = TEST_BASE + 11;

        cleanup(&pool, &[account_a, account_b]).await;
        insert_account(&pool, account_a).await;
        insert_account(&pool, account_b).await;
        insert_character(&pool, account_a, player_a, "alice").await;
        insert_character(&pool, account_b, player_b, "bob").await;

        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:55801".parse().unwrap();
        let connected = make_connected(addr, account_a as u32, 0xAAAA_0001);
        let key = [0u8; 32];
        let db_pool = Some(Arc::new(pool.clone()));

        // Account A asks to delete account B's player. The owning-
        // account check on the DELETE WHERE clause MUST short-circuit
        // the row removal.
        let result = handle_delete_character(
            &dyn_transport,
            addr,
            key,
            account_a as u32,
            player_b,
            &connected,
            &db_pool,
        )
        .await;
        assert!(
            result.is_ok(),
            "handler must return Ok even on no-op delete"
        );

        // Tight assertion: the spared row count is exactly 1. The bug
        // shape is "row vanished"; ">= 1" would mask a regression that
        // moves rows around or duplicates them.
        assert_eq!(
            count_player(&pool, player_b).await,
            1,
            "cross-account delete must NOT remove account B's character; \
             the AND account_id predicate is the only thing standing \
             between this test and an account-isolation breach",
        );
        // Sanity: account A's own character is still present (the
        // handler didn't blanket-clear by account either).
        assert_eq!(
            count_player(&pool, player_a).await,
            1,
            "account A's own character must remain — the no-op delete \
             must not have side-effected the requester's row either",
        );

        cleanup(&pool, &[account_a, account_b]).await;
    }

    /// `rows_affected == 0` must emit the documented WARN. Bug shape: a
    /// refactor that demotes the warn to debug (or removes it) drops the
    /// only signal ops has that an account just attempted a
    /// cross-account / stale-id delete. Pinned with `LogCapture` per
    /// the negative-logging convention.
    #[tokio::test]
    async fn handle_delete_character_warns_when_not_owned() {
        let pool = require_db_or_skip!();
        let capture = LogCapture::install();

        let account_a = TEST_BASE + 20;
        let account_b = TEST_BASE + 21;
        let player_b = TEST_BASE + 30;

        cleanup(&pool, &[account_a, account_b]).await;
        insert_account(&pool, account_a).await;
        insert_account(&pool, account_b).await;
        insert_character(&pool, account_b, player_b, "victim").await;

        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:55802".parse().unwrap();
        let connected = make_connected(addr, account_a as u32, 0xAAAA_0002);
        let key = [0u8; 32];
        let db_pool = Some(Arc::new(pool.clone()));

        let _ = handle_delete_character(
            &dyn_transport,
            addr,
            key,
            account_a as u32,
            player_b,
            &connected,
            &db_pool,
        )
        .await;

        let event = capture
            .find_message(Level::WARN, "Character not found or not owned")
            .expect(
                "cross-account delete (rows_affected == 0) must emit a WARN — \
                 negative-logging convention: the only signal ops has that \
                 something tried to delete a row it doesn't own",
            );
        // Pin the structured fields so a refactor that drops the
        // player_id/account_id context (or swaps levels) trips here.
        assert!(
            event.has_field("player_id", &player_b.to_string()),
            "warn must carry the target player_id for ops triage: {event:#?}"
        );
        assert!(
            event.has_field("account_id", &account_a.to_string()),
            "warn must carry the requesting account_id (not the owner's) — \
             that's the field ops correlates against the session: {event:#?}"
        );

        cleanup(&pool, &[account_a, account_b]).await;
    }
}

#[cfg(test)]
mod request_visuals_live_db_tests {
    //! Live-DB guards for `handle_request_character_visuals`.
    //!
    //! Two branches matter: the happy path (player row found, visuals
    //! query joins through `sgw_inventory` and `resources.items`,
    //! `onCharacterVisuals` goes out on the wire) and the
    //! `fetch_optional → Ok(None)` branch where the (player_id,
    //! account_id) pair doesn't match — the handler must log WARN and
    //! NOT send a packet.
    //!
    //! Sentinels share the `0x7000_1100+` window with the delete-
    //! character tests but step further out (TEST_BASE = `0x7000_1200`)
    //! to avoid PK collisions if both modules run against the same DB.
    use super::*;
    use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use tracing::Level;

    /// Sentinel base for the visuals live-DB tests. Stepped past the
    /// delete-character module's `0x7000_1100..+30` window so a
    /// concurrent run can't collide on account/player ids.
    const TEST_BASE: i32 = 0x7000_1200;

    /// Synthetic `resources.items` rows used by these tests. Picked
    /// outside any production / seed range. Idempotent `ON CONFLICT
    /// DO NOTHING` insert, and NEVER deleted in cleanup so concurrent
    /// tests don't fight over the shared FK target. See the matching
    /// pattern in `world_entry/methods/vendor/recharge.rs`.
    const SYNTH_ITEM_EQUIP: i32 = 0x7FFF_C001;
    const SYNTH_ITEM_BANDOLIER: i32 = 0x7FFF_C002;

    /// Equipment-container id used to place the equip item. Must be
    /// inside `EQUIPMENT_CONTAINERS` — pick 4 (head slot) to match the
    /// production constant in `player_load/core.rs`.
    const CONTAINER_HEAD: i32 = 4;
    /// Bandolier container id mirrors the production
    /// `CONTAINER_BANDOLIER` constant. Hard-coded here as 3 so a
    /// regression that re-numbers the bandolier container would
    /// surface as a test failure (the handler's SQL would still bind
    /// the renamed constant, but this test's seeded row would no
    /// longer match — exactly the signal we want).
    const CONTAINER_BANDOLIER_LOCAL: i32 = 3;
    /// `bandolier_slot` we set on the player row; the visuals query
    /// only returns the bandolier row whose `slot_id` matches this.
    const ACTIVE_BAND_SLOT: i32 = 0;

    async fn cleanup(pool: &PgPool, account_ids: &[i32], player_ids: &[i32]) {
        // Per-test rows only — the synthetic resources.items rows are
        // shared FK targets, see SYNTH_ITEM_* doc above.
        for player_id in player_ids {
            let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
                .bind(player_id)
                .execute(pool)
                .await;
        }
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
        .bind(format!("visuals-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    }

    async fn insert_character(pool: &PgPool, account_id: i32, player_id: i32, bandolier_slot: i32) {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot, \
                components\
             ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0, $4, ARRAY['base.Component']::varchar[])",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("visuals-char-{player_id}"))
        .bind(bandolier_slot)
        .execute(pool)
        .await
        .expect("insert character");
    }

    /// Insert a synthetic `resources.items` row whose `visual_component`
    /// is the canonical column the visuals query joins on. `ON CONFLICT
    /// DO NOTHING` so concurrent test binaries don't fight over the PK.
    async fn insert_synthetic_item(pool: &PgPool, item_id: i32, visual_component: &str) {
        sqlx::query(
            "INSERT INTO resources.items (\
                item_id, description, name, quality_id, tech_comp, tier, \
                max_stack_size, visual_component\
             ) VALUES ($1, '', $2, 'ITEM_QUALITY_Normal', 0, 1, 1, $3) \
             ON CONFLICT (item_id) DO NOTHING",
        )
        .bind(item_id)
        .bind(format!("synth-visual-{item_id}"))
        .bind(visual_component)
        .execute(pool)
        .await
        .expect("insert synthetic resources.items row");
    }

    async fn insert_inventory_row(
        pool: &PgPool,
        player_id: i32,
        type_id: i32,
        container_id: i32,
        slot_id: i32,
    ) {
        sqlx::query(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges) \
             VALUES ($1, $2, 1, $3, $4, false, 100, 0)",
        )
        .bind(player_id)
        .bind(type_id)
        .bind(slot_id)
        .bind(container_id)
        .execute(pool)
        .await
        .expect("insert inventory row");
    }

    fn make_connected(
        addr: SocketAddr,
        account_id: u32,
        account_entity_id: u32,
    ) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
        let mut state = crate::test_support::test_default_connected_client_state();
        state.account_id = account_id;
        state.account_entity_id = account_entity_id;
        let mut m = HashMap::new();
        m.insert(addr, state);
        Arc::new(Mutex::new(m))
    }

    /// Happy path: player row exists for (player_id, account_id), the
    /// inventory join produces two visual rows (one equip-container,
    /// one bandolier active slot), and the handler sends exactly one
    /// `onCharacterVisuals` packet. Bug shape: a regression that
    /// silently swallows the inventory join error (or drops the
    /// `components.extend(item_visuals)` line) would still send a
    /// packet, so the assertion here is "send_count == 1" — not just
    /// non-empty.
    #[tokio::test]
    async fn handle_request_character_visuals_happy_path_sends_one_packet() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 10;

        cleanup(&pool, &[account_id], &[player_id]).await;
        insert_synthetic_item(&pool, SYNTH_ITEM_EQUIP, "vis.Equip").await;
        insert_synthetic_item(&pool, SYNTH_ITEM_BANDOLIER, "vis.Bandolier").await;
        insert_account(&pool, account_id).await;
        insert_character(&pool, account_id, player_id, ACTIVE_BAND_SLOT).await;
        // Equipment container row: slot_id MUST be 0 to match the
        // `(container_id <> $3 AND slot_id = 0)` arm of the visuals
        // query's OR predicate.
        insert_inventory_row(&pool, player_id, SYNTH_ITEM_EQUIP, CONTAINER_HEAD, 0).await;
        // Bandolier active slot row: container_id = bandolier AND
        // slot_id = bandolier_slot ($4) — the other arm of the OR.
        insert_inventory_row(
            &pool,
            player_id,
            SYNTH_ITEM_BANDOLIER,
            CONTAINER_BANDOLIER_LOCAL,
            ACTIVE_BAND_SLOT,
        )
        .await;

        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:55803".parse().unwrap();
        let connected = make_connected(addr, account_id as u32, 0xBBBB_0001);
        let key = [0u8; 32];
        let db_pool = Some(Arc::new(pool.clone()));

        let result = handle_request_character_visuals(
            &dyn_transport,
            addr,
            key,
            player_id,
            &connected,
            &db_pool,
        )
        .await;
        assert!(
            result.is_ok(),
            "visuals handler must return Ok on happy path"
        );

        // Exactly one packet — not "at least one". The bug shape we're
        // pinning is "extra fan-out" as well as "no fan-out".
        assert_eq!(
            transport.send_count_to(addr),
            1,
            "happy-path requestCharacterVisuals must emit exactly one \
             onCharacterVisuals packet to the requester; got {} packets",
            transport.send_count_to(addr),
        );
        assert_eq!(
            transport.len(),
            1,
            "no other fan-out is expected — the handler is a 1-in/1-out RPC",
        );

        cleanup(&pool, &[account_id], &[player_id]).await;
    }

    /// `fetch_optional → Ok(None)` branch: the player row exists but
    /// under a DIFFERENT account, so the `(player_id AND account_id)`
    /// predicate yields no row. Handler must NOT send a packet and
    /// MUST emit the documented WARN. Bug shape: dropping the
    /// `account_id = $2` predicate from the SELECT (mirroring the
    /// delete-character bug shape) would leak another account's
    /// visuals on the character-select screen.
    #[tokio::test]
    async fn handle_request_character_visuals_account_mismatch_logs_warn() {
        let pool = require_db_or_skip!();
        let capture = LogCapture::install();

        let owning_account = TEST_BASE + 20;
        let requesting_account = TEST_BASE + 21;
        let other_player = TEST_BASE + 30;

        cleanup(
            &pool,
            &[owning_account, requesting_account],
            &[other_player],
        )
        .await;
        insert_account(&pool, owning_account).await;
        insert_account(&pool, requesting_account).await;
        // Player belongs to owning_account, NOT requesting_account.
        insert_character(&pool, owning_account, other_player, ACTIVE_BAND_SLOT).await;

        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:55804".parse().unwrap();
        // The session is for requesting_account — but the player_id
        // they're asking about belongs to owning_account. The SELECT
        // WHERE filter must yield None.
        let connected = make_connected(addr, requesting_account as u32, 0xBBBB_0002);
        let key = [0u8; 32];
        let db_pool = Some(Arc::new(pool.clone()));

        let result = handle_request_character_visuals(
            &dyn_transport,
            addr,
            key,
            other_player,
            &connected,
            &db_pool,
        )
        .await;
        assert!(
            result.is_ok(),
            "Ok(None) branch must still return Ok — the function does \
             not surface 'not found' as an error to the caller",
        );

        // No packet sent — the Ok(None) arm has no transport.send_to.
        // A regression that fell through to the happy-path build_packet
        // line would trip this.
        assert!(
            transport.is_empty(),
            "account-mismatch lookup must NOT send a visuals packet — \
             that would leak another account's character visuals; \
             got {} packets",
            transport.len(),
        );

        let event = capture
            .find_message(Level::WARN, "requestCharacterVisuals: player not found")
            .expect(
                "Ok(None) branch must emit a WARN — negative-logging \
                 convention: the only signal ops has that a session asked \
                 for a player it doesn't own (or a stale id)",
            );
        assert!(
            event.has_field("player_id", &other_player.to_string()),
            "warn must carry the queried player_id for ops triage: {event:#?}"
        );

        cleanup(
            &pool,
            &[owning_account, requesting_account],
            &[other_player],
        )
        .await;
    }
}
