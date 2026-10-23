//! The legacy x86 DirectX libraries SGW imports, independent of the OS DirectX version.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub mod native;
pub const COMPONENTS: [&str; 2] = ["d3dx9_40.dll", "xinput1_3.dll"];
// Microsoft's Download Center item 35, retrieved and signature-checked 2026-10-23.
const URL: &str = "https://download.microsoft.com/download/1/7/1/1718ccc4-6315-4d8e-9543-8e28a4e18c4c/dxwebsetup.exe";
const BYTES: usize = 295_320;
const SHA256: &str = "2cf71d098c608c56e07f4655855a886c3102553f648df88458df616b26fd612f";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Detection {
    pub missing: Vec<&'static str>,
}
impl Detection {
    pub fn installed(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Resolve the x86 system directory explicitly, including on Windows ARM64.
pub trait Probe {
    fn x86_directory(&self) -> Option<PathBuf>;
}
pub fn detect(probe: &dyn Probe, game: Option<&Path>) -> Detection {
    let system = probe.x86_directory();
    Detection {
        missing: COMPONENTS
            .into_iter()
            .filter(|name| {
                if let Some(game) = game {
                    let local = game.join(name);
                    match std::fs::symlink_metadata(&local) {
                        Ok(_) => return !x86_dll(&local),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                        Err(_) => return true,
                    }
                }
                !system.as_ref().is_some_and(|dir| x86_dll(&dir.join(name)))
            })
            .collect(),
    }
}

// Inspect the PE header without loading DLL code into the 64-bit launcher.
fn x86_dll(path: &Path) -> bool {
    (|| -> std::io::Result<bool> {
        let mut file = std::fs::File::open(path)?;
        let mut dos = [0; 64];
        file.read_exact(&mut dos)?;
        if &dos[..2] != b"MZ" {
            return Ok(false);
        }
        let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap());
        if !(64..=1_048_576).contains(&offset) {
            return Ok(false);
        }
        file.seek(SeekFrom::Start(u64::from(offset)))?;
        let mut pe = [0; 24];
        file.read_exact(&mut pe)?;
        Ok(&pe[..4] == b"PE\0\0"
            && pe[4..6] == [0x4c, 0x01]
            && u16::from_le_bytes([pe[22], pe[23]]) & 0x2000 != 0)
    })()
    .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Inspect { schema_version: u32 },
    Install { schema_version: u32 },
}
impl Command {
    pub fn schema_version(self) -> u32 {
        match self {
            Self::Inspect { schema_version } | Self::Install { schema_version } => schema_version,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    Unavailable,
    DownloadFailed,
    PackageMismatch,
    Io,
    Busy,
    UnsupportedSchema,
}

pub fn verify(bytes: &[u8]) -> Result<(), Error> {
    verify_pinned(bytes, BYTES, SHA256)
}

fn verify_pinned(bytes: &[u8], expected_bytes: usize, sha256: &str) -> Result<(), Error> {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if bytes.len() != expected_bytes || digest != sha256 {
        return Err(Error::PackageMismatch);
    }
    Ok(())
}

/// The URL and hash are compiled in. A renderer cannot select executable input.
pub async fn download() -> Result<Vec<u8>, Error> {
    download_pinned(URL, BYTES, SHA256).await
}

/// Shared with the native PhysX fallback. Only compiled vendor identities reach this API.
pub async fn download_pinned(
    url: &str,
    expected_bytes: usize,
    sha256: &str,
) -> Result<Vec<u8>, Error> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Error::DownloadFailed)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| Error::DownloadFailed)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(Error::DownloadFailed);
    }
    let mut bytes = Vec::with_capacity(expected_bytes);
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::DownloadFailed)? {
        if bytes.len() + chunk.len() > expected_bytes {
            return Err(Error::PackageMismatch);
        }
        bytes.extend_from_slice(&chunk);
    }
    verify_pinned(&bytes, expected_bytes, sha256)?;
    Ok(bytes)
}

/// Write, reopen under deny-write/delete sharing, and authenticate the executed bytes.
pub fn stage(root: &Path, bytes: &[u8]) -> Result<(tempfile::TempPath, std::fs::File), Error> {
    stage_pinned(root, bytes, BYTES, SHA256, ".exe")
}

pub fn stage_pinned(
    root: &Path,
    bytes: &[u8],
    expected_bytes: usize,
    sha256: &str,
    suffix: &str,
) -> Result<(tempfile::TempPath, std::fs::File), Error> {
    verify_pinned(bytes, expected_bytes, sha256)?;
    std::fs::create_dir_all(root).map_err(|_| Error::Io)?;
    let mut temporary = tempfile::Builder::new()
        .prefix("prerequisite-")
        .suffix(suffix)
        .tempfile_in(root)
        .map_err(|_| Error::Io)?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| Error::Io)?;
    let path = temporary.into_temp_path();
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1);
    }
    let mut guard = options.open(&path).map_err(|_| Error::Io)?;
    let mut saved = Vec::new();
    (&mut guard)
        .take(expected_bytes as u64 + 1)
        .read_to_end(&mut saved)
        .map_err(|_| Error::Io)?;
    verify_pinned(&saved, expected_bytes, sha256)?;
    Ok((path, guard))
}

#[cfg(test)]
mod tests;
