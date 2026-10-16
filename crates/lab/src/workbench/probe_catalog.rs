//! Versioned, per-user probe definitions and small operator preferences.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_CHUNK: usize = 8_192;
const MAX_PROBES: usize = 128;

/// Operator intent; Lua source is not statically proven pure.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeMode {
    Read,
    Mutate,
}

/// One saved snippet. The digest covers the executable source and mode.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProbeDefinition {
    pub schema_version: u32,
    pub name: String,
    pub mode: ProbeMode,
    pub chunk: String,
    pub sha256: String,
    #[serde(default)]
    pub favorite: bool,
}

impl ProbeDefinition {
    /// Validate and construct a versioned definition with a reproducible hash.
    pub fn new(name: String, mode: ProbeMode, chunk: String) -> Result<Self, String> {
        valid_name(&name)?;
        if chunk.is_empty() || chunk.len() > MAX_CHUNK || chunk.as_bytes().contains(&0) {
            return Err("probe chunk must be 1..8192 bytes without NUL".into());
        }
        let mut hash = Sha256::new();
        hash.update(b"cimmeria-lab-probe-v1\0");
        hash.update(match mode {
            ProbeMode::Read => b"read".as_slice(),
            ProbeMode::Mutate => b"mutate".as_slice(),
        });
        hash.update(b"\0");
        hash.update(chunk.as_bytes());
        Ok(Self {
            schema_version: 1,
            name,
            mode,
            chunk,
            sha256: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
            favorite: false,
        })
    }

    fn checked(self) -> Result<Self, String> {
        let expected = Self::new(self.name.clone(), self.mode, self.chunk.clone())?;
        if self.schema_version != 1 || self.sha256 != expected.sha256 {
            return Err("probe version or hash mismatch".into());
        }
        Ok(self)
    }
}

fn valid_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        Err("probe name must be 1..64 ASCII letters, digits, - or _".into())
    } else {
        Ok(())
    }
}

/// Per-user daemon storage; the instance label is validated before joining.
#[derive(Clone)]
pub struct ProbeCatalog {
    root: PathBuf,
}

impl ProbeCatalog {
    /// Open a catalog under an already selected per-user data directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn directory(&self, instance: &str) -> Result<PathBuf, String> {
        valid_name(instance)?;
        Ok(self.root.join("workbench").join("probes").join(instance))
    }

    /// Return all saved definitions in stable name order, rejecting invalid files.
    pub fn list(&self, instance: &str) -> Result<Vec<ProbeDefinition>, String> {
        let dir = self.directory(instance)?;
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = BTreeMap::new();
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json")
                || path.file_name().and_then(|s| s.to_str()) == Some("history.json")
            {
                continue;
            }
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                continue;
            }
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            if bytes.len() > MAX_CHUNK + 1_024 {
                return Err(format!("probe file too large: {}", path.display()));
            }
            let def: ProbeDefinition = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let def = def.checked()?;
            if path.file_stem().and_then(|s| s.to_str()) != Some(def.name.as_str()) {
                return Err("probe file name mismatch".into());
            }
            entries.insert(def.name.clone(), def);
            if entries.len() > MAX_PROBES {
                return Err("probe count exceeds 128".into());
            }
        }
        Ok(entries.into_values().collect())
    }

    /// Save one definition atomically after validating the existing catalog.
    pub fn save(&self, instance: &str, def: ProbeDefinition) -> Result<ProbeDefinition, String> {
        let def = def.checked()?;
        let list = self.list(instance)?;
        if list.len() >= MAX_PROBES && !list.iter().any(|old| old.name == def.name) {
            return Err("probe count exceeds 128".into());
        }
        let dir = self.directory(instance)?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.json", def.name));
        let temp = dir.join(format!(".{}.{}.tmp", def.name, uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(&def).map_err(|e| e.to_string())?;
        fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        Ok(def)
    }

    /// Load a named definition, if present.
    pub fn get(&self, instance: &str, name: &str) -> Result<Option<ProbeDefinition>, String> {
        valid_name(name)?;
        Ok(self
            .list(instance)?
            .into_iter()
            .find(|def| def.name == name))
    }

    /// Store a bounded, metadata-only history row without result content.
    pub fn record(&self, instance: &str, name: &str, status: &str) -> Result<(), String> {
        valid_name(name)?;
        let dir = self.directory(instance)?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("history.json");
        let mut rows: Vec<serde_json::Value> = fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        rows.push(serde_json::json!({"name": name, "status": status, "at_ms": chrono::Utc::now().timestamp_millis()}));
        if rows.len() > 100 {
            rows.drain(..rows.len() - 100);
        }
        fs::write(path, serde_json::to_vec(&rows).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    /// Read recent metadata without source or result bytes.
    pub fn history(&self, instance: &str) -> Result<Vec<serde_json::Value>, String> {
        let path = self.directory(instance)?.join("history.json");
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Separate caller-owned root for deterministic tests.
#[cfg(test)]
pub fn test_catalog(root: &std::path::Path) -> ProbeCatalog {
    ProbeCatalog::new(root.to_path_buf())
}
