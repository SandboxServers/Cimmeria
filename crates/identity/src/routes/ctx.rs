//! The state the `/auth/v1` routes share: the config, providers, stores and
//! the limiter, plus the database-backed dependencies from the auth crate.
//!
//! Handlers read the clock; the stores and the limiter take `now` as an
//! argument (contract C12).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ConnectInfo;
use cimmeria_auth::AuthRouteDeps;

use crate::config::IdentityConfig;
use crate::limits::{Bucket, RateLimiter};
use crate::link_code::LinkCodeStore;
use crate::providers::Providers;
use crate::routes::error::ApiError;
use crate::stores::Stores;

/// Everything the identity routes need besides the auth crate's deps.
#[derive(Clone)]
pub struct IdentityCtx {
    /// The validated `auth-providers.toml` and the kill switches.
    pub cfg: Arc<IdentityConfig>,
    /// The running providers, built once at startup.
    pub providers: Arc<Providers>,
    /// The in-memory sign-in stores (pending, handoff, access, link grants).
    pub stores: Arc<Stores>,
    /// The keyed limiter behind every C7 bucket.
    pub limiter: Arc<RateLimiter>,
    /// The GM link-code store (C10), shared process-wide.
    pub link_codes: &'static LinkCodeStore,
}

impl IdentityCtx {
    /// Counts one request against `bucket` for `key` and answers
    /// `429 rate_limited` when the bucket is full (contract C7).
    pub fn limit(&self, bucket: Bucket, key: &str, now: Instant) -> Result<(), ApiError> {
        self.limiter
            .check(bucket, key, now)
            .map_err(|wait| ApiError::RateLimited {
                retry_after: wait.secs,
            })
    }
}

/// The router state: the auth crate's deps (behind an `Arc`, since they are
/// not `Clone`) and the identity context.
#[derive(Clone)]
pub struct RouteState {
    /// The database pool, the launch-ticket store and developer mode.
    pub deps: Arc<AuthRouteDeps>,
    /// The identity context the handlers read.
    pub ctx: IdentityCtx,
}

/// The per-request key for the IP buckets: the peer address, as text.
///
/// The peer is the address the listener saw. The keys stay in memory only and
/// are never logged (contract C11).
pub fn client_ip(ConnectInfo(peer): ConnectInfo<SocketAddr>) -> String {
    peer.ip().to_string()
}
