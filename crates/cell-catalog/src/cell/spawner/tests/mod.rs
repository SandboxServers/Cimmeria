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
//!   columns, placement (the Gate Mail Clerk's spawn 490 included), the
//!   crate's harmless ability set, the vendor lists, loot table 3 and the hub
//!   dialogs.
//! - [`live_db_mail_clerk`]: live-DB guards for the debug hub's Gate Mail
//!   Clerk (social-systems SS-U3): template 390's role columns and dialog
//!   60104.
//! - [`live_db_crafting_hub`]: live-DB guards for the hub's crafting corner
//!   (stations 310-313, the supplies vendor 314, spawns 410-414, buy list
//!   310): the craft flags, placement and the supplies list.
//! - [`live_db_ability_ranges`]: live-DB guards that the loader converts the
//!   seeded UE3-unit ability ranges to metres (#919).
//! - [`live_db_ability_animation_links`]: live-DB guards that weapon-bound
//!   damage abilities carry the weapon family's event set, so their hits
//!   animate.
//! - [`live_db_pet_summons`]: live-DB guards for the pet seed (pets campaign
//!   PT-S): `pet_summons` rows name pet templates 350-359, pets are never in
//!   `spawnlist`, Summon Straegis carries its event set, and the Straegis pet
//!   template keeps its body, name and kit.
//! - [`live_db_pet_roster`]: live-DB guards for the rest of the Servant Lord
//!   roster (PT-11): Jaffa, Prime and Lo'taur summons resolve to unplaced pet
//!   templates 351-353 with their kits, looks and names.
//! - [`live_db_pet_trainer`]: live-DB guards for the debug hub's pet trainer
//!   (pets campaign PT-07): template 360, spawn 450 in the stasis room, and
//!   trainer list 350 offering the Goa'uld pet summons.
//! - [`live_db_debug_auctioneer`]: live-DB guards for the debug hub's Black
//!   Market auctioneer (BM-07): template 305 is an auctioneer and nothing
//!   else, the only `INT_AUCTION` template, and spawn 405 stands in the
//!   stasis room clear of every other NPC.
//! - [`live_db_deployables`]: live-DB guards for the deployables seed
//!   (Phase 0): the 1012 row, template 400 and the cooked numbers of 5065
//!   and 5066.
//! - [`live_db_debug_banker`]: live-DB guards for the debug hub's Banker
//!   (bank-vault BV-04): template 370 is a personal Banker and nothing else,
//!   and spawn 470 stands in the stasis room clear of every other NPC.
//! - [`live_db_debug_registrars`]: live-DB guards for the debug hub's
//!   organization registrars (ORG-05): templates 330 and 331 are Team and
//!   Command registrars and nothing else, the only ones, and spawns 430 and
//!   431 stand in the stasis room clear of every other NPC.
//! - [`live_db_spawnlist_sequence`]: live-DB guard that the `spawnlist` id
//!   sequence starts past every reserved campaign spawn block, so a row
//!   inserted without an id (`.savespawn`) never takes a reserved one.
//! - [`live_db_seed_sequences`]: the id sequences of the tables campaigns
//!   seed explicit id blocks into allocate past every seeded row.
//! - [`npc_ability_animation`]: live-DB seed linter (NA43) that every NPC combat
//!   ability resolves one Ability_End sequence, that 559 resolves the SMG
//!   burst, and that an armed hostile fires its weapon's ranged attack.

mod live_db_ability_animation_links;
mod live_db_ability_ranges;
mod live_db_castle_seed;
mod live_db_content_loaders;
mod live_db_crafting_hub;
mod live_db_debug_auctioneer;
mod live_db_debug_banker;
mod live_db_debug_hub;
mod live_db_debug_org_bankers;
mod live_db_debug_registrars;
mod live_db_deployables;
mod live_db_loaders;
mod live_db_mail_clerk;
mod live_db_pet_roster;
mod live_db_pet_summons;
mod live_db_pet_trainer;
mod live_db_seed_sequences;
mod live_db_spawnlist_sequence;
mod npc_ability_animation;
