//! Exact carried item counts of a staged fixture. FX-03 writes them; until
//! then an item fixture is refused rather than silently skipped, so a row
//! that needs an item never runs on a state without it.

use sqlx::PgPool;

use cimmeria_base_session::base::world_entry::lab_fixture::StagedItem;

/// The refusal of an item fixture until FX-03. The apply's precheck returns
/// it before any write; [`apply_items`] keeps it as the backstop.
pub(crate) const ITEMS_NOT_YET: &str = "item fixtures land in FX-03";

/// Set each item's carried count; returns `(removed, granted)` instances.
pub(crate) async fn apply_items(
    _pool: &PgPool,
    _player_id: i32,
    items: &[StagedItem],
) -> Result<(u64, u64), String> {
    if items.is_empty() {
        return Ok((0, 0));
    }
    Err(ITEMS_NOT_YET.to_string())
}
