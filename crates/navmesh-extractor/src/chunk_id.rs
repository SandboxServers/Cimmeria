//! Chunk ID encoding/decoding for UE3 `.umap` chunk filenames.
//!
//! SGW cooked maps split a single world into hex-named chunks
//! (`Castle_CellBlock-FFFEFFFD.umap`). The eight hex digits encode the
//! chunk's grid position as a packed `u32`:
//!
//! ```text
//! +--------+--------+
//! |  high  |  low   |
//! | u16    |  u16   |
//! +--------+--------+
//! ```
//!
//! Both halves are interpreted as **signed 16-bit** values (sign-extended
//! to `i32` on decode) — see `deprecated/cpp/src/nav_builder/chunk.cpp:27-28`
//! where the NavBuilder reads:
//!
//! ```cpp
//! positionX_ = static_cast<int16_t>(chunkId_);          // low  u16
//! positionZ_ = static_cast<int16_t>(chunkId_ >> 16);    // high u16
//! ```
//!
//! # Axis labels — corrected against the chunk content
//!
//! The C++ NavBuilder labels the low u16 as `positionX_` and the high u16
//! as `positionZ_`, but those names are NavBuilder's **post-swizzle BW**
//! coordinate names, not UE3 source-coordinate names. Cross-checking the
//! actual export positions inside chunk `fffefffd.umap` (low=−3, high=−2)
//! against the StaticMeshActor `Location.X`/`Location.Y` ranges:
//!
//! - **low u16  → UE3 Y** (lateral)
//! - **high u16 → UE3 X**
//!
//! After NavBuilder's `loadOBJ` swizzle (`v.x = obj.z/100, v.y = obj.y/100,
//! v.z = obj.x/100`), this lands as:
//!
//! - **low u16  → BW Z**
//! - **high u16 → BW X**
//!
//! For the extractor itself this only matters when we emit OBJ vertices —
//! we emit in UE3 cm coordinates and let NavBuilder do the swizzle. The
//! [`ChunkId::position_x`] / [`ChunkId::position_z`] accessors here
//! preserve the C++ NavBuilder field names for round-trip compatibility.

use std::path::Path;

use crate::ExtractError;

/// Decoded chunk ID + signed grid position.
///
/// The two grid coordinates use the C++ NavBuilder's field names for
/// downstream compatibility — see the module-level doc for the
/// UE3 ↔ BW axis-label mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkId {
    raw: u32,
    position_x: i16,
    position_z: i16,
}

impl ChunkId {
    /// Build a [`ChunkId`] directly from the raw packed `u32`.
    pub fn from_raw(raw: u32) -> Self {
        // Match the NavBuilder cast exactly. `as i16` reuses the low 16 bits;
        // shifting right keeps unsigned semantics on a u32 so `as i16` of
        // the high half is just the high u16 reinterpreted as signed.
        let position_x = raw as i16;
        let position_z = (raw >> 16) as i16;
        Self {
            raw,
            position_x,
            position_z,
        }
    }

    /// Decode a chunk ID from a `.umap` path.
    ///
    /// Accepts the standard cooked filename `<MapName>-<HEX8>.umap`. The
    /// hex digits are case-insensitive. An invalid filename returns
    /// [`ExtractError::InvalidChunkFilename`].
    pub fn from_umap_path(path: &Path) -> crate::Result<Self> {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ExtractError::InvalidChunkFilename(path.display().to_string()))?;

        // Format is "<MapName>-<HEX8>". Take the last 8 chars after the
        // final '-'. We don't validate the prefix is alphabetic — some
        // maps embed digits in their name (e.g. `Beta_Site_Evo_1`).
        let hex = stem
            .rsplit_once('-')
            .map(|(_, h)| h)
            .ok_or_else(|| ExtractError::InvalidChunkFilename(stem.to_string()))?;

        if hex.len() != 8 {
            return Err(ExtractError::InvalidChunkFilename(stem.to_string()));
        }

        let raw = u32::from_str_radix(hex, 16)
            .map_err(|_| ExtractError::InvalidChunkFilename(stem.to_string()))?;

        Ok(Self::from_raw(raw))
    }

    /// Decode a chunk ID from a NavBuilder-style OBJ filename of the form
    /// `<HEX8>o.obj`. This is what the C++ NavBuilder reads from disk
    /// (see `chunk.cpp:19-29`).
    ///
    /// The format is **strict**: the stem must be exactly nine
    /// characters — eight hex digits followed by a literal `o`.
    /// Filenames with extra prefix material (e.g. `prefix-12345678o.obj`)
    /// are rejected; if the C++ NavBuilder ever needs to accept a
    /// prefixed form, both sides must change together.
    pub fn from_obj_path(path: &Path) -> crate::Result<Self> {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ExtractError::InvalidChunkFilename(path.display().to_string()))?;

        // Require exactly 9 chars: 8 hex digits + trailing 'o'. Anything
        // longer would silently mis-parse a prefix as part of the hex.
        if stem.len() != 9 || !stem.ends_with('o') {
            return Err(ExtractError::InvalidChunkFilename(stem.to_string()));
        }
        let hex = &stem[..8];
        let raw = u32::from_str_radix(hex, 16)
            .map_err(|_| ExtractError::InvalidChunkFilename(stem.to_string()))?;
        Ok(Self::from_raw(raw))
    }

    /// Raw packed `u32`.
    pub fn raw(&self) -> u32 {
        self.raw
    }

    /// Low u16 reinterpreted as signed i16 — `positionX_` in the C++
    /// NavBuilder; corresponds to **UE3 Y** in source coordinates.
    pub fn position_x(&self) -> i16 {
        self.position_x
    }

    /// High u16 reinterpreted as signed i16 — `positionZ_` in the C++
    /// NavBuilder; corresponds to **UE3 X** in source coordinates.
    pub fn position_z(&self) -> i16 {
        self.position_z
    }

    /// Filename NavBuilder expects for this chunk's OBJ — eight
    /// lowercase hex digits followed by `o.obj`.
    pub fn obj_filename(&self) -> String {
        format!("{:08x}o.obj", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn origin_chunk_decodes_to_zero() {
        let id = ChunkId::from_raw(0x0000_0000);
        assert_eq!(id.position_x(), 0);
        assert_eq!(id.position_z(), 0);
        assert_eq!(id.raw(), 0);
    }

    #[test]
    fn positive_chunk_offsets() {
        // 0x00010002 → low=0x0002, high=0x0001 → x=2, z=1
        let id = ChunkId::from_raw(0x0001_0002);
        assert_eq!(id.position_x(), 2);
        assert_eq!(id.position_z(), 1);
    }

    #[test]
    fn negative_chunk_offsets_sign_extend() {
        // 0xFFFFFFFF → low=0xFFFF, high=0xFFFF → x=-1, z=-1
        let id = ChunkId::from_raw(0xFFFF_FFFF);
        assert_eq!(id.position_x(), -1);
        assert_eq!(id.position_z(), -1);
    }

    /// The six representative chunk filenames cited in the deep dive
    /// (issue #46) — confirms the (low → x, high → z) decode matches
    /// what NavBuilder reads. Any change to the cast semantics here
    /// would silently misplace every chunk relative to the C++ tool's
    /// expectations.
    #[test]
    fn six_chunks_decode_to_expected_positions() {
        let cases: &[(u32, i16, i16)] = &[
            (0x0000_0000, 0, 0),
            (0x0001_0000, 0, 1),
            (0x0000_0001, 1, 0),
            (0x0003_0003, 3, 3),
            (0xFFFE_FFFD, -3, -2),
            (0xFFFF_0001, 1, -1),
        ];
        for (raw, expect_x, expect_z) in cases {
            let id = ChunkId::from_raw(*raw);
            assert_eq!(
                id.position_x(),
                *expect_x,
                "raw=0x{raw:08x} position_x mismatch"
            );
            assert_eq!(
                id.position_z(),
                *expect_z,
                "raw=0x{raw:08x} position_z mismatch"
            );
        }
    }

    #[test]
    fn decodes_from_castle_cellblock_filename() {
        let p = PathBuf::from("Castle_CellBlock-FFFEFFFD.umap");
        let id = ChunkId::from_umap_path(&p).unwrap();
        assert_eq!(id.raw(), 0xFFFE_FFFD);
        assert_eq!(id.position_x(), -3);
        assert_eq!(id.position_z(), -2);
    }

    #[test]
    fn decodes_from_lowercase_hex() {
        let p = PathBuf::from("Castle_CellBlock-00010002.umap");
        let id = ChunkId::from_umap_path(&p).unwrap();
        assert_eq!(id.raw(), 0x0001_0002);
    }

    #[test]
    fn decodes_map_name_with_digits() {
        // `Beta_Site_Evo_1` is a real map name with a trailing digit.
        let p = PathBuf::from("Beta_Site_Evo_1-00000000.umap");
        let id = ChunkId::from_umap_path(&p).unwrap();
        assert_eq!(id.raw(), 0x0000_0000);
    }

    #[test]
    fn rejects_short_hex() {
        let p = PathBuf::from("Castle_CellBlock-12345.umap");
        assert!(ChunkId::from_umap_path(&p).is_err());
    }

    #[test]
    fn rejects_missing_dash() {
        let p = PathBuf::from("BadFilename.umap");
        assert!(ChunkId::from_umap_path(&p).is_err());
    }

    #[test]
    fn obj_filename_matches_navbuilder_convention() {
        let id = ChunkId::from_raw(0xFFFE_FFFD);
        assert_eq!(id.obj_filename(), "fffefffdo.obj");
    }

    #[test]
    fn obj_path_roundtrip() {
        let id = ChunkId::from_raw(0x0003_0003);
        let path = PathBuf::from(id.obj_filename());
        let decoded = ChunkId::from_obj_path(&path).unwrap();
        assert_eq!(decoded, id);
    }

    /// Tightened format check — pre-tightening, the slice-from-end
    /// loop would silently accept `prefix-12345678o.obj` and treat
    /// `12345678` as the chunk ID. The strict `stem.len() == 9`
    /// invariant rejects it.
    #[test]
    fn obj_path_rejects_prefixed_form() {
        let p = PathBuf::from("prefix-12345678o.obj");
        assert!(ChunkId::from_obj_path(&p).is_err());
    }

    /// Missing the trailing 'o' marker — also a malformed input.
    #[test]
    fn obj_path_rejects_missing_marker() {
        let p = PathBuf::from("12345678.obj");
        assert!(ChunkId::from_obj_path(&p).is_err());
    }
}
