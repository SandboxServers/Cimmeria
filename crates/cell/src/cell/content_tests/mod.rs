//! The content tests that drive the relog hydration,
//! `service::base_messages::player_init::mission_restore`, which is this
//! crate's. Waves C3 to C5a of the services crate split
//! (docs/architecture/services-crate-split.md) left them in `cimmeria-services`
//! as `cell::content_tests` beside the hydration; wave C6 moved them here with
//! it, under the same path.
//!
//! - [`mission_742_hydration`] (was `chain_replay_tests::mission_742` in the
//!   content crate's tree) and [`mission_relog_persistence`] (was
//!   `chain_replay_tests::mission_relog_persistence`).
//!
//! The content internals they call (`execute_actions`,
//! `populate_mission_context`, `load_single_chain_for_test`) are the content
//! crate's `test-support` hooks.
//!
//! The other tests of the old `cell::content_tests` went elsewhere:
//! `mission_701_persistence` drives the base's `query_saved_missions`, so it
//! stays in `cimmeria-services`; the GM half of `mission_abandoned` is
//! `cimmeria-cell-console`'s `cell::console::gm::mission_abandoned_tests`.

mod mission_742_hydration;
mod mission_relog_persistence;
