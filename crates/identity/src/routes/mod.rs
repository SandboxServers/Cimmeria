//! The `/auth/v1` HTTP routes of the sign-in flow (contract C6).
//!
//! [`router`] builds the routes the auth crate nests at `/auth/v1` behind its
//! TLS-only middleware. Every route passes the per-IP `AnyRoute` limit and the
//! kill switch, and bodies are capped at 8 KiB. The dev-mock route exists only
//! in developer mode.

pub mod account;
mod attach;
pub mod bearer;
pub mod ctx;
pub mod error;
pub mod link;
pub mod signin;
pub mod ticket;
pub mod token;

#[cfg(test)]
#[path = "cleanup_ownership_tests_live_db.rs"]
mod cleanup_ownership_tests_live_db;

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Request, State};
use axum::http::{header, HeaderValue};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Router;
use cimmeria_auth::AuthRouteDeps;

use crate::limits::Bucket;
use crate::routes::ctx::{client_ip, IdentityCtx, RouteState};
use crate::routes::error::ApiError;

/// The largest request body any `/auth/v1` route reads (C6).
pub const MAX_BODY: usize = 8 * 1024;

/// Builds the `/auth/v1` router. The paths are relative to `/auth/v1`.
pub fn router(deps: AuthRouteDeps, ctx: IdentityCtx) -> Router {
    let developer_mode = deps.developer_mode;
    let state = RouteState {
        deps: std::sync::Arc::new(deps),
        ctx,
    };
    let mut app = Router::new()
        .route("/providers", get(signin::providers))
        .route("/authorize", get(signin::authorize))
        .route("/callback/{provider}", get(signin::callback))
        .route("/token", post(token::token))
        .route("/logout", post(token::logout))
        .route("/link/new", post(link::new))
        .route("/link/existing", post(link::existing))
        .route("/link/start", post(account::link_start))
        .route("/account", get(account::get_account))
        .route("/identities/{identity_id}", delete(account::unlink))
        .route("/launch-ticket", post(ticket::mint))
        .route("/launch-ticket/heartbeat", post(ticket::heartbeat))
        .route("/launch-ticket/end", post(ticket::end));
    if developer_mode {
        app = app.route(
            "/dev-mock/authorize",
            get(signin::dev_mock_form).post(signin::dev_mock_post),
        );
    }
    app.layer(from_fn_with_state(state.clone(), guard))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}

/// Runs before every route: the `CIMMERIA_IDP_DISABLED` kill switch, then the
/// per-IP `AnyRoute` limit (C7). Every answer carries `Cache-Control: no-store`,
/// so no token, ticket or error body is cached by a proxy or the client.
async fn guard(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    let always_open = path == "/providers" || path.starts_with("/launch-ticket");
    let mut response = if state.ctx.cfg.kill.idp_disabled && !always_open {
        ApiError::Disabled.into_response()
    } else {
        let key = client_ip(ConnectInfo(peer));
        match state.ctx.limit(Bucket::AnyRoute, &key, Instant::now()) {
            Ok(()) => next.run(req).await,
            Err(refused) => refused.into_response(),
        }
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
