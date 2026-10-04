use super::*;
use crate::install::Progress;
use crate::install_progress::ProgressReporter;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

pub(super) async fn fetch(
    http: &reqwest::Client,
    url: &str,
    size: u64,
    root: &Path,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<tempfile::NamedTempFile, RuntimeError> {
    let response = tokio::select! {_ = cancel.cancelled()=>return Err(RuntimeError::Cancelled),
    result=http.get(url).send()=>result.map_err(|_|RuntimeError::Network)?};
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|declared| declared != size)
    {
        return Err(RuntimeError::Verification);
    }
    let archive = tempfile::NamedTempFile::new_in(root)?;
    let mut output = tokio::fs::File::from_std(archive.reopen()?);
    let mut received = 0;
    let mut stream = response.bytes_stream();
    loop {
        let next = tokio::select! {_ = cancel.cancelled()=>return Err(RuntimeError::Cancelled),next=stream.next()=>next};
        let Some(bytes) = next else {
            break;
        };
        let bytes = bytes.map_err(|_| RuntimeError::Network)?;
        received += bytes.len() as u64;
        if received > size {
            return Err(RuntimeError::Verification);
        }
        output.write_all(&bytes).await?;
        progress.report(Progress::Downloading {
            label: "compatibility runtime".into(),
            downloaded: received,
            total: size,
        });
    }
    output.sync_all().await?;
    drop(output);
    if received != size {
        return Err(RuntimeError::Verification);
    }
    Ok(archive)
}
