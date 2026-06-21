//! `placeBid` — refund the prior bidder, hold the new bid, advance the auction.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::helpers::{adjust_player_cash, now_unix_secs, CashError};
use super::player_name_for_player_id;
use super::send::{send_bm_auction_update, send_bm_error};
use super::types::AuctionRow;
use super::validate::validate_bid;
use super::wire::error_code;
use crate::base::ConnectedClientState;

/// Handle a `BMPlaceBid` forwarded from the cell.
///
/// All cash movement + the auction row mutation happen in one transaction so a
/// crash can't strand the bidder's held funds. The prior high bidder (if any)
/// is refunded their held cash before the new bid is deducted.
///
/// Buyout: if the bid clears a non-zero `buyout_price` the original game settled
/// the auction immediately. That settlement path is not yet wired (it needs the
/// CoD/mail payout, which the expiry sweep owns) — for now a buyout-or-higher
/// bid is recorded as a normal high bid and the auction is left active for the
/// sweep to settle at expiry. TODO: immediate buyout settlement (depends on the
/// shared sweep settlement helper).
#[tracing::instrument(
    name = "black_market.place_bid",
    level = "info",
    skip_all,
    fields(entity_id, player_id, sequence_id, bid_amount)
)]
#[allow(clippy::too_many_arguments)]
pub async fn handle_place_bid(
    entity_id: u32,
    player_id: i32,
    sequence_id: i32,
    bid_amount: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        tracing::debug!(entity_id, player_id, "placeBid: no DB pool");
        return;
    };

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(entity_id, player_id, "placeBid: begin tx failed: {e}");
            send_bm_error(
                entity_id,
                error_code::INTERNAL,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    // Lock the auction row for the duration of the bid so two concurrent bids
    // serialise (the prior-bidder refund + new-bid hold must be consistent).
    let auction = match sqlx::query_as::<_, AuctionRow>(
        "SELECT sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
                charges, starting_price, buyout_price, current_bid, current_bidder, \
                auction_length, created_at, expires_at, status \
         FROM sgw_auction WHERE sequence_id = $1 FOR UPDATE",
    )
    .bind(sequence_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(a)) => a,
        Ok(None) => {
            let _ = tx.rollback().await;
            tracing::info!(
                entity_id,
                player_id,
                sequence_id,
                "placeBid: auction not found"
            );
            send_bm_error(
                entity_id,
                error_code::AUCTION_GONE,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                sequence_id,
                "placeBid: fetch failed: {e}"
            );
            send_bm_error(
                entity_id,
                error_code::INTERNAL,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    // Read the bidder's balance (without mutating) to validate funds. The
    // authoritative debit below re-checks via the SQL overdraw guard.
    let balance = match sqlx::query_scalar::<_, i64>(
        "SELECT naquadah::bigint FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(b)) => b,
        Ok(None) => {
            let _ = tx.rollback().await;
            tracing::warn!(entity_id, player_id, "placeBid: bidder row missing");
            send_bm_error(
                entity_id,
                error_code::NOT_ENOUGH_FUNDS,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(entity_id, player_id, "placeBid: balance query failed: {e}");
            send_bm_error(
                entity_id,
                error_code::INTERNAL,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    if let Err(code) = validate_bid(&auction, player_id, bid_amount, balance) {
        let _ = tx.rollback().await;
        tracing::info!(
            entity_id,
            player_id,
            sequence_id,
            code,
            "placeBid: rejected"
        );
        send_bm_error(entity_id, code, transport, connected, entity_to_addr).await;
        return;
    }

    // Refund the prior high bidder their held cash (if any, and not the same
    // person re-bidding — though that's still correct: refund then re-deduct).
    if let Some(prev_bidder) = auction.current_bidder {
        if auction.current_bid > 0 {
            match adjust_player_cash(&mut tx, prev_bidder, auction.current_bid as i64).await {
                Ok(_) => {}
                Err(CashError::NoSuchPlayer) => {
                    // Prior bidder's account is gone; the held cash is
                    // unrecoverable but we must not block the new bid. Log loud.
                    tracing::warn!(
                        sequence_id,
                        prev_bidder,
                        amount = auction.current_bid,
                        "placeBid: prior bidder row missing, cannot refund"
                    );
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    tracing::error!(sequence_id, prev_bidder, "placeBid: refund failed: {e}");
                    send_bm_error(
                        entity_id,
                        error_code::INTERNAL,
                        transport,
                        connected,
                        entity_to_addr,
                    )
                    .await;
                    return;
                }
            }
        }
    }

    // Hold the new bid (debit the bidder). The overdraw guard makes this safe
    // even though we validated against a pre-refund balance snapshot.
    if let Err(e) = adjust_player_cash(&mut tx, player_id, -(bid_amount as i64)).await {
        let _ = tx.rollback().await;
        let code = match e {
            CashError::InsufficientFunds | CashError::NoSuchPlayer => error_code::NOT_ENOUGH_FUNDS,
            CashError::Db(_) => error_code::INTERNAL,
        };
        tracing::warn!(entity_id, player_id, "placeBid: hold failed: {e}");
        send_bm_error(entity_id, code, transport, connected, entity_to_addr).await;
        return;
    }

    let updated = match sqlx::query_as::<_, AuctionRow>(
        "UPDATE sgw_auction SET current_bid = $1, current_bidder = $2 \
         WHERE sequence_id = $3 \
         RETURNING sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
                   charges, starting_price, buyout_price, current_bid, current_bidder, \
                   auction_length, created_at, expires_at, status",
    )
    .bind(bid_amount)
    .bind(player_id)
    .bind(sequence_id)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(entity_id, sequence_id, "placeBid: update failed: {e}");
            send_bm_error(
                entity_id,
                error_code::INTERNAL,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    if let Err(e) = sqlx::query(
        "INSERT INTO sgw_auction_bid (sequence_id, bidder_id, amount, created_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(sequence_id)
    .bind(player_id)
    .bind(bid_amount)
    .bind(now_unix_secs())
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        tracing::error!(
            entity_id,
            sequence_id,
            "placeBid: bid-history insert failed: {e}"
        );
        send_bm_error(
            entity_id,
            error_code::INTERNAL,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(entity_id, sequence_id, "placeBid: commit failed: {e}");
        send_bm_error(
            entity_id,
            error_code::INTERNAL,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    }

    tracing::info!(
        entity_id,
        player_id,
        sequence_id,
        bid_amount,
        "placeBid: bid accepted"
    );

    // Push the new state to the bidder. The seller (if online) gets refreshed
    // by the sweep / next search; per-witness fan-out of bids is out of scope
    // for Phase 2.
    let seller_name =
        player_name_for_player_id(updated.seller_id, connected, entity_to_addr).unwrap_or_default();
    send_bm_auction_update(
        entity_id,
        &updated,
        &seller_name,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}
