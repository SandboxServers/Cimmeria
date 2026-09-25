//! Dev-session telemetry auth endpoints.
//!
//! Mints a short-lived HMAC-SHA256 token the launcher uses to upload
//! its session telemetry. The ingest endpoints under
//! [`crate::routes::telemetry`] verify the same token with the same
//! shared `CIMMERIA_TELEMETRY_HMAC_SECRET`.
//!
//! Token shape:
//!
//! ```text
//! payload = base64url(JSON {iss, sub, sid, iat, exp, scope})
//! sig     = base64url(HMAC-SHA256(secret, payload))
//! token   = payload || "." || sig
//! ```
//!
//! # Trust model
//!
//! Minting needs no credential — the launcher holds no static secret
//! and the `sub` claim is the caller's own `install_id`. What bounds
//! the damage is therefore not authentication but three limits:
//!
//! - the token carries only `telemetry.write`, and the ingest
//!   endpoints refuse a token without it;
//! - mint and refresh are quota-limited per peer address, and mint
//!   additionally per `install_id` ([`quota`]);
//! - a minted session cannot be extended past
//!   `CIMMERIA_TELEMETRY_MAX_SESSION_SECS` by chaining refreshes,
//!   because `iat` records the original mint and is never reset.
//!
//! Binding a mint to a registered installation is still open; it
//! needs a launcher-side handshake.
//!
//! `CIMMERIA_TELEMETRY_KILL_SWITCH=1` makes every mint and refresh
//! return 503 with `Retry-After: 60`.
//!
//! # Module layout
//!
//! - [`token`] — claims, HMAC encode/decode, secret loading, the
//!   endpoint family's error type.
//! - [`quota`] — the fixed-size mint/refresh counter tables.
//! - `handlers` — the two axum handlers and their operator-tunable
//!   policy.

pub mod quota;
pub mod token;

mod handlers;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;

use cimmeria_services::orchestrator::Orchestrator;

pub use handlers::{
    mint, refresh, DevSessionRequest, DevSessionResponse, RefreshRequest, TOKEN_TTL_SECONDS,
};
pub use token::{
    decode_token, encode_token, load_secret, AuthError, TokenClaims, MIN_SECRET_BYTES,
    SCOPE_TELEMETRY_WRITE,
};

#[cfg(test)]
pub use token::env_lock;

/// Request-body cap for both routes. The `Json` extractor reads and
/// deserializes the body before any quota is charged, so axum's 2 MiB
/// default would let an over-quota caller still make the server parse
/// megabytes per request. A real mint body is a few hundred bytes.
pub const MAX_BODY_BYTES: usize = 8 * 1024;

pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/dev-session", post(mint))
        .route("/dev-session/refresh", post(refresh))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}
