//! Mesh loading commands — resolve and deserialize StaticMesh exports.

use crate::state::AppState;
use cimmeria_upk::Package;
use cimmeria_upk_objects::static_mesh::deserialize_static_mesh;
use cimmeria_upk_objects::PackageIndex;
use serde::Serialize;
use std::path::Path;

/// Material info for a single mesh section, returned to the frontend.
#[derive(Serialize, Clone, Debug)]
pub struct MeshSectionMaterial {
    pub section_index: usize,
    pub material_name: String,
    pub texture_name: Option<String>,
}

/// Minimal mesh data sent to the frontend for 3D rendering.
#[derive(Serialize, Clone)]
pub struct MeshData {
    /// Flat array of vertex positions: [x0,y0,z0, x1,y1,z1, ...]
    pub positions: Vec<f32>,
    /// Flat array of vertex normals: [nx0,ny0,nz0, ...]
    pub normals: Vec<f32>,
    /// Flat array of UV coordinates: [u0,v0, u1,v1, ...]
    pub uvs: Vec<f32>,
    /// Triangle indices (referencing positions/3)
    pub indices: Vec<u32>,
    /// Bounding box: [origin_x, origin_y, origin_z, extent_x, extent_y, extent_z]
    pub bounds: [f32; 6],
}

/// Load a StaticMesh by name, using the package index for fast resolution.
///
/// On first call, loads `package_index.bin` from the CookedPC directory (built
/// by `build-package-index`). Falls back to brute-force search of `Packages/`
/// if the index isn't available.
#[tauri::command]
pub async fn load_mesh(
    state: tauri::State<'_, AppState>,
    mesh_name: String,
) -> Result<MeshData, String> {
    // Check cache first
    {
        let cache = state.mesh_cache.lock().unwrap();
        if let Some(data) = cache.get(&mesh_name) {
            return Ok(data.clone());
        }
    }

    let cooked_pc = {
        let guard = state.cooked_pc_path.lock().unwrap();
        guard.clone().ok_or("CookedPC path not set")?
    };

    // Ensure package index is loaded
    ensure_package_index(&state, &cooked_pc);

    // Try index-based lookup first
    let mesh_data = {
        let idx_guard = state.package_index.lock().unwrap();
        if let Some(idx) = idx_guard.as_ref() {
            load_mesh_via_index(idx, &mesh_name)
        } else {
            None
        }
    };

    let mesh_data = match mesh_data {
        Some(Ok(data)) => data,
        Some(Err(e)) => return Err(format!("Failed to load mesh '{}': {}", mesh_name, e)),
        None => {
            // Fallback: brute-force scan of Packages/
            let packages_dir = cooked_pc.join("Packages");
            if !packages_dir.is_dir() {
                return Err("Packages directory not found and no package index".into());
            }
            find_and_load_mesh_brute_force(&packages_dir, &mesh_name)
                .map_err(|e| format!("Failed to load mesh '{}': {}", mesh_name, e))?
        }
    };

    // Cache the result
    {
        let mut cache = state.mesh_cache.lock().unwrap();
        cache.insert(mesh_name, mesh_data.clone());
    }

    Ok(mesh_data)
}

/// Return a list of all unique StaticMesh names referenced by loaded actors.
#[tauri::command]
pub async fn list_mesh_refs(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let guard = state.loaded_zone.lock().unwrap();
    let zone = guard.as_ref().ok_or("No zone loaded")?;

    let mut meshes: Vec<String> = zone
        .actors
        .iter()
        .filter(|a| !a.static_mesh.is_empty())
        .map(|a| a.static_mesh.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    meshes.sort();

    Ok(meshes)
}

/// Load or build the package index, storing it in AppState.
fn ensure_package_index(state: &AppState, cooked_pc: &Path) {
    let mut idx_guard = state.package_index.lock().unwrap();
    if idx_guard.is_some() {
        return;
    }

    // Try loading from disk first
    let index_path = cooked_pc.join("package_index.bin");
    if index_path.is_file() {
        match PackageIndex::load(&index_path) {
            Ok(idx) => {
                tracing::info!(
                    "Loaded package index: {} exports across {} packages",
                    idx.export_count,
                    idx.package_count
                );
                *idx_guard = Some(idx);
                return;
            }
            Err(e) => {
                tracing::warn!("Failed to load package index: {}", e);
            }
        }
    }

    // Build index from Packages/ directory
    let packages_dir = cooked_pc.join("Packages");
    if packages_dir.is_dir() {
        tracing::info!("Building package index from Packages/...");
        match PackageIndex::build(&packages_dir) {
            Ok(idx) => {
                tracing::info!(
                    "Built package index: {} exports across {} packages",
                    idx.export_count,
                    idx.package_count
                );
                // Save for next time
                if let Err(e) = idx.save(&index_path) {
                    tracing::warn!("Failed to save package index: {}", e);
                }
                *idx_guard = Some(idx);
            }
            Err(e) => {
                tracing::warn!("Failed to build package index: {}", e);
            }
        }
    }
}

/// Use the package index to find a StaticMesh by object name.
fn load_mesh_via_index(index: &PackageIndex, mesh_name: &str) -> Option<Result<MeshData, String>> {
    // Search all StaticMesh exports for one matching the name
    let meshes = index.find_by_class("StaticMesh");
    let loc = meshes.iter().find_map(|(pkg_name, obj_name)| {
        if obj_name == mesh_name {
            index.find(pkg_name, obj_name)
        } else {
            None
        }
    })?;

    Some(load_mesh_from_location(&loc.file_path, loc.export_index))
}

/// Open a package file and deserialize a specific export as a StaticMesh.
fn load_mesh_from_location(file_path: &Path, export_index: usize) -> Result<MeshData, String> {
    let pkg = Package::open(file_path)
        .map_err(|e| format!("Open package {}: {e}", file_path.display()))?;

    let export = pkg
        .exports
        .get(export_index)
        .ok_or_else(|| format!("Export index {} out of range", export_index))?;

    let data = pkg
        .read_export_data(export)
        .map_err(|e| format!("Read export: {e}"))?;

    let mesh =
        deserialize_static_mesh(&data, &pkg.names).map_err(|e| format!("Deserialize: {e}"))?;

    convert_mesh_to_data(&mesh)
}

/// Convert a deserialized StaticMesh to flat-array MeshData for the frontend.
fn convert_mesh_to_data(
    mesh: &cimmeria_upk_objects::static_mesh::StaticMesh,
) -> Result<MeshData, String> {
    if mesh.lod_models.is_empty() {
        return Err("No LOD models".into());
    }

    let lod = &mesh.lod_models[0];
    let mut positions = Vec::with_capacity(lod.vertices.len() * 3);
    let mut normals = Vec::with_capacity(lod.vertices.len() * 3);
    let mut uvs = Vec::with_capacity(lod.vertices.len() * 2);

    for v in &lod.vertices {
        positions.extend_from_slice(&v.position);
        normals.extend_from_slice(&v.normal);
        uvs.extend_from_slice(&v.uv);
    }

    Ok(MeshData {
        positions,
        normals,
        uvs,
        indices: lod.indices.clone(),
        bounds: [
            mesh.bounds.origin[0],
            mesh.bounds.origin[1],
            mesh.bounds.origin[2],
            mesh.bounds.extent[0],
            mesh.bounds.extent[1],
            mesh.bounds.extent[2],
        ],
    })
}

/// Brute-force search for a StaticMesh across all .upk files.
fn find_and_load_mesh_brute_force(
    packages_dir: &Path,
    mesh_name: &str,
) -> Result<MeshData, String> {
    let walker = walk_upk_files(packages_dir);

    for upk_path in walker {
        let pkg = match Package::open(&upk_path) {
            Ok(p) => p,
            Err(_) => continue,
        };

        for export in &pkg.exports {
            let class = pkg.export_class_name(export);
            if class != "StaticMesh" {
                continue;
            }
            if export.object_name != mesh_name {
                continue;
            }

            let data = pkg
                .read_export_data(export)
                .map_err(|e| format!("Read export: {e}"))?;

            let mesh = deserialize_static_mesh(&data, &pkg.names)
                .map_err(|e| format!("Deserialize: {e}"))?;

            return convert_mesh_to_data(&mesh);
        }
    }

    Err(format!(
        "StaticMesh '{}' not found in any package",
        mesh_name
    ))
}

fn walk_upk_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    walk_upk_recursive(dir, &mut files);
    files
}

fn walk_upk_recursive(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_upk_recursive(&path, files);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .map_or(false, |e| e.eq_ignore_ascii_case("upk"))
            {
                files.push(path);
            }
        }
    }
}

/// Resolve material references on a StaticMesh's sections.
///
/// For each MeshSection, resolves `material_ref` to a material name and
/// attempts to find a matching Texture2D in the package index via heuristic
/// name matching. The frontend uses the returned texture names to call
/// `get_texture_thumbnail` for Three.js material setup.
#[tauri::command]
pub async fn get_mesh_materials(
    state: tauri::State<'_, AppState>,
    mesh_name: String,
) -> Result<Vec<MeshSectionMaterial>, String> {
    let cooked_pc = {
        let guard = state.cooked_pc_path.lock().unwrap();
        guard.clone().ok_or("CookedPC path not set")?
    };

    ensure_package_index(&state, &cooked_pc);

    // Locate the mesh package file and export index
    let (pkg, _export_index) = {
        let idx_guard = state.package_index.lock().unwrap();
        if let Some(idx) = idx_guard.as_ref() {
            let meshes = idx.find_by_class("StaticMesh");
            let loc = meshes
                .iter()
                .find_map(|(pkg_name, obj_name)| {
                    if obj_name == &mesh_name {
                        idx.find(pkg_name, obj_name)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| format!("StaticMesh '{}' not found in package index", mesh_name))?;

            let pkg = Package::open(&loc.file_path)
                .map_err(|e| format!("Open package {}: {e}", loc.file_path.display()))?;
            (pkg, loc.export_index)
        } else {
            return Err("Package index not available".into());
        }
    };

    // Find the StaticMesh export and deserialize it
    let export = pkg
        .exports
        .iter()
        .find(|e| pkg.export_class_name(e) == "StaticMesh" && e.object_name == mesh_name)
        .ok_or_else(|| format!("StaticMesh '{}' not found in package exports", mesh_name))?;

    let data = pkg
        .read_export_data(export)
        .map_err(|e| format!("Read export: {e}"))?;

    let mesh =
        deserialize_static_mesh(&data, &pkg.names).map_err(|e| format!("Deserialize: {e}"))?;

    if mesh.lod_models.is_empty() {
        return Ok(Vec::new());
    }

    let lod = &mesh.lod_models[0];

    // Collect all Texture2D exports from the package index for heuristic matching
    let texture_entries: Vec<(String, String)> = {
        let idx_guard = state.package_index.lock().unwrap();
        if let Some(idx) = idx_guard.as_ref() {
            idx.find_by_class("Texture2D").to_vec()
        } else {
            Vec::new()
        }
    };

    let mut results = Vec::with_capacity(lod.sections.len());

    for (i, section) in lod.sections.iter().enumerate() {
        let material_name = pkg.resolve_object_name(section.material_ref).to_string();

        // Try to find a matching texture by heuristic name matching
        let texture_name = find_texture_for_material(&material_name, &texture_entries);

        results.push(MeshSectionMaterial {
            section_index: i,
            material_name,
            texture_name,
        });
    }

    Ok(results)
}

/// Heuristic texture lookup: strip material prefixes and find a Texture2D
/// whose name contains the resulting base string.
///
/// UE3 naming conventions typically use prefixes like `MI_`, `M_`, `MIC_`,
/// `Mat_` for materials and the corresponding texture has a matching base
/// name with a `T_` prefix or `_D` (diffuse) / `_N` (normal) suffix.
fn find_texture_for_material(
    material_name: &str,
    texture_entries: &[(String, String)],
) -> Option<String> {
    if material_name == "None" || material_name.starts_with('<') {
        return None;
    }

    // Strip known material prefixes to get the base name
    let base = strip_material_prefix(material_name);
    if base.is_empty() {
        return None;
    }

    let base_lower = base.to_lowercase();

    // First pass: look for an exact texture name match (e.g., base name == texture name)
    for (_pkg_name, obj_name) in texture_entries {
        let obj_lower = obj_name.to_lowercase();
        if obj_lower == base_lower {
            return Some(obj_name.clone());
        }
    }

    // Second pass: look for textures containing the base name, preferring diffuse (_D suffix)
    let mut best_match: Option<&str> = None;
    let mut best_score: u32 = 0;

    for (_pkg_name, obj_name) in texture_entries {
        let obj_lower = obj_name.to_lowercase();

        if !obj_lower.contains(&base_lower) {
            continue;
        }

        // Score: prefer shorter names (closer match) and diffuse textures
        let mut score = 1;
        if obj_lower.ends_with("_d")
            || obj_lower.ends_with("_diff")
            || obj_lower.ends_with("_diffuse")
        {
            score += 10;
        }
        // Penalize normal maps, spec maps, etc.
        if obj_lower.ends_with("_n")
            || obj_lower.ends_with("_norm")
            || obj_lower.ends_with("_normal")
        {
            continue;
        }
        if obj_lower.ends_with("_s") || obj_lower.ends_with("_spec") {
            continue;
        }
        // Prefer shorter names (closer to an exact match)
        if obj_name.len() <= base.len() + 5 {
            score += 5;
        }

        if score > best_score {
            best_score = score;
            best_match = Some(obj_name.as_str());
        }
    }

    best_match.map(|s| s.to_string())
}

/// Strip common UE3 material prefixes: MI_, M_, MIC_, Mat_
fn strip_material_prefix(name: &str) -> &str {
    // Try longest prefixes first
    for prefix in &["MIC_", "Mat_", "MI_", "M_"] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return rest;
        }
    }
    // No recognized prefix -- use the full name as the base
    name
}
