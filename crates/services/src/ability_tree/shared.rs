//! The process's one loaded [`AbilityTreeCatalog`].
//!
//! The cell (trainer, purchase gate) and the base (player-load
//! `onAbilityTreeInfo`) run in one process and must present the same tree
//! in the same order. Both take it from here, so the table is read once per
//! process by [`AbilityTreeCatalog::load`] and both sides hold the same
//! snapshot. The table is seed data: it only changes on a redeploy, which
//! restarts the process.
//!
//! A failed load is not cached; the next caller retries.

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::OnceCell;

use super::AbilityTreeCatalog;

static SHARED: OnceCell<Arc<AbilityTreeCatalog>> = OnceCell::const_new();

/// The catalog, loaded from `pool` on first use.
///
/// Every later call returns the first snapshot whatever `pool` it passes;
/// the server has one database.
pub async fn shared_catalog(pool: &PgPool) -> Result<Arc<AbilityTreeCatalog>, sqlx::Error> {
    SHARED
        .get_or_try_init(|| async { AbilityTreeCatalog::load(pool).await.map(Arc::new) })
        .await
        .cloned()
}
