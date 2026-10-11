//! Debug log upload. The webview names no file or URL; native code picks both.
//! `log_upload_command` in `main.rs` reads the logs and runs the PUT between
//! these two calls, so neither the file reads nor the network hold the state
//! mutex.
use super::NativeHost;
use cimmeria_launcher_engine::log_upload::{
    LogStore, LogUploadCommand, LogUploadError, LogUploadStatus, LogUploadStep, StoredUpload,
};
use cimmeria_launcher_engine::StorageError;

impl NativeHost {
    /// Admission, game path and ledger load, under the state mutex.
    pub fn begin_log_upload(
        &self,
        request: LogUploadCommand,
        store: Option<&LogStore>,
    ) -> Result<LogUploadStep, LogUploadError> {
        let store_state = self.store()?;
        let state = store_state.lock().map_err(|_| StorageError::Io)?;
        state.begin_log_upload(request, store)
    }

    /// Record a stored upload's digest, under the state mutex.
    pub fn finish_log_upload(
        &self,
        stored: StoredUpload,
    ) -> Result<LogUploadStatus, LogUploadError> {
        let store = self.store()?;
        let state = store.lock().map_err(|_| StorageError::Io)?;
        Ok(state.finish_log_upload(stored))
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
            LogUploadStep::Collect(collection) => panic!("nothing should be read: {collection:?}"),
        }
    }

    #[test]
    fn a_build_without_a_store_reports_disabled() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        for json in [
            r#"{"command":"inspect","schema_version":1}"#,
            r#"{"command":"upload","schema_version":1}"#,
        ] {
            let request = serde_json::from_str(json).unwrap();
            assert_eq!(
                finished(host.begin_log_upload(request, None)),
                Ok(LogUploadState::Disabled)
            );
        }
    }

    #[test]
    fn an_https_store_without_an_installation_reports_not_installed() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let store = LogStore::new("https://example.invalid/c?sig=x").unwrap();
        let request = serde_json::from_str(r#"{"command":"inspect","schema_version":1}"#).unwrap();
        assert_eq!(
            finished(host.begin_log_upload(request, Some(&store))),
            Ok(LogUploadState::NotInstalled)
        );
    }
}
