//! Reading integer action params without silent `as i32` wrapping.
use tracing::warn;

use super::DbActionRow;

/// A param was present but outside `i32`; the caller drops the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OutOfRange;

/// `params[key]` as an `i32`: `Ok(None)` when absent or not an integer,
/// `Err(OutOfRange)` (with a WARN naming the chain) when it does not fit.
pub(super) fn opt_i32(row: &DbActionRow, key: &str) -> Result<Option<i32>, OutOfRange> {
    let Some(raw) = row.params.get(key) else {
        return Ok(None);
    };
    // serde_json keeps integers above i64::MAX as u64, where `as_i64` is
    // None; treating that as "absent" would apply the default silently.
    let Some(v) = raw.as_i64() else {
        if let Some(big) = raw.as_u64() {
            warn!(
                chain_id = row.chain_id,
                chain_name = cimmeria_names::book().chain(row.chain_id),
                action_type = %row.action_type,
                param = key,
                value = big,
                "action param is out of i32 range; dropping action"
            );
            return Err(OutOfRange);
        }
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
