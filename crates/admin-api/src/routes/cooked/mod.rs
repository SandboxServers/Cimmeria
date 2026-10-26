//! Cooked-data presync routes on the login port (LP-10).
//!
//! The desktop launcher reads the catalog the server built at startup
//! (LP-03, published by LP-11) to decide which cooked-data categories its
//! player's cache is missing. Two anonymous GET routes serve it:
//!
//! - `GET /api/cooked/v1/index`: the index JSON.
//! - `GET /api/cooked/v1/category/{id}?version=<v>`: one category's `CPS1`
//!   body, for the version the index named.
//!
//! Both are answered only from the published catalog. A refusal is JSON
//! `{"error":"<code>"}`: the same code the launcher reports when the presync
//! fails. The per-address quota and the kill switch
//! (`CIMMERIA_COOKED_PRESYNC_DISABLED`) apply before anything else.
//!
//! # Module layout
//!
//! - `handlers` — the axum handlers, the checks and their injectable cores.
//!
//! Test-only: `tests` drives the cores through every refusal and answer.

mod handlers;

#[cfg(test)]
mod tests;

use axum::routing::get;
use axum::Router;

/// The presync routes, for the login-port router to nest under `/api/cooked`.
pub fn routes() -> Router {
    Router::new()
        .route("/v1/index", get(handlers::index))
        .route("/v1/category/{id}", get(handlers::category))
}
