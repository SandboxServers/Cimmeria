use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::super::player_load::meta::query_bandolier_items_tx;
use crate::base::{helpers, ConnectedClientState};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};

pub async fn send_cash_changed_to_client(
    entity_id: u32,
    total: i32,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    helpers::send_to_witness_reliable(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_CASH_CHANGED,
                &total.to_le_bytes(),
            )
        },
    )
    .await;
}

pub async fn sync_bandolier_after_inventory_change(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    sync_bandolier_after_inventory_change_with_options(
        entity_id,
        player_id,
        db_pool,
        cell_tx,
        socket,
        connected,
        entity_to_addr,
        false,
    )
    .await;
}

/// Like [`sync_bandolier_after_inventory_change`] but allows the caller
/// to defer the `refresh_player_appearance` broadcast.
///
/// Used by the unequip path (move from bandolier→main bag, container
/// 3 → 1): the cell-side handler fires `Item_Unequip` and schedules
/// a Phase 2 broadcast via `holster_animation_complete_at` so the
/// mesh stays attached for the duration of the animation. If we
/// broadcast immediately here, the base yanks the mesh before the
/// animation plays — the user sees no holster animation, the weapon
/// just vanishes (or, when unequipping the last weapon, doesn't
/// vanish at all because the empty-bandolier branch below used to
/// skip the broadcast).
pub async fn sync_bandolier_after_inventory_change_with_options(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    defer_appearance_refresh: bool,
) {
    // The DB reconciliation must run whenever a pool is available, regardless
    // of whether the cell-sync channel is up — otherwise a `cell_tx == None`
    // window would skip the authoritative `bandolier_slot` UPDATE entirely
    // and leave the player's persisted active slot pointing at a vacated
    // bandolier entry. The `cell_tx`-only emit is gated separately below.
    let pool = match db_pool {
        Some(p) => p,
        None => return,
    };

    let mut db_tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(entity_id, player_id, "sync_bandolier: begin tx failed: {e}");
            return;
        }
    };

    let old_active: i32 = match sqlx::query_scalar(
        "SELECT bandolier_slot FROM sgw_player WHERE player_id = $1 FOR UPDATE",
    )
    .bind(player_id)
    .fetch_optional(&mut *db_tx)
    .await
    {
        Ok(v) => v.unwrap_or(0),
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "sync_bandolier: read slot failed: {e}"
            );
            return;
        }
    };

    // Read bandolier items *inside* the transaction so the FOR UPDATE lock above
    // protects this read against concurrent inventory mutations on container 3.
    let bandolier_items = match query_bandolier_items_tx(&mut db_tx, player_id).await {
        Ok(items) => items,
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "sync_bandolier: read items failed: {e}"
            );
            return;
        }
    };

    // Empty bandolier: nothing to reconcile, don't write a sentinel slot or
    // emit a witness packet for "active slot 0 of nothing". Still send
    // SyncBandolierItems so the cell-side cache drops any stale entries —
    // otherwise the previous bandolier set lingers in the cell HashMap until
    // the next non-empty change.
    if bandolier_items.is_empty() {
        if let Err(e) = db_tx.commit().await {
            tracing::error!(entity_id, player_id, "sync_bandolier: commit failed: {e}");
            return;
        }
        if let Some(tx) = cell_tx {
            let _ = tx
                .send(BaseToCellMsg::SyncBandolierItems {
                    entity_id,
                    active_bandolier_slot: old_active,
                    bandolier_items: Vec::new(),
                })
                .await;
        }
        // Fire the appearance refresh for the empty-bandolier case too
        // (unless the caller is going to drive it from the cell — the
        // unequip path defers so the holster animation can play
        // first). The original code returned here unconditionally,
        // which left the weapon mesh visible on the client whenever
        // a player unequipped their LAST weapon — the witness never
        // received a BeingAppearance with the empty ComponentList.
        if !defer_appearance_refresh {
            super::super::inventory::refresh_player_appearance(
                entity_id,
                player_id,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        return;
    }

    let mut active_slot = old_active;
    if !bandolier_items.iter().any(|(slot, _)| *slot == active_slot) {
        // Safe to unwrap: the empty-bandolier case is short-circuited above,
        // so `bandolier_items` is non-empty here and `min()` always yields Some.
        active_slot = bandolier_items
            .iter()
            .map(|(slot, _)| *slot)
            .min()
            .expect("bandolier_items is non-empty (empty case returned above)");
        if let Err(e) =
            sqlx::query("UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2")
                .bind(active_slot)
                .bind(player_id)
                .execute(&mut *db_tx)
                .await
        {
            let _ = db_tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                active_slot,
                "Failed to update bandolier slot: {e}"
            );
            return;
        }
        tracing::debug!(
            entity_id,
            player_id,
            active_slot,
            "Bandolier active slot updated"
        );
    }

    if let Err(e) = db_tx.commit().await {
        tracing::error!(entity_id, player_id, "sync_bandolier: commit failed: {e}");
        return;
    }

    if active_slot != old_active {
        // Container 3 = bandolier; matches CONTAINER_BANDOLIER in player_load/core.rs.
        const CONTAINER_BANDOLIER: i32 = 3;
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
        args.extend_from_slice(&(active_slot + 1).to_le_bytes());
        helpers::send_to_witness_reliable(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_ACTIVE_SLOT_UPDATE,
                    &args,
                )
            },
        )
        .await;
    }

    if let Some(tx) = cell_tx {
        let _ = tx
            .send(BaseToCellMsg::SyncBandolierItems {
                entity_id,
                active_bandolier_slot: active_slot,
                bandolier_items,
            })
            .await;
    }

    // Whenever the bandolier composition changes (item moved into/out of
    // the active slot, active-slot auto-promoted because the previous
    // selection was vacated), the appearance query in player_load needs
    // to re-run — its bandolier visual filter keys off `bandolier_slot`
    // AND the item rows present at that slot. Without this rebroadcast
    // the player keeps seeing the previous weapon visual until the next
    // login.
    //
    // Mirrors the refresh in `handle_grant_item` and the `ActiveSlotUpdate`
    // handler. Idempotent on the wire — witnesses just receive the same
    // packet they would have on next login.
    //
    // Caller can defer this (e.g., the unequip path) so the cell-side
    // holster animation has time to play before the mesh removal
    // broadcasts. In that case the cell's `holster_timer_tick` Phase 2
    // fires `RefreshAppearance` after `HOLSTER_ANIMATION_DURATION`, which
    // routes back through `refresh_player_appearance` from the base side.
    if !defer_appearance_refresh {
        super::super::inventory::refresh_player_appearance(
            entity_id,
            player_id,
            db_pool,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
    }
}

#[cfg(test)]
mod sync_bandolier_tests {
    //! Live-DB integration tests for sync_bandolier_after_inventory_change.
    //!
    //! Skip cleanly when DATABASE_URL is unset; against the bundled
    //! local Postgres they exercise the active-slot reconciliation
    //! ladder: vacated-slot fixup, still-valid passthrough, and the
    //! empty-bandolier path.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for sync_bandolier tests. Distinct from prior
    /// live-DB sentinels (outbox 0x000 / grant_cash +0x100 /
    /// move +0x200 / grant_item +0x300 / missions +0x400 / mail +0x500 /
    /// vendor/repair +0x600 / paid_repair +0x700 / sell +0x800 /
    /// buyback +0x900 / purchase +0x0A00 / ammo +0x0B00 /
    /// vendor_data +0x0C00 / player_load +0x0D00).
    const TEST_BASE: i32 = 0x7000_0E00;

    /// Bandolier-allowed weapon. Picked deliberately: must satisfy
    /// the JOIN to resources.items in query_bandolier_items_tx.
    const WEAPON_TYPE_ID: i32 = 3241;
    const INV_BANDOLIER: i32 = 3;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_account_and_player(
        pool: &PgPool,
        account_id: i32,
        player_id: i32,
        bandolier_slot: i32,
    ) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("sync-band-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0, $4)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("test-{player_id}"))
        .bind(bandolier_slot)
        .execute(pool)
        .await
        .expect("insert player");
    }

    async fn insert_bandolier_row(pool: &PgPool, player_id: i32, slot_id: i32) {
        sqlx::query(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges) \
             VALUES ($1, $2, 1, $3, $4, false, 100, 0)",
        )
        .bind(player_id)
        .bind(WEAPON_TYPE_ID)
        .bind(slot_id)
        .bind(INV_BANDOLIER)
        .execute(pool)
        .await
        .expect("insert bandolier row");
    }

    async fn bandolier_slot_of(pool: &PgPool, player_id: i32) -> i32 {
        sqlx::query_scalar("SELECT bandolier_slot FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(pool)
            .await
            .expect("read bandolier_slot")
    }

    fn make_state(
        entity_id: u32,
    ) -> (
        Arc<UdpSocket>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    ) {
        let std_sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind UDP");
        std_sock.set_nonblocking(true).unwrap();
        let socket = Arc::new(UdpSocket::from_std(std_sock).expect("from_std"));
        let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let entity_to_addr = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(entity_id, fake_addr);
            m
        }));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        (socket, entity_to_addr, connected)
    }

    /// Active-slot fixup: the player's persisted bandolier_slot points
    /// at a vacated slot (e.g., the equipped weapon was sold). The
    /// function must pick min(remaining slots) and UPDATE sgw_player
    /// to that. Bug shape this catches: the previous-bandolier-slot
    /// preservation regressing to "leave bandolier_slot pointing at a
    /// vacated entry" — which the cell-side then renders as no
    /// active weapon.
    #[tokio::test]
    async fn vacated_active_slot_falls_back_to_min_remaining_slot() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        // bandolier_slot=1 — but we will only seed slots 2 and 3.
        insert_account_and_player(&pool, account_id, player_id, 1).await;
        insert_bandolier_row(&pool, player_id, 3).await;
        insert_bandolier_row(&pool, player_id, 2).await;

        let (socket, e2a, conn) = make_state(0x7000_0E01);
        let db_pool = Some(Arc::new(pool.clone()));
        let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(4);

        sync_bandolier_after_inventory_change(
            0x7000_0E01,
            player_id,
            &db_pool,
            &Some(cell_tx),
            &socket,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            bandolier_slot_of(&pool, player_id).await,
            2,
            "vacated active slot must be replaced with min() of remaining slots",
        );

        match cell_rx.try_recv() {
            Ok(BaseToCellMsg::SyncBandolierItems {
                active_bandolier_slot,
                bandolier_items,
                ..
            }) => {
                assert_eq!(active_bandolier_slot, 2);
                assert_eq!(bandolier_items.len(), 2);
            }
            Ok(_) => panic!("expected SyncBandolierItems, got a different cell-tx variant"),
            Err(e) => panic!("expected SyncBandolierItems, channel error: {e}"),
        }

        cleanup(&pool, account_id, player_id).await;
    }

    /// Active-slot still valid: the persisted bandolier_slot still has
    /// an entry, so no UPDATE fires and no witness packet is emitted.
    /// The cell tx still receives SyncBandolierItems so the cell-side
    /// cache stays in sync after any inventory mutation.
    #[tokio::test]
    async fn valid_active_slot_passes_through_unchanged() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id, 2).await;
        // Slots 1, 2, 3 — slot 2 is the active one and remains valid.
        insert_bandolier_row(&pool, player_id, 1).await;
        insert_bandolier_row(&pool, player_id, 2).await;
        insert_bandolier_row(&pool, player_id, 3).await;

        let (socket, e2a, conn) = make_state(0x7000_0E11);
        let db_pool = Some(Arc::new(pool.clone()));
        let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(4);

        sync_bandolier_after_inventory_change(
            0x7000_0E11,
            player_id,
            &db_pool,
            &Some(cell_tx),
            &socket,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            bandolier_slot_of(&pool, player_id).await,
            2,
            "active slot must NOT change when the slot is still occupied",
        );

        match cell_rx.try_recv() {
            Ok(BaseToCellMsg::SyncBandolierItems {
                active_bandolier_slot,
                bandolier_items,
                ..
            }) => {
                assert_eq!(active_bandolier_slot, 2);
                assert_eq!(bandolier_items.len(), 3);
            }
            Ok(_) => panic!("expected SyncBandolierItems, got a different cell-tx variant"),
            Err(e) => panic!("expected SyncBandolierItems, channel error: {e}"),
        }

        cleanup(&pool, account_id, player_id).await;
    }

    /// Empty-bandolier path: no INV_BANDOLIER rows. The function must
    /// NOT touch sgw_player.bandolier_slot (so it stays pointing at
    /// whatever the caller left it at) and still emit
    /// SyncBandolierItems with an empty list — otherwise the cell-side
    /// cache holds stale weapon entries until the next non-empty change.
    #[tokio::test]
    async fn empty_bandolier_preserves_slot_and_emits_empty_sync() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 200;
        let player_id = TEST_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        // bandolier_slot CHECK constrains the column to 0..=3, so pick 3 —
        // distinct from the other tests' values so the assertion isn't a
        // coincidence with whatever the function might have written.
        insert_account_and_player(&pool, account_id, player_id, 3).await;
        // Deliberately no bandolier rows.

        let (socket, e2a, conn) = make_state(0x7000_0E21);
        let db_pool = Some(Arc::new(pool.clone()));
        let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(4);

        sync_bandolier_after_inventory_change(
            0x7000_0E21,
            player_id,
            &db_pool,
            &Some(cell_tx),
            &socket,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            bandolier_slot_of(&pool, player_id).await,
            3,
            "empty bandolier must NOT scribble bandolier_slot — caller's value is preserved",
        );

        match cell_rx.try_recv() {
            Ok(BaseToCellMsg::SyncBandolierItems {
                active_bandolier_slot,
                bandolier_items,
                ..
            }) => {
                assert_eq!(active_bandolier_slot, 3);
                assert!(
                    bandolier_items.is_empty(),
                    "empty bandolier must drive an empty SyncBandolierItems payload",
                );
            }
            Ok(_) => panic!("expected SyncBandolierItems, got a different cell-tx variant"),
            Err(e) => panic!("expected SyncBandolierItems, channel error: {e}"),
        }

        cleanup(&pool, account_id, player_id).await;
    }
}
