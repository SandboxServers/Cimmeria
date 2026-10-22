//! The Azure Blob container the logs go to, addressed by a SAS URL compiled in
//! from `LAUNCHER_LOG_SAS_URL`. The SAS query is a write credential: it never
//! appears in an error, a log line, a status or `Debug` output.
use super::{events, LogUploadError, PendingUpload};
use std::time::Duration;

pub struct LogStore {
    sas_url: String,
    https_only: bool,
}

impl std::fmt::Debug for LogStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LogStore { .. }")
    }
}

impl LogStore {
    /// The store this build was compiled with, or `None` when the release
    /// carries no SAS URL (the feature is then disabled, not broken).
    pub fn compiled() -> Option<Self> {
        option_env!("LAUNCHER_LOG_SAS_URL").and_then(Self::new)
    }

    /// Refuses anything but `https://`: the SAS signature must not cross the
    /// network in clear text.
    pub fn new(sas_url: &str) -> Option<Self> {
        sas_url.starts_with("https://").then(|| Self {
            sas_url: sas_url.to_owned(),
            https_only: true,
        })
    }

    /// The wiremock server speaks plain HTTP.
    #[cfg(test)]
    pub(crate) fn insecure_for_tests(sas_url: &str) -> Self {
        Self {
            sas_url: sas_url.to_owned(),
            https_only: false,
        }
    }

    /// One `Put Blob`: a capped upload (`MAX_UPLOAD_BYTES`, 64 MiB) is far
    /// below the single-request limit, and one request is one billable
    /// transaction. Takes the upload by value so the body moves into the
    /// request instead of being copied.
    pub async fn put(&self, pending: PendingUpload) -> Result<StoredUpload, LogUploadError> {
        let PendingUpload {
            digest,
            blob_name,
            body,
            trimmed,
        } = pending;
        let bytes = body.len();
        let http = reqwest::Client::builder()
            .https_only(self.https_only)
            // A redirect would replay the signed URL somewhere else.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(put_timeout(bytes))
            .build()
            .map_err(|error| network_error(&error.without_url()))?;
        let response = http
            .put(self.blob_url(&blob_name))
            .header("x-ms-blob-type", "BlockBlob")
            .header("content-type", "application/zip")
            .body(body)
            .send()
            .await
            .map_err(|error| network_error(&error.without_url()))?;
        let status = response.status();
        if !status.is_success() {
            // A 3xx lands here too: the policy above does not follow it. The
            // body is the store's error XML; it is not needed to triage.
            events::rejected(status.as_u16());
            return Err(LogUploadError::Rejected);
        }
        events::stored(bytes);
        Ok(StoredUpload {
            digest,
            blob_name,
            trimmed,
        })
    }
    /// The blob path goes before the SAS query: `<container>/<blob>?<sas>`.
    fn blob_url(&self, blob_name: &str) -> String {
        let (base, query) = match self.sas_url.find('?') {
            Some(at) => self.sas_url.split_at(at),
            None => (self.sas_url.as_str(), ""),
        };
        format!("{}/{blob_name}{query}", base.trim_end_matches('/'))
    }
}

/// What [`LogStore::put`] stored, for [`super::DesktopState::finish_log_upload`].
#[derive(Debug)]
pub struct StoredUpload {
    pub(super) digest: String,
    pub(super) blob_name: String,
    pub(super) trimmed: bool,
}

/// One minute plus one second per 128 KiB, so a slow uplink of 1 Mbit/s
/// still finishes: a full 64 MiB zip gets about nine and a half minutes. A
/// fixed short timeout would fail every large upload on a slow line, and the
/// player would press again for another doomed attempt.
fn put_timeout(bytes: usize) -> Duration {
    Duration::from_secs(60 + (bytes as u64).div_ceil(128 * 1024))
}

/// `error` has already had its URL stripped (`without_url`), so neither the
/// fixed reason logged here nor anything formatting it later can carry the
/// SAS signature.
pub(super) fn network_error(error: &reqwest::Error) -> LogUploadError {
    events::failed(transport_reason(error));
    LogUploadError::Network
}

fn transport_reason(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else if error.is_redirect() {
        "redirect"
    } else if error.is_builder() {
        "builder"
    } else {
        "transport"
    }
}
