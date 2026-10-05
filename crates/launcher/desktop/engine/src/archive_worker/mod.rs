//! One-request archive helper contract. The parent owns the durable operation.
//! Input paths come from that native parent, never directly from webview commands.
use crate::{
    install_progress::ProgressSink,
    unpack::{self, UnpackError},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, Read},
    path::PathBuf,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub const MAX_FRAME: usize = 8192;
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractRequest {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub archive: PathBuf,
    /// Must not exist. Partial output is retained for parent-owned reconciliation.
    pub destination: PathBuf,
    pub sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRequest {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub cancel: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractError {
    InvalidRequest,
    Io,
    HashMismatch,
    DestinationExists,
    Cancelled,
    Archive,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerEvent {
    pub schema_version: u32,
    pub operation_id: Option<Uuid>,
    #[serde(flatten)]
    pub event: EventKind,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventKind {
    Progress {
        current: u64,
        total: u64,
    },
    Finished {
        #[serde(deserialize_with = "required_error")]
        error: Option<ExtractError>,
    },
}

/// Bounded NDJSON, including its newline. EOF before a frame is distinct from
/// an oversized or unterminated frame. Does not read past the current frame.
pub fn read_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, ExtractError> {
    let mut frame = Vec::new();
    loop {
        let bytes = reader.fill_buf().map_err(|_| ExtractError::Io)?;
        if bytes.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(ExtractError::InvalidRequest)
            };
        }
        let newline = bytes.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(bytes.len(), |index| index + 1);
        if count > MAX_FRAME - frame.len() {
            return Err(ExtractError::InvalidRequest);
        }
        frame.extend_from_slice(&bytes[..count]);
        reader.consume(count);
        if newline.is_some() {
            return Ok(Some(frame));
        }
    }
}

pub fn cancellation_matches(frame: &[u8], id: Uuid) -> bool {
    serde_json::from_slice::<CancelRequest>(frame).is_ok_and(|request| {
        request.schema_version == 1 && request.operation_id == id && request.cancel
    })
}

/// Hash authentication completes before creating output. A new destination is
/// required: this helper never overlays an existing installation or retries a
/// partial extraction silently. The caller later promotes/reconciles staging.
pub fn extract(
    request: &ExtractRequest,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Result<(), ExtractError> {
    if request.schema_version != 1
        || !request.archive.is_absolute()
        || !request.destination.is_absolute()
        || request.destination.file_name().is_none()
        || request.sha256.len() != 64
        || !request.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ExtractError::InvalidRequest);
    }
    let cancelled = || {
        if cancel.is_cancelled() {
            Err(ExtractError::Cancelled)
        } else {
            Ok(())
        }
    };
    cancelled()?;
    let mut archive = open_archive(&request.archive)?;
    if !archive.metadata().map_err(|_| ExtractError::Io)?.is_file() {
        return Err(ExtractError::InvalidRequest);
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        cancelled()?;
        let n = archive.read(&mut buffer).map_err(|_| ExtractError::Io)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    let hash = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !hash.eq_ignore_ascii_case(&request.sha256) {
        return Err(ExtractError::HashMismatch);
    }
    cancelled()?;
    std::fs::create_dir(&request.destination).map_err(|error| match error.kind() {
        std::io::ErrorKind::AlreadyExists => ExtractError::DestinationExists,
        _ => ExtractError::Io,
    })?;
    unpack::unpack(
        &request.archive,
        &request.destination,
        &unpack::UnpackSink {
            progress,
            label: "seed".into(),
            cancel,
        },
    )
    .map_err(|error| match error {
        UnpackError::Cancelled => ExtractError::Cancelled,
        UnpackError::Io(_) => ExtractError::Io,
        _ => ExtractError::Archive,
    })
}

fn open_archive(path: &std::path::Path) -> Result<std::fs::File, ExtractError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    // Keep this handle alive through unpack(), whose libraries reopen the path.
    // Windows sharing rules deny both mutation and replacement during that gap.
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ);
    }
    options.open(path).map_err(|_| ExtractError::Io)
}

#[cfg(test)]
mod tests;

mod process;
pub use process::serve_stdio;

fn required_error<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ExtractError>, D::Error> {
    Option::<ExtractError>::deserialize(deserializer)
}
