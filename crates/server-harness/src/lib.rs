//! Spawn/readiness/reap harness for the `cimmeria-server` process.
//!
//! Integration tests that need a *real* server — a bound Mercury UDP socket,
//! a live auth endpoint, the actual world-entry path — need to start one,
//! know when it is usable, and be certain it dies afterwards. The
//! [wireclient ADR](../../../docs/architecture/wireclient.md) lists that
//! lifecycle as an unsolved risk to be settled before any harness lands:
//! *"Crash safety + port collisions defined before the harness lands, not
//! after."* This crate is that answer.
//!
//! # The three problems, and the answers
//!
//! | Problem | Answer | Module |
//! |---|---|---|
//! | When is it ready? | `GET /api/config/status` reports `auth`/`base`/`cell` healthy — an observed condition, never a sleep | [`readiness`] |
//! | Will it collide? | Every one of the six ports is reserved from the OS and passed by environment | [`ports`] |
//! | Will it leak? | `Drop` kill+reap, plus a kernel-level kill-on-parent-death layer | [`process`] |
//!
//! # Usage
//!
//! ```no_run
//! use cimmeria_server_harness::{HarnessConfig, HarnessError, ServerHarness};
//!
//! # fn main() {
//! let config = HarnessConfig::default();
//! let server = match ServerHarness::start(config) {
//!     Ok(server) => server,
//!     // Skip rather than fail when the binary hasn't been built — the same
//!     // discipline the pcap-replay fixtures use.
//!     Err(HarnessError::ServerBinaryNotFound { .. }) => return,
//!     Err(e) => panic!("harness failed: {e}"),
//! };
//!
//! // Talk to the server on `server.logon_addr()` / `server.base_addr()`.
//!
//! // Reaped here, including if the body above panicked.
//! drop(server);
//! # }
//! ```
//!
//! # Scope
//!
//! Everything here is server-side. The harness can bring a server up to
//! "accepting connections"; it does not drive the game client, and nothing in
//! it assumes a client exists.

pub mod harness;
pub mod ports;
pub mod process;
pub mod readiness;

pub use harness::{HarnessConfig, HarnessError, ServerHarness, SERVER_EXE_ENV};
pub use ports::{PortKind, PortSet};
pub use process::{is_process_alive, ChildGuard, OrphanProtection};
pub use readiness::{ProbeOutcome, ReadinessCheck};
