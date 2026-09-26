//! The content tests that stayed in this crate when wave C3 of the services
//! crate split (docs/architecture/services-crate-split.md) moved the content
//! executor, missions and the ring dispatcher to `cimmeria-cell-content`.
//!
//! Each drives something that is still here or above the content crate, so it
//! could not move with the code it tests:
//!
//! - [`mission_abandoned`] — the GM cell-method dispatcher, `cell::console::gm`
//!   (was `event_dispatch::mission_abandoned_tests`; its chain-action test
//!   moved to the content crate, and its Missionary tests to
//!   `cimmeria-cell-methods` in wave C5a).
//! - [`mission_701_persistence`] — the base's `query_saved_missions`
//!   (was `chain_replay_tests::mission_701::persistence`).
//! - [`mission_742_hydration`] and [`mission_relog_persistence`] — the relog
//!   hydration, `player_init::mission_restore` (were in
//!   `chain_replay_tests`).
//!
//! `stargate_grant_dial`, which drives the gate dial, was here too until wave
//! C4 moved the dial to `cimmeria-cell-interactions`; it is that crate's
//! `cell::gate_travel::tests::stargate_grant_dial` now.
//! `aoe_health_below`, which drives the `useAbilityOnGround` cell method, went
//! the same way in wave C5a: it is `cimmeria-cell-methods`'
//! `cell::cell_methods::player::combat::tests::aoe_health_below`.
//!
//! Where a file kept only some of its tests, the fixtures they share are
//! copies. The content internals they call (`execute_actions`,
//! `populate_mission_context`, `load_single_chain_for_test`) are the content
//! crate's `test-support` hooks.

mod mission_701_persistence;
mod mission_742_hydration;
mod mission_abandoned;
mod mission_relog_persistence;
