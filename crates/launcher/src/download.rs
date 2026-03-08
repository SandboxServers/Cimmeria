use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::path::Path;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Download cancelled")]
    Cancelled,
    #[error("Server returned {0}")]
    BadStatus(u16),
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f32,
    pub speed_bps: u64,
    pub eta_secs: u64,
}

pub async fn download_file<F>(
    url: &str,
    dest: &Path,
    cancel: CancellationToken,
    mut on_progress: F,
) -> Result<(), DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let client = Client::new();

    let existing_len = if dest.exists() {
        tokio::fs::metadata(dest).await?.len()
    } else {
        0
    };

    let mut request = client.get(url);
    if existing_len > 0 {
        request = request.header("Range", format!("bytes={}-", existing_len));
    }

    let response = request.send().await?;

    if !response.status().is_success() && response.status().as_u16() != 206 {
        return Err(DownloadError::BadStatus(response.status().as_u16()));
    }

    let total = if response.status().as_u16() == 206 {
        existing_len + response.content_length().unwrap_or(0)
    } else {
        response.content_length().unwrap_or(0)
    };

    let mut file = if existing_len > 0 && response.status().as_u16() == 206 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(dest)
            .await?
    } else {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::File::create(dest).await?
    };

    let mut downloaded = if response.status().as_u16() == 206 {
        existing_len
    } else {
        0
    };
    let start_time = std::time::Instant::now();
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }

        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed_bps = if elapsed > 0.0 {
            (downloaded as f64 / elapsed) as u64
        } else {
            0
        };
        let remaining = total.saturating_sub(downloaded);
        let eta_secs = if speed_bps > 0 {
            remaining / speed_bps
        } else {
            0
        };

        on_progress(DownloadProgress {
            downloaded,
            total,
            percent: if total > 0 {
                (downloaded as f32 / total as f32) * 100.0
            } else {
                0.0
            },
            speed_bps,
            eta_secs,
        });
    }

    file.flush().await?;
    Ok(())
}
