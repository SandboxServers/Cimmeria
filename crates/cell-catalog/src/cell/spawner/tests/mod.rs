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
//! - [`live_db_debug_hub`]: live-DB guards for the Castle_CellBlock
//!   stasis-room debug hub seed (templates 300-304, spawns 400-404): role
//!   columns, placement, the crate's harmless ability set, the vendor lists,
//!   loot table 3 and the hub dialogs.
//! - [`live_db_ability_animation_links`]: live-DB guards that weapon-bound
//!   damage abilities carry the weapon family's event set, so their hits
//!   animate.
//! - [`live_db_pet_summons`]: live-DB guards for the pet seed (pets campaign
//!   PT-S): `pet_summons` rows name pet templates 350-359, pets are never in
//!   `spawnlist`, Summon Straegis carries its event set, and the Straegis pet
//!   template keeps its body, name and kit.
//! - [`live_db_pet_trainer`]: live-DB guards for the debug hub's pet trainer
//!   (pets campaign PT-07): template 360, spawn 450 in the stasis room, and
//!   trainer list 350 offering the Goa'uld pet summons.
//! - [`npc_ability_animation`]: live-DB seed linter (NA43) that every NPC combat
//!   ability resolves one Ability_End sequence, that 559 resolves the SMG
//!   burst, and that an armed hostile fires its weapon's ranged attack.

mod live_db_ability_animation_links;
mod live_db_castle_seed;
mod live_db_content_loaders;
mod live_db_debug_hub;
mod live_db_loaders;
mod live_db_pet_summons;
mod live_db_pet_trainer;
mod npc_ability_animation;
