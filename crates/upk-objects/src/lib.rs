//! UE3 Object Deserializers for SGW package files.
//!
//! This crate provides typed deserializers for UE3 object classes found in
//! Stargate Worlds `.upk`/`.umap` packages, building on top of `cimmeria-upk`'s
//! raw package parsing.
//!
//! # Modules
//! - [`PackageIndex`] — Cross-package export index for resolving import references
//! - [`bulk_data`] — FUntypedBulkData / TLazyArray parsers for mesh and texture data
//! - [`texture2d`] — Texture2D mip data extraction (DXT format detection, mip chain parsing)
//! - [`static_mesh`] — StaticMesh vertex/index buffer extraction (40-byte vertex format, kDOP, LODs)

pub mod bulk_data;
pub mod error;
pub mod package_index;
pub mod static_mesh;
pub mod texture2d;

pub use error::{ObjectError, Result};
pub use package_index::PackageIndex;
pub use static_mesh::{
    deserialize_static_mesh, BoundingBox, KdopTriangle, LodModel, MeshSection, StaticMesh, Vertex,
};
pub use texture2d::{deserialize_texture2d, MipLevel, PixelFormat, Texture2D};
