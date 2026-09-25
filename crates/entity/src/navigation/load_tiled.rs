//! The tiled `.nav` layout (`XRCT`), written by `NavBuilder tile=<cells>`.
//!
//! ```text
//! magic           4 bytes "XRCT"
//! version         u32 = 1
//! agent_height    f32
//! agent_climb     f32
//! agent_radius    f32
//! orig            3 × f32   dtNavMeshParams::orig (tile 0,0's min corner)
//! tile_width      f32       world metres along X
//! tile_height     f32       world metres along Z
//! ntiles          u32
//! max_tile_polys  u32       largest npolys of any tile
//! ntiles × {
//!     tile_x      i32
//!     tile_y      i32
//!     <poly-mesh block>     the single-mesh layout from `nverts` on
//! }
//! ```
//!
//! # Why this and not Detour's own `MSET` tile set
//!
//! RecastDemo's `MSET` file stores each tile as the raw bytes
//! `dtCreateNavMeshData` produced: Detour's in-memory struct layout, tied
//! to `DT_NAVMESH_VERSION` and the build's pointer size, and handed to
//! `dtNavMesh::addTile` with no validation beyond its magic and version.
//! Keeping the Recast arrays on disk instead means the tiles go through the
//! same capped, streaming [`PolyMeshBlock::read`] as a single mesh, and the
//! Detour serialisation stays an implementation detail of the loader.
//!
//! # The poly-ref budget
//!
//! A 32-bit `dtPolyRef` packs salt, tile and polygon index, and
//! `dtNavMesh::init` refuses fewer than 10 salt bits, so
//! `bits(ntiles) + bits(max_tile_polys) <= 22`. NavBuilder refuses to write
//! a mesh that breaks it; the loader checks again rather than trusting the
//! file.

use std::io::Read as IoRead;

use crate::detour_ffi::{self, dt_status_failed};

use super::fingerprint::AgentParams;
use super::load::{build_failed, BuiltMesh, OwnedDetourMesh};
use super::poly_block::{BlockCaps, PolyMeshBlock};
use super::xrc::{check_count, read_f32, read_i32, read_u32, reject, MAX_NVP};

/// First four bytes of a tiled `.nav`.
pub(super) const XRCT_MAGIC: [u8; 4] = *b"XRCT";

/// The only version this loader understands.
const XRCT_VERSION: u32 = 1;

/// Upper bound on `ntiles`. The poly-ref budget already limits a usable
/// mesh to `2^22` tiles of one polygon each; this is the practical cap. A
/// whole outdoor map at `cs=0.3` in 128-cell tiles is a few thousand.
pub(super) const MAX_TILES: u32 = 1 << 16;

/// Per-tile caps. A tile is one `rcPolyMesh`, whose vertex and polygon
/// indices are `u16` with `0xffff` reserved, and Detour's
/// `DT_VERTS_PER_POLYGON` is 6.
const TILE_CAPS: BlockCaps = BlockCaps {
    nverts: 0xfffe,
    npolys: 0xfffe,
    nvp: if MAX_NVP < 6 { MAX_NVP } else { 6 },
};

/// Bits a 32-bit `dtPolyRef` has for tile and polygon index together.
const POLY_REF_INDEX_BITS: u32 = 22;

/// `dtIlog2(dtNextPow2(v))`, as `dtNavMesh::init` computes it.
fn index_bits(v: u32) -> u32 {
    v.max(1).next_power_of_two().trailing_zeros()
}

/// Everything after the magic.
pub(super) fn read_tiled(r: &mut impl IoRead, name: &str) -> cimmeria_common::Result<BuiltMesh> {
    let version = read_u32(r)?;
    if version != XRCT_VERSION {
        return Err(reject(
            "version",
            version as u64,
            "unknown tiled .nav version",
        ));
    }

    let agent = AgentParams {
        height: read_f32(r)?,
        climb: read_f32(r)?,
        radius: read_f32(r)?,
    };
    let orig = [read_f32(r)?, read_f32(r)?, read_f32(r)?];
    let tile_width = read_f32(r)?;
    let tile_height = read_f32(r)?;
    if !(tile_width.is_finite() && tile_width > 0.0 && tile_height.is_finite() && tile_height > 0.0)
        || !orig.iter().all(|v| v.is_finite())
    {
        return Err(reject(
            "tile_width",
            tile_width.to_bits() as u64,
            "tile size must be finite and positive, origin finite",
        ));
    }

    let ntiles = check_count(read_u32(r)?, MAX_TILES, "ntiles")?;
    if ntiles == 0 {
        return Err(reject("ntiles", 0, "a tiled .nav needs at least one tile"));
    }
    let max_tile_polys = check_count(read_u32(r)?, TILE_CAPS.npolys, "max_tile_polys")?;
    if max_tile_polys == 0 {
        return Err(reject(
            "max_tile_polys",
            0,
            "a tiled .nav needs at least one polygon",
        ));
    }
    let bits = index_bits(ntiles) + index_bits(max_tile_polys);
    if bits > POLY_REF_INDEX_BITS {
        return Err(reject(
            "ntiles",
            ntiles as u64,
            "tile + polygon index bits exceed the 22 a 32-bit dtPolyRef allows",
        ));
    }

    // SAFETY: `orig` is three live floats; the call only reads them.
    let handle = unsafe {
        detour_ffi::detour_create_tiled_navmesh(
            orig.as_ptr(),
            tile_width,
            tile_height,
            ntiles as i32,
            max_tile_polys as i32,
        )
    };
    let mesh = OwnedDetourMesh::new(handle)
        .ok_or_else(|| build_failed(name, "create tiled Detour navmesh"))?;

    let mut built = BuiltMesh {
        mesh,
        agent,
        tiles: ntiles,
        nverts: 0,
        npolys: 0,
        nvp: 0,
        detail_nmeshes: 0,
        detail_nverts: 0,
        detail_ntris: 0,
        bmin: [f32::INFINITY; 3],
        bmax: [f32::NEG_INFINITY; 3],
    };

    // dtNavMesh sized its polygon index for `max_tile_polys`, so a tile
    // claiming more is rejected at its header, before its arrays are read.
    let caps = BlockCaps {
        npolys: max_tile_polys,
        ..TILE_CAPS
    };
    for _ in 0..ntiles {
        let tile_x = read_i32(r)?;
        let tile_y = read_i32(r)?;
        let block = PolyMeshBlock::read(r, caps)?;

        let data = block
            .build_tile(agent, tile_x, tile_y)
            .ok_or_else(|| build_failed(name, "build Detour tile data"))?;
        // SAFETY: both pointers are live; addTile copies `data`.
        let status =
            unsafe { detour_ffi::detour_add_tile(built.mesh.as_ptr(), data.as_ptr(), data.len()) };
        if dt_status_failed(status) {
            tracing::error!(
                target: "navmesh.load",
                name = %name,
                tile_x,
                tile_y,
                status,
                reason = "add_tile_failed",
                "Detour refused a tile (duplicate position or pool full) -- space will be navmesh-less"
            );
            return Err(build_failed(name, "add a Detour tile"));
        }

        built.nverts = built.nverts.saturating_add(block.nverts);
        built.npolys = built.npolys.saturating_add(block.npolys);
        built.nvp = built.nvp.max(block.nvp);
        built.detail_nmeshes = built.detail_nmeshes.saturating_add(block.detail_nmeshes);
        built.detail_nverts = built.detail_nverts.saturating_add(block.detail_nverts);
        built.detail_ntris = built.detail_ntris.saturating_add(block.detail_ntris);
        for k in 0..3 {
            built.bmin[k] = built.bmin[k].min(block.bmin[k]);
            built.bmax[k] = built.bmax[k].max(block.bmax[k]);
        }
    }

    Ok(built)
}
