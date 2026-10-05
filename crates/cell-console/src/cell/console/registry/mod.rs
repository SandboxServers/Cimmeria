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
///
/// `Copy` so [`commands`]'s compile-time concatenation of the per-family
/// tables can move rows into the flattened array in a `const fn`.
#[derive(Clone, Copy)]
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
/// `.announce` (SS-C2) has no legacy docstring; this row is written from
/// the command itself. One positional "arg": the words of the line, whose
/// optional leading `space` picks the scope.
const ANNOUNCE_ARGS: &[ArgSpec] = &[arg(
    "text",
    "str",
    "Required. The line to send to every online player; start it with `space` to reach only your space",
)];

/// `.pet` (pets PT-07) has no legacy counterpart; written from
/// `console/pet.rs`. `verb` is required (`.pet` has `min = 1`), `id` is the
/// second word, which only `summon` and `stance` read.
const PET_ARGS: &[ArgSpec] = &[
    arg(
        "verb",
        "str",
        "summon | dismiss | stance | info | list. summon replaces your pet at once, with no warmup; info reads the selected pet, else yours; list covers your space",
    ),
    arg(
        "id",
        "int",
        "For summon: a summon ability id (2826) or a template id (350). For stance: 0 passive, 1 defensive, 2 aggressive",
    ),
];
/// `.giveability` (pets PT-07) has no legacy counterpart; written from
/// `console/give_ability.rs`.
const GIVEABILITY_ARGS: &[ArgSpec] = &[arg(
    "abilityId",
    "int",
    "The ability to grant to the selected player (else you); saved to the character, survives relog and respec, costs no points",
)];
/// The duel GM tools (SS-U2) have no legacy docstring either.
const DUEL_STATUS_ARGS: &[ArgSpec] = &[arg(
    "name",
    "str",
    "Optional. The character whose duel to show (exact case); yours when omitted",
)];
const DUEL_END_ARGS: &[ArgSpec] = &[arg(
    "name",
    "str",
    "Required. The character whose duel or duel challenge to end (exact case)",
)];
/// `.mute` / `.unmute` (social-systems SS-C3, D-SS26) have no legacy
/// counterpart; written from `console/social.rs`.
const MUTE_ARGS: &[ArgSpec] = &[
    arg(
        "name",
        "str",
        "Required. An online character; a mute holds across relog until it ends or the server restarts",
    ),
    arg("minutes", "int", "Required. 1 to 10080 (7 days)"),
    arg("reason", "str", "Optional. Logged for the other GMs, not shown to the player"),
];
const UNMUTE_ARGS: &[ArgSpec] = &[arg("name", "str", "Required. An online, muted character")];
/// `.craftkit` and `.learnblueprint` have no legacy docstring; these rows
/// are written from the commands themselves.
const CRAFTKIT_ARGS: &[ArgSpec] = &[
    arg(
        "blueprintId",
        "int",
        "Blueprint whose component set 1 the target gets",
    ),
    arg("count", "int", "Crafts' worth to grant, 1-10 (default 1)"),
];
const LEARNBLUEPRINT_ARGS: &[ArgSpec] =
    &[arg("blueprintId", "int", "Blueprint to teach the target")];

/// The GM mail tools (SS-U1) have no legacy docstring.
const MAIL_ARGS: &[ArgSpec] = &[
    arg(
        "to",
        "str",
        "Optional. `to <name>`: the recipient, online or not; yourself when omitted",
    ),
    arg(
        "cash",
        "int",
        "Optional. `cash <n>`: naquadah to attach, minted (0 to 2147483647)",
    ),
    arg(
        "item",
        "int",
        "Optional. `item <typeId> [qty]`: an item to attach, minted; a number after the type id is the quantity",
    ),
    arg(
        "cod",
        "int",
        "Optional. `cod <n>`: make it a COD mail from you at this price; needs an item and no cash",
    ),
    arg(
        "subject",
        "str",
        "Optional. The rest of the line; \"GM test mail\" when omitted",
    ),
];
const MAILBOX_ARGS: &[ArgSpec] = &[arg(
    "name",
    "str",
    "Optional. The character whose mailbox to show; yours when omitted",
)];
const MAIL_EXPIRE_ARGS: &[ArgSpec] = &[arg(
    "mailId",
    "int",
    "Required. The mail to expire (see .mailbox); refused until mail expiry lands",
)];

/// The ability lab commands (AB-L2) have no legacy counterpart; written
/// from `console/abilities/`.
const COOLDOWNS_ARGS: &[ArgSpec] = &[
    arg(
        "reset",
        "str",
        "Optional. `reset` clears your cooldowns and sends your client the clear timers; without it, lists them",
    ),
    arg(
        "abilityId",
        "int",
        "Optional, after `reset`. Clear only this ability (and its moniker groups); the clear is sent even if the server had none running",
    ),
];
const DUMMY_ARGS: &[ArgSpec] = &[
    arg(
        "mode",
        "str",
        "Optional. `hostile` (default) or `friendly` places a dummy 3 m in front of you; `caster` places a hostile one that casts an ability at you; `clear` removes your own dummies",
    ),
    arg(
        "templateId",
        "int",
        "Optional. The entity template to use (default 34, SGC Jaffa); after `caster`, the ability it casts",
    ),
    arg(
        "intervalSecs",
        "int",
        "Optional, `caster` only. Seconds between casts (default 8, 1-290, so two casts fit its 10 minutes); at least the ability's cooldown and longer than its warmup",
    ),
];

pub(crate) fn arg_specs(name: &str) -> &'static [ArgSpec] {
    match name {
        "cooldowns" => COOLDOWNS_ARGS,
        "dummy" => DUMMY_ARGS,
        "mute" => MUTE_ARGS,
        "unmute" => UNMUTE_ARGS,
        "help" => HELP_ARGS,
        "searchitem" => SEARCH_ITEM_ARGS,
        "searchmission" => SEARCH_MISSION_ARGS,
        "searchtemplate" => SEARCH_TEMPLATE_ARGS,
        "announce" => ANNOUNCE_ARGS,
        "pet" => PET_ARGS,
        "giveability" => GIVEABILITY_ARGS,
        "duel_status" => DUEL_STATUS_ARGS,
        "duel_end" => DUEL_END_ARGS,
        "mail" => MAIL_ARGS,
        "mailbox" => MAILBOX_ARGS,
        "mail_expire" => MAIL_EXPIRE_ARGS,
        "craftkit" => CRAFTKIT_ARGS,
        "learnblueprint" => LEARNBLUEPRINT_ARGS,
        _ => &[],
    }
}
