// cimmeria-defs: Entity definitions parsed from .def XML files at compile time.
//
// The build.rs script reads entities.xml and the individual .def files from
// the entities/defs/ directory, then generates Rust structs, property metadata,
// and method metadata for each entity type. The generated code is included here
// via the include! macro.
//
// If entity files are missing or malformed, the build script gracefully
// generates empty stubs so this crate always compiles.

// Include generated entity types, property structs, and static registry data.
// Property names mirror BigWorld entity defs (camelCase), not Rust conventions.
#[allow(non_snake_case)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated_entities.rs"));
}
pub use generated::*;

pub mod registry;
