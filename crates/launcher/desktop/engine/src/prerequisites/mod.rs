//! Authenticated prerequisite inputs, shared with the native x86 worker.
//! Execution and durable operation ownership are separate responsibilities.
pub use cimmeria_runtime_probe::prerequisite::package::{
    physx_msi, PackageError, PHYSX_EXE_BYTES, PHYSX_EXE_SHA256,
};
