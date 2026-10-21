use super::{events, policy, Config, Error, Offer};
use futures_util::StreamExt;
use reqwest::{redirect::Policy, Url};
use std::time::{Duration, Instant};

pub async fn check(config: &Config) -> Result<Option<Offer>, Error> {
    let started = Instant::now();
    let bytes = get(
        config,
        config.endpoint.clone(),
        policy::MAX_FEED,
        Duration::from_secs(30),
        true,
    )
    .await?;
    let offer = match bytes {
        None => None,
        Some(bytes) => policy::parse(config, &bytes)
            .inspect_err(|error| events::fetch_failed(true, *error, None))?,
    };
    events::check_finished(
        if offer.is_some() {
            "available"
        } else {
            "up_to_date"
        },
        started,
    );
    Ok(offer)
}
pub async fn download(config: &Config, offer: &Offer) -> Result<Vec<u8>, Error> {
    let started = Instant::now();
    config.newer(&offer.version)?;
    let url = Url::parse(&offer.url).map_err(|_| Error::Policy)?;
    let bytes = get(
        config,
        url,
        policy::MAX_ARTIFACT,
        Duration::from_secs(300),
        false,
    )
    .await?
    .ok_or(Error::Feed)?;
    events::downloaded(bytes.len() as u64, started);
    Ok(bytes)
}
/// One request. A failure is logged once here, as a check (`no_update`) or a
/// download, because only this function sees the HTTP status.
pub(super) async fn get(
    config: &Config,
    url: Url,
    limit: usize,
    timeout: Duration,
    no_update: bool,
) -> Result<Option<Vec<u8>>, Error> {
    config
        .allow(&url)
        .map_err(|error| failed(no_update, error, None))?;
    let policy = config.clone();
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .redirect(Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 || policy.allow(attempt.url()).is_err() {
                attempt.error("updater redirect rejected")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| failed(no_update, Error::Transport, None))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| request_failed(no_update, &e))?;
    if response.status() == reqwest::StatusCode::NO_CONTENT && no_update {
        return Ok(None);
    }
    if response.status() != reqwest::StatusCode::OK {
        let status = response.status().as_u16();
        return Err(failed(no_update, Error::Transport, Some(status)));
    }
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(failed(no_update, Error::Size, None));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| request_failed(no_update, &e))?;
        if chunk.len() > limit.saturating_sub(body.len()) {
            return Err(failed(no_update, Error::Size, None));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Some(body))
}

/// Log a failed request under C6 and return its error unchanged.
fn failed(no_update: bool, error: Error, status: Option<u16>) -> Error {
    events::fetch_failed(no_update, error, status);
    error
}

/// A request that never produced a usable answer: a timeout or a transport
/// failure, with the HTTP status when the server did answer.
fn request_failed(no_update: bool, error: &reqwest::Error) -> Error {
    let code = if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Transport
    };
    failed(no_update, code, error.status().map(|s| s.as_u16()))
}
