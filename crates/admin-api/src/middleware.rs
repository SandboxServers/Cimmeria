//! Admin API middleware.
//!
//! Provides CORS configuration and JWT authentication middleware for the
//! admin API endpoints.

use tower_http::cors::{Any, CorsLayer};

/// Build the CORS middleware layer.
///
/// Configured permissively for development (allows any origin). In production
/// this should be restricted to the ServerEd desktop application origin.
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

// TODO: JWT authentication middleware
//
// The admin API uses JWT tokens for authentication, separate from the game
// client auth flow. The middleware should:
// 1. Extract the Bearer token from the Authorization header
// 2. Validate the JWT signature using jsonwebtoken
// 3. Inject the authenticated admin user into request extensions
// 4. Return 401 Unauthorized for missing/invalid tokens
//
// Endpoints under /api/auth/login are exempt from authentication.
