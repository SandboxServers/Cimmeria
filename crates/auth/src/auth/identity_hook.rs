//! Hook for mounting `/auth/v1` routes from a crate above the auth crate.
//!
//! The identity crate builds its routes (`cimmeria-identity`) and hands the
//! builder to [`AuthService::set_identity_routes`]. `start` calls it once the
//! database pool is known, nests the result at `/auth/v1` behind `require_tls`
//! and serves it on every listener, so the routes exist only over TLS.

use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;

use super::{AuthService, LaunchTicketStore};

/// What `start` passes to the identity-route builder.
pub struct AuthRouteDeps {
    /// The database pool, `None` when no database is configured.
    pub db: Option<Arc<PgPool>>,
    /// The launch-ticket store shared with the SOAP handlers.
    pub launch_tickets: Arc<LaunchTicketStore>,
    /// Developer mode (`ServerConfig::developer_mode`).
    pub developer_mode: bool,
}

/// Builds the `/auth/v1` router. Called at most once, from `start`.
/// `Sync` keeps `AuthService` shareable across threads.
pub type IdentityRoutes = Box<dyn FnOnce(AuthRouteDeps) -> Router + Send + Sync>;

impl AuthService {
    /// Install the `/auth/v1` route builder. Must be called before
    /// [`AuthService::start`]; a later call replaces an earlier one.
    pub fn set_identity_routes(&mut self, routes: IdentityRoutes) {
        self.identity_routes = Some(routes);
    }
}
