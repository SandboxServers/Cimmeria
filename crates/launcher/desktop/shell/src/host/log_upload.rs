//! Debug log upload (LF-15). The webview names no file or URL; native code picks
//! both. `log_upload_command` in `main.rs` runs the admitted ingest after this
//! call returns, so neither the file reads nor the paced posts hold the state
//! mutex.
use super::NativeHost;
use cimmeria_launcher_engine::log_upload::{LogUploadCommand, LogUploadError, LogUploadStep};
use cimmeria_launcher_engine::StorageError;

impl NativeHost {
    /// Decides under the state mutex: answers `Inspect`, or admits an `Upload`
    /// (one at a time, whatever state the installation is in).
    pub fn begin_log_upload(
        &self,
        request: LogUploadCommand,
    ) -> Result<LogUploadStep, LogUploadError> {
        let store_state = self.store()?;
        let state = store_state.lock().map_err(|_| StorageError::Io)?;
        state.begin_log_upload(request, &self.log_uploader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_launcher_engine::log_upload::LogUploadState;

    fn finished(
        step: Result<LogUploadStep, LogUploadError>,
    ) -> Result<LogUploadState, LogUploadError> {
        match step? {
            LogUploadStep::Finished(status) => Ok(status.state),
            other => panic!("expected a finished status, got {other:?}"),
        }
    }

    #[test]
    fn inspect_without_an_installation_does_not_report_disabled() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let request = serde_json::from_str(r#"{"command":"inspect","schema_version":1}"#).unwrap();
        // A press is consent (D-LU1): neither a missing store nor a missing
        // installation turns the button off.
        let state = finished(host.begin_log_upload(request)).unwrap();
        assert_ne!(state, LogUploadState::Disabled);
        assert_ne!(state, LogUploadState::NotInstalled);
    }

    #[test]
    fn a_second_upload_while_one_is_admitted_is_busy() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let upload = || serde_json::from_str(r#"{"command":"upload","schema_version":1}"#).unwrap();
        let first = host.begin_log_upload(upload());
        // With no telemetry target the first press finishes at once and
        // releases the flag; only an admitted ingest holds it.
        if let Ok(LogUploadStep::Ingest(_held)) = first {
            assert_eq!(
                host.begin_log_upload(upload()).map(|_| ()),
                Err(LogUploadError::Storage(StorageError::Busy))
            );
        }
    }
}
