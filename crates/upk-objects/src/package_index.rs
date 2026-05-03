//! Cross-package export index for resolving references across .upk/.umap files.
//!
//! StaticMeshActors in .umap files reference meshes via import entries pointing to
//! .upk packages. The scene editor needs a package index built at startup to resolve
//! these cross-package references efficiently.
//!
//! Building the index scans all ~5,000 packages' export tables (~50 seconds).
//! The result is cached to disk via bincode for instant subsequent launches.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ObjectError, Result};

/// Index of all exports across all packages for cross-reference resolution.
#[derive(Serialize, Deserialize)]
pub struct PackageIndex {
    /// `(package_name, object_name)` -> `ExportLocation`
    pub exports: HashMap<(String, String), ExportLocation>,
    /// `class_name` -> list of `(package_name, object_name)` keys
    pub by_class: HashMap<String, Vec<(String, String)>>,
    /// Total packages scanned.
    pub package_count: u32,
    /// Total exports indexed.
    pub export_count: u32,
}

/// Location of an export within a package file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportLocation {
    pub file_path: PathBuf,
    pub export_index: usize,
    pub class_name: String,
    pub serial_size: i32,
}

impl PackageIndex {
    /// Build an index from a CookedPC directory by scanning all `.upk` and `.umap` files.
    ///
    /// This walks the directory recursively, opens each package, and records every
    /// export with `serial_size > 0` into the index.
    pub fn build(cooked_pc_path: &Path) -> Result<Self> {
        let mut exports = HashMap::new();
        let mut by_class: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut package_count = 0u32;
        let mut export_count = 0u32;

        // Collect all .upk and .umap files recursively
        let mut files = Vec::new();
        collect_package_files(cooked_pc_path, &mut files);
        files.sort();

        let total_files = files.len();

        for file_path in &files {
            let pkg = match cimmeria_upk::Package::open(file_path) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Failed to open {}: {}", file_path.display(), e);
                    continue;
                }
            };
            package_count += 1;

            // Derive package name from filename (without extension)
            let pkg_name = file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            for (i, export) in pkg.exports.iter().enumerate() {
                if export.serial_size <= 0 {
                    continue;
                }
                let class_name = pkg.export_class_name(export).to_string();
                let obj_name = export.object_name.clone();

                let key = (pkg_name.clone(), obj_name.clone());
                let loc = ExportLocation {
                    file_path: file_path.clone(),
                    export_index: i,
                    class_name: class_name.clone(),
                    serial_size: export.serial_size,
                };

                exports.insert(key.clone(), loc);
                by_class.entry(class_name).or_default().push(key);
                export_count += 1;
            }

            if package_count.is_multiple_of(500) {
                tracing::info!(
                    "Indexed {}/{} packages ({} exports so far)...",
                    package_count,
                    total_files,
                    export_count
                );
            }
        }

        Ok(Self {
            exports,
            by_class,
            package_count,
            export_count,
        })
    }

    /// Look up an export by package name and object name.
    pub fn find(&self, package_name: &str, object_name: &str) -> Option<&ExportLocation> {
        self.exports
            .get(&(package_name.to_string(), object_name.to_string()))
    }

    /// Find all exports of a given class.
    pub fn find_by_class(&self, class_name: &str) -> &[(String, String)] {
        self.by_class
            .get(class_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Save the index to a bincode file for fast subsequent loading.
    pub fn save(&self, path: &Path) -> Result<()> {
        let data = bincode::serialize(self)
            .map_err(|e| ObjectError::InvalidData(format!("bincode serialize: {e}")))?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load the index from a previously saved bincode file.
    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        bincode::deserialize(&data)
            .map_err(|e| ObjectError::InvalidData(format!("bincode deserialize: {e}")))
    }
}

/// Recursively collect all `.upk` and `.umap` files under a directory.
fn collect_package_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_package_files(&path, files);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("upk") || ext.eq_ignore_ascii_case("umap") {
                files.push(path);
            }
        }
    }
}
