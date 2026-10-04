//! Resources are supplied by native bundle resolution, never webview input.
use super::*;
use std::io::Read;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    path: PathBuf,
    sha256: [u8; 32],
}
impl Artifact {
    /// The expected digest must come from the trusted build, not a nearby file.
    pub fn open(path: PathBuf, expected_hex: &str) -> Result<Self, IntentError> {
        if expected_hex.len() != 64 || !expected_hex.is_ascii() {
            return Err(StorageError::Corrupt.into());
        }
        let mut sha256 = [0; 32];
        for (i, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&expected_hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| StorageError::Corrupt)?;
        }
        let artifact = Self { path, sha256 };
        artifact.verify()?;
        Ok(artifact)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn verify(&self) -> Result<(), IntentError> {
        let metadata =
            std::fs::symlink_metadata(&self.path).map_err(|_| StorageError::UnsafeFile)?;
        if !metadata.is_file()
            || metadata.len() > 128 * 1024 * 1024
            || self
                .path
                .canonicalize()
                .map_err(|_| StorageError::UnsafeFile)?
                != self.path
        {
            return Err(StorageError::UnsafeFile.into());
        }
        let mut file = File::open(&self.path).map_err(|_| StorageError::Io)?;
        let mut hash = Sha256::new();
        let mut bytes = [0; 65536];
        loop {
            let n = file.read(&mut bytes).map_err(|_| StorageError::Io)?;
            if n == 0 {
                break;
            }
            hash.update(&bytes[..n]);
        }
        if <[u8; 32]>::from(hash.finalize()) != self.sha256 {
            return Err(StorageError::Corrupt.into());
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    pub(super) fn stage(&self, path: &Path) -> Result<(), IntentError> {
        self.verify()?;
        if path.try_exists().map_err(|_| StorageError::Io)? {
            return Self {
                path: path.into(),
                sha256: self.sha256,
            }
            .verify();
        }
        let parent = path.parent().ok_or(StorageError::UnsafeFile)?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| StorageError::Io)?;
        std::io::copy(
            &mut File::open(&self.path).map_err(|_| StorageError::Io)?,
            &mut temporary,
        )
        .map_err(|_| StorageError::Io)?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| StorageError::Io)?;
        temporary
            .persist_noclobber(path)
            .map_err(|_| StorageError::UnsafeFile)?;
        Self {
            path: path.into(),
            sha256: self.sha256,
        }
        .verify()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub helper: Artifact,
    /// None explicitly launches without client patches; never enables game telemetry.
    pub client_patches: Option<Artifact>,
    pub graphics: Option<Graphics>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Graphics {
    pub d3d9: Artifact,
    /// Optional accelerator plus its companion library. None uses stock Rosetta.
    pub rosetta_x87: Option<(Artifact, Artifact)>,
}
impl Resources {
    pub fn verify(&self) -> Result<(), IntentError> {
        self.helper.verify()?;
        if let Some(patches) = &self.client_patches {
            patches.verify()?;
        }
        if let Some(graphics) = &self.graphics {
            graphics.d3d9.verify()?;
            if let Some((executable, library)) = &graphics.rosetta_x87 {
                executable.verify()?;
                library.verify()?;
                if executable.path().parent() != library.path().parent() {
                    return Err(StorageError::UnsafeFile.into());
                }
            }
        }
        Ok(())
    }
}
