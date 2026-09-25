//! The tiled XRC `.nav` layout (`XRCT`), written by `NavBuilder tile=<cells>`,
//! and [`NavFile`], which reads either layout.
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
//! Each tile is a complete `rcPolyMesh` built with a border, so the edges
//! on the tile's sides carry Recast's portal marker (`0x8000 | dir`) in
//! their neighbour slot instead of a polygon index. The server's loader
//! (`crates/entity/src/navigation/load_tiled.rs`) hands every tile to one
//! `dtNavMesh` and Detour links the portals; [`crate::nav_components`]
//! links them the same way for the offline connectivity report.

use std::io::{Read, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::nav_roundtrip::{check_count, XrcNav};
use crate::ExtractError;

/// First four bytes of a tiled `.nav`.
pub const XRCT_MAGIC: [u8; 4] = *b"XRCT";
const XRCT_VERSION: u32 = 1;

/// Same caps as the runtime loader: tiles, and a tile's `u16` index space
/// and Detour's six vertices per polygon.
const MAX_TILES: u32 = 1 << 16;
const MAX_TILE_NVERTS: u32 = 0xfffe;
const MAX_TILE_NPOLYS: u32 = 0xfffe;
const MAX_TILE_NVP: u32 = 6;

/// One tile: its grid position and its poly mesh. The mesh's agent fields
/// are the file header's.
#[derive(Debug, Clone)]
pub struct XrcTile {
    pub tile_x: i32,
    pub tile_y: i32,
    pub mesh: XrcNav,
}

/// A parsed tiled `.nav`.
#[derive(Debug, Clone)]
pub struct XrcTiledNav {
    pub agent_height: f32,
    pub agent_climb: f32,
    pub agent_radius: f32,
    pub orig: [f32; 3],
    pub tile_width: f32,
    pub tile_height: f32,
    pub max_tile_polys: u32,
    pub tiles: Vec<XrcTile>,
}

impl XrcTiledNav {
    /// Read a tiled `.nav`, magic included. Errors on a bad magic or
    /// version, any count over its cap, a short read, or trailing bytes.
    pub fn read<R: Read + Seek>(r: &mut R) -> crate::Result<Self> {
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if magic != XRCT_MAGIC {
            return Err(ExtractError::Other(
                "not a tiled .nav (no XRCT magic)".into(),
            ));
        }
        let version = r.read_u32::<LittleEndian>()?;
        if version != XRCT_VERSION {
            return Err(ExtractError::NavHeaderOutOfRange {
                field: "version",
                value: version as u64,
                reason: "unknown tiled .nav version",
            });
        }
        let agent = [
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
        ];
        let orig = [
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
            r.read_f32::<LittleEndian>()?,
        ];
        let tile_width = r.read_f32::<LittleEndian>()?;
        let tile_height = r.read_f32::<LittleEndian>()?;
        let ntiles = check_count(r.read_u32::<LittleEndian>()?, MAX_TILES, "ntiles")?;
        let max_tile_polys = check_count(
            r.read_u32::<LittleEndian>()?,
            MAX_TILE_NPOLYS,
            "max_tile_polys",
        )?;

        let mut tiles = Vec::with_capacity(ntiles as usize);
        for _ in 0..ntiles {
            let tile_x = r.read_i32::<LittleEndian>()?;
            let tile_y = r.read_i32::<LittleEndian>()?;
            let mesh =
                XrcNav::read_block(r, agent, [MAX_TILE_NVERTS, max_tile_polys, MAX_TILE_NVP])?;
            tiles.push(XrcTile {
                tile_x,
                tile_y,
                mesh,
            });
        }

        let pos = r.stream_position()?;
        let end = r.seek(SeekFrom::End(0))?;
        if pos != end {
            return Err(ExtractError::Other(format!(
                "trailing bytes after the last tile: {} bytes",
                end - pos
            )));
        }

        Ok(Self {
            agent_height: agent[0],
            agent_climb: agent[1],
            agent_radius: agent[2],
            orig,
            tile_width,
            tile_height,
            max_tile_polys,
            tiles,
        })
    }

    /// Emit the file byte-exact with NavBuilder's `xrcSaveTiledMesh`.
    pub fn write<W: Write>(&self, w: &mut W) -> crate::Result<()> {
        w.write_all(&XRCT_MAGIC)?;
        w.write_u32::<LittleEndian>(XRCT_VERSION)?;
        for v in [self.agent_height, self.agent_climb, self.agent_radius] {
            w.write_f32::<LittleEndian>(v)?;
        }
        for v in self.orig {
            w.write_f32::<LittleEndian>(v)?;
        }
        w.write_f32::<LittleEndian>(self.tile_width)?;
        w.write_f32::<LittleEndian>(self.tile_height)?;
        w.write_u32::<LittleEndian>(self.tiles.len() as u32)?;
        w.write_u32::<LittleEndian>(self.max_tile_polys)?;
        for t in &self.tiles {
            w.write_i32::<LittleEndian>(t.tile_x)?;
            w.write_i32::<LittleEndian>(t.tile_y)?;
            t.mesh.write_block(w)?;
        }
        Ok(())
    }

    /// Total vertices / polygons over every tile.
    pub fn totals(&self) -> (u64, u64) {
        self.tiles.iter().fold((0, 0), |(v, p), t| {
            (v + t.mesh.nverts as u64, p + t.mesh.npolys as u64)
        })
    }
}

/// Either `.nav` layout, told apart by the first four bytes.
#[derive(Debug, Clone)]
pub enum NavFile {
    Single(XrcNav),
    Tiled(XrcTiledNav),
}

impl NavFile {
    /// Parse a whole `.nav` held in memory.
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        let mut cursor = std::io::Cursor::new(bytes);
        if bytes.len() >= 4 && bytes[..4] == XRCT_MAGIC {
            Ok(Self::Tiled(XrcTiledNav::read(&mut cursor)?))
        } else {
            Ok(Self::Single(XrcNav::read(&mut cursor)?))
        }
    }

    /// Agent height, climb, radius.
    pub fn agent(&self) -> [f32; 3] {
        match self {
            Self::Single(n) => [n.agent_height, n.agent_climb, n.agent_radius],
            Self::Tiled(t) => [t.agent_height, t.agent_climb, t.agent_radius],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav_components::test_mesh::MeshFixture;

    fn tile(x: i32) -> XrcTile {
        let mut f = MeshFixture::new();
        f.quad(0, 0, 0);
        let mesh = f.build();
        XrcTile {
            tile_x: x,
            tile_y: 0,
            mesh,
        }
    }

    fn two_tile_file() -> XrcTiledNav {
        let [a, b] = [tile(0), tile(1)];
        XrcTiledNav {
            agent_height: a.mesh.agent_height,
            agent_climb: a.mesh.agent_climb,
            agent_radius: a.mesh.agent_radius,
            orig: [0.0; 3],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tile_polys: 1,
            tiles: vec![a, b],
        }
    }

    #[test]
    fn a_tiled_file_round_trips_byte_exact() {
        let nav = two_tile_file();
        let mut bytes = Vec::new();
        nav.write(&mut bytes).unwrap();
        assert_eq!(&bytes[..4], b"XRCT");

        let NavFile::Tiled(back) = NavFile::from_bytes(&bytes).unwrap() else {
            panic!("tiled file read as single");
        };
        assert_eq!(back.tiles.len(), 2);
        assert_eq!(back.tiles[1].tile_x, 1);
        let mut again = Vec::new();
        back.write(&mut again).unwrap();
        assert_eq!(bytes, again);
    }

    #[test]
    fn a_single_mesh_file_is_still_read_as_single() {
        let mut bytes = Vec::new();
        tile(0).mesh.write(&mut bytes).unwrap();
        assert!(matches!(
            NavFile::from_bytes(&bytes).unwrap(),
            NavFile::Single(_)
        ));
    }

    #[test]
    fn a_tile_over_the_header_poly_count_is_rejected() {
        let mut nav = two_tile_file();
        nav.max_tile_polys = 0;
        let mut bytes = Vec::new();
        nav.write(&mut bytes).unwrap();
        match NavFile::from_bytes(&bytes) {
            Err(ExtractError::NavHeaderOutOfRange { field, .. }) => assert_eq!(field, "npolys"),
            other => panic!("expected npolys rejection, got {other:?}"),
        }
    }

    #[test]
    fn trailing_bytes_are_an_error() {
        let mut bytes = Vec::new();
        two_tile_file().write(&mut bytes).unwrap();
        bytes.push(0);
        assert!(NavFile::from_bytes(&bytes).is_err());
    }
}
