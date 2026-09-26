//! The crate's integration tests, built as one test binary.
//!
//! Cargo makes every `.rs` file directly under `tests/` its own crate,
//! and each of those links the whole dependency graph again, which here
//! includes all of `cimmeria-services`. Declaring the test files as
//! modules of this one crate pays that link once. Add a new integration
//! test as a module in this directory, never as a new file directly
//! under `tests/`; checked-in fixtures stay in `tests/fixtures/`.
//!
//! Run one module with
//! `cargo test -p cimmeria-wireclient --test it <module>`, e.g.
//! `--test it two_client_castle_visibility_chaos`. The live-DB modules
//! still need `-- --test-threads=1` under `cargo test`; see `support`.

mod auth_smoke;
mod support;
mod trace_load;
mod two_client_castle_visibility;
mod two_client_castle_visibility_chaos;
