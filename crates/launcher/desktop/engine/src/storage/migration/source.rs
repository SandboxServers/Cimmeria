use super::*;
use sha2::{Digest, Sha256};
use std::io::Read;

// Combined UTF-8 sources plus parsed values must fit the 64 KiB atomic record.
const MAX_SOURCE: u64 = 12 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Sources {
    pub config: String,
    pub identity: String,
    pub ledger: String,
}
impl Sources {
    pub fn parse(&self, source: LegacySource) -> Result<LegacyImport, MigrationError> {
        let config: LegacyConfig = decode(&self.config)?;
        let identity: LegacyIdentity = decode(&self.identity)?;
        let ledger: LegacyLedger = decode(&self.ledger)?;
        if !matches!(config.schema_version, 1 | 2) || identity.schema_version != 1 {
            return Err(MigrationError::UnsupportedSchema);
        }
        // Do not normalize values, deduplicate the ordered patch ledger or mint IDs.
        if identity.install_id.is_nil()
            || identity.machine_id.is_empty()
            || identity.created_by_launcher_version.is_empty()
            || config.install_path.as_os_str().is_empty()
            || config.manifest_url.is_empty()
        {
            return Err(MigrationError::InvalidSource);
        }
        let mut hash = Sha256::new();
        for bytes in [
            serde_json::to_vec(&source).map_err(|_| MigrationError::InvalidSource)?,
            self.config.as_bytes().to_vec(),
            self.identity.as_bytes().to_vec(),
            self.ledger.as_bytes().to_vec(),
        ] {
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(bytes);
        }
        let confirmation = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(LegacyImport {
            source,
            identity,
            config,
            ledger,
            confirmation,
        })
    }
}
fn decode<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, MigrationError> {
    serde_json::from_str(text).map_err(|_| MigrationError::Storage(StorageError::Corrupt))
}
fn source_text(path: &Path) -> Result<String, MigrationError> {
    super::super::ensure_regular_or_absent(path)?;
    let file = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            MigrationError::MissingSource
        } else {
            MigrationError::Storage(StorageError::Io)
        }
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StorageError::Io)?;
    if bytes.len() as u64 > MAX_SOURCE {
        return Err(StorageError::TooLarge.into());
    }
    String::from_utf8(bytes).map_err(|_| StorageError::Corrupt.into())
}
pub(super) fn load(source: &LegacySource) -> Result<Sources, MigrationError> {
    Ok(Sources {
        config: source_text(&source.launcher_directory.join("launcher-config.json"))?,
        identity: source_text(&source.launcher_directory.join("install.json"))?,
        ledger: source_text(&source.game_directory.join("launcher-installed.json"))?,
    })
}
pub(super) fn canonical(source: &LegacySource) -> Result<LegacySource, MigrationError> {
    fn directory(path: &Path) -> Result<PathBuf, MigrationError> {
        if !path.is_absolute() || path.parent().is_none() {
            return Err(MigrationError::InvalidSource);
        }
        let meta = std::fs::symlink_metadata(path).map_err(|_| MigrationError::MissingSource)?;
        if !meta.is_dir() {
            return Err(StorageError::UnsafeFile.into());
        }
        path.canonicalize().map_err(|_| StorageError::Io.into())
    }
    Ok(LegacySource {
        launcher_directory: directory(&source.launcher_directory)?,
        game_directory: directory(&source.game_directory)?,
    })
}
/// Same file and OS exclusive locking primitive as fs4 in the legacy launcher.
/// Keep the inode and handle alive through the complete import transaction.
pub(super) fn lock(source: &LegacySource) -> Result<File, MigrationError> {
    let path = source.launcher_directory.join("launcher.lock");
    super::super::ensure_regular_or_absent(&path)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|_| StorageError::Io)?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(MigrationError::Busy),
        Err(std::fs::TryLockError::Error(_)) => Err(StorageError::Io.into()),
    }
}
