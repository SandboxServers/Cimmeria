//! # cimmeria-auth
//!
//! The authentication service of the Cimmeria server emulator: the SOAP/HTTP
//! login handshake (Phase 1 credential check, Phase 2 shard selection), its
//! TLS termination and certificate hot-reload, stored-password verification,
//! login audit events, and log-safe credential rendering.
//!
//! Split out of `cimmeria-services` (wave W1a of
//! `docs/architecture/services-crate-split.md`). The module paths are the ones
//! the code had there, so `crate::auth::…`, `crate::audit::…` and
//! `crate::credential_redaction::…` resolve unchanged, and `cimmeria-services`
//! re-exports all three at their old paths.
//!
//! Tracing targets are this crate's module paths (`cimmeria_auth::auth::…`);
//! `auth.log` and the OTLP filter in `cimmeria-server` name them.

#![warn(unreachable_pub)]

pub mod audit;
pub mod auth;
pub mod credential_redaction;

// Generic helpers from `cimmeria-test-support` (a dev-dependency), imported by
// tests as `crate::test_support::…`, as they were in cimmeria-services.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
