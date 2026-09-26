//! Test helpers for this crate's tests.
//!
//! The generic helpers live in `cimmeria-test-support` and are re-exported
//! here, so tests keep importing them from `crate::test_support`:
//!
//! - **Live-DB**: [`require_db_or_skip!`] opens a pool against
//!   `DATABASE_URL`. It skips when the variable is unset and **fails** when it
//!   is set but unreachable (#615). See
//!   `docs/architecture/integration-test-infra.md`.
//! - **Log capture**: [`LogCapture`] for negative-logging regression guards.
//! - **Transport fake**: [`TestTransport`], the recording UDP fake behind the
//!   **fan-out byte test** type in `TESTING.md`. See
//!   `docs/architecture/transport-trait.md`.
//!
//! The domain fixtures live next to the types they build and are re-exported
//! here: `make_space_manager*`, `seed_ability_defs`, the occluder and
//! arrival-mesh helpers and the `ContentEvents` fakes come from
//! `cimmeria_cell_world::test_fixtures` (wave C1), and
//! `test_default_connected_client_state` from
//! `cimmeria_base_session::test_fixtures` (wave B1). See
//! `docs/architecture/services-crate-split.md` §3.

pub(crate) use cimmeria_base_session::test_fixtures::*;
pub(crate) use cimmeria_cell_world::test_fixtures::*;
pub(crate) use cimmeria_test_support::*;
