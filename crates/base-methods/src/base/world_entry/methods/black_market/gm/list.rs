//! `.bm_list` (BM-07): the newest active auctions, with the ids the other
//! GM commands take. The client's window shows no auction ids.

use std::sync::Arc;

use sqlx::PgPool;

use super::super::helpers::now_unix_secs;
use super::super::types::auction_status;
use super::{pool_or_refuse, GmCtx};

/// How many auctions one `.bm_list` shows.
pub(super) const LIST_LIMIT: i64 = 10;

/// One `.bm_list` row, as read.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(super) struct ListRow {
    pub sequence_id: i32,
    pub item: Option<String>,
    pub item_def_id: i32,
    pub stack_size: i32,
    pub seller: Option<String>,
    pub starting_price: i32,
    pub buyout_price: i32,
    pub current_bid: i32,
    pub has_bidder: bool,
    pub expires_at: i32,
}

/// One row as the GM reads it: id, item, seller, price, time left.
pub(super) fn format_row(r: &ListRow, now: i32) -> String {
    let item = r
        .item
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("item {}", r.item_def_id));
    let stack = if r.stack_size > 1 {
        format!(" x{}", r.stack_size)
    } else {
        String::new()
    };
    let price = if r.has_bidder {
        format!("bid {}", r.current_bid)
    } else {
        format!("starts {}", r.starting_price)
    };
    let buyout = if r.buyout_price > 0 {
        format!(", buyout {}", r.buyout_price)
    } else {
        String::new()
    };
    let left = (i64::from(r.expires_at) - i64::from(now)).max(0);
    format!(
        "#{} {item}{stack} from {}: {price}{buyout}, {}h{:02}m left",
        r.sequence_id,
        r.seller.as_deref().unwrap_or("?"),
        left / 3_600,
        (left % 3_600) / 60,
    )
}

/// `.bm_list`: up to [`LIST_LIMIT`] active auctions, newest first, one line
/// each, then the total.
#[tracing::instrument(
    name = "black_market.gm_list",
    level = "info",
    skip_all,
    fields(entity_id = ctx.actor.entity_id, player_id = ctx.actor.player_id)
)]
pub async fn gm_list(ctx: GmCtx<'_>, db_pool: &Option<Arc<PgPool>>) {
    let Some(pool) = pool_or_refuse(&ctx, "bm_list", db_pool).await else {
        return;
    };
    let now = now_unix_secs();
    let read = async {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sgw_auction WHERE status = $1 AND expires_at > $2",
        )
        .bind(auction_status::ACTIVE)
        .bind(now)
        .fetch_one(pool)
        .await?;
        let rows: Vec<ListRow> = sqlx::query_as(
            "SELECT a.sequence_id, ri.name AS item, a.item_def_id, a.stack_size, \
                    p.player_name AS seller, a.starting_price, a.buyout_price, \
                    a.current_bid, a.current_bidder IS NOT NULL AS has_bidder, a.expires_at \
               FROM sgw_auction a \
               LEFT JOIN resources.items ri ON ri.item_id = a.item_def_id \
               LEFT JOIN sgw_player p ON p.player_id = a.seller_id \
              WHERE a.status = $1 AND a.expires_at > $2 \
              ORDER BY a.sequence_id DESC LIMIT $3",
        )
        .bind(auction_status::ACTIVE)
        .bind(now)
        .bind(LIST_LIMIT)
        .fetch_all(pool)
        .await?;
        Ok::<_, sqlx::Error>((total, rows))
    };
    let (total, rows) = match read.await {
        Ok(r) => r,
        Err(e) => {
            let line = format!(".bm_list: the database failed ({e}).");
            ctx.refuse("bm_list", "db_error", &line).await;
            return;
        }
    };
    tracing::info!(
        event = "bm.gm_action",
        action = "bm_list",
        entity_id = ctx.actor.entity_id,
        account_id = ctx.actor.account_id,
        player_id = ctx.actor.player_id,
        total,
        shown = rows.len(),
        "GM .bm_list read the active auctions"
    );
    if rows.is_empty() {
        ctx.tell("The Black Market has no active auctions. Type .bm_seed to list some.")
            .await;
        return;
    }
    for r in &rows {
        ctx.tell(&format_row(r, now)).await;
    }
    ctx.tell(&format!(
        "{} active auction(s), newest {} shown.",
        total,
        rows.len()
    ))
    .await;
}
