//! # cimmeria-content-engine
//!
//! Data-driven content system for the Cimmeria server emulator. Replaces
//! hand-written Python scripts with database-configured trigger/condition/action
//! chains. Each chain binds a trigger event to a sequence of conditional checks
//! and resulting actions, allowing game designers to author content without code
//! changes.
//!
//! ## Architecture
//!
//! The engine operates on **chains**, each consisting of:
//!
//! 1. **Trigger** -- the event that activates the chain (entity created, ability
//!    used, region entered, etc.)
//! 2. **Conditions** -- zero or more predicates that must all pass before actions
//!    fire (property checks, item ownership, faction standing, etc.)
//! 3. **Actions** -- ordered list of effects to apply (grant XP, spawn entity,
//!    advance mission, teleport, etc.)
//!
//! Chains are loaded from the database at startup and indexed by trigger type for
//! efficient event dispatch. When a game event fires, the [`ChainEngine`] finds
//! all matching chains, evaluates their conditions against the current
//! [`ExecutionContext`], and executes actions in priority order.
//!
//! [`ChainEngine`]: chain::ChainEngine
//! [`ExecutionContext`]: context::ExecutionContext

pub mod chain;
pub mod triggers;
pub mod conditions;
pub mod actions;
pub mod context;
pub mod loader;
