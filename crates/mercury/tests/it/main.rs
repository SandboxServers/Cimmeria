//! The crate's integration tests, built as one test binary.
//!
//! Cargo makes every `.rs` file directly under `tests/` its own crate,
//! and each of those links the whole dependency graph again. Declaring
//! the test files as modules of this one crate pays that link once. Add
//! a new integration test as a module in this directory, never as a new
//! file directly under `tests/`.
//!
//! They need the `test-support` feature (`LossyTransport`), which the
//! crate's self dev-dependency turns on for them.
//!
//! Run one module with
//! `cargo test -p cimmeria-mercury --test it <module>`, e.g.
//! `--test it chaos_lossy_transport_integration`.

mod chaos_lossy_transport_integration;
