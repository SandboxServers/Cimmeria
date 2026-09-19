//! Chunk-directory scan and OBJ ingest for [`SlabSet`].
//!
//! Split out of `mod.rs` at the I/O seam: everything in the parent
//! module is pure geometry over triangles that are already in memory,
//! and everything here is about *finding* those triangles — which files
//! can possibly contribute, and how a `v`/`f` line becomes a [`Tri`] in
//! BigWorld metres.
//!
//! # One OBJ parser, not two
//!
//! The reader is [`crate::obj::read_obj_from`], the same one the floor
//! probe uses. An earlier revision of this module hand-rolled a second
//! parser that dropped any line it could not understand; because OBJ
//! face indices are *declaration positions*, dropping one malformed `v`
//! silently shifts every later index by one and the tool then measures
//! geometry that does not exist. `read_obj_from` errors with the line
//! number instead.

use std::io::BufReader;
use std::path::{Path, PathBuf};

use super::{Slab, Tri};
use crate::chunk_id::ChunkId;

/// Centimetres per BigWorld unit — the `/ 100` in NavBuilder's `loadOBJ`.
const CM_PER_BW_UNIT: f32 = 100.0;

/// One OBJ `v` line's three columns → BigWorld metres.
///
/// A chunk OBJ carries UE3 centimetres with Y and Z swapped
/// (`v <ue.X> <ue.Z> <ue.Y>`), and `Mesh::loadOBJ` (`mesh.cpp:106-108`)
/// reads `bw = (col2, col1, col0) / 100`. Doing the same here is what
/// lets a measurement be compared with a `.nav` coordinate directly.
pub fn obj_to_bw(v: [f32; 3]) -> [f32; 3] {
    [
        v[2] / CM_PER_BW_UNIT,
        v[1] / CM_PER_BW_UNIT,
        v[0] / CM_PER_BW_UNIT,
    ]
}

/// A batch of boxes, filled in one pass over the chunk directory.
#[derive(Debug)]
pub struct SlabSet {
    pub slabs: Vec<Slab>,
    /// Extra metres a chunk's grid cell is grown by before deciding it cannot
    /// touch any box. Actors are placed in the chunk that owns them but their
    /// meshes overhang.
    pub margin: f32,
    /// Chunk stems actually opened.
    pub chunks_read: Vec<String>,
    pub chunks_skipped: usize,
}

impl SlabSet {
    pub fn new(slabs: Vec<Slab>) -> Self {
        Self {
            slabs,
            margin: 60.0,
            chunks_read: Vec::new(),
            chunks_skipped: 0,
        }
    }

    /// Read every `<hex8>o.obj` in `dir` that could touch a box, and fill the
    /// slabs.
    ///
    /// A malformed OBJ is an error, not a partial read: this tool's
    /// output is used as evidence about what is in the map, and
    /// "silently measured the wrong triangles" is worse than "refused to
    /// measure".
    pub fn load(&mut self, dir: &Path) -> crate::Result<()> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("obj"))
            .collect();
        files.sort();

        for path in files {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            if !self.chunk_may_touch(&stem) {
                self.chunks_skipped += 1;
                continue;
            }
            self.chunks_read.push(stem.clone());
            self.read_obj(&path, &stem)?;
        }
        Ok(())
    }

    /// Chunk-grid rejection test. Unparseable stems are always read — better
    /// slow than silently missing geometry.
    fn chunk_may_touch(&self, stem: &str) -> bool {
        let Some(hex) = stem.strip_suffix('o') else {
            return true;
        };
        let Ok(raw) = u32::from_str_radix(hex, 16) else {
            return true;
        };
        let id = ChunkId::from_raw(raw);
        // Low u16 → BW x index, high u16 → BW z index, 100 m per cell.
        let x0 = id.position_x() as f32 * 100.0 - self.margin;
        let x1 = (id.position_x() as f32 + 1.0) * 100.0 + self.margin;
        let z0 = id.position_z() as f32 * 100.0 - self.margin;
        let z1 = (id.position_z() as f32 + 1.0) * 100.0 + self.margin;
        self.slabs
            .iter()
            .any(|s| s.bmax[0] >= x0 && s.bmin[0] <= x1 && s.bmax[2] >= z0 && s.bmin[2] <= z1)
    }

    fn read_obj(&mut self, path: &Path, stem: &str) -> crate::Result<()> {
        let file = std::fs::File::open(path)?;
        let soup = crate::obj::read_obj_from(BufReader::with_capacity(1 << 20, file))
            .map_err(|e| crate::ExtractError::Other(format!("{}: {e}", path.display())))?;

        for t in soup.triangles_in(0..soup.triangle_count()) {
            let tri = Tri {
                v: [obj_to_bw(t[0]), obj_to_bw(t[1]), obj_to_bw(t[2])],
            };
            for slab in &mut self.slabs {
                if tri.overlaps(slab.bmin, slab.bmax) {
                    slab.tris.push(tri);
                    *slab.by_chunk.entry(stem.to_string()).or_insert(0) += 1;
                }
            }
        }
        Ok(())
    }
}
