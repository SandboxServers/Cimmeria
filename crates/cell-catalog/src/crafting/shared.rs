//! The process's one loaded [`CraftingCatalog`].
//!
//! The cell (argument checks, the station gate) and the base (every verb's
//! validation) run in one process and must judge a request against the same
//! data. Both take the catalog from here, so the tables are read once per
//! process. They are seed data and only change on a redeploy, which restarts
//! the process.
//!
//! A failed load is not cached; the next caller retries.

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::OnceCell;

use super::CraftingCatalog;

static SHARED: OnceCell<Arc<CraftingCatalog>> = OnceCell::const_new();

/// The catalog, loaded from `pool` on first use.
///
/// Every later call returns the first snapshot whatever `pool` it passes;
/// the server has one database.
pub async fn shared_crafting_catalog(pool: &PgPool) -> Result<Arc<CraftingCatalog>, sqlx::Error> {
    SHARED
        .get_or_try_init(|| async { CraftingCatalog::load(pool).await.map(Arc::new) })
        .await
        .cloned()
}
