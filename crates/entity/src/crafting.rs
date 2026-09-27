//! Player crafting state — disciplines, expertise, blueprints, paradigms, ASP.
//!
//! Mirrors the Python `Crafter` class (`deprecated/python/cell/Crafter.py`)
//! state half: the values a character carries between sessions and the values
//! the cell layer mutates during craft/research/alloy/reveng activity.
//!
//! Phase 1 just defines the struct + a couple of helpers and leaves the
//! activity mutators stubbed (those land in Phase 2/3).
//!
//! Persistence lives in the services crate (see
//! `crates/base-session/src/base/crafting/persistence.rs`); the entity crate is
//! sqlx-free by design.

use std::collections::HashMap;

/// Crafting state carried per player.
///
/// Field provenance (cross-referenced with `sgw_player.sql`):
/// - `discipline_ids` ↔ `sgw_player.discipline_ids integer[]` (which crafting
///   disciplines the character has unlocked).
/// - `expertise` ↔ `sgw_player_discipline_expertise` table (normalised
///   per-(player, discipline) — see the SQL file for rationale).
/// - `blueprint_ids` ↔ `sgw_player.blueprint_ids integer[]` (recipes the
///   character has learned).
/// - `applied_science_points` ↔ `sgw_player.applied_science_points integer`
///   (ASP currency spendable on discipline unlocks).
/// - `racial_paradigm_levels` ↔ `sgw_player.racial_paradigm_levels integer[]`,
///   keyed by paradigm id. Python stored these as a `{paradigm_id -> level}`
///   map; the SQL array is parallel to the paradigm id sequence. The map
///   shape here matches Python's intent and keeps the discipline-prerequisite
///   check (paradigm-id-keyed) ergonomic.
///
/// Python stored `discipline_ids` and `expertise` as one `{id -> expertise}`
/// map. We split them so the wire-emit path can iterate the *known* set in a
/// deterministic order (the `Vec`) without paying for a sort on every
/// `onUpdateDiscipline` resend.
#[derive(Debug, Clone, Default)]
pub struct CraftingState {
    /// Disciplines the player has unlocked, in insertion order. Parallel to
    /// `sgw_player.discipline_ids` (a Postgres `integer[]`). Persistence
    /// lives in `crates/base-session/src/base/crafting/persistence.rs` —
    /// `load_crafting_state` reads the array verbatim, so reload order is
    /// whatever order was on the row when last saved. (The parallel
    /// expertise rows in `sgw_player_discipline_expertise` are loaded
    /// `ORDER BY discipline_id`, but those land in `expertise`, not here.)
    pub discipline_ids: Vec<i32>,

    /// Expertise per discipline, in `[0, 100]`. Only contains entries for
    /// disciplines in `discipline_ids` — looking up an unknown discipline
    /// returns `None`. Python parity: `Crafter.disciplines[id]` raised
    /// `KeyError` for unknowns; here, callers must check `discipline_ids`
    /// first or use the `.get()` accessor.
    pub expertise: HashMap<i32, i32>,

    /// Blueprints (crafting recipes) the player has learned. Mirrors
    /// `sgw_player.blueprint_ids`. Sent to the client via
    /// `onUpdateKnownCrafts` (method 139) on world entry.
    pub blueprint_ids: Vec<i32>,

    /// Spendable Applied Science Points. Each `spendAppliedSciencePoints`
    /// call consumes 1; content-engine actions (level-up, mission rewards)
    /// add to this. Wire syncing uses `onEntityProperty(type=2, value)`,
    /// not a dedicated message.
    pub applied_science_points: i32,

    /// Racial paradigm levels, keyed by paradigm id. Discipline unlocks
    /// gate on `racial_paradigm_levels[discipline.racial_paradigm_id] >=
    /// discipline.racial_paradigm_level`. A character with no stored levels
    /// gets [`DEFAULT_RACIAL_PARADIGM_LEVELS`] (D-CR03), not Python's
    /// all-ones, under which no real root discipline was learnable
    /// (audit C-21).
    ///
    /// `i8` matches the wire encoding of `onUpdateRacialParadigmLevel`
    /// (method 138, payload `INT32 paradigmId, INT8 level`). Levels top out
    /// at 10 (D-CR03, the Racial Paradigm Guide text).
    pub racial_paradigm_levels: HashMap<i32, i8>,
}

/// Starting racial-paradigm levels, `(paradigm id, level)` in id order
/// (D-CR03, owner-approved): Common (1) at 5, so the four root disciplines,
/// which need Common 5, are learnable; Human, Goa'uld, Asgard and Ancient
/// (2-5) at 1. The `sgw_player.racial_paradigm_levels` column default in
/// `db/sgw/Players/Tables/sgw_player.sql` is the same list as an array.
pub const DEFAULT_RACIAL_PARADIGM_LEVELS: [(i32, i8); 5] = [(1, 5), (2, 1), (3, 1), (4, 1), (5, 1)];

impl CraftingState {
    /// Empty default state. New characters get this until their first
    /// `spendAppliedSciencePoints` call (which inserts a discipline +
    /// expertise=1 row).
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current expertise for a discipline, if the player has
    /// learned it. Returns `None` for unknown disciplines (matches
    /// "is this in `discipline_ids`?" without forcing the caller to
    /// scan the Vec).
    pub fn get_expertise(&self, discipline_id: i32) -> Option<i32> {
        self.expertise.get(&discipline_id).copied()
    }

    /// Set expertise for a discipline, clamped to `[0, 100]` per Python's
    /// `gainExpertise` hard cap. Does *not* update `discipline_ids`; the
    /// caller is responsible for adding the discipline to the known list
    /// when it transitions from unknown to known. (Phase 2 work — Phase 1
    /// only exposes the primitive.)
    pub fn set_expertise(&mut self, discipline_id: i32, value: i32) {
        let clamped = value.clamp(0, 100);
        self.expertise.insert(discipline_id, clamped);
    }

    /// Give a character with no stored paradigm levels the starting levels
    /// ([`DEFAULT_RACIAL_PARADIGM_LEVELS`]). Stored levels, even a partial
    /// list, are left alone. Characters created before D-CR03 have an empty
    /// array, so applying this on load covers them without a migration.
    pub fn apply_default_paradigm_levels(&mut self) {
        if self.racial_paradigm_levels.is_empty() {
            self.racial_paradigm_levels
                .extend(DEFAULT_RACIAL_PARADIGM_LEVELS);
        }
    }

    /// Whether the player knows `discipline_id`: it is in `discipline_ids`.
    /// A stray expertise row alone does not count.
    pub fn knows_discipline(&self, discipline_id: i32) -> bool {
        self.discipline_ids.contains(&discipline_id)
    }
}

// ── Wire-format serializers (Phase 1 minimum) ───────────────────────────────
//
// The mutation paths that *call* these land in Phase 2. We define the
// serializer here in Phase 1 so:
//   1. The wire shape is locked in by a test (byte-exact regression guard).
//   2. The world-entry resend path can call it without forward-referencing
//      a Phase 2 module.
//
// Method indices and wire shapes are sourced from
// `docs/protocol/client-method-dispatch-table.md`.

/// Serialize the `onUpdateDiscipline` args (method index 136).
///
/// Wire shape: `[disciplineSeqId: i32 LE][expertise: i32 LE]` — 8 bytes total.
///
/// Source: `docs/protocol/client-method-dispatch-table.md` row for index 136.
/// Python emit site: `Crafter.onUpdateDiscipline` (called from `gainExpertise`
/// and `spendAppliedSciencePoints`).
pub fn serialize_on_update_discipline(discipline_id: i32, expertise: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    buf.extend_from_slice(&discipline_id.to_le_bytes());
    buf.extend_from_slice(&expertise.to_le_bytes());
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let s = CraftingState::new();
        assert!(s.discipline_ids.is_empty());
        assert!(s.expertise.is_empty());
        assert!(s.blueprint_ids.is_empty());
        assert_eq!(s.applied_science_points, 0);
        assert!(s.racial_paradigm_levels.is_empty());
    }

    /// D-CR03: an empty map gets Common 5 and the other four at 1; stored
    /// levels, even a partial list, are kept as they are.
    #[test]
    fn default_paradigm_levels_apply_only_when_nothing_is_stored() {
        let mut fresh = CraftingState::new();
        fresh.apply_default_paradigm_levels();
        assert_eq!(fresh.racial_paradigm_levels.len(), 5);
        assert_eq!(fresh.racial_paradigm_levels[&1], 5, "Common starts at 5");
        for paradigm_id in 2..=5 {
            assert_eq!(fresh.racial_paradigm_levels[&paradigm_id], 1);
        }

        let mut stored = CraftingState::new();
        stored.racial_paradigm_levels.insert(2, 7);
        stored.apply_default_paradigm_levels();
        assert_eq!(stored.racial_paradigm_levels.len(), 1);
        assert_eq!(stored.racial_paradigm_levels[&2], 7);
    }

    #[test]
    fn get_expertise_returns_none_for_unknown() {
        let s = CraftingState::new();
        assert_eq!(s.get_expertise(42), None);
    }

    #[test]
    fn set_expertise_clamps_to_cap() {
        let mut s = CraftingState::new();
        s.set_expertise(5, 150);
        assert_eq!(s.get_expertise(5), Some(100));

        s.set_expertise(5, -10);
        assert_eq!(s.get_expertise(5), Some(0));

        s.set_expertise(5, 73);
        assert_eq!(s.get_expertise(5), Some(73));
    }

    /// Byte-exact regression guard for `onUpdateDiscipline` (method 136).
    ///
    /// Wire shape per the deep dive: `INT32 disciplineSeqId, INT32 expertise`.
    /// A regression here desyncs the client's discipline UI (wrong row,
    /// wrong percentage) — the client trusts both fields without
    /// validation. A swap or width change would manifest as the wrong
    /// discipline lighting up at the wrong percent.
    #[test]
    fn on_update_discipline_byte_layout_matches_spec() {
        // discipline_id = 0x11223344, expertise = 0x55667788
        let bytes = serialize_on_update_discipline(0x11223344, 0x55667788);
        assert_eq!(bytes.len(), 8, "onUpdateDiscipline must be exactly 8 bytes");
        // LE: low byte first.
        assert_eq!(
            bytes,
            vec![0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55],
            "field order: disciplineSeqId then expertise, both i32 LE",
        );
    }

    /// Edge case: expertise = 0 (transitional state immediately after
    /// `spendAppliedSciencePoints` but before the initial-value INSERT)
    /// and discipline_id = 0 (sentinel — should still serialize without
    /// special-casing).
    #[test]
    fn on_update_discipline_handles_zero_values() {
        let bytes = serialize_on_update_discipline(0, 0);
        assert_eq!(bytes, vec![0; 8]);
    }
}
