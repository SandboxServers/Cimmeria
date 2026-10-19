//! Live-DB guards for the Beta E1 seed (UB-02a;
//! `docs/analysis/union-zone-planning/beta-packets/UB-02a.md`).
//!
//! These are seed-data guards: the rows under test are the production seed,
//! and each test fails when its rows are removed or changed.
//!
//! - [`hub`]: UB-02b, the hub cast and objects (spawns 23000-23021) and the
//!   Storage and AdminBuilding regions: rows, positions, standability and tag
//!   uniqueness.
//! - [`templates`]: UB-02a, the cast and object templates of
//!   `db/resources/Entities/Seed/entity_templates_beta_e1.sql`: the roster, a
//!   name on each, nothing hostile or attackable, the visual settings, the
//!   object and NPC interaction bits, and the reserved ids left empty.

mod hub;
mod nav;
mod templates;

use crate::cell::spawner::{load_spawn_templates, SpawnRecord};
use crate::test_support::require_db_or_skip;

/// The Beta E1 block's template ids (UB-02a, 2300-2349). Ids 2303, 2327 and
/// 2336 are reserved and have no row; the rest of the block is unused.
const BLOCK: (i32, i32) = (2300, 2349);
