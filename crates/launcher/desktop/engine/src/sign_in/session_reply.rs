//! The shared helpers of the sign-in step replies (C6): the scheme check, the
//! error of a signed-out session and of a non-success reply, and the log line.

use url::Url;

use super::api::ApiErrorBody;
use super::discovery::{self, MAX_BODY};
use super::session::Outcome;
use super::SignInError;

/// Tokens and passwords go over TLS only. Debug builds and tests may use a plain
/// local server (`CIMMERIA_LAUNCHER_AUTH_BASE`, `wiremock`).
pub(super) fn scheme_allowed(base: &Url, relaxed: bool) -> bool {
    base.scheme() == "https" || relaxed
}

pub(super) fn signed_out() -> SignInError {
    SignInError::Server("signed_out".to_owned())
}

/// The C6 error of a non-success reply. A `429` keeps `retry_after` in the text,
/// for the sign-in panel. A reply with no readable error body gives its status.
pub(super) async fn server_error(response: reqwest::Response) -> SignInError {
    let status = response.status();
    let body = discovery::read_capped(response, MAX_BODY)
        .await
        .and_then(|bytes| serde_json::from_slice::<ApiErrorBody>(&bytes).ok());
    match body {
        Some(body) if body.error == "rate_limited" => SignInError::Server(format!(
            "rate_limited retry_after={}",
            body.retry_after.unwrap_or(0)
        )),
        Some(body) => SignInError::Server(body.error),
        None => SignInError::Server(status.as_u16().to_string()),
    }
}

/// Logs how a sign-in step ended: the outcome, or the error's fixed reason text.
/// No code, token, grant or account value goes into the log.
pub(super) fn record(result: Result<Outcome, SignInError>) -> Result<Outcome, SignInError> {
    match &result {
        Ok(Outcome::SignedIn(_)) => {
            tracing::info!(target: "launcher.sign_in", event = "completed", outcome = "signed_in")
        }
        Ok(Outcome::NeedsLink(_)) => {
            tracing::info!(target: "launcher.sign_in", event = "completed", outcome = "needs_link")
        }
        Err(e) => {
            tracing::info!(target: "launcher.sign_in", event = "failed", outcome = "failed", reason = e.reason())
        }
    }
    result
}
