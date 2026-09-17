//! The `.`-console command registry: the [`Spec`] / [`Target`] types and the
//! static [`COMMANDS`] table — the Rust analogue of the legacy
//! `Command.add([...])` table plus the FanMMORPG `path_*` additions.

use cimmeria_entity::cell_entity::CellEntity;

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
    pub(crate) fn matches(self, e: &CellEntity) -> bool {
        match self {
            Target::None | Target::Being | Target::Spawnable => true,
            Target::Player => e.is_player,
            Target::Mob => !e.is_player,
        }
    }

    /// Human label for the "wrong target type" feedback line.
    pub(crate) fn label(self) -> &'static str {
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
/// execution is routed by name in [`super::exec`].
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
/// Every name here is reachable from [`super::exec`];
/// `tests::every_spec_is_dispatched` asserts no entry falls through to the
/// "not implemented" arm.
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
    // ── I. entity / combat inspection (read-only) ──────────────────────────────
    spec(
        "info",
        0,
        1,
        Target::None,
        "Show detailed info about the target (selection wins over [entityId])",
    ),
    spec(
        "facing",
        0,
        0,
        Target::Spawnable,
        "Show facing angle/class and distance to the target",
    ),
    spec(
        "combatinfo",
        0,
        0,
        Target::Mob,
        "Diagnose combat-readiness issues on the targeted NPC",
    ),
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

/// One documented positional argument of a console command, used by
/// `.help`'s `<= 3`-match detail view (mirrors the legacy `Command.args`
/// tuples — `(name, doc, type)` — recovered from `inspect.getargspec` plus
/// the `@param`/`@type` docstring tags in `ConsoleCommands.py::Command.__init__`).
pub(crate) struct ArgSpec {
    /// Argument name, as legacy `inspect.getargspec` reported it.
    pub name: &'static str,
    /// Legacy `@type` annotation (`str`/`int`/`float`/`bool`).
    pub ty: &'static str,
    /// Legacy `@param` description.
    pub desc: &'static str,
}

const fn arg(name: &'static str, ty: &'static str, desc: &'static str) -> ArgSpec {
    ArgSpec { name, ty, desc }
}

/// Per-argument metadata for `.help`'s `<= 3`-match detail view, keyed by
/// command name.
///
/// Only commands whose legacy docstrings this packet (P01) actually read are
/// populated here: `help`'s own `command` arg
/// (`deprecated/python/cell/ConsoleCommands.py`) and the search family's
/// `name`/`name2` args (`deprecated/python/cell/commands/Resource.py`).
/// Every other registered command returns an empty slice, so `.help` simply
/// omits the per-argument detail line for those until the packet that
/// restores each command's real behavior also adds its `ArgSpec` row here —
/// building accurate per-arg names/types/descriptions for the other ~59
/// commands requires reading their owning legacy family files (Entity.py,
/// Net.py, Player.py, Mission.py, Crafting.py), which are outside this
/// packet's read set.
const HELP_ARGS: &[ArgSpec] = &[arg("command", "str", "Get help about this command")];
const SEARCH_ITEM_ARGS: &[ArgSpec] = &[
    arg("name", "str", "Item name to search for"),
    arg("name2", "str", "Item name to search for"),
];
const SEARCH_MISSION_ARGS: &[ArgSpec] = &[
    arg("name", "str", "Name to search for"),
    arg("name2", "str", "Name to search for"),
];
const SEARCH_TEMPLATE_ARGS: &[ArgSpec] = &[
    arg("name", "str", "Name to search for"),
    arg("name2", "str", "Name to search for"),
];

pub(crate) fn arg_specs(name: &str) -> &'static [ArgSpec] {
    match name {
        "help" => HELP_ARGS,
        "searchitem" => SEARCH_ITEM_ARGS,
        "searchmission" => SEARCH_MISSION_ARGS,
        "searchtemplate" => SEARCH_TEMPLATE_ARGS,
        _ => &[],
    }
}
