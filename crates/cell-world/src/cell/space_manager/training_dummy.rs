//! Training dummies: NPCs that never fight back (Debug Area D-DA7, ability
//! lab D-AU6).
//!
//! The [`TrainingDummy`] mark is the one "never attacks" switch. Two things
//! put it on an NPC:
//!
//! - the GM `.dummy` command, beside its [`super::LabDummy`] owner and
//!   lifetime bookkeeping;
//! - a seeded template with `entity_templates.training_dummy = true`, at
//!   spawn (`spawn_npc_from_record_into`), so a placed spawn in the Debug
//!   Area's dummies range is one too, and so is a GM `.spawn` of that
//!   template.
//!
//! What the mark does:
//!
//! - **No AI turn.** [`SpaceManager::ai_driven_npc_entity_ids`] leaves a
//!   marked NPC out, so it never attacks, chases, leashes, patrols or walks
//!   home, however much threat it takes. A NEUTRAL faction-10 NPC without the
//!   mark still fires back once shot, which is why D-DA7 asked for it.
//! - **[`TRAINING_DUMMY_HEALTH`] Health** for a seeded dummy (the `.dummy`
//!   command sets the same figure itself). A friendly seeded dummy starts at
//!   half of it, so a heal has room to land and shows its size.
//! - **An ally for beneficial casts** when the caster may not attack it (the
//!   friendly one): a heal or buff aimed at it lands on it instead of falling
//!   back to the caster (`use_ability::support_shot::classify`).
//! - **Combat ends on its own.** With no AI turn there is no leash to drain a
//!   dummy from its attackers' combat sets, so a player who hit one stayed in
//!   combat (no regen, no out-of-combat holster) until the dummy despawned,
//!   which a seeded dummy never does. [`SpaceManager::quiet_training_dummies`]
//!   finds a dummy whose threat has not moved for
//!   [`TRAINING_DUMMY_COMBAT_TIMEOUT`]; the cell's 1 Hz sweep in
//!   `cimmeria-cell-combat` then releases it from every player's combat and
//!   puts its Health back to [`TrainingDummy::rest_health`], so damage never
//!   builds up across testers into a kill (review F2). A kill that happens
//!   anyway pays no XP (`death::side_effects::grant_kill_xp`).

use std::time::{Duration, Instant};

use super::SpaceManager;

/// The Health a training dummy is given, current and max: enough that no
/// test run kills it by accident. Shared with the `.dummy` command.
pub const TRAINING_DUMMY_HEALTH: i32 = 1_000_000;

/// How long a dummy's threat list may stand still (no hit landed, no threat
/// added) before its attackers leave combat with it.
pub const TRAINING_DUMMY_COMBAT_TIMEOUT: Duration = Duration::from_secs(10);

/// The "never fights back" mark (`CellEntity::extensions`), with the state
/// the combat-release sweep keeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrainingDummy {
    /// The threat total the last sweep saw.
    pub threat_seen: f32,
    /// When that total last changed (or the mark was placed).
    pub quiet_since: Instant,
    /// The Health the dummy goes back to when its fight goes quiet: max for
    /// a hostile dummy, half for the friendly heal target.
    pub rest_health: i32,
}

impl TrainingDummy {
    /// A fresh mark: no threat seen, quiet from `now`, resting at
    /// [`TRAINING_DUMMY_HEALTH`].
    pub fn new(now: Instant) -> Self {
        Self {
            threat_seen: 0.0,
            quiet_since: now,
            rest_health: TRAINING_DUMMY_HEALTH,
        }
    }

    /// The same mark resting at `rest_health`.
    pub fn with_rest_health(self, rest_health: i32) -> Self {
        Self {
            rest_health,
            ..self
        }
    }
}

impl SpaceManager {
    /// Whether `entity_id` is a training dummy (seeded or `.dummy`).
    pub fn is_training_dummy(&self, entity_id: u32) -> bool {
        self.get_entity(entity_id)
            .is_some_and(|e| e.extensions.contains::<TrainingDummy>())
    }

    /// The training dummies whose non-empty threat list has not changed for
    /// [`TRAINING_DUMMY_COMBAT_TIMEOUT`] by `now`, sorted. Every dummy's
    /// tracking state is advanced: a changed total restarts its quiet
    /// window, and an empty list keeps it at zero. A returned dummy is
    /// expected to be released (its threat list cleared), which restarts
    /// the window on the next sweep.
    pub fn quiet_training_dummies(&mut self, now: Instant) -> Vec<u32> {
        let mut out = Vec::new();
        for space in self.spaces.values_mut() {
            for e in space.entities.values_mut() {
                // The mark first: this runs over every entity every second.
                if !e.extensions.contains::<TrainingDummy>() {
                    continue;
                }
                let total: f32 = e.threat_list.values().sum();
                let empty = e.threat_list.is_empty();
                let id = e.entity_id.0 as u32;
                let Some(mark) = e.extensions.get_mut::<TrainingDummy>() else {
                    continue;
                };
                if empty {
                    mark.threat_seen = 0.0;
                    mark.quiet_since = now;
                } else if total != mark.threat_seen {
                    mark.threat_seen = total;
                    mark.quiet_since = now;
                } else if now.saturating_duration_since(mark.quiet_since)
                    >= TRAINING_DUMMY_COMBAT_TIMEOUT
                {
                    out.push(id);
                }
            }
        }
        out.sort_unstable();
        out
    }
}
