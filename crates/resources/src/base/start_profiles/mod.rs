//! Start profiles (Class Start v6, CS-02): where a new character of each
//! `CharDefId` starts and what it starts with.
//!
//! One row of `resources.char_creation` per char_def, plus its
//! `char_creation_abilities` and `char_creation_items` rows, is the whole
//! profile: profile id, start world and spawn point, start level, the
//! `debug_kit` flag (lock L2), the holding-state label, the starter items
//! and the starter ability grants with their provenance kind. The debug kit
//! itself is `char_creation_debug_kit_abilities` / `_items`.
//!
//! Every consumer reads this one source:
//!
//! - character creation (`cimmeria-base`, `character_create`) loads it fresh
//!   from the database for each creation ([`load_all`]);
//! - the sync consumers (the GM-only world redirect, the console's
//!   `.gotolocation <world>` entry point, the respawn fallback) read the
//!   process registry ([`installed`]) the base and the cell fill at boot
//!   ([`load_at_boot`]).
//!
//! A start level never comes from a mission's seeded level: it is the
//! profile's own `start_level` column (preflight finding, ledger
//! `docs/analysis/class-start-v6/README.md`; guarded by
//! `no_start_level_is_derived_from_missions`).

mod load;
mod registry;

#[cfg(any(test, feature = "test-support"))]
pub mod fixture;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod live_db_tests;

pub use load::{load_all, load_at_boot, LoadError};
pub use registry::{install, installed};

/// The highest level a profile may start at: the level cap
/// (`cimmeria_game::player::MAX_LEVEL`; this crate does not depend on
/// `cimmeria-game`). The `char_creation_start_level_check` constraint uses
/// the same range.
pub const MAX_START_LEVEL: i32 = 50;

/// Whether a profile is the campaign's design or a placeholder kept while
/// its real start is blocked (OD-CS08, OD-CS09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartState {
    /// The v6 design: the profile's own world, kit and grants.
    Canonical,
    /// Today's runtime kept literally until a blocker clears (Goa'uld B4,
    /// Asgard B1-B3): the legacy universal kit, the old start world. It
    /// must not shape the canonical design and is removed as one unit.
    NonCanonicalBlockedLegacy,
}

impl StartState {
    /// The `start_state` column text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "CANONICAL",
            Self::NonCanonicalBlockedLegacy => "NON_CANONICAL_BLOCKED_LEGACY",
        }
    }
}

impl TryFrom<&str> for StartState {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        [Self::Canonical, Self::NonCanonicalBlockedLegacy]
            .into_iter()
            .find(|s| s.as_str() == value)
            .ok_or_else(|| format!("unknown start_state {value:?}"))
    }
}

/// Where a starter ability came from: `char_creation_abilities.source_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitSource {
    /// A race's core sustain (OD-CS05); a `racial_core` provenance row.
    RacialCore,
    /// The class's free signature ability (OD-CS06); a `signature` row.
    Signature,
    /// A tutorial ability granted at creation; a `tutorial` row.
    Tutorial,
    /// A mission-style grant at creation; a `mission` row.
    Mission,
    /// The legacy universal kit of a holding state (OD-CS08/09): known at
    /// creation with no provenance row and no branch credit, as every
    /// starter was before CS-02.
    LegacyKit,
}

impl KitSource {
    /// Every kind, in the column `CHECK`'s order.
    pub const ALL: [Self; 5] = [
        Self::RacialCore,
        Self::Signature,
        Self::Tutorial,
        Self::Mission,
        Self::LegacyKit,
    ];

    /// The `source_kind` column text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RacialCore => "racial_core",
            Self::Signature => "signature",
            Self::Tutorial => "tutorial",
            Self::Mission => "mission",
            Self::LegacyKit => "legacy_kit",
        }
    }

    /// The `sgw_player_ability_grants.source_kind` creation writes, or
    /// `None` for the legacy kit, which gets no row.
    pub fn provenance_kind(self) -> Option<&'static str> {
        match self {
            Self::LegacyKit => None,
            other => Some(other.as_str()),
        }
    }
}

impl TryFrom<&str> for KitSource {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == value)
            .ok_or_else(|| format!("unknown char_creation_abilities.source_kind {value:?}"))
    }
}

/// One ability a profile starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KitAbility {
    pub ability_id: i32,
    pub source: KitSource,
}

/// One item a profile (or the debug kit) starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KitItem {
    pub item_id: i32,
    pub stack_size: i32,
}

/// One char_def's start profile.
#[derive(Debug, Clone, PartialEq)]
pub struct StartProfile {
    pub char_def_id: i32,
    /// `PRA_OPCORE_SOLDIER`, `SGU_FREE_JAFFA`, ... (the ledger's ProfileID).
    pub profile_id: String,
    /// `EAlignment` ordinal (1 Praxis, 2 SGU), as `sgw_player.alignment`.
    pub alignment: i32,
    /// `EArchetype` ordinal, as `sgw_player.archetype`.
    pub archetype: i32,
    /// The start world (`resources.worlds.world` spelling).
    pub world: String,
    /// The spawn point. No facing: world entry always sends rotation 0.
    pub position: [f32; 3],
    pub start_level: i32,
    /// Lock L2: add the debug kit at creation. Never derived from access
    /// level.
    pub debug_kit: bool,
    pub start_state: StartState,
    /// Ascending ability id.
    pub abilities: Vec<KitAbility>,
    /// Ascending item id.
    pub items: Vec<KitItem>,
}

/// What is wrong with a profile; creation refuses one that has any.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileProblem {
    EmptyProfileId,
    EmptyWorld,
    /// A coordinate is NaN or infinite.
    NonFinitePosition([f32; 3]),
    /// `(0, 0, 0)`: the unauthored sentinel, never a real spawn.
    OriginPosition,
    StartLevelOutOfRange(i32),
    NonPositiveStackSize {
        item_id: i32,
        stack_size: i32,
    },
    /// A canonical profile carrying a legacy-kit ability: the universal kit
    /// must never come back on a v6 profile (OD-CS01, OD-CS04).
    LegacyKitOnCanonicalProfile {
        ability_id: i32,
    },
}

impl ProfileProblem {
    /// The `reason` value for log lines.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::EmptyProfileId => "empty_profile_id",
            Self::EmptyWorld => "empty_world",
            Self::NonFinitePosition(_) => "non_finite_position",
            Self::OriginPosition => "origin_position",
            Self::StartLevelOutOfRange(_) => "start_level_out_of_range",
            Self::NonPositiveStackSize { .. } => "non_positive_stack_size",
            Self::LegacyKitOnCanonicalProfile { .. } => "legacy_kit_on_canonical_profile",
        }
    }
}

impl StartProfile {
    /// Every problem with this profile, empty when it is usable. Whether its
    /// world exists and has a loaded cell space is the caller's check: this
    /// crate knows neither.
    pub fn problems(&self) -> Vec<ProfileProblem> {
        let mut out = Vec::new();
        if self.profile_id.trim().is_empty() {
            out.push(ProfileProblem::EmptyProfileId);
        }
        if self.world.trim().is_empty() {
            out.push(ProfileProblem::EmptyWorld);
        }
        if self.position.iter().any(|c| !c.is_finite()) {
            out.push(ProfileProblem::NonFinitePosition(self.position));
        } else if self.position == [0.0; 3] {
            out.push(ProfileProblem::OriginPosition);
        }
        if !(1..=MAX_START_LEVEL).contains(&self.start_level) {
            out.push(ProfileProblem::StartLevelOutOfRange(self.start_level));
        }
        for item in &self.items {
            if item.stack_size <= 0 {
                out.push(ProfileProblem::NonPositiveStackSize {
                    item_id: item.item_id,
                    stack_size: item.stack_size,
                });
            }
        }
        if self.start_state == StartState::Canonical {
            for a in &self.abilities {
                if a.source == KitSource::LegacyKit {
                    out.push(ProfileProblem::LegacyKitOnCanonicalProfile {
                        ability_id: a.ability_id,
                    });
                }
            }
        }
        out
    }

    /// The abilities that get no provenance row (the legacy kit).
    pub fn plain_starters(&self) -> impl Iterator<Item = i32> + '_ {
        self.abilities
            .iter()
            .filter(|a| a.source.provenance_kind().is_none())
            .map(|a| a.ability_id)
    }
}

/// The debug kit (lock L2): what creation adds for a `debug_kit` profile, or
/// for a test's forced debug kit. Abilities get no provenance row; a gun
/// starts empty like every other (OD-CS13 amendment).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DebugKit {
    /// Ascending ability id.
    pub abilities: Vec<i32>,
    /// Ascending item id.
    pub items: Vec<KitItem>,
}

/// Every start profile and the debug kit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StartProfiles {
    /// Ascending `char_def_id`.
    profiles: Vec<StartProfile>,
    debug_kit: DebugKit,
}

impl StartProfiles {
    /// Build from rows in any order; profiles are kept by ascending
    /// `char_def_id`, so every "first profile" lookup is deterministic.
    pub fn new(mut profiles: Vec<StartProfile>, debug_kit: DebugKit) -> Self {
        profiles.sort_by_key(|p| p.char_def_id);
        Self {
            profiles,
            debug_kit,
        }
    }

    pub fn profiles(&self) -> &[StartProfile] {
        &self.profiles
    }

    pub fn debug_kit(&self) -> &DebugKit {
        &self.debug_kit
    }

    pub fn by_char_def(&self, char_def_id: i32) -> Option<&StartProfile> {
        self.profiles.iter().find(|p| p.char_def_id == char_def_id)
    }

    /// The start of a character of `alignment` and `archetype`: the lowest
    /// char_def with both; else (a hand-made row with an alignment no
    /// profile has, such as 0) the lowest char_def of the archetype; else
    /// (an archetype no profile has) the lowest char_def of the alignment;
    /// else `None`. Free Jaffa (SGU, Shol'va) go home to Dakara_E1, SGU
    /// humans to SGC_W1, Praxis to the Cellblock.
    pub fn home_for(&self, alignment: i32, archetype: i32) -> Option<&StartProfile> {
        let first = |f: &dyn Fn(&StartProfile) -> bool| self.profiles.iter().find(|p| f(p));
        first(&|p| p.alignment == alignment && p.archetype == archetype)
            .or_else(|| first(&|p| p.archetype == archetype))
            .or_else(|| first(&|p| p.alignment == alignment))
    }

    /// Where a new character is placed in `world`, if some profile starts
    /// there: the lowest such char_def's point. Case-insensitive, the way the
    /// GM console's world names are.
    pub fn start_position(&self, world: &str) -> Option<[f32; 3]> {
        self.profiles
            .iter()
            .find(|p| p.world.eq_ignore_ascii_case(world))
            .map(|p| p.position)
    }

    /// Every profile's start world, deduplicated, in char_def order.
    pub fn start_worlds(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for p in &self.profiles {
            if !out.contains(&p.world.as_str()) {
                out.push(&p.world);
            }
        }
        out
    }

    /// The abilities a GM / Debug NPC reset gives back to a character of
    /// `archetype` before its provenance rows are added: every ability any
    /// profile of the archetype starts with, and the debug kit when the
    /// character carries it. `None` when no profile has the archetype.
    pub fn reset_abilities(&self, archetype: i32, debug_kit: bool) -> Option<Vec<i32>> {
        let mut found = false;
        let mut out: Vec<i32> = Vec::new();
        for p in self.profiles.iter().filter(|p| p.archetype == archetype) {
            found = true;
            out.extend(p.abilities.iter().map(|a| a.ability_id));
        }
        if !found {
            return None;
        }
        if debug_kit {
            out.extend(self.debug_kit.abilities.iter().copied());
        }
        out.sort_unstable();
        out.dedup();
        Some(out)
    }
}
