//! CB-N05-3 — mission 62050 "The Remaining Pods" (Castle_CellBlock,
//! world 12). Live-DB chain-replay guards for chains 11000-11005 seeded in
//! `db/resources/Content/Seed/castle_cellblock_chains.sql`. Literals are
//! frozen in `docs/analysis/cellblock-continuation/w3-frozen.md` § F4; the
//! seal is `docs/analysis/cellblock-continuation/worknotes/cb-n05-seal.md`.
//!
//! [`rows`] pins the seeded rows and the load path. [`resolve`] pins each
//! offer, accept and follow-up gate, the Marsh completion beside the core
//! chains, and that no dialog path moves the mission. The helpers below are shared with the
//! executor replay (CB-N05-7).

use cimmeria_content_engine::context::ExecutionContext;

mod executor;
mod resolve;
mod rows;

/// The six chains of mission 62050, in seed order.
pub(super) const CHAINS: [i32; 6] = [11000, 11001, 11002, 11003, 11004, 11005];

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
