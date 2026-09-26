//! The content test that stayed in this crate when wave C3 of the services
//! crate split (docs/architecture/services-crate-split.md) moved the content
//! executor, missions and the ring dispatcher to `cimmeria-cell-content`.
//!
//! - [`mission_701_persistence`] — the base's `query_saved_missions`
//!   (was `chain_replay_tests::mission_701::persistence`). It drives base
//!   code, so it waits here for wave F with the other cross-track tests.
//!
//! The others went to the crate of the code they drive as later waves moved
//! it: `stargate_grant_dial` (the gate dial) to `cimmeria-cell-interactions`
//! in wave C4, `aoe_health_below` and the Missionary half of
//! `mission_abandoned` (the cell methods) to `cimmeria-cell-methods` in wave
//! C5a, and, in wave C6, the GM half of `mission_abandoned` to
//! `cimmeria-cell-console` (`cell::console::gm::mission_abandoned_tests`) and
//! `mission_742_hydration` and `mission_relog_persistence` (the relog
//! hydration, `player_init::mission_restore`) to `cimmeria-cell`, at the same
//! `cell::content_tests` path.

mod mission_701_persistence;
