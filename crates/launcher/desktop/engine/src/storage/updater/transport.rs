use super::{policy, Config, Error, Offer};
use futures_util::StreamExt;
use reqwest::{redirect::Policy, Url};
use std::time::Duration;

pub async fn check(config: &Config) -> Result<Option<Offer>, Error> {
    let bytes = get(
        config,
        config.endpoint.clone(),
        policy::MAX_FEED,
        Duration::from_secs(30),
        true,
    )
    .await?;
    match bytes {
        None => Ok(None),
        Some(bytes) => policy::parse(config, &bytes),
    }
}
pub async fn download(config: &Config, offer: &Offer) -> Result<Vec<u8>, Error> {
    config.newer(&offer.version)?;
    let url = Url::parse(&offer.url).map_err(|_| Error::Policy)?;
    get(
        config,
        url,
        policy::MAX_ARTIFACT,
        Duration::from_secs(300),
        false,
    )
    .await?
    .ok_or(Error::Feed)
}
pub(super) async fn get(
    config: &Config,
    url: Url,
    limit: usize,
    timeout: Duration,
    no_update: bool,
) -> Result<Option<Vec<u8>>, Error> {
    config.allow(&url)?;
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
        .map_err(|_| Error::Transport)?;
    let response = client.get(url).send().await.map_err(|e| {
        if e.is_timeout() {
            Error::Timeout
        } else {
            Error::Transport
        }
    })?;
    if response.status() == reqwest::StatusCode::NO_CONTENT && no_update {
        return Ok(None);
    }
    if response.status() != reqwest::StatusCode::OK {
        return Err(Error::Transport);
    }
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err(Error::Size);
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            if e.is_timeout() {
                Error::Timeout
            } else {
                Error::Transport
            }
        })?;
        if chunk.len() > limit.saturating_sub(body.len()) {
            return Err(Error::Size);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Some(body))
}
