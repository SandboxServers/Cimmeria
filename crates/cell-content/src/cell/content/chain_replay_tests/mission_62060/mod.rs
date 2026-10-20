//! CB-N06-3 — mission 62060 "Before the Patrol Returns" (Castle_CellBlock,
//! world 12). Live-DB chain-replay guards for chains 11030-11041 seeded in
//! `db/resources/Content/Seed/castle_cellblock_chains.sql`. Literals are
//! frozen in `docs/analysis/cellblock-continuation/w5-frozen.md` § F4; the
//! seal is `docs/analysis/cellblock-continuation/worknotes/cb-n06-seal.md`.
//!
//! [`rows`] pins the seeded rows and the load path. [`resolve`] and
//! [`executor`] pin the resolver and the chain executor against the real
//! seeded chains.

use cimmeria_content_engine::context::ExecutionContext;

mod executor;
mod resolve;
mod rows;

/// The twelve chains of mission 62060, in seed order.
pub(super) const CHAINS: [i32; 12] = [
    11030, 11031, 11032, 11033, 11034, 11035, 11036, 11037, 11038, 11039, 11040, 11041,
];

/// `resources.worlds.world_id` of `Castle_CellBlock`.
pub(super) const CELLBLOCK_WORLD_ID: i32 = 12;

/// Build an [`ExecutionContext`] for one replay: `world` goes to
/// `ctx.world_id` (`None` leaves it unset, which the `world` condition
/// fails closed on), each `(key, value)` pair becomes a string param (the
/// `mission_<m>_status` and `mission_<m>_step_<s>_status` keys), and
/// `archetype` becomes an i64 param.
pub(super) fn ctx(
    world: Option<i32>,
    params: &[(&str, &str)],
    archetype: Option<i64>,
) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = world;
    for (key, value) in params {
        ctx.set_param((*key).to_string(), serde_json::json!(*value));
    }
    if let Some(archetype) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    }
    ctx
}
