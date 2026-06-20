//! Discord-notifications configuration.
//!
//! TOML schema, `${ENV_VAR}` substitution, live reload via `notify`. The
//! config is held inside an `ArcSwap` so the sender task and tracing layer
//! see swaps atomically without locking.
//!
//! # Schema
//!
//! See `config/discord.toml.example` for the annotated user-facing version.
//! In code, the parsed shape is:
//!
//! ```toml
//! [discord]
//! enabled = true
//! username = "Cimmeria"
//! avatar_url = ""
//!
//! [discord.channels.lifecycle]
//! url = "${DISCORD_LIFECYCLE_WEBHOOK}"
//! rate_limit_per_min = 30
//!
//! [discord.events]
//! server_startup = true
//! # ... 38 more flags
//! ```
//!
//! # Live reload semantics
//!
//! `ConfigWatcher` spawns a tokio task that owns a `notify::RecommendedWatcher`.
//! When the file changes, parse + validate are attempted. On success the
//! `ArcSwap` is updated; on failure the previous config stays in place and
//! the error is logged at `warn!` (the new config doesn't break the running
//! server — operator gets a chance to fix it without restart).
//!
//! `ConfigWatcher::handle()` returns a cheap `Config` snapshot any time.
//! Both sender + tracing layer hold this handle.
//!
//! The module is split along four seams:
//!
//! - [`model`] — the parsed [`Config`] / [`ChannelConfig`] types + lookups.
//! - [`toggles`] — the [`EventToggles`] dense toggle map.
//! - [`parse`] — TOML schema, env interpolation, validation, [`ConfigError`].
//! - [`watcher`] — [`ConfigWatcher`] live reload.

mod model;
mod parse;
mod toggles;
mod watcher;

pub use model::{ChannelConfig, Config};
pub use parse::ConfigError;
pub use toggles::EventToggles;
pub use watcher::ConfigWatcher;
