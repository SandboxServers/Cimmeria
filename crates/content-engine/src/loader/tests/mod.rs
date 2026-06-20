//! Tests for chain loading and DB-row → typed-enum conversion,
//! split by theme. Each submodule imports the loader surface it needs
//! via `super::super::*` (the `loader` module) plus explicit `crate::`
//! paths for the typed enums.

mod action_conversion;
mod chain_loading;
mod condition_conversion;
mod trigger_conversion;
