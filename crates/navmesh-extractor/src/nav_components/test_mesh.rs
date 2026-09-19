//! Synthetic `XrcNav` fixtures for the connectivity and gap tests.
//!
//! Adjacency is supplied explicitly rather than re-derived from the geometry,
//! so the tests exercise the decode path instead of a second copy of the
//! thing under test.

use crate::nav_roundtrip::XrcNav;

/// Builds a poly mesh out of axis-aligned quads. `nvp` is fixed at 4, so each
/// polygon is `[v0 v1 v2 v3 n0 n1 n2 n3]`.
///
/// With the default `cs = ch = 1.0` and `bmin = 0`, grid coordinates are world
/// metres; lower `cs`/`ch` to author sub-metre gaps and steps.
pub struct MeshFixture {
    pub cs: f32,
    pub ch: f32,
    pub verts: Vec<[u16; 3]>,
    pub polys: Vec<[u16; 8]>,
}

impl MeshFixture {
    pub fn new() -> Self {
        Self {
            cs: 1.0,
            ch: 1.0,
            verts: Vec::new(),
            polys: Vec::new(),
        }
    }

    pub fn vert(&mut self, x: u16, y: u16, z: u16) -> u16 {
        self.verts.push([x, y, z]);
        (self.verts.len() - 1) as u16
    }

    /// Unit quad with its lower corner at grid `(x, z)`, at grid height `y`.
    pub fn quad(&mut self, x: u16, y: u16, z: u16) -> u16 {
        self.wide_quad(x, y, z, 1, 1)
    }

    /// `w` x `d` quad, lower corner at `(x, z)`, at grid height `y`.
    pub fn wide_quad(&mut self, x: u16, y: u16, z: u16, w: u16, d: u16) -> u16 {
        let a = self.vert(x, y, z);
        let b = self.vert(x + w, y, z);
        let c = self.vert(x + w, y, z + d);
        let e = self.vert(x, y, z + d);
        self.polys
            .push([a, b, c, e, 0xffff, 0xffff, 0xffff, 0xffff]);
        (self.polys.len() - 1) as u16
    }

    /// Link poly `a` edge `ea` to poly `b` edge `eb`, both directions.
    pub fn link(&mut self, a: u16, ea: usize, b: u16, eb: usize) {
        self.polys[a as usize][4 + ea] = b;
        self.polys[b as usize][4 + eb] = a;
    }

    pub fn build(self) -> XrcNav {
        let npolys = self.polys.len() as u32;
        XrcNav {
            agent_height: 0.6,
            agent_climb: 0.9,
            agent_radius: 0.6,
            nverts: self.verts.len() as u32,
            npolys,
            nvp: 4,
            border_size: 0,
            cs: self.cs,
            ch: self.ch,
            bmin: [0.0, 0.0, 0.0],
            bmax: [100.0, 100.0, 100.0],
            verts: self.verts.iter().flat_map(|v| v.iter().copied()).collect(),
            polys: self.polys.iter().flat_map(|p| p.iter().copied()).collect(),
            regs: vec![0; npolys as usize],
            flags: vec![1; npolys as usize],
            areas: vec![63; npolys as usize],
            detail_nmeshes: 0,
            detail_nverts: 0,
            detail_ntris: 0,
            detail_meshes: vec![],
            detail_verts: vec![],
            detail_tris: vec![],
        }
    }
}
