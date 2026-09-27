//! Tests for the spawner's DB loaders, split by theme (issue #529). The
//! tests that also need `SpaceManager`, combat or the GM spawn handler stay
//! in `cimmeria-services` (`cell::spawner_tests`).
//!
//! - [`live_db_loaders`]: live-DB sanity guards for the spawner loader queries
//!   themselves — column renames, type drift, JOIN breakage.
//! - [`live_db_content_loaders`]: the same, for the mission / objective /
//!   dialog-set / monologue loaders.
//! - [`live_db_castle_seed`]: live-DB guards for Castle (World 8) *seed content*
//!   that loads fine and is nonetheless wrong — actors outside the box that is
//!   meant to contain them, missing display names, missing respawn timers.
//! - [`live_db_ability_animation_links`]: live-DB guards that weapon-bound
//!   damage abilities carry the weapon family's event set, so their hits
//!   animate.
//! - [`npc_ability_animation`]: live-DB seed linter (NA43) that every NPC combat
//!   ability resolves one Ability_End sequence, that 559 resolves the SMG
//!   burst, and that an armed hostile fires its weapon's ranged attack.

mod live_db_ability_animation_links;
mod live_db_castle_seed;
mod live_db_content_loaders;
mod live_db_loaders;
mod npc_ability_animation;
