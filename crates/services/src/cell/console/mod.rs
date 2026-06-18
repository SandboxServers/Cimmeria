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
//! [`COMMANDS`] is the registry: `name -> (min/max arg count, required target
//! type, summary)`, mirroring the legacy `Command` table. [`handle_console_command`]
//! parses `.<cmd> <args...>`, validates access (already GM by the channel gate),
//! arg count, and target type, logs the accepted command at `info` for the
//! audit trail, then routes to a family handler. Output goes back
//! to the GM only via [`feedback`] (`onPlayerCommunication` on `CHAN_FEEDBACK`),
//! the same single-recipient channel the native `gm*` query cluster uses.
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
//! The console-channel design is documented in
//! `docs/architecture/dev-console-channel.md`; the player-facing command list is
//! in `docs/commands.md`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::CellEntity;
use tokio::sync::mpsc;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

mod crafting;
mod entity;
mod mission;
mod net;
mod patrol;
mod query;
mod seed;
mod server;
mod spawn;
mod stats;

#[cfg(test)]
mod tests;

/// Re-export of the single-recipient GM feedback line so console handlers and
/// the framework share one delivery path with the native `gm*` cluster.
pub(crate) use super::cell_methods::gm::feedback::send_gm_feedback;

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

/// The kind of selected-target an entity-scoped command requires. Mirrors the
/// `targetType` column of the legacy `Command` table
/// (`deprecated/python/cell/ConsoleCommands.py`). The target is always the
/// caller's currently-selected entity (`current_target_id`, set by
/// `setTargetID` / `gmSetTarget`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Target {
    /// No target needed. The handler operates on the caller (or its own args).
    /// A current target, if any, is still passed through for the few legacy
    /// commands that opt to use it.
    None,
    /// Any player entity.
    Player,
    /// Any NPC entity (`SGWMob` in the legacy hierarchy).
    Mob,
    /// Any being (player or NPC — anything with a stat block). The legacy
    /// `SGWBeing` target type.
    Being,
    /// Any spawnable entity. The legacy `SGWSpawnableEntity` target type — in
    /// practice any entity in the world.
    Spawnable,
}

impl Target {
    /// Does `e` satisfy this target-type requirement?
    fn matches(self, e: &CellEntity) -> bool {
        match self {
            Target::None | Target::Being | Target::Spawnable => true,
            Target::Player => e.is_player,
            Target::Mob => !e.is_player,
        }
    }

    /// Human label for the "wrong target type" feedback line.
    fn label(self) -> &'static str {
        match self {
            Target::None => "none",
            Target::Player => "a player",
            Target::Mob => "an NPC",
            Target::Being => "a being",
            Target::Spawnable => "a spawnable entity",
        }
    }
}

/// One registered console command. The static [`COMMANDS`] table is the single
/// source of truth for validation (`min`/`max`/`target`) and `.help` text;
/// execution is routed by name in [`exec`].
pub(crate) struct Spec {
    /// Command name as typed after the `.` (e.g. `"savespawn"`).
    pub name: &'static str,
    /// Minimum positional arg count.
    pub min: usize,
    /// Maximum positional arg count (`usize::MAX` = unbounded).
    pub max: usize,
    /// Required selected-target type.
    pub target: Target,
    /// One-line summary shown by `.help`.
    pub help: &'static str,
}

const fn spec(
    name: &'static str,
    min: usize,
    max: usize,
    target: Target,
    help: &'static str,
) -> Spec {
    Spec {
        name,
        min,
        max,
        target,
        help,
    }
}

/// The console command registry — the Rust analogue of the legacy
/// `Command.add([...])` table plus the FanMMORPG `path_*` additions.
///
/// Every name here is reachable from [`exec`]; `tests::every_spec_is_dispatched`
/// asserts no entry falls through to the "not implemented" arm.
pub(crate) static COMMANDS: &[Spec] = &[
    // ── meta ────────────────────────────────────────────────────────────────
    spec(
        "help",
        0,
        1,
        Target::None,
        "List console commands (optionally filter by substring)",
    ),
    spec(
        "seedconfirm",
        0,
        0,
        Target::None,
        "Emit your pending authoring changes per seed file (log/Discord)",
    ),
    spec(
        "seedpending",
        0,
        0,
        Target::None,
        "List your pending authoring changes",
    ),
    spec(
        "seedcancel",
        0,
        0,
        Target::None,
        "Discard your pending authoring changes",
    ),
    // ── D. search / query ─────────────────────────────────────────────────────
    spec(
        "searchitem",
        1,
        2,
        Target::None,
        "Search item designs by name",
    ),
    spec(
        "searchmission",
        1,
        2,
        Target::None,
        "Search mission designs by name",
    ),
    spec(
        "searchtemplate",
        1,
        2,
        Target::None,
        "Search entity templates by name",
    ),
    spec("players", 0, 0, Target::None, "List players in your space"),
    // ── F. granular stat readouts ──────────────────────────────────────────────
    spec(
        "primarystats",
        0,
        0,
        Target::Being,
        "Show primary attribute stats of the target",
    ),
    spec(
        "speedstats",
        0,
        0,
        Target::Being,
        "Show movement/action speed stats of the target",
    ),
    spec(
        "armorstats",
        0,
        0,
        Target::Being,
        "Show armor + resistance stats of the target",
    ),
    spec(
        "qrstats",
        0,
        0,
        Target::Being,
        "Show QR-system combat stats of the target",
    ),
    spec(
        "absorbstats",
        0,
        0,
        Target::Being,
        "Show damage-absorption stats of the target",
    ),
    spec(
        "stealthstats",
        0,
        0,
        Target::Being,
        "Show stealth/disguise stats of the target",
    ),
    // ── A. entity / content authoring ──────────────────────────────────────────
    spec(
        "tag",
        1,
        1,
        Target::Spawnable,
        "Set the content tag of the target ('none' clears)",
    ),
    spec(
        "name",
        1,
        usize::MAX,
        Target::Being,
        "Set the display name of the target",
    ),
    spec(
        "alignment",
        1,
        1,
        Target::Being,
        "Set alignment (undefined|praxis|sgu) of the target",
    ),
    spec(
        "nameid",
        1,
        1,
        Target::Spawnable,
        "Set the localized name-id of the target",
    ),
    spec(
        "staticmesh",
        1,
        1,
        Target::Spawnable,
        "Set the static mesh name of the target",
    ),
    spec(
        "bodyset",
        1,
        1,
        Target::Spawnable,
        "Set the body set of the target",
    ),
    spec(
        "eventset",
        1,
        1,
        Target::Spawnable,
        "Set the kismet event-set id of the target",
    ),
    spec(
        "interactiontype",
        1,
        1,
        Target::Spawnable,
        "Set the interaction-type flags of the target",
    ),
    spec(
        "lookat",
        0,
        0,
        Target::Spawnable,
        "Rotate the target to face you",
    ),
    spec(
        "visible",
        1,
        1,
        Target::Spawnable,
        "Show/hide the target (1/0)",
    ),
    spec(
        "setcombatant",
        1,
        1,
        Target::Being,
        "Set a combatant state flag on the target",
    ),
    spec(
        "unsetcombatant",
        1,
        1,
        Target::Being,
        "Clear a combatant state flag on the target",
    ),
    spec(
        "addcomponent",
        1,
        1,
        Target::Being,
        "Add a body component to the target",
    ),
    spec(
        "delcomponent",
        1,
        1,
        Target::Being,
        "Remove a body component from the target",
    ),
    spec(
        "adddialog",
        2,
        2,
        Target::Spawnable,
        "Add a dialog choice (templateId setMapId) to the target",
    ),
    spec(
        "removedialog",
        2,
        2,
        Target::Spawnable,
        "Remove a dialog choice (templateId setMapId) from the target",
    ),
    spec(
        "dynamicupdate",
        0,
        0,
        Target::Spawnable,
        "Re-broadcast the target's dynamic properties to witnesses",
    ),
    // ── H. low-level net / AI debug ────────────────────────────────────────────
    spec(
        "net_seq",
        1,
        2,
        Target::Spawnable,
        "Play a kismet sequence on the target",
    ),
    spec(
        "net_seqto",
        1,
        2,
        Target::None,
        "Play a sequence from you to the target",
    ),
    spec(
        "net_seqfrom",
        1,
        2,
        Target::Spawnable,
        "Play a sequence from the target to you",
    ),
    spec(
        "net_timer",
        2,
        4,
        Target::Spawnable,
        "Start a client timer on the target",
    ),
    spec(
        "net_mapinfo",
        3,
        5,
        Target::Player,
        "Send onMapInfo to the target",
    ),
    spec(
        "net_speak",
        1,
        2,
        Target::Spawnable,
        "Make the target speak (message [channel])",
    ),
    spec(
        "net_dialog",
        1,
        1,
        Target::None,
        "Open a dialog with the target",
    ),
    spec(
        "net_challenge",
        5,
        5,
        Target::None,
        "Send onClientChallenge to the target",
    ),
    spec(
        "debug_velocity",
        3,
        3,
        Target::Spawnable,
        "Set the velocity of the target",
    ),
    spec(
        "debug_controller",
        0,
        0,
        Target::Spawnable,
        "Toggle the debug movement controller on the target",
    ),
    spec(
        "debug_follow",
        0,
        0,
        Target::Spawnable,
        "Toggle the follow controller on the target",
    ),
    spec(
        "threaten",
        1,
        1,
        Target::Mob,
        "Generate threat on the targeted mob",
    ),
    spec(
        "aggression",
        1,
        1,
        Target::Mob,
        "Set the aggression level of the targeted mob",
    ),
    // ── E. crafting / discipline ───────────────────────────────────────────────
    spec(
        "allcraft",
        0,
        0,
        Target::Player,
        "Grant all blueprints + max disciplines to the target",
    ),
    spec(
        "learndiscipline",
        1,
        2,
        Target::Player,
        "Learn/raise a discipline (disciplineId [expertise])",
    ),
    spec(
        "forgetdiscipline",
        1,
        1,
        Target::Player,
        "Forget a discipline (disciplineId)",
    ),
    // ── Mission gaps ──────────────────────────────────────────────────────────
    spec(
        "missionfail",
        1,
        1,
        Target::Player,
        "Force-fail a mission on the target (designId)",
    ),
    spec(
        "missionrewards",
        1,
        1,
        Target::Player,
        "Preview a mission's reward set (designId)",
    ),
    // ── G. server / maintenance ────────────────────────────────────────────────
    spec("save", 0, 0, Target::None, "Persist your player entity now"),
    spec(
        "reloadmap",
        0,
        0,
        Target::None,
        "Reload the current map on yourself",
    ),
    spec(
        "reloadres",
        0,
        1,
        Target::None,
        "Reload server resource caches",
    ),
    spec(
        "removerespawner",
        1,
        1,
        Target::Player,
        "Remove a respawner from the target (respawnerId)",
    ),
    spec(
        "loglevel",
        1,
        2,
        Target::None,
        "Set the server log level (level [category])",
    ),
    spec(
        "logclient",
        0,
        0,
        Target::None,
        "Toggle forwarding server logs to your client",
    ),
    // ── B. spawn authoring / persistence ───────────────────────────────────────
    spec(
        "savespawn",
        0,
        0,
        Target::Spawnable,
        "Persist the target's spawn to the spawnlist",
    ),
    spec(
        "delspawn",
        0,
        0,
        Target::Spawnable,
        "Delete the target's spawnlist row",
    ),
    spec(
        "autosavespawn",
        1,
        1,
        Target::None,
        "Toggle auto-persisting newly spawned entities (1/0)",
    ),
    spec(
        "respawnall",
        0,
        0,
        Target::None,
        "Respawn every NPC in your space",
    ),
    spec(
        "spawnrandom",
        3,
        4,
        Target::None,
        "Spawn N random-scattered copies of a template (templateId xRange zRange [count])",
    ),
    // ── C. patrol authoring (FanMMORPG) ─────────────────────────────────────────
    spec(
        "path_add",
        1,
        1,
        Target::None,
        "Append your position as the next waypoint of a path (pathId)",
    ),
    spec(
        "path_show",
        1,
        1,
        Target::None,
        "Show all waypoints of a path (pathId)",
    ),
    spec(
        "path_clear",
        1,
        1,
        Target::None,
        "Delete all waypoints of a path (pathId)",
    ),
    spec(
        "path_assign",
        1,
        2,
        Target::Mob,
        "Assign a path to the target NPC and start it (pathId [delay])",
    ),
    spec(
        "path_unassign",
        0,
        0,
        Target::Mob,
        "Remove the patrol from the target NPC",
    ),
    spec(
        "path_set_seq",
        3,
        3,
        Target::None,
        "Set a kismet sequence on a waypoint (pathId index seqId)",
    ),
    spec(
        "path_clear_seq",
        2,
        2,
        Target::None,
        "Clear the sequence on a waypoint (pathId index)",
    ),
    spec(
        "path_set_tp",
        2,
        2,
        Target::None,
        "Set a waypoint's teleport dest to your position (pathId index)",
    ),
    spec(
        "path_clear_tp",
        2,
        2,
        Target::None,
        "Clear a waypoint's teleport (pathId index)",
    ),
    spec(
        "path_set_tp_seq",
        3,
        3,
        Target::None,
        "Set a waypoint's arrival sequence (pathId index seqId)",
    ),
    spec(
        "path_set_tp_delay",
        3,
        3,
        Target::None,
        "Set a waypoint's teleport delay (pathId index delay)",
    ),
];

/// Parse, validate, and dispatch one GM `.`-console line.
///
/// `text` is the raw chat body including the leading `.`. The caller is already
/// confirmed `access_level >= GameMaster` by the channel gate in
/// [`crate::cell::chat`]. Every accepted command is logged at `info` for the
/// audit trail; validation failures reply to the GM via [`feedback`] and abort.
pub(crate) async fn handle_console_command(
    caller_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let body = text.strip_prefix('.').unwrap_or(text);
    let parts: Vec<&str> = body.split_whitespace().collect();
    let Some(&name) = parts.first() else {
        send_gm_feedback(caller_id, "Empty command. Try .help", tx).await;
        return;
    };
    let args: Vec<&str> = parts[1..].to_vec();

    let Some(spec) = COMMANDS.iter().find(|c| c.name == name) else {
        send_gm_feedback(
            caller_id,
            &format!("Unknown command: .{name} (try .help)"),
            tx,
        )
        .await;
        return;
    };

    // Audit trail: every accepted GM console command, with the
    // caller + arg count. Command args may contain names/positions but never
    // secrets, so logging the name + count (not the raw text) is the right
    // privacy/observability balance — mirrors the chat.send span policy.
    tracing::info!(
        entity_id = caller_id,
        command = name,
        argc = args.len(),
        "GM .-console command accepted"
    );

    if args.len() < spec.min {
        send_gm_feedback(
            caller_id,
            &format!(
                ".{name}: not enough arguments (got {}, need at least {})",
                args.len(),
                spec.min
            ),
            tx,
        )
        .await;
        return;
    }
    if args.len() > spec.max {
        send_gm_feedback(
            caller_id,
            &format!(
                ".{name}: too many arguments (got {}, at most {})",
                args.len(),
                spec.max
            ),
            tx,
        )
        .await;
        return;
    }

    // Resolve + validate the selected target (if the command requires one).
    let target_id = match resolve_target(caller_id, spec, space_mgr) {
        Ok(t) => t,
        Err(msg) => {
            send_gm_feedback(caller_id, &format!(".{name}: {msg}"), tx).await;
            return;
        }
    };

    exec(name, caller_id, &args, target_id, tx, space_mgr, engine).await;
}

/// Resolve the caller's current target and validate it against `spec.target`.
///
/// - For [`Target::None`] commands, returns the caller's current target if it's
///   set and resolvable (legacy `findEntity(targetId) if targetId else None`),
///   else `None`. A bad/cross-space current target is simply dropped to `None`
///   rather than failing — these commands don't depend on it.
/// - For typed commands, a target is **required**: returns `Err(msg)` (with a
///   GM-facing reason) when there is no current target, it doesn't resolve, it's
///   in another space, or it's the wrong type.
fn resolve_target(
    caller_id: u32,
    spec: &Spec,
    space_mgr: &SpaceManager,
) -> Result<Option<u32>, String> {
    let caller_space = space_mgr.get_entity(caller_id).map(|e| e.space_id.0);
    let current = space_mgr
        .get_entity(caller_id)
        .and_then(|e| e.current_target_id)
        .filter(|&id| id > 0)
        .and_then(|id| u32::try_from(id).ok());

    if spec.target == Target::None {
        // Optional: pass the current target through only if it resolves in the
        // caller's space; otherwise None.
        let resolved =
            current.filter(|&id| space_mgr.get_entity(id).map(|e| e.space_id.0) == caller_space);
        return Ok(resolved);
    }

    let Some(target_id) = current else {
        return Err("a target is required for this command".to_string());
    };
    let Some(target) = space_mgr.get_entity(target_id) else {
        return Err(format!("targeted entity {target_id} is unknown"));
    };
    if Some(target.space_id.0) != caller_space {
        return Err(format!("targeted entity {target_id} is in another space"));
    }
    if !spec.target.matches(target) {
        return Err(format!("expected {} as a target", spec.target.label()));
    }
    Ok(Some(target_id))
}

/// Route a validated command to its family handler. The big match mirrors
/// `gm::dispatch`; each arm receives only the params it needs.
#[allow(clippy::too_many_lines)]
async fn exec(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    match name {
        "help" => query::help(caller_id, args, tx).await,
        // authoring commit workflow
        "seedconfirm" => seed::confirm(caller_id, tx, space_mgr).await,
        "seedpending" => seed::pending(caller_id, tx, space_mgr).await,
        "seedcancel" => seed::cancel(caller_id, tx, space_mgr).await,
        // D. search / query
        "searchitem" => query::search_item(caller_id, args, tx, space_mgr).await,
        "searchmission" => query::search_mission(caller_id, args, tx, space_mgr).await,
        "searchtemplate" => query::search_template(caller_id, args, tx, space_mgr).await,
        "players" => query::players(caller_id, tx, space_mgr).await,
        // F. stat dumps
        "primarystats" | "speedstats" | "armorstats" | "qrstats" | "absorbstats"
        | "stealthstats" => stats::show(name, caller_id, target_id, tx, space_mgr).await,
        // A. entity authoring
        "tag" | "name" | "alignment" | "nameid" | "staticmesh" | "bodyset" | "eventset"
        | "interactiontype" | "lookat" | "visible" | "setcombatant" | "unsetcombatant"
        | "addcomponent" | "delcomponent" | "adddialog" | "removedialog" | "dynamicupdate" => {
            entity::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // H. net / debug
        "net_seq" | "net_seqto" | "net_seqfrom" | "net_timer" | "net_mapinfo" | "net_speak"
        | "net_dialog" | "net_challenge" | "debug_velocity" | "debug_controller"
        | "debug_follow" | "threaten" | "aggression" => {
            net::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // E. crafting
        "allcraft" | "learndiscipline" | "forgetdiscipline" => {
            crafting::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // Mission gaps
        "missionfail" => mission::fail(caller_id, args, target_id, tx, space_mgr).await,
        "missionrewards" => mission::rewards(caller_id, args, target_id, tx, space_mgr).await,
        // G. server / maintenance
        "save" | "reloadmap" | "reloadres" | "removerespawner" | "loglevel" | "logclient" => {
            server::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // B. spawn authoring
        "savespawn" | "delspawn" | "autosavespawn" | "respawnall" | "spawnrandom" => {
            spawn::dispatch(name, caller_id, args, target_id, tx, space_mgr, engine).await
        }
        // C. patrol authoring
        "path_add" | "path_show" | "path_clear" | "path_assign" | "path_unassign"
        | "path_set_seq" | "path_clear_seq" | "path_set_tp" | "path_clear_tp"
        | "path_set_tp_seq" | "path_set_tp_delay" => {
            patrol::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        other => {
            // Unreachable in practice — every COMMANDS entry has an arm above,
            // pinned by `tests::every_spec_is_dispatched`.
            send_gm_feedback(caller_id, &format!(".{other}: not implemented"), tx).await;
        }
    }
}

// ── Shared arg-parsing helpers ────────────────────────────────────────────────

/// Parse the `idx`th positional arg as `i32`, feeding back a GM-facing error and
/// returning `None` on a malformed value. Callers `let Some(v) = parse_i32(...)
/// else { return }`.
pub(crate) async fn parse_i32(
    caller_id: u32,
    args: &[&str],
    idx: usize,
    label: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<i32> {
    match args.get(idx).and_then(|s| s.parse::<i32>().ok()) {
        Some(v) => Some(v),
        None => {
            send_gm_feedback(caller_id, &format!("{label} must be an integer"), tx).await;
            None
        }
    }
}

/// Parse the `idx`th positional arg as `f32`, feeding back a GM-facing error and
/// returning `None` on a malformed value.
pub(crate) async fn parse_f32(
    caller_id: u32,
    args: &[&str],
    idx: usize,
    label: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<f32> {
    // Reject non-finite values: `NaN`/`inf` parse fine but Display as bareword
    // `NaN`/`inf`, which is invalid as a SQL numeric literal — it would fail the
    // live authoring write AND bake a broken statement into the recorded seed.
    match args
        .get(idx)
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|v| v.is_finite())
    {
        Some(v) => Some(v),
        None => {
            send_gm_feedback(caller_id, &format!("{label} must be a finite number"), tx).await;
            None
        }
    }
}

/// Parse a `0`/`1`/`true`/`false` boolean arg, feeding back on a malformed value.
pub(crate) async fn parse_bool(
    caller_id: u32,
    args: &[&str],
    idx: usize,
    label: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<bool> {
    match args.get(idx).map(|s| s.to_ascii_lowercase()) {
        Some(s) if s == "1" || s == "true" => Some(true),
        Some(s) if s == "0" || s == "false" => Some(false),
        _ => {
            send_gm_feedback(caller_id, &format!("{label} must be 0/1/true/false"), tx).await;
            None
        }
    }
}
