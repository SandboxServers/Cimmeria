//! The SGWInventoryManager bandolier handlers.
//!
//! `constants` is wire contract (`cimmeria-wire`); it is re-exported here so
//! the moved handlers keep naming it as `super::super::constants` and
//! `cell_methods::inventory::build_entity_property_args`.

pub mod bandolier;
pub(crate) use cimmeria_wire::cell::cell_methods::inventory::constants;
pub(crate) use constants::{build_entity_property_args, GENERICPROPERTY_AMMO_TYPE_ID};
