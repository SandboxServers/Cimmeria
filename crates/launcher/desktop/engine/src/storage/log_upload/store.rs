//! The Azure Blob container the logs go to, addressed by a SAS URL compiled in
//! from `LAUNCHER_LOG_SAS_URL`. The SAS query is a write credential: it never
//! appears in an error, a log line, a status or `Debug` output.
use super::{LogUploadError, PendingUpload};
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

    /// One `Put Blob`: debug-log zips are far below the single-request limit,
    /// and one request is one billable transaction.
    pub async fn put(&self, pending: &PendingUpload) -> Result<(), LogUploadError> {
        let http = reqwest::Client::builder()
            .https_only(self.https_only)
            // A redirect would replay the signed URL somewhere else.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|_| LogUploadError::Network)?;
        let response = http
            .put(self.blob_url(&pending.blob_name))
            .header("x-ms-blob-type", "BlockBlob")
            .header("content-type", "application/zip")
            .body(pending.body.clone())
            .send()
            .await
            .map_err(|error| {
                // `reqwest::Error`'s text carries the URL; log a fixed reason.
                tracing::warn!(reason = transport_reason(&error), "log upload failed");
                LogUploadError::Network
            })?;
        let status = response.status();
        if !status.is_success() {
            // The body is the store's error XML; it is not needed to triage.
            tracing::warn!(status = status.as_u16(), "log upload was refused");
            return Err(LogUploadError::Rejected);
        }
        tracing::info!(blob = %pending.blob_name, bytes = pending.body.len(), "log upload stored");
        Ok(())
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
