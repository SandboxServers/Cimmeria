//! GM `.`-console command channel.
//!
//! The 2009 SGW client's native slash roster (the 266 `Event_SlashCmd`
//! classes / cell-method indices) is fixed and baked into `SGW.exe`; we can
//! add native `/gm*` commands only for indices that already exist.
//! The remaining dev/authoring commands the legacy Python server and the
//! [doko972/FanMMORPG](https://github.com/doko972/FanMMORPG) fork shipped have
//! **no** native slash binding and never can.
//!
//! Both the legacy server and the fork deliver those through a separate
//! `.`-prefixed in-game console: the client does **not** intercept `.`-prefixed
//! input — it forwards it as an ordinary `CHAN_SAY` chat message. The server
//! intercepts it before broadcast. This module is the Rust analogue of
//! `deprecated/python/cell/ConsoleCommands.py` (the legacy `Command` table) plus
//! the fork's `path_*` additions.
//!
//! # Channel + auth
//!
//! [`crate::cell::chat::handle_chat_message`] calls [`handle_console_command`]
//! from its `CHAN_SAY` arm when (a) the text starts with `.` **and** (b) the
//! sender's [`CellEntity::access_level`](cimmeria_entity::cell_entity::CellEntity::access_level)
//! is `>= GameMaster`. A GM's `.`-text is consumed (never broadcast to other
//! players); a non-GM's `.`-text falls through to normal chat. Authorization is
//! always on the server-side `access_level` (sourced from `account.accesslevel`
//! at login, never a client-asserted byte) — the same trust model as
//! [`crate::cell::dispatch::gm_gate`].
//!
//! # Dispatch model
//!
//! [`registry::COMMANDS`] is the registry: `name -> (min/max arg count, required
//! target type, summary)`, mirroring the legacy `Command` table.
//! [`handle_console_command`] parses `.<cmd> <args...>`, validates access
//! (already GM by the channel gate), arg count, and target type, logs the
//! accepted command at `info` for the audit trail, then routes to a family
//! handler. Output goes back to the GM only via [`feedback`]
//! (`onPlayerCommunication` on `CHAN_FEEDBACK`), the same single-recipient
//! channel the native `gm*` query cluster uses.
//!
//! Handlers are grouped by family, mirroring the `gm/` submodule split:
//! - [`query`] — read-only search / inspection (`searchitem`, `players`, …).
//! - [`stats`] — granular per-domain stat dumps (`primarystats`, …).
//! - [`entity`] — live entity authoring (`tag`, `name`, `visible`, …).
//! - [`net`] — low-level net / AI debug (`net_seq`, `threaten`, …).
//! - [`crafting`] — discipline / blueprint grants (`allcraft`, …).
//! - [`mission`] — mission gaps (`missionfail`, `missionrewards`).
//! - [`server`] — server / maintenance (`save`, `loglevel`, …).
//! - [`spawn`] — spawn persistence (`savespawn`, `delspawn`, …).
//! - [`patrol`] — FanMMORPG patrol authoring (`path_add`, `path_assign`, …).
//!
//! The framework itself splits into:
//! - [`registry`] — the [`Spec`]/[`Target`] types + the static `COMMANDS` table.
//! - [`dispatch`] — parse / validate / route ([`handle_console_command`]).
//! - [`parse`] — shared arg-parsing helpers ([`parse_i32`] / [`parse_f32`] / …).
//!
//! The console-channel design is documented in
//! `docs/architecture/dev-console-channel.md`; the player-facing command list is
//! in `docs/commands.md`.

mod crafting;
mod dispatch;
mod entity;
mod mission;
mod net;
mod parse;
mod patrol;
mod query;
mod registry;
mod seed;
mod server;
mod spawn;
mod stats;

#[cfg(test)]
mod tests;

/// Re-export of the single-recipient GM feedback line so console handlers and
/// the framework share one delivery path with the native `gm*` cluster.
pub(crate) use super::cell_methods::gm::feedback::send_gm_feedback;

// Framework re-exports: handlers reach these via `super::*`, so the split into
// `registry` / `dispatch` / `parse` stays an internal refactor with no
// public-surface change.
pub(crate) use dispatch::handle_console_command;
pub(crate) use parse::{parse_bool, parse_f32, parse_i32};
pub(crate) use registry::{Spec, COMMANDS};

// `exec` is only driven directly by the dispatch-coverage test; gating the
// re-export keeps the non-test build from flagging it as unused.
#[cfg(test)]
pub(crate) use dispatch::exec;

/// Minimum `access_level` (the `account.accesslevel` byte) that unlocks the
/// `.`-console. `2` is `AccessLevel::GameMaster` — see
/// [`crate::cell::dispatch::gm_gate`] for the canonical mapping. Kept as a bare
/// constant (not a typed compare) so the channel gate in
/// [`crate::cell::chat`] is one cheap integer test on the hot chat path.
const GM_ACCESS_LEVEL: u32 = 2;

/// Returns `true` if `access_level` is GameMaster-or-higher and may therefore
/// use the `.`-console. The chat interceptor calls this before consuming a
/// `.`-prefixed `CHAN_SAY` line.
pub(crate) fn is_gm(access_level: u32) -> bool {
    access_level >= GM_ACCESS_LEVEL
}
