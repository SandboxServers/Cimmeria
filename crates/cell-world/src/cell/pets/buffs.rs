//! Owner buffs on a pet (pets PT-08): the stat ledger behind Holy Warrior,
//! To The Death and Lord's Concentration.
//!
//! A buff is a [`PetBuff`] on the pet's [`PetState::buffs`]. It records the
//! delta each stat really moved, so taking the buff off restores exactly
//! that much, whatever else changed the stat meanwhile (python's
//! `statChanges`, `AbilityManager.py:438-441`, removed at `:342-343`).
//!
//! **Bounds widen instead of clamping (a deliberate deviation).** Several
//! combat stats default to `[0, 0]` (`DEFENSE`, `INTERRUPT_RES`), which would
//! clamp Holy Warrior's -100 Defense and Lord's Concentration's +50 away,
//! as python's clamp did. The ledger widens that one pet's bound to admit the
//! delta instead. Nothing else about the stat changes.
//!
//! The buff lives and dies with the pet entity: a despawn, a replacing
//! summon or the owner's death (which despawns the pet, D-PT08) takes every
//! buff with it, so the owner carries no "buff on" state. Expiry is driven
//! by the owner-pet tick in `cimmeria-cell-combat`
//! (`use_ability::owner_pet::owner_pet_tick`), which also carries out To The
//! Death ([`PetState::doomed_at`]).
//!
//! Log target `pets.buff`.
//!
//! [`PetState::buffs`]: cimmeria_entity::cell_entity::PetState::buffs
//! [`PetState::doomed_at`]: cimmeria_entity::cell_entity::PetState::doomed_at

use cimmeria_entity::cell_entity::PetState;
use std::time::Instant;

use cimmeria_entity::cell_entity::{PetBuff, PlayerIdentity};
use cimmeria_entity::stats::StatList;

use super::super::space_manager::SpaceManager;

/// Why a buff came off a pet: the `reason` of the `buff_removed` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuffRemoval {
    /// The owner pressed the toggle again.
    ToggledOff,
    /// Its duration ran out.
    Expired,
    /// The same effect was applied again and replaced it.
    Refreshed,
    /// An effect script's `on_remove`.
    Removed,
}

impl BuffRemoval {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::ToggledOff => "toggled_off",
            Self::Expired => "expired",
            Self::Refreshed => "refreshed",
            Self::Removed => "removed",
        }
    }
}

/// Move `stat` by `delta` on `stats`, widening its bound rather than
/// clamping (see the module docs). Returns the delta applied and the value
/// before it, or `None` when the entity has no such stat.
pub fn shift_stat(stats: &mut StatList, stat: i32, delta: i32) -> Option<(i32, i32)> {
    let s = stats.get_mut(stat)?;
    let before = s.cur;
    let wanted = before.saturating_add(delta);
    if wanted < s.min {
        s.set_min(wanted);
    }
    if wanted > s.max {
        s.set_max(wanted);
    }
    Some((s.change(delta), before))
}

impl SpaceManager {
    /// The identity of the player who summoned `pet`, for a `pets.buff`
    /// row (captured at summon; `UNKNOWN` when not a pet).
    pub fn pet_summoner_identity(&self, pet: u32) -> PlayerIdentity {
        super::teardown::owner_identity(self, pet, self.pets.owner_of(pet))
    }

    /// Whether `pet` carries the buff of `effect_id`.
    pub fn has_pet_buff(&self, pet: u32, effect_id: i32) -> bool {
        self.get_entity(pet)
            .and_then(|e| e.extensions.get::<PetState>())
            .is_some_and(|p| p.buffs.iter().any(|b| b.effect_id == effect_id))
    }

    /// Apply `mods` (`(stat, delta)`) to `pet` as the buff of `effect_id`,
    /// replacing a buff of the same effect first (a refresh: its deltas are
    /// taken back, then the new ones applied). `expires_at = None` is a
    /// toggle. Returns the deltas applied, or `None` when `pet` is not a
    /// pet. Logs `buff_applied` (and `buff_removed reason=refreshed`).
    pub fn apply_pet_buff(
        &mut self,
        pet: u32,
        effect_id: i32,
        ability_id: i32,
        mods: &[(i32, i32)],
        expires_at: Option<Instant>,
    ) -> Option<Vec<(i32, i32)>> {
        self.get_entity(pet)?.extensions.get::<PetState>()?;
        let _ = self.remove_pet_buff(pet, effect_id, BuffRemoval::Refreshed);
        let entity = self.get_entity_mut(pet)?;
        let mut applied = Vec::with_capacity(mods.len());
        let mut before = Vec::with_capacity(mods.len());
        for &(stat, delta) in mods {
            if let Some((moved, was)) = shift_stat(&mut entity.stats, stat, delta) {
                applied.push((stat, moved));
                before.push((stat, was));
            }
        }
        let after: Vec<(i32, i32)> = applied
            .iter()
            .map(|&(stat, _)| (stat, entity.stats.get(stat).map_or(0, |s| s.cur)))
            .collect();
        let template_id = entity.template_id;
        let state = entity.extensions.get_mut::<PetState>()?;
        state.buffs.push(PetBuff {
            effect_id,
            ability_id,
            stat_deltas: applied.clone(),
            expires_at,
        });
        let owner_id = state.owner_id;
        let id = self.pet_summoner_identity(pet);
        tracing::debug!(
            target: "pets.buff",
            event = "buff_applied",
            decision_outcome = "buff_applied",
            entity_id = pet,
            pet_id = pet,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            template_id,
            ability_id,
            effect_id,
            toggle = expires_at.is_none(),
            duration_secs = expires_at.map(|at| at.saturating_duration_since(Instant::now()).as_secs_f32()),
            stat_deltas = ?applied,
            stats_before = ?before,
            stats_after = ?after,
            "owner buff applied to the pet"
        );
        Some(applied)
    }

    /// Take the buff of `effect_id` off `pet`, restoring each stat by the
    /// delta it applied. Returns the buff, or `None` when there was none.
    /// Logs `buff_removed` with `reason`.
    pub fn remove_pet_buff(
        &mut self,
        pet: u32,
        effect_id: i32,
        why: BuffRemoval,
    ) -> Option<PetBuff> {
        let entity = self.get_entity_mut(pet)?;
        let state = entity.extensions.get_mut::<PetState>()?;
        let idx = state.buffs.iter().position(|b| b.effect_id == effect_id)?;
        let buff = state.buffs.remove(idx);
        let owner_id = state.owner_id;
        let template_id = entity.template_id;
        let mut before = Vec::with_capacity(buff.stat_deltas.len());
        let mut after = Vec::with_capacity(buff.stat_deltas.len());
        for &(stat, delta) in &buff.stat_deltas {
            if let Some((_, was)) = shift_stat(&mut entity.stats, stat, -delta) {
                before.push((stat, was));
                after.push((stat, entity.stats.get(stat).map_or(0, |s| s.cur)));
            }
        }
        let id = self.pet_summoner_identity(pet);
        tracing::debug!(
            target: "pets.buff",
            event = "buff_removed",
            decision_outcome = "buff_removed",
            entity_id = pet,
            pet_id = pet,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            template_id,
            ability_id = buff.ability_id,
            effect_id,
            reason = why.reason(),
            stat_deltas = ?buff.stat_deltas,
            stats_before = ?before,
            stats_after = ?after,
            "owner buff removed from the pet"
        );
        Some(buff)
    }

    /// Every `(pet, effect_id)` whose buff has run out by `now`. Toggles
    /// never expire.
    pub fn expired_pet_buffs(&self, now: Instant) -> Vec<(u32, i32)> {
        let mut out = Vec::new();
        for (pet, _) in self.pets.pairs() {
            let Some(state) = self
                .get_entity(pet)
                .and_then(|e| e.extensions.get::<PetState>())
            else {
                continue;
            };
            out.extend(
                state
                    .buffs
                    .iter()
                    .filter(|b| b.expires_at.is_some_and(|at| now >= at))
                    .map(|b| (pet, b.effect_id)),
            );
        }
        out.sort_unstable();
        out
    }

    /// Every pet whose To The Death timer has run out by `now`.
    pub fn doomed_pets_due(&self, now: Instant) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .pets
            .pairs()
            .into_iter()
            .filter(|&(pet, _)| {
                self.get_entity(pet)
                    .and_then(|e| e.extensions.get::<PetState>())
                    .and_then(|p| p.doomed_at)
                    .is_some_and(|at| now >= at)
            })
            .map(|(pet, _)| pet)
            .collect();
        out.sort_unstable();
        out
    }

    /// Whether any pet carries a buff or a doom, so the owner-pet tick can
    /// return at once on a world with none.
    pub fn any_pet_buff_or_doom(&self) -> bool {
        self.pets.pairs().into_iter().any(|(pet, _)| {
            self.get_entity(pet)
                .and_then(|e| e.extensions.get::<PetState>())
                .is_some_and(|p| !p.buffs.is_empty() || p.doomed_at.is_some())
        })
    }
}
