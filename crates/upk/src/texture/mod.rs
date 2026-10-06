//! Texture decoding, encoding and rebuilding for cooked SGW packages. Every
//! step is integer-only and deterministic, so a texture rebuilt on a player's
//! machine matches the hash a patch recipe pinned.

pub mod dxt1;
pub mod lzo_chunks;
pub mod resample;
pub mod texture2d;
pub mod world_map;

pub use world_map::{rebake as rebake_world_map, WorldMapRebake};

#[cfg(test)]
mod tests;
