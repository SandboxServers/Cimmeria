//! The `/auth/v1` HTTP routes of the sign-in flow (contract C6).
//!
//! [`router`] builds the routes the auth crate nests at `/auth/v1` behind its
//! TLS-only middleware. Every route passes the per-IP `AnyRoute` limit and the
//! kill switch, and bodies are capped at 8 KiB. The dev-mock route exists only
//! in developer mode.

pub mod ctx;
pub mod error;
pub mod signin;
pub mod token;

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Request, State};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
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
        .route("/logout", post(token::logout));
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
/// per-IP `AnyRoute` limit (C7).
async fn guard(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    let always_open = path == "/providers" || path.starts_with("/launch-ticket");
    if state.ctx.cfg.kill.idp_disabled && !always_open {
        return ApiError::Disabled.into_response();
    }
    let key = client_ip(ConnectInfo(peer));
    match state.ctx.limit(Bucket::AnyRoute, &key, Instant::now()) {
        Ok(()) => next.run(req).await,
        Err(refused) => refused.into_response(),
    }
}
