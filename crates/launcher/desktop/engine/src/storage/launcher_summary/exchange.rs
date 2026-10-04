//! The two requests of one attempt: the dev-session mint and the summary POST.
//! Each has one deadline for the whole exchange, body included, and reads its
//! answer up to a limit. A status decides the next step by the table in the
//! wire contract; a body is parsed into closed types only and never logged.
//!
//! Both are plain futures with no task of their own: dropping one aborts the
//! request, which is how a withdrawal of consent stops it (`export::attempt`).
use super::{
    export::{ExportEnv, ExportTuning, Step, Verdicts},
    schema::{MintRequest, SummaryResponse},
};
use reqwest::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER},
    StatusCode,
};
use std::time::Duration;

const MINT_RESPONSE_LIMIT: usize = 8 * 1024;
const INGEST_RESPONSE_LIMIT: usize = 4 * 1024;
const MAX_TOKEN_BYTES: usize = 4096;
const JSON: &str = "application/json";

/// What one request came to. No status: refused, timed out or cut off.
struct Reply {
    status: Option<StatusCode>,
    retry_after: Option<Duration>,
    /// Read only for a `200`; `None` when it is over the limit.
    body: Option<Vec<u8>>,
}
impl Reply {
    fn retry(&self, tuning: &ExportTuning) -> Step {
        let asked = matches!(
            self.status,
            Some(StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE)
        );
        Step::Retry(asked.then(|| {
            self.retry_after
                .unwrap_or(tuning.retry_after_cap)
                .min(tuning.retry_after_cap)
        }))
    }
}

async fn exchange(request: reqwest::RequestBuilder, deadline: Duration, limit: usize) -> Reply {
    let answered = async {
        let mut response = request.send().await.ok()?;
        let status = response.status();
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .map(Duration::from_secs);
        let mut body = None;
        if status == StatusCode::OK {
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.ok()? {
                if bytes.len() + chunk.len() > limit {
                    return Some(Reply {
                        status: Some(status),
                        retry_after,
                        body: None,
                    });
                }
                bytes.extend_from_slice(&chunk);
            }
            body = Some(bytes);
        }
        Some(Reply {
            status: Some(status),
            retry_after,
            body,
        })
    };
    // The deadline covers the whole exchange, body included.
    tokio::time::timeout(deadline, answered)
        .await
        .ok()
        .flatten()
        .unwrap_or(Reply {
            status: None,
            retry_after: None,
            body: None,
        })
}

pub(super) async fn mint(env: &ExportEnv) -> Result<HeaderValue, Step> {
    let request = MintRequest::new((env.mint_id)(), env.launcher_version);
    let Ok(body) = serde_json::to_vec(&request) else {
        return Err(Step::Retry(None));
    };
    let request = env
        .http
        .post(env.endpoint.mint_url())
        .header(CONTENT_TYPE, JSON)
        .body(body);
    let reply = exchange(request, env.tuning.mint_timeout, MINT_RESPONSE_LIMIT).await;
    match reply.status {
        Some(StatusCode::OK) => reply
            .body
            .as_deref()
            .and_then(bearer)
            .ok_or(Step::Retry(None)),
        // The server does not know the summary session kind.
        Some(
            StatusCode::BAD_REQUEST
            | StatusCode::NOT_FOUND
            | StatusCode::METHOD_NOT_ALLOWED
            | StatusCode::UNSUPPORTED_MEDIA_TYPE
            | StatusCode::UNPROCESSABLE_ENTITY,
        ) => Err(env.stop()),
        _ => Err(reply.retry(&env.tuning)),
    }
}

// Only `token` is used; `upload_endpoint` and the rest of the answer are ignored.
#[derive(serde::Deserialize)]
struct Minted {
    token: String,
}

fn bearer(body: &[u8]) -> Option<HeaderValue> {
    let minted: Minted = serde_json::from_slice(body).ok()?;
    if minted.token.is_empty() || minted.token.len() > MAX_TOKEN_BYTES {
        return None;
    }
    // Refuses a token that is not a legal header value, such as one with CR or LF.
    let mut value = HeaderValue::from_str(&format!("Bearer {}", minted.token)).ok()?;
    value.set_sensitive(true);
    Some(value)
}

pub(super) async fn post(
    env: &ExportEnv,
    token: HeaderValue,
    body: Vec<u8>,
    summaries: usize,
) -> Result<Verdicts, Step> {
    let request = env
        .http
        .post(env.endpoint.ingest_url())
        .header(AUTHORIZATION, token)
        .header(CONTENT_TYPE, JSON)
        .body(body);
    let reply = exchange(request, env.tuning.post_timeout, INGEST_RESPONSE_LIMIT).await;
    match reply.status {
        // One known verdict per summary, or the answer is not trusted at all.
        Some(StatusCode::OK) => reply
            .body
            .as_deref()
            .and_then(|body| serde_json::from_slice::<SummaryResponse>(body).ok())
            .map(|response| response.results)
            .filter(|results| results.len() == summaries)
            .map(Verdicts::Each)
            .ok_or(Step::Retry(None)),
        Some(StatusCode::BAD_REQUEST | StatusCode::PAYLOAD_TOO_LARGE) => Ok(Verdicts::Refused),
        Some(StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED) => Err(env.stop()),
        _ => Err(reply.retry(&env.tuning)),
    }
}
