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
//! [`chat::handle_chat_message`] calls [`handle_console_command`]
//! from its `CHAN_SAY` arm when (a) the text starts with `.` **and** (b) the
//! sender's [`CellEntity::access_level`](cimmeria_entity::cell_entity::CellEntity::access_level)
//! is `>= GameMaster`. A GM's `.`-text is consumed (never broadcast to other
//! players). The few player commands (`player_commands`: `.respeccraft`)
//! are consumed for GMs and players alike, before this gate. A non-GM's
//! `.`-text that names a registered command is consumed too and answered
//! "is a GM command" (pets campaign PT-07); any other non-GM `.`-text falls
//! through to normal chat. Authorization is
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
//! - [`give`] — selected-target player grants (`givecash`, `givexp`).
//! - [`give_ability`] — `giveability`, persisted through the base.
//! - [`pet`] — pet UAT tools (`.pet summon|dismiss|stance|info|list`).
//! - [`net`] — low-level net / AI debug (`net_seq`, `threaten`, …).
//! - [`aggro`] — the GM's own proximity-aggro switch (`.aggro on|off`).
//! - [`bank`] — the GM's vault shortcut (`.bank`) and the read-only vault
//!   listing (`.bankdump`).
//! - [`crafting`] — discipline / blueprint grants (`allcraft`, …).
//! - [`mission`] — mission gaps (`missionfail`, `missionrewards`).
//! - [`server`] — server / maintenance (`save`, `loglevel`, …).
//! - [`spawn`] — spawn lifecycle + persistence (`spawn`, `despawn`,
//!   `savespawn`, `delspawn`, …).
//! - [`patrol`] — FanMMORPG patrol authoring (`path_add`, `path_assign`, …).
//! - [`travel`] — player-administration travel (`gotoxyz`, `goto`, `summon`,
//!   `gotolocation`).
//! - [`placement`] — selected-entity read/set position + orientation
//!   (`location`, `rotation`).
//! - [`social`] — the GM broadcast (`announce`), the console twin of the
//!   native `/gmshout`.
//! - [`duel`] — duel GM tools (`duel_status`, `duel_end`).
//! - [`mail`] — mail GM tools (`mail`, `mailbox`, `mail_expire`).
//! - [`squad`] — squad tools (`squad_invite`, `squad_join`, `squad_info`),
//!   routed to the squad handlers in `cimmeria-cell-methods`.
//! - [`org`] — Team and Command tools (`org_disband`, `org_join`,
//!   `org_rank`, `org_info`, `org_list`, `org_set_perms`), forwarded to the
//!   base.
//! - [`org_create`] — `org_create`, founding a Team or Command on the base
//!   without the registrar (ORG-05).
//!
//! The framework itself splits into:
//! - [`registry`] — the [`Spec`]/[`Target`] types + the static `COMMANDS` table.
//! - [`dispatch`] — parse / validate / route ([`handle_console_command`]).
//! - [`parse`] — shared arg-parsing helpers ([`parse_i32`] / [`parse_f32`] / …).
//!
//! The console-channel design is documented in
//! `docs/architecture/dev-console-channel.md`; the player-facing command list is
//! in `docs/commands.md`.

mod aggro;
mod bank;
mod black_market;
mod bookmark;
// The chat interceptor that routes a GM's `.`-lines here, and the native
// `gm*` cell methods (SGWGmPlayer, index 109+). Both call into the console,
// and the console calls the GM handlers, so all three are one crate in the
// services split (`docs/architecture/services-crate-split.md` §2H). `cell::chat`
// re-exports `chat` at its old path.
pub mod chat;
mod crafting;
mod dispatch;
mod duel;
mod entity;
mod give;
mod give_ability;
pub mod gm;
mod mail;
mod mission;
mod net;
mod org;
mod org_create;
mod parse;
mod patrol;
mod pet;
mod placement;
mod player_commands;
mod query;
mod registry;
mod seed;
mod server;
mod social;
mod spawn;
mod squad;
mod stats;
mod travel;

#[cfg(test)]
mod tests;

/// Re-export of the single-recipient GM feedback line so console handlers and
/// the framework share one delivery path with the native `gm*` cluster.
pub(crate) use gm::feedback::send_gm_feedback;

// Framework re-exports: handlers reach these via `super::*`, so the split into
// `registry` / `dispatch` / `parse` stays an internal refactor with no
// public-surface change.
pub use dispatch::handle_console_command;
pub(crate) use dispatch::refuse_non_gm_command;
pub(crate) use parse::{parse_bool, parse_f32, parse_i32};
pub(crate) use player_commands::handle_player_command;
pub(crate) use registry::{Spec, COMMANDS};

// `exec` is only driven directly by the dispatch-coverage test and by the
// NPC movement tick's `.speed` tests in `cimmeria-cell`; gating the
// re-export keeps the non-test build from flagging it as unused.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use dispatch::exec;

// The `.`-console privilege test lives with the cell's GM gate, in
// cimmeria-cell-world, because the NPC AI's GM checks sit below the console.
// Public for the live-research-lab console exec in `cimmeria-services`.
pub use crate::cell::dispatch::is_gm;

/// Public, owned description of one registered `.`-console command. Exposed so
/// out-of-crate callers — the live-research-lab MCP `server_console_list` tool
/// (issue #687) — can enumerate the registry without touching the crate-private
/// [`Spec`]/[`COMMANDS`]/[`registry::Target`] types.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    /// Command name as typed after the `.` (e.g. `"spawn"`).
    pub name: String,
    /// Minimum positional argument count.
    pub min_args: usize,
    /// Maximum positional argument count; `None` means unbounded.
    pub max_args: Option<usize>,
    /// Required selected-target type (human label, e.g. `"a player"`).
    pub target: String,
    /// One-line summary (the `.help` text).
    pub help: String,
}

/// Enumerate every registered `.`-console command, in `.help` order. The single
/// source of truth is the same [`COMMANDS`] table the in-world console
/// dispatches from, so the two can never drift.
pub fn command_catalog() -> Vec<CommandInfo> {
    COMMANDS
        .iter()
        .map(|s| CommandInfo {
            name: s.name.to_string(),
            min_args: s.min,
            max_args: (s.max != usize::MAX).then_some(s.max),
            target: s.target.label().to_string(),
            help: s.help.to_string(),
        })
        .collect()
}
