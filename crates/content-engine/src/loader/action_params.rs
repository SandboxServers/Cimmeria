//! Reading integer action params without silent `as i32` wrapping.
use tracing::warn;

use super::DbActionRow;

/// A param was present but outside `i32`; the caller drops the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OutOfRange;

/// `params[key]` as an `i32`: `Ok(None)` when absent or not an integer,
/// `Err(OutOfRange)` (with a WARN naming the chain) when it does not fit.
pub(super) fn opt_i32(row: &DbActionRow, key: &str) -> Result<Option<i32>, OutOfRange> {
    let Some(v) = row.params.get(key).and_then(|v| v.as_i64()) else {
        return Ok(None);
    };
    i32::try_from(v).map(Some).map_err(|_| {
        warn!(
            chain_id = row.chain_id,
            chain_name = cimmeria_names::book().chain(row.chain_id),
            action_type = %row.action_type,
            param = key,
            value = v,
            "action param is out of i32 range; dropping action"
        );
        OutOfRange
    })
}
