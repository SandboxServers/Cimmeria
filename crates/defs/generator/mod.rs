// generator/mod.rs - Code generation module placeholder.
//
// The actual entity definition parsing and Rust code generation logic lives
// in build.rs, because Cargo build scripts are compiled as separate binaries
// and cannot import from the crate's own library source.
//
// This module exists as a reference point and future home for shared
// generation logic if the build script is ever refactored to use a shared
// library crate (e.g. cimmeria-defs-codegen).

pub mod parser;
pub mod emitter;
