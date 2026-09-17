//! The `.`-console command registry: the [`Spec`] / [`Target`] types (this
//! file) and the static [`COMMANDS`] table ([`commands`]) — the Rust
//! analogue of the legacy `Command.add([...])` table plus the FanMMORPG
//! `path_*` additions.

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

mod commands;
pub(crate) use commands::COMMANDS;

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
