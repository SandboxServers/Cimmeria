//! The crate's integration tests, built as one test binary.
//!
//! Cargo makes every `.rs` file directly under `tests/` its own crate,
//! and each of those links the whole dependency graph again. Declaring
//! the test files as modules of this one crate pays that link once. Add
//! a new integration test as a module in this directory, never as a new
//! file directly under `tests/`.
//!
//! Run one module with
//! `cargo test -p cimmeria-content-engine --test it <module>`, e.g.
//! `--test it interact_tag_linter`.

mod dialog_button_linter;
mod interact_tag_linter;
mod onitemuse_remove_item_pairing;
