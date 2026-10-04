//! Authenticated prerequisite material. These APIs do not execute installers,
//! establish installation ownership, or assert game readiness.
mod physx_package;
pub use physx_package::{physx_msi, PackageError, PHYSX_EXE_BYTES, PHYSX_EXE_SHA256};
