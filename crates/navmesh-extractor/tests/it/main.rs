//! The crate's integration tests, built as one test binary.
//!
//! Cargo makes every `.rs` file directly under `tests/` its own crate,
//! and each of those links the whole dependency graph again. Declaring
//! the test files as modules of this one crate pays that link once. Add
//! a new integration test as a module in this directory, never as a new
//! file directly under `tests/`.
//!
//! `cargo test` runs every module's tests on threads of one process, so
//! a scratch directory must be unique per test, not just per process:
//! `test_support::scratch_dir` and the local `unique_tempdir` helpers
//! mix in the thread id or a timestamp, and the CLI tests' `scratch`
//! helpers prefix a per-file name and a per-test tag. The CLI tests find
//! the crate's binaries through `CARGO_BIN_EXE_*`, which Cargo sets for
//! every integration-test target.
//!
//! Run one module with
//! `cargo test -p cimmeria-navmesh-extractor --test it <module>`, e.g.
//! `--test it navbuilder_axis_roundtrip`.

mod archetype_castle;
mod bsp_castle_floor_evidence;
mod bsp_castle_hull_cap;
mod bsp_castle_model_decode;
mod bsp_support;
mod castle_coverage_and_probe;
mod extract_map_castle_cellblock;
mod extract_map_cli_synthetic;
mod extract_map_synthetic;
mod nav_inspect_cli;
mod nav_roundtrip_castle_cellblock;
mod navbuilder_axis_roundtrip;
mod navbuilder_tiled;
mod obj_slab_cli;
mod occluder_extract_synthetic;
mod staticmesh_castle_cellblock;
mod terrain_castle;
