//! Tests for the `resources` module, split by theme (issue #529):
//! - [`category_map`]: client-registration category numbering. The behavior-event
//!   wire-tag test drives the fragment builder, so it stayed in `cimmeria-services`
//!   (`base::resource_fragment_tests`).
//! - [`inventory_slots`]: bag-slot helpers + `pick_first_open_bag`.
//! - [`committed_paks`]: invariants on the PAK files under `data/cache/`.
//! - [`mission_additions_dakara`]: the production Dakara mission 63100 (DK-10fs): exact XML, new ids, served catalog, seed agreement.
//! - [`overrides`]: mission + item override application and metadata bumps.
//! - [`dialog_overrides`]: dialog override application and metadata bumps.
//! - [`world_info_overrides`]: the historical CellBlock worlds on category 12.
//! - [`stargate_overrides`]: the Debug Area gate (29) on category 13.
//! - [`seed_id_uniqueness`]: template and spawn ids unique across the per-zone seed splits.

mod category_map;
mod committed_paks;
mod dialog_overrides;
mod inventory_slots;
mod mission_additions;
mod mission_additions_castle;
mod mission_additions_castle_defense_q1;
mod mission_additions_castle_defense_q3;
mod mission_additions_castle_medical_q2;
mod mission_additions_castle_repeatable_q1;
mod mission_additions_castle_return_q1;
mod mission_additions_castle_return_q2;
mod mission_additions_cellblock;
mod mission_additions_cellblock_62060;
mod mission_additions_dakara;
mod overrides;
mod seed_id_uniqueness;
mod stargate_overrides;
mod world_info_overrides;
