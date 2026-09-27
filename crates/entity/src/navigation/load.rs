//! XRC `.nav` parsing and Detour mesh construction — everything that runs
//! once, at space creation, to turn a file into a [`NavMesh`].
//!
//! Split out of `navigation/mod.rs`, which keeps the runtime query API.
//!
//! Two layouts, told apart by the first four bytes:
//!
//! - **Single mesh** — three agent floats and one poly-mesh block
//!   ([`super::poly_block`]). What NavBuilder has always written; loaded as
//!   a one-tile `dtNavMesh`.
//! - **Tiled** — the `XRCT` magic, a tile-grid header and one block per
//!   tile ([`super::load_tiled`]). Written by `NavBuilder tile=<cells>` for
//!   maps too big for one `rcPolyMesh`; every tile goes into one
//!   `dtNavMesh`, so queries cross tile borders the way they cross polygon
//!   edges.
//!
//! The parse streams: it reads through a [`HashingReader`] wrapped
//! around a `BufReader`, so every header count reaches
//! [`super::xrc::check_count`] after ~60 bytes have been read and long
//! before the allocation it would drive, while the [`NavMeshFingerprint`]'s
//! hash still covers every byte of the file. An earlier revision read the
//! whole file with `std::fs::read` to get bytes to hash; that put a
//! whole-file allocation *ahead* of the hostile-header caps, so a
//! corrupt or sparse deployment asset became a startup OOM instead of a
//! rejected file (PR #700 review).

use std::ffi::c_void;
use std::fs::File;
use std::io::{BufReader, Read as IoRead};
use std::path::Path;

use crate::detour_ffi;

use super::fingerprint::{AgentParams, HashingReader, NavMeshFingerprint};
use super::load_tiled::{self, XRCT_MAGIC};
use super::poly_block::{BlockCaps, PolyMeshBlock};
use super::xrc::{check_file_size, read_f32, MAX_NPOLYS, MAX_NVERTS, MAX_NVP};
use super::NavMesh;

/// The caps a single-mesh block is read under.
const SINGLE_CAPS: BlockCaps = BlockCaps {
    nverts: MAX_NVERTS,
    npolys: MAX_NPOLYS,
    nvp: MAX_NVP,
};

/// An initialised `dtNavMesh`, freed on drop unless handed to the
/// [`NavMesh`] with [`OwnedDetourMesh::into_raw`].
pub(super) struct OwnedDetourMesh(*mut c_void);

impl OwnedDetourMesh {
    /// Wrap a handle; `None` when it is null.
    pub fn new(handle: *mut c_void) -> Option<Self> {
        (!handle.is_null()).then_some(Self(handle))
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }

    fn into_raw(self) -> *mut c_void {
        let handle = self.0;
        std::mem::forget(self);
        handle
    }
}

impl Drop for OwnedDetourMesh {
    fn drop(&mut self) {
        // SAFETY: the handle came from detour_create_navmesh or
        // detour_create_tiled_navmesh and is freed exactly once.
        unsafe { detour_ffi::detour_free_navmesh(self.0) }
    }
}

/// What either layout's parser hands back: the populated mesh plus the
/// totals the fingerprint and the load line report.
pub(super) struct BuiltMesh {
    pub mesh: OwnedDetourMesh,
    pub agent: AgentParams,
    pub tiles: u32,
    pub nverts: u32,
    pub npolys: u32,
    pub nvp: u32,
    pub detail_nmeshes: u32,
    pub detail_nverts: u32,
    pub detail_ntris: u32,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
}

pub(super) fn build_failed(name: &str, what: &str) -> cimmeria_common::CimmeriaError {
    cimmeria_common::CimmeriaError::Entity(format!("Failed to {what} for '{name}'"))
}

impl NavMesh {
    /// Load a navigation mesh from an XRC-format `.nav` file, single-mesh
    /// or tiled.
    ///
    /// Parses the XRC binary, builds the Detour tile(s), and initializes
    /// a query object. The single-mesh path follows the C++ reference
    /// implementation in `navigation.cpp` exactly.
    ///
    /// Rejection is ordered by how little it costs to decide:
    ///
    /// 1. The `metadata` length against [`check_file_size`] — no bytes
    ///    read, the file is not even opened.
    /// 2. Each header count against its `MAX_*` cap, ~60 bytes in (or
    ///    ~60 bytes into its tile), before the `Vec` it sizes exists.
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

        // A single-mesh file starts with agent_height as an f32. "XRCT"
        // read that way is ~3.4e12, which no agent is, so the magic is
        // unambiguous.
        let mut head = [0u8; 4];
        r.read_exact(&mut head)?;
        let built = if head == XRCT_MAGIC {
            load_tiled::read_tiled(&mut r, &name)?
        } else {
            read_single(&mut r, f32::from_le_bytes(head), &name)?
        };

        // Nothing else is parsed out of the file; drain whatever is left
        // so the fingerprint covers trailing bytes too.
        let (content_hash, file_bytes) = r.finish()?;

        // ── Init dtNavMeshQuery (2048 nodes, matching C++ reference) ────
        // SAFETY: the mesh handle is live; the query keeps a pointer to it
        // and is freed before it (NavMesh's Drop).
        let query_handle = unsafe { detour_ffi::detour_create_query(built.mesh.as_ptr(), 2048) };
        if query_handle.is_null() {
            return Err(build_failed(&name, "create Detour navmesh query"));
        }

        let fingerprint = NavMeshFingerprint::new(
            path,
            file_bytes,
            content_hash,
            built.nverts,
            built.npolys,
            built.tiles,
            built.agent,
        );

        tracing::info!(
            name = %name,
            tiles = built.tiles,
            nverts = built.nverts,
            npolys = built.npolys,
            nvp = built.nvp,
            detail_nmeshes = built.detail_nmeshes,
            detail_nverts = built.detail_nverts,
            detail_ntris = built.detail_ntris,
            agent_height = built.agent.height,
            agent_climb = built.agent.climb,
            agent_radius = built.agent.radius,
            file_bytes = fingerprint.file_bytes,
            navmesh_hash = %fingerprint.content_hash,
            navmesh_short_hash = %fingerprint.short_hash,
            "NavMesh loaded via Detour FFI"
        );

        Ok(NavMesh {
            query: query_handle,
            mesh: built.mesh.into_raw(),
            name,
            fingerprint,
            agent_height: built.agent.height,
            agent_radius: built.agent.radius,
            bmin: built.bmin,
            bmax: built.bmax,
        })
    }
}

/// The single-mesh layout, from after the first four bytes (which were
/// `agent_height`).
fn read_single(
    r: &mut impl IoRead,
    agent_height: f32,
    name: &str,
) -> cimmeria_common::Result<BuiltMesh> {
    // ── Section 1: Agent parameters ─────────────────────────────────────
    let agent = AgentParams {
        height: agent_height,
        climb: read_f32(r)?,
        radius: read_f32(r)?,
    };

    // ── Sections 2-12: the one poly-mesh block ──────────────────────────
    let block = PolyMeshBlock::read(r, SINGLE_CAPS)?;

    // ── Build the Detour tile and a one-tile dtNavMesh ──────────────────
    // This mirrors the C++ navigation.cpp lines 109-138 exactly:
    // populate dtNavMeshCreateParams and call dtCreateNavMeshData.
    let data = block
        .build_tile(agent, 0, 0)
        .ok_or_else(|| build_failed(name, "build Detour navmesh data"))?;
    // SAFETY: `data` is a live dtCreateNavMeshData buffer; the call copies it.
    let handle = unsafe { detour_ffi::detour_create_navmesh(data.as_ptr(), data.len()) };
    let mesh =
        OwnedDetourMesh::new(handle).ok_or_else(|| build_failed(name, "create Detour navmesh"))?;

    Ok(BuiltMesh {
        mesh,
        agent,
        tiles: 1,
        nverts: block.nverts,
        npolys: block.npolys,
        nvp: block.nvp,
        detail_nmeshes: block.detail_nmeshes,
        detail_nverts: block.detail_nverts,
        detail_ntris: block.detail_ntris,
        bmin: block.bmin,
        bmax: block.bmax,
    })
}
