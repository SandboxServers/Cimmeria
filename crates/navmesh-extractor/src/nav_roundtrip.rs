//! Phase 0 round-trip smoke for the XRC `.nav` format.
//!
//! Reads a `.nav` file produced by the C++ NavBuilder
//! (`deprecated/cpp/src/nav_builder/builder.cpp::xrcSavePolyMesh`) and
//! re-emits it. A byte-exact match against the source proves the
//! format is fully understood and that any future Rust replacement
//! for NavBuilder can write the same wire shape.
//!
//! The layout matches `crates/entity/src/navigation.rs::NavMesh::load`
//! exactly:
//!
//! ```text
//! agent_height       f32
//! agent_climb        f32
//! agent_radius       f32
//! nverts             u32
//! npolys             u32
//! nvp                u32
//! border_size        u32
//! cs                 f32
//! ch                 f32
//! bmin               3 × f32
//! bmax               3 × f32
//! verts              nverts × 3 × u16
//! polys              npolys × nvp × 2 × u16
//! regs               npolys × u16
//! flags              npolys × u16
//! areas              npolys × u8
//! detail_nmeshes     u32
//! detail_nverts      u32
//! detail_ntris       u32
//! detail_meshes      detail_nmeshes × 4 × u32
//! detail_verts       detail_nverts × 3 × f32
//! detail_tris        detail_ntris × 4 × u8
//! ```
//!
//! All scalars are little-endian; there is no padding between sections.

use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::ExtractError;

/// In-memory representation of a parsed XRC `.nav` file.
///
/// Field ordering deliberately mirrors the on-disk layout so a writer
/// can stream out the struct top-to-bottom without conditional logic.
#[derive(Debug, Clone)]
pub struct XrcNav {
    pub agent_height: f32,
    pub agent_climb: f32,
    pub agent_radius: f32,
    pub nverts: u32,
    pub npolys: u32,
    pub nvp: u32,
    pub border_size: u32,
    pub cs: f32,
    pub ch: f32,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    pub verts: Vec<u16>,
    pub polys: Vec<u16>,
    pub regs: Vec<u16>,
    pub flags: Vec<u16>,
    pub areas: Vec<u8>,
    pub detail_nmeshes: u32,
    pub detail_nverts: u32,
    pub detail_ntris: u32,
    pub detail_meshes: Vec<u32>,
    pub detail_verts: Vec<f32>,
    pub detail_tris: Vec<u8>,
}

impl XrcNav {
    /// Read the entire `.nav` file from any seekable reader. Returns
    /// an error if the stream is truncated, malformed, or has trailing
    /// bytes after the documented sections (anything past `detail_tris`
    /// would indicate the format has been extended without our knowing).
    pub fn read<R: Read + Seek>(r: &mut R) -> crate::Result<Self> {
        let agent_height = r.read_f32::<LittleEndian>()?;
        let agent_climb = r.read_f32::<LittleEndian>()?;
        let agent_radius = r.read_f32::<LittleEndian>()?;

        let nverts = r.read_u32::<LittleEndian>()?;
        let npolys = r.read_u32::<LittleEndian>()?;
        let nvp = r.read_u32::<LittleEndian>()?;
        let border_size = r.read_u32::<LittleEndian>()?;

        let cs = r.read_f32::<LittleEndian>()?;
        let ch = r.read_f32::<LittleEndian>()?;
        let bmin = [
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
        ];
        let bmax = [
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
        ];

        let mut verts = vec![0u16; (nverts * 3) as usize];
        for v in &mut verts {
            *v = r.read_u16::<LittleEndian>()?;
        }
        let mut polys = vec![0u16; (npolys * nvp * 2) as usize];
        for v in &mut polys {
            *v = r.read_u16::<LittleEndian>()?;
        }
        let mut regs = vec![0u16; npolys as usize];
        for v in &mut regs {
            *v = r.read_u16::<LittleEndian>()?;
        }
        let mut flags = vec![0u16; npolys as usize];
        for v in &mut flags {
            *v = r.read_u16::<LittleEndian>()?;
        }
        let mut areas = vec![0u8; npolys as usize];
        r.read_exact(&mut areas)?;

        let detail_nmeshes = r.read_u32::<LittleEndian>()?;
        let detail_nverts = r.read_u32::<LittleEndian>()?;
        let detail_ntris = r.read_u32::<LittleEndian>()?;

        let mut detail_meshes = vec![0u32; (detail_nmeshes * 4) as usize];
        for v in &mut detail_meshes {
            *v = r.read_u32::<LittleEndian>()?;
        }
        let mut detail_verts = vec![0.0f32; (detail_nverts * 3) as usize];
        for v in &mut detail_verts {
            *v = r.read_f32::<LittleEndian>()?;
        }
        let mut detail_tris = vec![0u8; (detail_ntris * 4) as usize];
        r.read_exact(&mut detail_tris)?;

        // Confirm no trailing bytes — would indicate a format extension
        // we haven't accounted for. The C++ writer emits exactly the
        // sections above and then stops.
        let pos = r.stream_position()?;
        let end = r.seek(SeekFrom::End(0))?;
        if pos != end {
            return Err(ExtractError::Other(format!(
                "trailing bytes after detail_tris: {} bytes",
                end - pos
            )));
        }

        Ok(Self {
            agent_height,
            agent_climb,
            agent_radius,
            nverts,
            npolys,
            nvp,
            border_size,
            cs,
            ch,
            bmin,
            bmax,
            verts,
            polys,
            regs,
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

    /// Emit the parsed nav back to a byte sink. The output is
    /// byte-exact with the C++ `xrcSavePolyMesh` order — see the
    /// module-level layout doc.
    pub fn write<W: Write>(&self, w: &mut W) -> crate::Result<()> {
        w.write_f32::<LittleEndian>(self.agent_height)?;
        w.write_f32::<LittleEndian>(self.agent_climb)?;
        w.write_f32::<LittleEndian>(self.agent_radius)?;
        w.write_u32::<LittleEndian>(self.nverts)?;
        w.write_u32::<LittleEndian>(self.npolys)?;
        w.write_u32::<LittleEndian>(self.nvp)?;
        w.write_u32::<LittleEndian>(self.border_size)?;
        w.write_f32::<LittleEndian>(self.cs)?;
        w.write_f32::<LittleEndian>(self.ch)?;
        for v in &self.bmin {
            w.write_f32::<LittleEndian>(*v)?;
        }
        for v in &self.bmax {
            w.write_f32::<LittleEndian>(*v)?;
        }
        for v in &self.verts {
            w.write_u16::<LittleEndian>(*v)?;
        }
        for v in &self.polys {
            w.write_u16::<LittleEndian>(*v)?;
        }
        for v in &self.regs {
            w.write_u16::<LittleEndian>(*v)?;
        }
        for v in &self.flags {
            w.write_u16::<LittleEndian>(*v)?;
        }
        w.write_all(&self.areas)?;
        w.write_u32::<LittleEndian>(self.detail_nmeshes)?;
        w.write_u32::<LittleEndian>(self.detail_nverts)?;
        w.write_u32::<LittleEndian>(self.detail_ntris)?;
        for v in &self.detail_meshes {
            w.write_u32::<LittleEndian>(*v)?;
        }
        for v in &self.detail_verts {
            w.write_f32::<LittleEndian>(*v)?;
        }
        w.write_all(&self.detail_tris)?;
        Ok(())
    }

    /// Convenience: round-trip a `.nav` file in memory and assert
    /// that the re-emitted bytes match the source byte-for-byte.
    /// Returns the parsed [`XrcNav`] on success; otherwise yields a
    /// [`ExtractError::RoundTripMismatch`] or
    /// [`ExtractError::RoundTripSizeMismatch`] pointing at the first
    /// diverging byte.
    pub fn round_trip(bytes: &[u8]) -> crate::Result<Self> {
        let mut cursor = std::io::Cursor::new(bytes);
        let parsed = XrcNav::read(&mut cursor)?;

        let mut emitted = Vec::with_capacity(bytes.len());
        parsed.write(&mut emitted)?;

        if emitted.len() != bytes.len() {
            return Err(ExtractError::RoundTripSizeMismatch {
                original: bytes.len(),
                reemitted: emitted.len(),
            });
        }
        for (i, (a, b)) in bytes.iter().zip(emitted.iter()).enumerate() {
            if a != b {
                return Err(ExtractError::RoundTripMismatch {
                    offset: i,
                    original: *a,
                    reemitted: *b,
                });
            }
        }
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Construct a minimal valid XRC nav with one vertex / no polys —
    /// exercises every section without needing the real Castle fixture.
    fn synthesise_minimal() -> XrcNav {
        XrcNav {
            agent_height: 0.6,
            agent_climb: 0.9,
            agent_radius: 0.6,
            nverts: 1,
            npolys: 0,
            nvp: 6,
            border_size: 0,
            cs: 0.3,
            ch: 0.2,
            bmin: [-1.0, -1.0, -1.0],
            bmax: [1.0, 1.0, 1.0],
            verts: vec![0, 0, 0],
            polys: vec![],
            regs: vec![],
            flags: vec![],
            areas: vec![],
            detail_nmeshes: 0,
            detail_nverts: 0,
            detail_ntris: 0,
            detail_meshes: vec![],
            detail_verts: vec![],
            detail_tris: vec![],
        }
    }

    #[test]
    fn synthetic_nav_roundtrips_byte_exact() {
        let nav = synthesise_minimal();
        let mut buf = Vec::new();
        nav.write(&mut buf).unwrap();

        // Reading should consume everything.
        let mut cursor = Cursor::new(&buf);
        let reparsed = XrcNav::read(&mut cursor).unwrap();
        assert_eq!(reparsed.nverts, 1);
        assert_eq!(reparsed.agent_height, 0.6);

        // Round-trip helper.
        let parsed = XrcNav::round_trip(&buf).expect("round-trip mismatch");
        assert_eq!(parsed.nverts, 1);
    }

    #[test]
    fn trailing_bytes_after_detail_tris_is_an_error() {
        let nav = synthesise_minimal();
        let mut buf = Vec::new();
        nav.write(&mut buf).unwrap();
        buf.push(0xAA); // tail garbage
        let mut cursor = Cursor::new(&buf);
        assert!(XrcNav::read(&mut cursor).is_err());
    }

    #[test]
    fn truncated_input_is_an_error() {
        let nav = synthesise_minimal();
        let mut buf = Vec::new();
        nav.write(&mut buf).unwrap();
        buf.truncate(buf.len() - 4); // chop the final detail_tris vec entry
        let mut cursor = Cursor::new(&buf);
        assert!(XrcNav::read(&mut cursor).is_err());
    }
}
