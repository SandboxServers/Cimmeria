//! XRC `.nav` parsing and Detour tile construction — everything that runs
//! once, at space creation, to turn a file into a [`NavMesh`].
//!
//! Split out of `navigation/mod.rs`, which keeps the runtime query API.
//!
//! The parse streams: it reads through a [`HashingReader`] wrapped
//! around a `BufReader`, so every header count reaches
//! [`check_count`] after ~60 bytes have been read and long before the
//! allocation it would drive, while the [`NavMeshFingerprint`]'s hash
//! still covers every byte of the file. An earlier revision read the
//! whole file with `std::fs::read` to get bytes to hash; that put a
//! whole-file allocation *ahead* of the hostile-header caps, so a
//! corrupt or sparse deployment asset became a startup OOM instead of a
//! rejected file (PR #700 review).

use std::fs::File;
use std::io::{BufReader, Read as IoRead};
use std::path::Path;

use crate::detour_ffi;

use super::fingerprint::{AgentParams, HashingReader, NavMeshFingerprint};
use super::xrc::{
    check_count, check_file_size, checked_alloc_size, read_f32, read_u16, read_u32, read_u8,
    MAX_DETAIL_NMESHES, MAX_DETAIL_NTRIS, MAX_DETAIL_NVERTS, MAX_NPOLYS, MAX_NVERTS, MAX_NVP,
};
use super::NavMesh;

impl NavMesh {
    /// Load a navigation mesh from an XRC-format `.nav` file.
    ///
    /// Parses the XRC binary, builds a Detour navmesh tile, and initializes
    /// a query object. Follows the exact pipeline from the C++ reference
    /// implementation in `navigation.cpp`.
    ///
    /// Rejection is ordered by how little it costs to decide:
    ///
    /// 1. The `metadata` length against [`check_file_size`] — no bytes
    ///    read, the file is not even opened.
    /// 2. Each header count against its `MAX_*` cap, ~60 bytes in, before
    ///    the `Vec` it sizes exists.
    /// 3. Short reads, from any truncated section.
    ///
    /// The [`NavMeshFingerprint`] falls out of the same single pass: the
    /// reader hashes what it hands to the parser and
    /// [`HashingReader::finish`] drains and hashes the rest, so the value
    /// is byte-for-byte what hashing the whole file would give.
    pub fn load(path: &Path) -> cimmeria_common::Result<Self> {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        check_file_size(std::fs::metadata(path)?.len())?;
        let mut r = HashingReader::new(BufReader::new(File::open(path)?));

        // ── Section 1: Agent parameters ─────────────────────────────────
        let agent = AgentParams {
            height: read_f32(&mut r)?,
            climb: read_f32(&mut r)?,
            radius: read_f32(&mut r)?,
        };

        // ── Section 2: Mesh metadata ────────────────────────────────────
        //
        // Every header count is validated against its `MAX_*` cap before the
        // matching allocation. A `.nav` file with `nverts = 0xFFFFFFFF` would
        // otherwise wrap `nverts * 3` to a 3-element `Vec<u16>` and the
        // following read loop would consume only three u16s while the rest
        // of the claimed `0xFFFFFFFF * 3` vertex region went phantom —
        // leaving every downstream section offset wrong and (on a u32
        // multiplication that does *not* wrap) demanding a 12 GB allocation
        // that crashes the server at startup. Operator-deployable input,
        // strict bounds check.
        let nverts = check_count(read_u32(&mut r)?, MAX_NVERTS, "nverts")?;
        let npolys = check_count(read_u32(&mut r)?, MAX_NPOLYS, "npolys")?;
        let nvp = check_count(read_u32(&mut r)?, MAX_NVP, "nvp")?;
        let _border_size = read_u32(&mut r)?;

        // ── Section 3: Grid config (quantization parameters) ────────────
        let cs = read_f32(&mut r)?;
        let ch = read_f32(&mut r)?;
        let bmin = [read_f32(&mut r)?, read_f32(&mut r)?, read_f32(&mut r)?];
        let bmax = [read_f32(&mut r)?, read_f32(&mut r)?, read_f32(&mut r)?];

        // ── Section 4: Quantized vertices ───────────────────────────────
        let verts_len = checked_alloc_size(nverts, 3, "nverts", "verts = nverts * 3 u16s")?;
        let mut verts = vec![0u16; verts_len];
        for v in &mut verts {
            *v = read_u16(&mut r)?;
        }

        // ── Section 5: Polygon connectivity ─────────────────────────────
        // Fold the `npolys * nvp * 2` product into one checked mul via
        // `npolys * (nvp * 2)`. `nvp` is already bounded by `MAX_NVP = 64`
        // so `nvp * 2` cannot overflow u32; `saturating_mul` is belt-and-
        // suspenders against future cap changes. The `field` slot stays
        // `"npolys"` (a real header field) and the multiplication shape
        // moves into `alloc_desc` so an operator seeing the error knows
        // both *which header field* and *which downstream allocation*
        // would have busted.
        let polys_len = checked_alloc_size(
            npolys,
            nvp.saturating_mul(2),
            "npolys",
            "polys = npolys * nvp * 2 u16s",
        )?;
        let mut polys = vec![0u16; polys_len];
        for p in &mut polys {
            *p = read_u16(&mut r)?;
        }

        // ── Sections 6-8: Regions, flags, areas ─────────────────────────
        // These are parallel `npolys`-length arrays (stride 1). `npolys`
        // is already capped by `check_count` above, so they're safe as
        // raw `as usize` today — but route them through `checked_alloc_size`
        // anyway. Defense in depth: if `MAX_NPOLYS` is ever raised, this
        // multiplication still gets checked, and the allocation pattern
        // stays uniform across every count-driven `Vec` in `NavMesh::load`.
        let regs_len = checked_alloc_size(npolys, 1, "npolys", "regs = npolys u16s")?;
        let mut regs = vec![0u16; regs_len];
        for v in &mut regs {
            *v = read_u16(&mut r)?;
        }
        let flags_len = checked_alloc_size(npolys, 1, "npolys", "flags = npolys u16s")?;
        let mut flags = vec![0u16; flags_len];
        for v in &mut flags {
            *v = read_u16(&mut r)?;
        }
        let areas_len = checked_alloc_size(npolys, 1, "npolys", "areas = npolys bytes")?;
        let mut areas = vec![0u8; areas_len];
        for v in &mut areas {
            *v = read_u8(&mut r)?;
        }

        // ── Sections 9-12: Detail mesh ──────────────────────────────────
        let detail_nmeshes = check_count(read_u32(&mut r)?, MAX_DETAIL_NMESHES, "detail_nmeshes")?;
        let detail_nverts = check_count(read_u32(&mut r)?, MAX_DETAIL_NVERTS, "detail_nverts")?;
        let detail_ntris = check_count(read_u32(&mut r)?, MAX_DETAIL_NTRIS, "detail_ntris")?;

        let detail_meshes_len = checked_alloc_size(
            detail_nmeshes,
            4,
            "detail_nmeshes",
            "detail_meshes = detail_nmeshes * 4 u32s",
        )?;
        let mut detail_meshes = vec![0u32; detail_meshes_len];
        for v in &mut detail_meshes {
            *v = read_u32(&mut r)?;
        }

        let detail_verts_len = checked_alloc_size(
            detail_nverts,
            3,
            "detail_nverts",
            "detail_verts = detail_nverts * 3 f32s",
        )?;
        let mut detail_verts = vec![0.0f32; detail_verts_len];
        for v in &mut detail_verts {
            *v = read_f32(&mut r)?;
        }

        let detail_tris_len = checked_alloc_size(
            detail_ntris,
            4,
            "detail_ntris",
            "detail_tris = detail_ntris * 4 bytes",
        )?;
        let mut detail_tris = vec![0u8; detail_tris_len];
        r.read_exact(&mut detail_tris)?;

        // Nothing else is parsed out of the file; drain whatever is left
        // so the fingerprint covers trailing bytes too, and close the
        // reader before the FFI block below.
        let (content_hash, file_bytes) = r.finish()?;

        // ── Build Detour navmesh tile ───────────────────────────────────
        // This mirrors the C++ navigation.cpp lines 109-138 exactly:
        // populate dtNavMeshCreateParams and call dtCreateNavMeshData.
        let mut nav_data: *mut u8 = std::ptr::null_mut();
        let mut nav_data_size: i32 = 0;

        let build_ok = unsafe {
            detour_ffi::detour_build_navmesh_data(
                verts.as_ptr(),
                nverts as i32,
                polys.as_ptr(),
                npolys as i32,
                nvp as i32,
                flags.as_ptr(),
                areas.as_ptr(),
                bmin.as_ptr(),
                bmax.as_ptr(),
                cs,
                ch,
                agent.height,
                agent.radius,
                agent.climb,
                detail_meshes.as_ptr(),
                detail_nmeshes as i32,
                detail_verts.as_ptr(),
                detail_nverts as i32,
                detail_tris.as_ptr(),
                detail_ntris as i32,
                &mut nav_data,
                &mut nav_data_size,
            )
        };

        if build_ok == 0 || nav_data.is_null() {
            return Err(cimmeria_common::CimmeriaError::Entity(format!(
                "Failed to build Detour navmesh data for '{name}'"
            )));
        }

        // ── Init dtNavMesh ──────────────────────────────────────────────
        let mesh_handle = unsafe { detour_ffi::detour_create_navmesh(nav_data, nav_data_size) };

        // Free the intermediate tile data — detour_create_navmesh made its own copy
        unsafe {
            detour_ffi::detour_free_data(nav_data);
        }

        if mesh_handle.is_null() {
            return Err(cimmeria_common::CimmeriaError::Entity(format!(
                "Failed to create Detour navmesh for '{name}'"
            )));
        }

        // ── Init dtNavMeshQuery (2048 nodes, matching C++ reference) ────
        let query_handle = unsafe { detour_ffi::detour_create_query(mesh_handle, 2048) };

        if query_handle.is_null() {
            unsafe {
                detour_ffi::detour_free_navmesh(mesh_handle);
            }
            return Err(cimmeria_common::CimmeriaError::Entity(format!(
                "Failed to create Detour navmesh query for '{name}'"
            )));
        }

        let fingerprint =
            NavMeshFingerprint::new(path, file_bytes, content_hash, nverts, npolys, agent);

        tracing::info!(
            name = %name,
            nverts,
            npolys,
            nvp,
            detail_nmeshes,
            detail_nverts,
            detail_ntris,
            agent_height = agent.height,
            agent_climb = agent.climb,
            agent_radius = agent.radius,
            file_bytes = fingerprint.file_bytes,
            navmesh_hash = %fingerprint.content_hash,
            navmesh_short_hash = %fingerprint.short_hash,
            "NavMesh loaded via Detour FFI"
        );

        Ok(NavMesh {
            query: query_handle,
            mesh: mesh_handle,
            name,
            fingerprint,
            agent_height: agent.height,
            agent_radius: agent.radius,
            bmin,
            bmax,
        })
    }
}
