//! Lab target dummies (ability-mechanics AB-L2, decision D-AU6).
//!
//! A dummy is an ordinary template NPC spawned by the GM `.dummy` command
//! and marked with a [`LabDummy`] extension. The mark is what makes it a
//! dummy:
//!
//! - **No AI turn.** `.dummy` also puts the [`super::TrainingDummy`] mark on
//!   it, the same mark a seeded training dummy carries (Debug Area D-DA7), and
//!   [`SpaceManager::ai_driven_npc_entity_ids`] leaves a marked NPC out, so it
//!   never fights back, chases, leashes, patrols or walks home, however much
//!   threat it takes. `.spawn` plus `.aggro off` only stops a mob noticing
//!   you, not hitting back (the reason for D-AU6).
//! - **A lifetime and an owner.** It despawns [`LAB_DUMMY_LIFETIME`] after
//!   it was placed, or when its owner logs out; the sweeps live with the
//!   command in `cimmeria-cell-console`.
//! - **Optionally, one ability.** A `.dummy caster` also carries a
//!   [`LabCaster`]: still no AI turn, but the console's 1 Hz sweep launches
//!   that one ability at the owner every interval, through the same
//!   `handle_use_ability` launch the NPC fight tick uses, so warmups,
//!   interrupts, effects and telemetry are the real ones (AB-U20, AB-U22).
//!
//! The extension is runtime only: nothing reaches the client or the
//! database, and a dummy has no `spawnlist` row and never respawns.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::{MobAggression, PlayerIdentity};

use super::SpaceManager;

/// How long a dummy stands before it despawns on its own (D-AU6).
pub const LAB_DUMMY_LIFETIME: Duration = Duration::from_secs(600);

/// The Health a dummy is given, current and max (D-AU6): enough that no
/// test run kills it by accident. The seeded training dummies' figure.
pub const LAB_DUMMY_HEALTH: i32 = super::TRAINING_DUMMY_HEALTH;

/// The most dummies one GM may have standing at once, so a stuck macro
/// cannot fill a zone.
pub const LAB_DUMMY_MAX_PER_OWNER: usize = 4;

/// The mark on a dummy NPC (`CellEntity::extensions`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabDummy {
    /// The GM who placed it. Only they can clear it (colo rule: a GM touches
    /// only what their own lab characters spawned).
    pub owner_id: u32,
    /// The owner's identity at placement, so the despawn row still names
    /// them after they logged out.
    pub owner_identity: PlayerIdentity,
    /// The disposition it was placed with (`Hostile` or `Friendly`).
    pub disposition: MobAggression,
    /// When it despawns on its own.
    pub expires_at: Instant,
}

/// How often a caster dummy casts when no interval is named.
pub const LAB_CASTER_DEFAULT_INTERVAL: Duration = Duration::from_secs(8);

/// The second mark on a `.dummy caster`: the one ability it casts at its
/// owner, and when. Only ever beside a [`LabDummy`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabCaster {
    /// The ability it casts. It is also in the dummy's known abilities, or
    /// the launch would refuse it.
    pub ability_id: i32,
    /// Time between two cast attempts.
    pub interval: Duration,
    /// When it next tries. Advanced by one interval on every attempt, cast
    /// or held, so a held caster never bursts when it resumes.
    pub next_cast_at: Instant,
}

impl SpaceManager {
    /// Whether `entity_id` is a lab dummy.
    pub fn is_lab_dummy(&self, entity_id: u32) -> bool {
        self.get_entity(entity_id)
            .is_some_and(|e| e.extensions.contains::<LabDummy>())
    }

    /// The dummies `owner_id` placed, sorted.
    pub fn lab_dummies_of(&self, owner_id: u32) -> Vec<u32> {
        self.lab_dummies_where(|d| d.owner_id == owner_id)
    }

    /// The dummies whose lifetime ran out by `now`, sorted.
    pub fn expired_lab_dummies(&self, now: Instant) -> Vec<u32> {
        self.lab_dummies_where(|d| d.expires_at <= now)
    }

    /// The caster dummies whose next cast is due by `now`, sorted.
    pub fn lab_casters_due(&self, now: Instant) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .spaces
            .values()
            .flat_map(|s| s.entities.values())
            .filter(|e| e.extensions.contains::<LabDummy>())
            .filter(|e| {
                e.extensions
                    .get::<LabCaster>()
                    .is_some_and(|c| c.next_cast_at <= now)
            })
            .map(|e| e.entity_id.0 as u32)
            .collect();
        out.sort_unstable();
        out
    }

    fn lab_dummies_where(&self, pred: impl Fn(&LabDummy) -> bool) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .spaces
            .values()
            .flat_map(|s| s.entities.values())
            .filter(|e| e.extensions.get::<LabDummy>().is_some_and(&pred))
            .map(|e| e.entity_id.0 as u32)
            .collect();
        out.sort_unstable();
        out
    }
}
