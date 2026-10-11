//! Debug log upload. Scaffold (LX-02): the engine half answers `unavailable`
//! until LX-05. The webview names no file or URL; native code picks both.
use super::NativeHost;
use cimmeria_launcher_engine::log_upload::{LogUploadCommand, LogUploadError, LogUploadStatus};
use cimmeria_launcher_engine::StorageError;

impl NativeHost {
    /// Run one log-upload request on the shared state, under its mutex.
    pub fn log_upload_command(
        &self,
        request: LogUploadCommand,
    ) -> Result<LogUploadStatus, LogUploadError> {
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| StorageError::Io)?;
        state.log_upload(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_reaches_the_engine_scaffold() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let request = serde_json::from_str(r#"{"command":"inspect","schema_version":1}"#).unwrap();
        assert_eq!(
            host.log_upload_command(request),
            Err(LogUploadError::Unavailable)
        );
    }
}
