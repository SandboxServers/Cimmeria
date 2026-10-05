//! `.bm_seed [count]` (BM-07): list system-seller auctions from the UAT set.

use std::sync::Arc;

use sqlx::PgPool;

use super::super::helpers::now_unix_secs;
use super::super::seed::{
    ensure_system_seller, insert_system_listing, uat_specs, SystemSellerError,
};
use super::{pool_or_refuse, GmCtx, MAX_SEED_COUNT};

/// `.bm_seed [count]`: check the system seller exactly as the boot seed
/// does, then insert `count` listings cycling through the UAT set, in one
/// transaction, and tell the GM the new auction ids.
#[tracing::instrument(
    name = "black_market.gm_seed",
    level = "info",
    skip_all,
    fields(entity_id = ctx.actor.entity_id, player_id = ctx.actor.player_id, count)
)]
pub async fn gm_seed(ctx: GmCtx<'_>, count: u8, db_pool: &Option<Arc<PgPool>>) {
    let Some(pool) = pool_or_refuse(&ctx, "bm_seed", db_pool).await else {
        return;
    };
    let count = count.clamp(1, MAX_SEED_COUNT);
    if let Err(e) = ensure_system_seller(pool).await {
        let line = match &e {
            SystemSellerError::Conflict { found, .. } => format!(
                ".bm_seed: refused, the reserved system seller ids are taken ({found}). \
                 Nothing was listed."
            ),
            SystemSellerError::Db(_) => {
                ".bm_seed: the database failed checking the system seller. Nothing was listed."
                    .to_string()
            }
        };
        let reason = match &e {
            SystemSellerError::Conflict { reason, .. } => reason,
            SystemSellerError::Db(_) => "db_error",
        };
        ctx.refuse("bm_seed", reason, &line).await;
        return;
    }

    let ids = match insert_listings(pool, count).await {
        Ok(ids) => ids,
        Err(e) => {
            let line = format!(".bm_seed: the database failed ({e}). Nothing was listed.");
            ctx.refuse("bm_seed", "db_error", &line).await;
            return;
        }
    };
    let (first, last) = (ids[0], ids[ids.len() - 1]);
    let id = ctx.identity();
    tracing::info!(
        event = "bm.gm_action",
        action = "bm_seed",
        entity_id = ctx.actor.entity_id,
        entity_name = id.player_name,
        account_id = ctx.actor.account_id,
        account_name = id.account_name,
        player_id = ctx.actor.player_id,
        player_name = id.player_name,
        count = ids.len(),
        first_auction_id = first, // nt:id-only seeded system listings have no name column
        last_auction_id = last,   // nt:id-only seeded system listings have no name column
        "GM .bm_seed listed system-seller auctions"
    );
    ctx.tell(&format!(
        "Listed {} Black Market auction(s) from the system seller: ids {first} to {last}. \
         Search the Black Market, or type .bm_list.",
        ids.len()
    ))
    .await;
}

/// Insert `count` listings, cycling through [`uat_specs`], in one
/// transaction; returns their ids in order.
pub(super) async fn insert_listings(pool: &PgPool, count: u8) -> Result<Vec<i32>, sqlx::Error> {
    let now = now_unix_secs();
    let specs = uat_specs();
    let mut tx = pool.begin().await?;
    let mut ids = Vec::with_capacity(usize::from(count));
    for spec in specs.iter().cycle().take(usize::from(count)) {
        ids.push(insert_system_listing(&mut tx, spec, now).await?);
    }
    tx.commit().await?;
    Ok(ids)
}
