//! Source transforms applied before a delta is built or applied.

use std::path::Path;

use cimmeria_upk::patcher::PatchSession;
use cimmeria_upk::texture::{rebake_world_map, WorldMapRebake};

use crate::recipe::{Transform, WorldMapParams};
use crate::{io_err, PatchsetError, Result};

/// Read `path` and apply `transform`. Also returns the SHA-256 of the raw
/// file, which the recipe pins.
///
/// For [`Transform::None`] the hash covers exactly the returned bytes. For
/// [`Transform::UpkNormalize`] the package is reopened by path
/// (`PatchSession` reads only from a file), so a file swapped between the
/// two reads would be normalized unhashed. The recipe's `result_sha256` is
/// the integrity gate in that case: the rebuilt file must match it or
/// nothing is written.
pub fn load(path: &Path, transform: &Transform) -> Result<(Vec<u8>, String)> {
    let raw = std::fs::read(path).map_err(io_err(path))?;
    let raw_sha = crate::sha256_hex(&raw);
    let bytes = match transform {
        Transform::None => raw,
        Transform::UpkNormalize => upk_normalize(path)?,
        Transform::WorldMapRebake(p) => world_map_rebake(path, p)?,
    };
    Ok((bytes, raw_sha))
}

/// Rebuild a world map package's overview texture from its own tiles.
/// The whole job happens here, on the player's machine: nothing derived from
/// the picture travels in the patch zip.
pub fn world_map_rebake(path: &Path, p: &WorldMapParams) -> Result<Vec<u8>> {
    let params = WorldMapRebake {
        texture: p.texture.clone(),
        tile_prefix: p.tile_prefix.clone(),
        lo: (p.lo[0], p.lo[1]),
        hi: (p.hi[0], p.hi[1]),
        size: p.size,
        pad: p.pad,
        carry: p.carry,
    };
    rebake_world_map(path, &params).map_err(|e| PatchsetError::Upk {
        path: path.display().to_string(),
        detail: e.to_string(),
    })
}

/// Open a UE3 package with the append-only patcher and write it back
/// unchanged: decompressed, tables at the end. Deterministic for a given
/// input, which is what lets a delta built on one machine apply on another.
pub fn upk_normalize(path: &Path) -> Result<Vec<u8>> {
    let upk_err = |e: cimmeria_upk::UpkError| PatchsetError::Upk {
        path: path.display().to_string(),
        detail: e.to_string(),
    };
    PatchSession::open(path)
        .map_err(upk_err)?
        .finish()
        .map_err(upk_err)
}
