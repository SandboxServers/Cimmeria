//! The dialog display choke point, under the `cell::interactions` path it had
//! in `cimmeria-services`.
//!
//! The content executor opens dialogs through [`send_dialog_display`], so it
//! sits in this crate (docs/architecture/services-crate-split.md §2G). The rest
//! of the NPC interaction handler (the `interact` dispatch, loot, trainers,
//! vendors, the DHD) sits above it, in `cimmeria-cell-interactions`'
//! `cell::interactions`, which imports this module under the same name.

pub mod dialog;

pub use dialog::send_dialog_display;
