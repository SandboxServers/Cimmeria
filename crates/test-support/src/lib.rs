//! # cimmeria-test-support
//!
//! Generic test helpers shared by the Cimmeria service crates. Use it as a
//! **dev-dependency only**.
//!
//! - **Live-DB gate**: tests that need PostgreSQL call
//!   [`require_db_or_skip!`]. They skip when `DATABASE_URL` is unset and
//!   **fail** when it is set but unreachable (#615). See
//!   `docs/architecture/integration-test-infra.md`.
//! - **Log capture**: [`LogCapture`] records tracing events so a regression
//!   guard can assert that a WARN/ERROR fired with the right structured
//!   fields. See `docs/architecture/negative-logging-convention.md`.
//! - **Transport fake**: [`TestTransport`] is the recording UDP fake behind
//!   the fan-out byte tests. See `docs/architecture/transport-trait.md`.
//! - **Source scans**: [`source_scan`] walks every crate's Rust sources for
//!   guard tests that must hold across the workspace, with paths that survive
//!   a module moving to another crate.
//!
//! This crate must never depend on a service crate: those crates' tests
//! would then link two copies of the service crate, each with its own
//! statics. Domain fixtures (`make_space_manager`, ...) stay in the crate
//! that owns their types. See `docs/architecture/services-crate-split.md`
//! §3.
//!
//! Each consuming crate keeps a shim so its tests import these helpers from
//! `crate::test_support`:
//!
//! ```ignore
//! #[cfg(test)]
//! mod test_support {
//!     pub(crate) use cimmeria_test_support::*;
//! }
//! ```

#![warn(unreachable_pub)]

mod live_db_gate;
mod log_capture;
pub mod source_scan;

pub use live_db_gate::{pool_or_skip, test_pool, test_pool_from_url, SkipReason};
pub use log_capture::{Captured, LogCapture, LogCaptureGuard};

/// The canonical recording UDP fake: a
/// [`cimmeria_mercury::transport::Transport`] that handler tests pass as
/// `&Arc<dyn Transport>` in place of a real socket, then assert byte-exact,
/// address-correct fan-out on.
pub use cimmeria_mercury::test_transport::TestTransport;
