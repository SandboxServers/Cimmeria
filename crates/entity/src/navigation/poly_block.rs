//! One Recast polygon mesh as the XRC formats store it — everything from
//! `nverts` to the last detail triangle — and its conversion into a
//! serialised Detour tile.
//!
//! A single-mesh `.nav` is three agent floats followed by one block; a
//! tiled (`XRCT`) `.nav` repeats the block once per tile (see
//! [`super::load_tiled`]). Both read it through [`PolyMeshBlock::read`],
//! so every header count is checked against its cap before the `Vec` it
//! sizes exists, whichever layout it came from.

use std::io::Read as IoRead;

use crate::detour_ffi;

use super::fingerprint::AgentParams;
use super::xrc::{
    check_count, checked_alloc_size, read_f32, read_u16, read_u32, read_u8, MAX_DETAIL_NMESHES,
    MAX_DETAIL_NTRIS, MAX_DETAIL_NVERTS,
};

/// Upper bounds on the three counts that size a block's polygon arrays.
/// The detail-mesh caps are the same for both layouts.
#[derive(Debug, Clone, Copy)]
pub(super) struct BlockCaps {
    pub nverts: u32,
    pub npolys: u32,
    pub nvp: u32,
}

/// A parsed poly-mesh block. Field names follow `rcPolyMesh` /
/// `rcPolyMeshDetail`.
pub(super) struct PolyMeshBlock {
    pub nverts: u32,
    pub npolys: u32,
    pub nvp: u32,
    pub cs: f32,
    pub ch: f32,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    verts: Vec<u16>,
    polys: Vec<u16>,
    flags: Vec<u16>,
    areas: Vec<u8>,
    pub detail_nmeshes: u32,
    pub detail_nverts: u32,
    pub detail_ntris: u32,
    detail_meshes: Vec<u32>,
    detail_verts: Vec<f32>,
    detail_tris: Vec<u8>,
}

/// A serialised Detour tile from `dtCreateNavMeshData`, freed on drop.
/// `detour_create_navmesh` and `detour_add_tile` both copy it.
pub(super) struct DetourTileData {
    ptr: *mut u8,
    len: i32,
}

impl DetourTileData {
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn len(&self) -> i32 {
        self.len
    }
}

impl Drop for DetourTileData {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from dtCreateNavMeshData (dtAlloc) and is
        // freed exactly once, here.
        unsafe { detour_ffi::detour_free_data(self.ptr) }
    }
}

impl PolyMeshBlock {
    /// Read one block, validating each count before its allocation.
    ///
    /// Every header count is validated against its cap before the
    /// matching allocation. A `.nav` file with `nverts = 0xFFFFFFFF` would
    /// otherwise wrap `nverts * 3` to a 3-element `Vec<u16>` and the
    /// following read loop would consume only three u16s while the rest
    /// of the claimed `0xFFFFFFFF * 3` vertex region went phantom —
    /// leaving every downstream section offset wrong and (on a u32
    /// multiplication that does *not* wrap) demanding a 12 GB allocation
    /// that crashes the server at startup. Operator-deployable input,
    /// strict bounds check.
    pub fn read(r: &mut impl IoRead, caps: BlockCaps) -> cimmeria_common::Result<Self> {
        let nverts = check_count(read_u32(r)?, caps.nverts, "nverts")?;
        let npolys = check_count(read_u32(r)?, caps.npolys, "npolys")?;
        let nvp = check_count(read_u32(r)?, caps.nvp, "nvp")?;
        let _border_size = read_u32(r)?;

        // Grid config (quantization parameters).
        let cs = read_f32(r)?;
        let ch = read_f32(r)?;
        let bmin = [read_f32(r)?, read_f32(r)?, read_f32(r)?];
        let bmax = [read_f32(r)?, read_f32(r)?, read_f32(r)?];

        let verts_len = checked_alloc_size(nverts, 3, "nverts", "verts = nverts * 3 u16s")?;
        let mut verts = vec![0u16; verts_len];
        for v in &mut verts {
            *v = read_u16(r)?;
        }

        // Fold the `npolys * nvp * 2` product into one checked mul via
        // `npolys * (nvp * 2)`. `nvp` is already bounded by its cap, so
        // `nvp * 2` cannot overflow u32; `saturating_mul` is belt-and-
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
            *p = read_u16(r)?;
        }

        // Regions, flags, areas: parallel `npolys`-length arrays. Routed
        // through `checked_alloc_size` anyway so the allocation pattern
        // stays uniform and a raised cap is still checked. Regions are
        // read and dropped; Detour has no use for them.
        let regs_len = checked_alloc_size(npolys, 1, "npolys", "regs = npolys u16s")?;
        for _ in 0..regs_len {
            read_u16(r)?;
        }
        let flags_len = checked_alloc_size(npolys, 1, "npolys", "flags = npolys u16s")?;
        let mut flags = vec![0u16; flags_len];
        for v in &mut flags {
            *v = read_u16(r)?;
        }
        let areas_len = checked_alloc_size(npolys, 1, "npolys", "areas = npolys bytes")?;
        let mut areas = vec![0u8; areas_len];
        for v in &mut areas {
            *v = read_u8(r)?;
        }

        // Detail mesh.
        let detail_nmeshes = check_count(read_u32(r)?, MAX_DETAIL_NMESHES, "detail_nmeshes")?;
        let detail_nverts = check_count(read_u32(r)?, MAX_DETAIL_NVERTS, "detail_nverts")?;
        let detail_ntris = check_count(read_u32(r)?, MAX_DETAIL_NTRIS, "detail_ntris")?;
        // dtCreateNavMeshData reads four detail-mesh words per polygon, so
        // a detail section with fewer sub-meshes than polygons is an
        // out-of-bounds read in C++, not a smaller mesh. Recast always
        // writes one per polygon.
        if detail_nmeshes != 0 && detail_nmeshes != npolys {
            return Err(super::xrc::reject(
                "detail_nmeshes",
                detail_nmeshes as u64,
                "detail sub-mesh count must equal npolys",
            ));
        }

        let detail_meshes_len = checked_alloc_size(
            detail_nmeshes,
            4,
            "detail_nmeshes",
            "detail_meshes = detail_nmeshes * 4 u32s",
        )?;
        let mut detail_meshes = vec![0u32; detail_meshes_len];
        for v in &mut detail_meshes {
            *v = read_u32(r)?;
        }

        let detail_verts_len = checked_alloc_size(
            detail_nverts,
            3,
            "detail_nverts",
            "detail_verts = detail_nverts * 3 f32s",
        )?;
        let mut detail_verts = vec![0.0f32; detail_verts_len];
        for v in &mut detail_verts {
            *v = read_f32(r)?;
        }

        let detail_tris_len = checked_alloc_size(
            detail_ntris,
            4,
            "detail_ntris",
            "detail_tris = detail_ntris * 4 bytes",
        )?;
        let mut detail_tris = vec![0u8; detail_tris_len];
        r.read_exact(&mut detail_tris)?;

        Ok(Self {
            nverts,
            npolys,
            nvp,
            cs,
            ch,
            bmin,
            bmax,
            verts,
            polys,
            flags,
            areas,
            detail_nmeshes,
            detail_nverts,
            detail_ntris,
            detail_meshes,
            detail_verts,
            detail_tris,
        })
    }

    /// Serialise this block as the Detour tile at grid position
    /// `(tile_x, tile_y)` — `dtCreateNavMeshData`, exactly as the C++
    /// reference `navigation.cpp` did for its one tile. `None` when Detour
    /// refuses the data (it validates `nvp` and the vertex count, not the
    /// polygon indices).
    pub fn build_tile(
        &self,
        agent: AgentParams,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<DetourTileData> {
        let mut nav_data: *mut u8 = std::ptr::null_mut();
        let mut nav_data_size: i32 = 0;

        // dtCreateNavMeshData tests `params->detailMeshes` for null to
        // decide whether to triangulate the polygons itself. An empty
        // Vec's pointer is dangling, not null, so a block without a detail
        // section must pass an explicit null or Detour reads through it.
        let (detail_meshes, detail_verts, detail_tris) = if self.detail_nmeshes == 0 {
            (std::ptr::null(), std::ptr::null(), std::ptr::null())
        } else {
            (
                self.detail_meshes.as_ptr(),
                self.detail_verts.as_ptr(),
                self.detail_tris.as_ptr(),
            )
        };

        // SAFETY: every pointer is into a Vec owned by `self` whose length
        // matches the count passed beside it (checked in `read`).
        let build_ok = unsafe {
            detour_ffi::detour_build_navmesh_data(
                self.verts.as_ptr(),
                self.nverts as i32,
                self.polys.as_ptr(),
                self.npolys as i32,
                self.nvp as i32,
                self.flags.as_ptr(),
                self.areas.as_ptr(),
                self.bmin.as_ptr(),
                self.bmax.as_ptr(),
                self.cs,
                self.ch,
                agent.height,
                agent.radius,
                agent.climb,
                detail_meshes,
                self.detail_nmeshes as i32,
                detail_verts,
                self.detail_nverts as i32,
                detail_tris,
                self.detail_ntris as i32,
                tile_x,
                tile_y,
                &mut nav_data,
                &mut nav_data_size,
            )
        };

        if build_ok == 0 || nav_data.is_null() {
            return None;
        }
        Some(DetourTileData {
            ptr: nav_data,
            len: nav_data_size,
        })
    }
}
