//! The one request of an attempt: the anonymous summary POST. It has one
//! deadline for the whole exchange, body included, and reads its answer up to a
//! limit. The status decides the next step by the table in the wire contract;
//! the body is parsed into closed types only and never logged.
//!
//! The request carries a content type and the batch. No `Authorization` header,
//! no cookie and no identifier of the sender goes with it, and nothing from an
//! answer is kept or sent back in a later request.
//!
//! It is a plain future with no task of its own: dropping it aborts the
//! request, which is how a withdrawal of consent stops it (`export::attempt`).
use super::{
    export::{CycleOutcome, ExportEnv, Step, Verdicts},
    schema::SummaryResponse,
};
use reqwest::{
    header::{CONTENT_TYPE, RETRY_AFTER},
    StatusCode,
};
use std::time::Duration;

const RESPONSE_LIMIT: usize = 4 * 1024;

/// What the request came to. No status: refused, timed out or cut off.
struct Reply {
    status: Option<StatusCode>,
    retry_after: Option<Duration>,
    /// Read only for a `200`; `None` when it is over the limit.
    body: Option<Vec<u8>>,
}

async fn exchange(request: reqwest::RequestBuilder, deadline: Duration) -> Reply {
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
                if bytes.len() + chunk.len() > RESPONSE_LIMIT {
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

pub(super) async fn post(
    env: &ExportEnv,
    body: Vec<u8>,
    summaries: usize,
) -> Result<Verdicts, Step> {
    let request = env
        .http
        .post(env.endpoint.ingest_url())
        .header(CONTENT_TYPE, "application/json")
        .body(body);
    let reply = exchange(request, env.tuning.post_timeout).await;
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
        // The server will never take this body, so sending it again is no use.
        Some(
            StatusCode::BAD_REQUEST
            | StatusCode::PAYLOAD_TOO_LARGE
            | StatusCode::UNSUPPORTED_MEDIA_TYPE
            | StatusCode::UNPROCESSABLE_ENTITY,
        ) => Ok(Verdicts::Refused),
        // The server does not have the summary route.
        Some(StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED) => Err(env.stop()),
        // This address's allowance is spent, and a retry would spend more of it:
        // the cycle ends here, whatever wait the answer names.
        Some(StatusCode::TOO_MANY_REQUESTS) => Err(Step::Done(CycleOutcome::GaveUp)),
        // Switched off or overloaded: the server says for how long.
        Some(StatusCode::SERVICE_UNAVAILABLE) => {
            let cap = env.tuning.retry_after_cap;
            Err(Step::Retry(Some(reply.retry_after.unwrap_or(cap).min(cap))))
        }
        _ => Err(Step::Retry(None)),
    }
}
