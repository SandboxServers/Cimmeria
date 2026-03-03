// generator/emitter.rs - Rust source code emitter placeholder.
//
// The code emission logic that transforms parsed entity definitions into Rust
// source code currently lives inline in build.rs (see generate_rust_code,
// generate_entity_struct, and generate_registry_data).
//
// TODO: If this crate is refactored to use a separate codegen library crate
// (e.g. cimmeria-defs-codegen), move the emitter functions here. The emitter
// should handle:
//
// - Property structs with typed fields mapped from BigWorld types
// - Default impls with appropriate zero/empty values
// - EntityDefTrait implementations with static property/method metadata
// - PropertyFlags enum and PropertyInfo/MethodInfo structs
// - Static ENTITY_TYPE_DATA array for registry population
// - Proper handling of Rust keywords in identifiers (r#type, r#move, etc.)
