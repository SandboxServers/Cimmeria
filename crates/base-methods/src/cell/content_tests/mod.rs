//! A content replay that drives only this crate: mission 701's world hop
//! must not lose the player's saved missions.
//!
//! - [`mission_701_persistence`] — the base's `query_saved_missions`
//!   (was `cell::content::chain_replay_tests::mission_701::persistence` in
//!   `cimmeria-services`). Wave C3 of the services crate split
//!   (docs/architecture/services-crate-split.md) left it in the services
//!   crate as `cell::content_tests::mission_701_persistence`, because the
//!   content crate cannot reach the base; its only non-test dependency is
//!   this crate's mission query, so wave F moved it here under the same
//!   module path.

mod mission_701_persistence;
