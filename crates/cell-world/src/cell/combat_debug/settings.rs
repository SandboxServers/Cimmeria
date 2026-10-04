//! One watcher's combat-debug toggles.
//!
//! These are the `SGWAbilityManager.def` CELL_PRIVATE properties
//! `bCombatDebug`, `bCombatVerboseDebug`, `debugAbilityList` and
//! `debugAbilityTargetID`, plus the heal toggle (`toggleHealDebug`) and the
//! mob list `gmDebugAbilityOnMob` fills. They live in the cell's
//! [`super::CombatDebug`] map rather than on the entity: nothing else reads
//! them, and the map being empty is what keeps the pipeline from noting
//! anything. Like `gmSetGodMode` they are memory only: a relog, or a world
//! change that recreates the entity, starts with everything off.
//!
//! `debugEffectList` has no setter the client can reach and no command sets
//! it, so it is not kept.

/// The toggles of one watcher (a GM, or a crafted caller of cells 2, 3, 6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DebugSettings {
    /// The `player_id` the watcher had when it turned debugging on. A
    /// watcher whose entity id now names someone else is dropped.
    pub player_id: Option<i32>,
    /// `bCombatDebug`: one line per hostile hit the watcher casts or takes.
    pub combat: bool,
    /// `bCombatVerboseDebug`: the hostile lines plus every plan, NVP entry,
    /// ledger entry and pulse.
    pub verbose: bool,
    /// Heal debug: one line per beneficial cast the watcher casts or
    /// receives.
    pub heal: bool,
    /// `debugAbilityList`: these abilities print whatever the other toggles
    /// say, when the watcher casts them or is touched by them.
    pub abilities: Vec<i32>,
    /// `gmDebugAbilityOnMob`: `(mob entity id, ability id)`, ability 0 for
    /// every ability of the mob. That mob's casts print to the watcher.
    pub mobs: Vec<(u32, i32)>,
    /// `debugAbilityTargetID`: who receives the lines. `None` is the
    /// watcher itself.
    pub target: Option<u32>,
}

impl DebugSettings {
    /// Nothing is on and the target is the default: the entry can go.
    pub fn is_idle(&self) -> bool {
        !self.combat
            && !self.verbose
            && !self.heal
            && self.abilities.is_empty()
            && self.mobs.is_empty()
            && self.target.is_none()
    }
}
