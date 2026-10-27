//! The error answers of the `/auth/v1` routes (contract C6).
//!
//! Every route answers an error through [`ApiError`]: one variant per C6 error
//! code, each with the status and the JSON body the contract names. A body
//! carries the code only, never a subject, token, code or email.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::repo::RepoError;

/// One C6 error answer. [`IntoResponse`] gives the status and JSON body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// `400 invalid_request`: a parameter is missing, malformed or unsupported.
    InvalidRequest,
    /// `404 unknown_provider`: no running provider has this id.
    UnknownProvider,
    /// `503 disabled`: the `CIMMERIA_IDP_DISABLED` kill switch is on.
    Disabled,
    /// `400 invalid_grant`: a grant, code or link grant is unknown, spent or expired.
    InvalidGrant,
    /// `403 account_disabled`: the account is disabled.
    AccountDisabled,
    /// `403 signup_closed`: sign-up is closed.
    SignupClosed,
    /// `429 rate_limited`: the caller must wait `retry_after` seconds.
    RateLimited {
        /// Whole seconds until the request may succeed.
        retry_after: u64,
    },
    /// `401 invalid_credentials`: the account name or password did not match.
    InvalidCredentials,
    /// `403 not_linkable`: the account cannot take a link.
    NotLinkable,
    /// `401 invalid_token`: the bearer token is unknown or expired.
    InvalidToken,
    /// `403 reauth_required`: the token is too old for this action.
    ReauthRequired,
    /// `404 not_found`: the named record does not exist for this account.
    NotFound,
    /// `409 last_method`: the action would leave the account with no sign-in method.
    LastMethod,
    /// `503 unavailable`: a backing service (the database) is not configured or not reachable.
    Unavailable,
}

impl ApiError {
    /// Maps a repository failure: a conflict is the caller's `invalid_grant`, a
    /// database failure is `unavailable`.
    pub(super) fn from_repo(error: impl Into<RepoError>) -> ApiError {
        match error.into() {
            RepoError::Conflict => ApiError::InvalidGrant,
            RepoError::Db => ApiError::Unavailable,
        }
    }

    /// The C6 error code, the `error` field of the JSON body.
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::InvalidRequest => "invalid_request",
            ApiError::UnknownProvider => "unknown_provider",
            ApiError::Disabled => "disabled",
            ApiError::InvalidGrant => "invalid_grant",
            ApiError::AccountDisabled => "account_disabled",
            ApiError::SignupClosed => "signup_closed",
            ApiError::RateLimited { .. } => "rate_limited",
            ApiError::InvalidCredentials => "invalid_credentials",
            ApiError::NotLinkable => "not_linkable",
            ApiError::InvalidToken => "invalid_token",
            ApiError::ReauthRequired => "reauth_required",
            ApiError::NotFound => "not_found",
            ApiError::LastMethod => "last_method",
            ApiError::Unavailable => "unavailable",
        }
    }

    /// The HTTP status C6 gives this code.
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::InvalidRequest | ApiError::InvalidGrant => StatusCode::BAD_REQUEST,
            ApiError::UnknownProvider | ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Disabled | ApiError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::AccountDisabled
            | ApiError::SignupClosed
            | ApiError::NotLinkable
            | ApiError::ReauthRequired => StatusCode::FORBIDDEN,
            ApiError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::InvalidCredentials | ApiError::InvalidToken => StatusCode::UNAUTHORIZED,
            ApiError::LastMethod => StatusCode::CONFLICT,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        match self {
            ApiError::RateLimited { retry_after } => {
                let body = Json(json!({"error": "rate_limited", "retry_after": retry_after}));
                let mut response = (status, body).into_response();
                // Whole seconds, so the header value is always valid ASCII.
                if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
                    response.headers_mut().insert(header::RETRY_AFTER, value);
                }
                response
            }
            other => (status, Json(json!({"error": other.code()}))).into_response(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_match_c6() {
        assert_eq!(ApiError::InvalidRequest.status(), StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::UnknownProvider.status(), StatusCode::NOT_FOUND);
        assert_eq!(ApiError::Disabled.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(ApiError::LastMethod.status(), StatusCode::CONFLICT);
        assert_eq!(
            ApiError::RateLimited { retry_after: 5 }.status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(ApiError::InvalidToken.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rate_limited_body_and_header() {
        let response = ApiError::RateLimited { retry_after: 42 }.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "42");
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("a small body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        assert_eq!(body, json!({"error": "rate_limited", "retry_after": 42}));
    }

    #[tokio::test]
    async fn plain_errors_carry_only_the_code() {
        let response = ApiError::UnknownProvider.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("a small body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        assert_eq!(body, json!({"error": "unknown_provider"}));
    }
}
