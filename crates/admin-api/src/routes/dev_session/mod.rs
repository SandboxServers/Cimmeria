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
mod session_kind_tests;
#[cfg(test)]
mod tests;

use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;

pub use handlers::{
    mint, refresh, DevSessionRequest, DevSessionResponse, RefreshRequest, TOKEN_TTL_SECONDS,
};
pub use token::{
    decode_token, encode_token, AuthError, TokenClaims, MIN_SECRET_BYTES, SCOPE_TELEMETRY_WRITE,
    SESSION_KIND_LAB, SESSION_KIND_PLAYER,
};

pub(crate) use token::load_secret;

#[cfg(test)]
pub use token::env_lock;

/// Request-body cap for both routes. The `Json` extractor reads and
/// deserializes the body before any quota is charged, so axum's 2 MiB
/// default would let an over-quota caller still make the server parse
/// megabytes per request. A real mint body is a few hundred bytes.
pub const MAX_BODY_BYTES: usize = 8 * 1024;

/// The handlers read no router state, so the routes fit any router: the
/// admin API nests them under `/api/auth`, and
/// [`crate::login_port_telemetry_router`] mounts them on the public SOAP
/// login port.
pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/dev-session", post(mint))
        .route("/dev-session/refresh", post(refresh))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}

/// Say once, at startup, whether this server can mint and accept telemetry
/// tokens, and where it tells callers to upload. Without it a missing
/// `CIMMERIA_TELEMETRY_HMAC_SECRET` only ever showed as a 500 on the
/// launcher's side and an empty `cimmeria-client` index here.
pub fn log_boot_status() {
    let upload_endpoint = handlers::upload_endpoint_env();
    let kill_switch = handlers::kill_switch_active();
    match load_secret() {
        Ok(_) => tracing::info!(
            upload_endpoint = %upload_endpoint,
            kill_switch,
            "dev-session telemetry: mint and ingest enabled"
        ),
        Err(e) => tracing::warn!(
            upload_endpoint = %upload_endpoint,
            reason = "dev_session_secret_unusable",
            "dev-session telemetry: every mint and upload will be refused: {e}"
        ),
    }
}
