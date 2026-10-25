//! The `Bearer` extractor of the signed-in routes (contract C6): the access
//! token of `Authorization: Bearer <token>`, checked against the access store.
//!
//! A missing, malformed or unknown token is `401 invalid_token`. The token text
//! is never logged or stored: the store compares its hash (contract C11).

use std::time::Instant;

use axum::extract::FromRequestParts;
use axum::http::{header, request::Parts};

use crate::routes::ctx::RouteState;
use crate::routes::error::ApiError;
use crate::stores::AccessGrant;

/// The grant behind a live `Authorization: Bearer` token.
pub struct Bearer(pub AccessGrant);

impl FromRequestParts<RouteState> for Bearer {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &RouteState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(ApiError::InvalidToken)?;
        state
            .ctx
            .stores
            .access
            .check(token, Instant::now())
            .map(Bearer)
            .ok_or(ApiError::InvalidToken)
    }
}
