//! Live-DB guards for the Dakara_E1 rebuild's seed (worlds 61 `Dakara_E1` and
//! 62 `Dakara_E1_StoryRm`; `docs/analysis/dakara-e1-rebuild/work-packets.md`).
//!
//! These are seed-data guards: the rows under test are the production seed,
//! and each test fails when its rows are removed or changed. A directory from
//! the start, because the campaign's later packets add siblings here (spawn
//! rows, the hostile roster).
//!
//! - [`templates`]: DK-03, the cast and prop templates of
//!   `db/resources/Entities/Seed/entity_templates_dakara_e1.sql`: the roster,
//!   a name on every one, nothing hostile or attackable, a look some other
//!   template already wears, and which props are clickable from spawn.

mod templates;
mod travel;

use crate::cell::spawner::{load_spawn_templates, SpawnRecord};
use crate::test_support::require_db_or_skip;

/// The campaign's template ids for the cast and the interactable props
/// (DK-03). 460-479, the rest of the campaign's block, is DK-20's hostiles
/// and defenders, which these guards deliberately leave out.
const CAST_AND_PROPS: (i32, i32) = (440, 459);
