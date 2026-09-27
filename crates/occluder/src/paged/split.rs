//! Cutting a built [`Occluder`] into square pages, one self-contained
//! occluder per page.

use crate::build::BuildError;
use crate::grid::{
    Layer, Occluder, TileHead, SLOT_EMPTY, SLOT_UNCOVERED, TILE, TILE_CELLS, VARIABLE,
};
use crate::heightfield::{Heightfield, TILE_VERTS};

use super::PageGrid;

/// Tiles of edge `tile_w` per page of edge `page`, when it divides evenly.
fn tiles_per_page(page: f32, tile_w: f32) -> Result<i64, BuildError> {
    let n = page / tile_w;
    if n < 1.0 || (n - n.round()).abs() > 1e-4 {
        return Err(BuildError::BadParams(
            "page size must be a whole multiple of every layer's tile width",
        ));
    }
    Ok(n.round() as i64)
}

/// The world rectangle `(min, max)` over every layer.
fn world_bounds(occ: &Occluder) -> Option<([f32; 2], [f32; 2])> {
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    let mut grow = |a: [f32; 2], b: [f32; 2]| {
        lo = [lo[0].min(a[0]), lo[1].min(a[1])];
        hi = [hi[0].max(b[0]), hi[1].max(b[1])];
    };
    for l in &occ.layers {
        let (a, b) = l.bounds();
        grow(a, b);
    }
    if let Some(h) = &occ.heightfield {
        let w = h.tiles_x as f32 * TILE as f32 * h.pitch;
        let d = h.tiles_z as f32 * TILE as f32 * h.pitch;
        grow(h.origin, [h.origin[0] + w, h.origin[1] + d]);
    }
    lo[0].is_finite().then_some((lo, hi))
}

/// Split `occ` into pages of edge `page` metres. Pages with no covered tile
/// in any layer are left out.
pub(crate) fn split(
    occ: &Occluder,
    page: f32,
) -> Result<(PageGrid, Vec<(u32, Occluder)>), BuildError> {
    if !(page.is_finite() && page > 0.0) {
        return Err(BuildError::BadParams("page size must be > 0"));
    }
    for l in &occ.layers {
        tiles_per_page(page, l.cell * TILE as f32)?;
    }
    if let Some(h) = &occ.heightfield {
        tiles_per_page(page, h.pitch * TILE as f32)?;
    }
    let (lo, hi) = world_bounds(occ).ok_or(BuildError::Empty)?;
    let px0 = (lo[0] / page).floor() as i64;
    let pz0 = (lo[1] / page).floor() as i64;
    let px1 = ((hi[0] / page).ceil() as i64).max(px0 + 1);
    let pz1 = ((hi[1] / page).ceil() as i64).max(pz0 + 1);
    let grid = PageGrid {
        size: page,
        px0: px0 as i32,
        pz0: pz0 as i32,
        nx: (px1 - px0) as u32,
        nz: (pz1 - pz0) as u32,
    };
    let mut pages = Vec::new();
    for pz in pz0..pz1 {
        for px in px0..px1 {
            let x0 = px as f32 * page;
            let z0 = pz as f32 * page;
            let layers: Vec<Layer> = occ
                .layers
                .iter()
                .filter_map(|l| layer_window(l, x0, z0, page))
                .collect();
            let heightfield = occ
                .heightfield
                .as_ref()
                .and_then(|h| heightfield_window(h, x0, z0, page));
            if layers.is_empty() && heightfield.is_none() {
                continue;
            }
            let idx = (pz - pz0) as u32 * grid.nx + (px - px0) as u32;
            pages.push((
                idx,
                Occluder {
                    y_base: occ.y_base,
                    y_step: occ.y_step,
                    source_hash: occ.source_hash,
                    label: String::new(),
                    layers,
                    heightfield,
                    content_hash: 0,
                    short_hash: String::new(),
                },
            ));
        }
    }
    Ok((grid, pages))
}

/// The tile range `[a, b)` of a layer starting at `origin` with tiles of
/// `tile_w`, that the page `[x0, x0 + page)` covers, clamped to `n`.
fn window(origin: f32, tile_w: f32, x0: f32, page: f32, n: u32) -> Option<(i64, i64)> {
    let a = ((x0 - origin) / tile_w).round() as i64;
    let b = a + (page / tile_w).round() as i64;
    let (a, b) = (a.max(0), b.min(n as i64));
    (a < b).then_some((a, b))
}

fn layer_window(l: &Layer, x0: f32, z0: f32, page: f32) -> Option<Layer> {
    let tile_w = l.cell * TILE as f32;
    let (tx0, tx1) = window(l.origin[0], tile_w, x0, page, l.tiles_x)?;
    let (tz0, tz1) = window(l.origin[1], tile_w, z0, page, l.tiles_z)?;
    let mut out = Layer {
        kind: l.kind,
        cell: l.cell,
        origin: [
            l.origin[0] + tx0 as f32 * tile_w,
            l.origin[1] + tz0 as f32 * tile_w,
        ],
        tiles_x: (tx1 - tx0) as u32,
        tiles_z: (tz1 - tz0) as u32,
        slots: Vec::new(),
        heads: Vec::new(),
        cell_end: Vec::new(),
        spans: Vec::new(),
    };
    let mut covered = false;
    for tz in tz0..tz1 {
        for tx in tx0..tx1 {
            let s = l.slots[(tz * l.tiles_x as i64 + tx) as usize];
            if s == SLOT_UNCOVERED || s == SLOT_EMPTY {
                covered |= s == SLOT_EMPTY;
                out.slots.push(s);
                continue;
            }
            covered = true;
            let head = l.heads[s as usize];
            let base = head.span_base as usize;
            let count = if head.uniform != VARIABLE {
                head.uniform as usize * TILE_CELLS
            } else {
                l.cell_end[head.ends as usize + TILE_CELLS - 1] as usize
            };
            let mut nh = TileHead {
                span_base: out.spans.len() as u32,
                uniform: head.uniform,
                ends: 0,
            };
            if head.uniform == VARIABLE {
                nh.ends = out.cell_end.len() as u32;
                out.cell_end.extend_from_slice(
                    &l.cell_end[head.ends as usize..head.ends as usize + TILE_CELLS],
                );
            }
            out.spans.extend_from_slice(&l.spans[base..base + count]);
            out.slots.push(out.heads.len() as u32);
            out.heads.push(nh);
        }
    }
    covered.then_some(out)
}

fn heightfield_window(h: &Heightfield, x0: f32, z0: f32, page: f32) -> Option<Heightfield> {
    let tile_w = h.pitch * TILE as f32;
    let (tx0, tx1) = window(h.origin[0], tile_w, x0, page, h.tiles_x)?;
    let (tz0, tz1) = window(h.origin[1], tile_w, z0, page, h.tiles_z)?;
    let mut out = Heightfield {
        pitch: h.pitch,
        origin: [
            h.origin[0] + tx0 as f32 * tile_w,
            h.origin[1] + tz0 as f32 * tile_w,
        ],
        tiles_x: (tx1 - tx0) as u32,
        tiles_z: (tz1 - tz0) as u32,
        slots: Vec::new(),
        base: Vec::new(),
        heights: Vec::new(),
        flags: Vec::new(),
    };
    let mut covered = false;
    for tz in tz0..tz1 {
        for tx in tx0..tx1 {
            let s = h.slots[(tz * h.tiles_x as i64 + tx) as usize];
            if s == SLOT_UNCOVERED || s == SLOT_EMPTY {
                covered |= s == SLOT_EMPTY;
                out.slots.push(s);
                continue;
            }
            covered = true;
            let s = s as usize;
            out.slots.push(out.base.len() as u32);
            out.base.push(h.base[s]);
            out.heights
                .extend_from_slice(&h.heights[s * TILE_VERTS..(s + 1) * TILE_VERTS]);
            out.flags
                .extend_from_slice(&h.flags[s * TILE_CELLS..(s + 1) * TILE_CELLS]);
        }
    }
    covered.then_some(out)
}
