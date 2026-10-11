//! The crate's integration tests, built as one test binary.
//!
//! Cargo makes every `.rs` file directly under `tests/` its own crate,
//! and each of those links the whole dependency graph again, which here
//! includes all of `cimmeria-services`. Declaring the test files as
//! modules of this one crate pays that link once. Add a new integration
//! test as a module in this directory, never as a new file directly
//! under `tests/`; checked-in fixtures stay in `tests/fixtures/`.
//!
//! Run the live-DB modules with
//! `pwsh tools/build-lane/live-db-test.ps1 --wireclient <module>`, which
//! gives each running test its own database clone (`wireclient-e2e`
//! profile). The no-DB modules run under a plain `cargo nextest run`.

mod auth_smoke;
mod character_create_live_db;
mod client_calls;
mod duel_two_duelists_and_a_spectator;
mod isolation;
mod login_probe;
mod praxis_start_live_db;
mod script_session_live_db;
mod sparbot_duel;
mod support;
mod trace_load;
mod two_client_castle_visibility;
mod two_client_castle_visibility_chaos;
mod two_client_command_invite;
mod two_client_mail_cod;
mod two_client_squad;
mod two_client_tell;
