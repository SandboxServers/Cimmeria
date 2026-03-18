//! NavMesh loading and visualization for the Scene Editor.
//!
//! Loads XRC-format .nav files (custom Recast output format used by the
//! Cimmeria NavBuilder). Parses the coarse polygon mesh and detail mesh,
//! then returns triangle data for Three.js rendering.
//!
//! XRC format: agent params → poly mesh metadata → quantized verts →
//! polygon connectivity → region/flags/areas → detail mesh → detail verts → detail tris

use std::io::{BufReader, Read as IoRead};
use std::path::PathBuf;

use serde::Serialize;

use crate::state::AppState;

// ── Response types ─────────────────────────────────────────────────────────

/// NavMesh data ready for Three.js rendering.
#[derive(Debug, Serialize)]
pub struct NavMeshData {
    /// Flat array of triangle vertices: [x0,y0,z0, x1,y1,z1, x2,y2,z2, ...]
    /// In UE3/Three.js coordinate space.
    pub vertices: Vec<f32>,
    /// Per-triangle area ID (0=blocked, 1=ground, 2+=custom). One per triangle.
    pub area_ids: Vec<u8>,
    /// Bounding box min [x, y, z] in UE3 coordinates.
    pub bounds_min: [f32; 3],
    /// Bounding box max [x, y, z] in UE3 coordinates.
    pub bounds_max: [f32; 3],
    /// Total polygon count (coarse mesh).
    pub poly_count: u32,
    /// Total triangle count (detail mesh).
    pub tri_count: u32,
    /// Agent parameters.
    pub agent_height: f32,
    pub agent_radius: f32,
}

// ── XRC parser ─────────────────────────────────────────────────────────────

struct XrcMesh {
    cs: f32,
    ch: f32,
    bmin: [f32; 3],
    _bmax: [f32; 3],
    nverts: u32,
    npolys: u32,
    nvp: u32,
    verts: Vec<u16>,     // nverts * 3 (quantized)
    polys: Vec<u16>,     // npolys * nvp * 2
    _regs: Vec<u16>,
    _flags: Vec<u16>,
    areas: Vec<u8>,      // npolys
    // Detail mesh
    detail_meshes: Vec<u32>,  // nmeshes * 4
    detail_verts: Vec<f32>,   // nverts * 3
    detail_tris: Vec<u8>,     // ntris * 4
    detail_nmeshes: u32,
    detail_nverts: u32,
    detail_ntris: u32,
    // Agent
    agent_height: f32,
    agent_radius: f32,
}

fn read_f32(r: &mut impl IoRead) -> std::io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_u32(r: &mut impl IoRead) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u16(r: &mut impl IoRead) -> std::io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u8(r: &mut impl IoRead) -> std::io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn load_xrc(path: &str) -> Result<XrcMesh, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open nav file: {e}"))?;
    let mut r = BufReader::new(file);

    // Section 1: Agent parameters
    let agent_height = read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let agent_climb = read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let agent_radius = read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let _ = agent_climb; // Not needed for rendering

    // Section 2: Mesh metadata
    let nverts = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let npolys = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let nvp = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let _border_size = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?;

    // Section 3: Grid config
    let cs = read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let ch = read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let bmin = [
        read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?,
        read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?,
        read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?,
    ];
    let bmax = [
        read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?,
        read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?,
        read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?,
    ];

    // Section 4: Quantized vertices
    let mut verts = vec![0u16; (nverts * 3) as usize];
    for v in &mut verts {
        *v = read_u16(&mut r).map_err(|e| format!("Read error: {e}"))?;
    }

    // Section 5: Polygon connectivity
    let mut polys = vec![0u16; (npolys * nvp * 2) as usize];
    for p in &mut polys {
        *p = read_u16(&mut r).map_err(|e| format!("Read error: {e}"))?;
    }

    // Section 6-8: Region IDs, flags, areas
    let mut regs = vec![0u16; npolys as usize];
    for v in &mut regs { *v = read_u16(&mut r).map_err(|e| format!("Read error: {e}"))?; }
    let mut flags = vec![0u16; npolys as usize];
    for v in &mut flags { *v = read_u16(&mut r).map_err(|e| format!("Read error: {e}"))?; }
    let mut areas = vec![0u8; npolys as usize];
    for v in &mut areas { *v = read_u8(&mut r).map_err(|e| format!("Read error: {e}"))?; }

    // Section 9: Detail mesh metadata
    let detail_nmeshes = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let detail_nverts = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?;
    let detail_ntris = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?;

    // Section 10: Detail mesh descriptors
    let mut detail_meshes = vec![0u32; (detail_nmeshes * 4) as usize];
    for v in &mut detail_meshes { *v = read_u32(&mut r).map_err(|e| format!("Read error: {e}"))?; }

    // Section 11: Detail vertices (float32)
    let mut detail_verts = vec![0.0f32; (detail_nverts * 3) as usize];
    for v in &mut detail_verts { *v = read_f32(&mut r).map_err(|e| format!("Read error: {e}"))?; }

    // Section 12: Detail triangles
    let mut detail_tris = vec![0u8; (detail_ntris * 4) as usize];
    r.read_exact(&mut detail_tris).map_err(|e| format!("Read error: {e}"))?;

    Ok(XrcMesh {
        cs, ch, bmin, _bmax: bmax, nverts, npolys, nvp,
        verts, polys, _regs: regs, _flags: flags, areas,
        detail_meshes, detail_verts, detail_tris,
        detail_nmeshes, detail_nverts, detail_ntris,
        agent_height, agent_radius,
    })
}

/// Dequantize a vertex from the coarse mesh.
fn dequantize_vert(mesh: &XrcMesh, idx: usize) -> [f32; 3] {
    let x = mesh.bmin[0] + mesh.verts[idx * 3] as f32 * mesh.cs;
    let y = mesh.bmin[1] + mesh.verts[idx * 3 + 1] as f32 * mesh.ch;
    let z = mesh.bmin[2] + mesh.verts[idx * 3 + 2] as f32 * mesh.cs;
    [x, y, z]
}

/// Convert BigWorld coordinates to Three.js/UE3 viewport coordinates.
/// BW: X-forward, Y-up, Z-right → Three.js: X-right, Y-up, Z=-forward
/// For navmesh, coordinates are already in Recast space (X-right, Y-up, Z-forward)
/// which maps to Three.js as X=X, Y=Y, Z=-Z.
fn bw_to_three(v: [f32; 3]) -> [f32; 3] {
    // Recast uses same handedness as Three.js but with Z flipped
    // Scale from meters to centimeters for consistency with UE3 viewport
    [v[0] * 100.0, v[1] * 100.0, -v[2] * 100.0]
}

/// Build triangle list from the detail mesh for rendering.
fn build_triangles(mesh: &XrcMesh) -> (Vec<f32>, Vec<u8>) {
    let mut vertices = Vec::new();
    let mut area_ids = Vec::new();

    // Iterate through each polygon's detail mesh
    for i in 0..mesh.detail_nmeshes as usize {
        let base_vert = mesh.detail_meshes[i * 4] as usize;
        let _vert_count = mesh.detail_meshes[i * 4 + 1] as usize;
        let base_tri = mesh.detail_meshes[i * 4 + 2] as usize;
        let tri_count = mesh.detail_meshes[i * 4 + 3] as usize;

        let area_id = if i < mesh.areas.len() { mesh.areas[i] } else { 0 };

        for t in 0..tri_count {
            let ti = (base_tri + t) * 4;
            if ti + 2 >= mesh.detail_tris.len() { break; }

            let idx0 = mesh.detail_tris[ti] as usize;
            let idx1 = mesh.detail_tris[ti + 1] as usize;
            let idx2 = mesh.detail_tris[ti + 2] as usize;

            // Detail vertices are stored as:
            // - If index < nvp, it references the coarse mesh poly vertex
            // - If index >= nvp, it references detail_verts at (base_vert + index - nvp)
            let nvp = mesh.nvp as usize;
            let poly_base = i * nvp * 2; // Index into polys array for this polygon

            for &vi in &[idx0, idx1, idx2] {
                let v = if vi < nvp {
                    // Reference to coarse mesh vertex
                    let vert_idx = if poly_base + vi < mesh.polys.len() {
                        mesh.polys[poly_base + vi] as usize
                    } else {
                        0
                    };
                    dequantize_vert(mesh, vert_idx)
                } else {
                    // Reference to detail vertex
                    let dv = base_vert + vi - nvp;
                    if dv * 3 + 2 < mesh.detail_verts.len() {
                        [
                            mesh.detail_verts[dv * 3],
                            mesh.detail_verts[dv * 3 + 1],
                            mesh.detail_verts[dv * 3 + 2],
                        ]
                    } else {
                        [0.0, 0.0, 0.0]
                    }
                };

                let tv = bw_to_three(v);
                vertices.extend_from_slice(&tv);
            }

            area_ids.push(area_id);
        }
    }

    (vertices, area_ids)
}

// ── Tauri commands ─────────────────────────────────────────────────────────

/// Load a navmesh from a .nav file and return triangle data for 3D rendering.
///
/// The `nav_path` should be the full path to the .nav file in `data/spaces/`.
#[tauri::command]
pub async fn load_navmesh(
    _state: tauri::State<'_, AppState>,
    nav_path: String,
) -> Result<NavMeshData, String> {
    tracing::info!("Loading navmesh from {}", nav_path);

    let mesh = load_xrc(&nav_path)?;

    let (vertices, area_ids) = build_triangles(&mesh);
    let tri_count = area_ids.len() as u32;

    // Convert bounds to Three.js space
    let bmin = bw_to_three(mesh.bmin);
    let bmax = bw_to_three(mesh._bmax);

    tracing::info!(
        polys = mesh.npolys,
        detail_tris = mesh.detail_ntris,
        rendered_tris = tri_count,
        "NavMesh loaded"
    );

    Ok(NavMeshData {
        vertices,
        area_ids,
        bounds_min: [bmin[0].min(bmax[0]), bmin[1].min(bmax[1]), bmin[2].min(bmax[2])],
        bounds_max: [bmin[0].max(bmax[0]), bmin[1].max(bmax[1]), bmin[2].max(bmax[2])],
        poly_count: mesh.npolys,
        tri_count,
        agent_height: mesh.agent_height,
        agent_radius: mesh.agent_radius,
    })
}

/// List available navmesh files in the data/spaces directory.
#[tauri::command]
pub async fn list_navmeshes(
    _state: tauri::State<'_, AppState>,
) -> Result<Vec<NavMeshInfo>, String> {
    let spaces_dir = "data/spaces";
    let entries = match std::fs::read_dir(spaces_dir) {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };

    let mut navmeshes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "nav") {
            let name = path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            navmeshes.push(NavMeshInfo {
                name,
                path: path.to_string_lossy().to_string(),
                size_bytes: size,
            });
        }
    }

    navmeshes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(navmeshes)
}

#[derive(Debug, Serialize)]
pub struct NavMeshInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
}
