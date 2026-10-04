//! The logged `SpaceManager` face of the timed effect ledger. The
//! arithmetic is `CellEntity::apply_timed_effect` /
//! `remove_timed_effects_where` in `cimmeria-entity`; this layer adds the
//! telemetry rows (`stat_buff_applied`, `stat_buff_removed` with a `reason`,
//! `effect_bar_overflow`) that make an entry debuggable from SigNoz alone.
//!
//! Identity (instrumentation-discipline rule 5): `account_id` / `player_id`
//! name the **invoker**, the player whose cast or consumable put the entry
//! on; `entity_id` / `target_id` and `target_player_id` name the entity
//! that holds it. A stimpack's user is both.

use std::time::Instant;

use cimmeria_entity::cell_entity::{
    PlayerIdentity, TimedEffect, TimedEffectApplied, TimedEffectSpec, EFFECT_BAR_SLOTS_PER_SIDE,
};

use crate::cell::space_manager::SpaceManager;

/// Why an entry came off: the `reason` of its `stat_buff_removed` row. The
/// contract's list (work-packets "Timed effect ledger") plus the two the
/// ledger itself produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatBuffRemoval {
    /// Its duration ran out (the stat-buff tick).
    Expired,
    /// A new application took it off: the same source refreshing it, or a
    /// stimpack on the same stat.
    Replaced,
    /// Its effect script's `on_remove`.
    Removed,
    /// A second press of its toggle (AB-08).
    ToggledOff,
    /// A "Remove Effect of moniker" effect (AB-08, AB-10).
    RemovedByMoniker,
    /// Its entity died and the effect carries `EF_ClearOnDeath`.
    Death,
    /// Its entity took damage and the effect carries `EF_ClearOnDamage`
    /// (AB-11).
    Damage,
    /// Its entity was revived and the effect carries `EF_ClearOnRez`
    /// (AB-11).
    Revive,
    /// A bandolier slot change and `EF_RemoveOnBandolierSlotChange` (AB-11).
    BandolierSwap,
    /// A cleanse effect (AB-10).
    Cleansed,
}

impl StatBuffRemoval {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Expired => "expired",
            Self::Replaced => "replaced",
            Self::Removed => "removed",
            Self::ToggledOff => "toggled_off",
            Self::RemovedByMoniker => "removed_by_moniker",
            // "died" since decision 28: SigNoz queries and the UAT guide use it.
            Self::Death => "died",
            Self::Damage => "damage",
            Self::Revive => "revive",
            Self::BandolierSwap => "bandolier_swap",
            Self::Cleansed => "cleansed",
        }
    }
}

impl SpaceManager {
    /// Apply one timed effect to `target` (taking off whatever its stacking
    /// rule replaces). Returns `None`, logging why, when the target or every
    /// named stat is missing.
    pub fn apply_timed_effect(
        &mut self,
        target: u32,
        spec: TimedEffectSpec,
        now: Instant,
    ) -> Option<TimedEffectApplied> {
        let who = self.player_identity(spec.invoker_id);
        let target_who = self.player_identity(target);
        let (effect_id, ability_id, invoker_id) =
            (spec.effect_id, spec.ability_id, spec.invoker_id);
        let Some(entity) = self.get_entity_mut(target) else {
            tracing::warn!(
                target: "abilities",
                event = "stat_buff_skipped",
                reason = "target_missing",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = target,
                target_id = target,
                target_player_id = target_who.player_id,
                source_id = invoker_id,
                effect_id,
                ability_id,
                "timed effect target entity is gone; nothing applied"
            );
            return None;
        };
        let stat_ids: Vec<i32> = spec.stats.iter().map(|&(s, _)| s).collect();
        let before: Vec<Option<i32>> = stat_ids
            .iter()
            .map(|&s| entity.stats.get(s).map(|st| st.cur))
            .collect();
        let Some(out) = entity.apply_timed_effect(spec, now) else {
            tracing::warn!(
                target: "abilities",
                event = "stat_buff_skipped",
                reason = "stat_missing",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = target,
                target_id = target,
                target_player_id = target_who.player_id,
                source_id = invoker_id,
                effect_id,
                ability_id,
                stat_ids = ?stat_ids,
                "timed effect names only stats the entity does not have; nothing applied"
            );
            return None;
        };
        let after: Vec<Option<i32>> = stat_ids
            .iter()
            .map(|&s| entity.stats.get(s).map(|st| st.cur))
            .collect();
        let beneficial = out.applied.is_beneficial();
        let icons = entity.stat_buffs.bar_icons(beneficial);
        for old in &out.replaced {
            log_removed(target, who, target_who, old, StatBuffRemoval::Replaced);
        }
        tracing::info!(
            target: "abilities",
            event = "stat_buff_applied",
            decision_outcome = if out.replaced.is_empty() { "applied" } else { "replaced" },
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = target,
            target_id = target,
            target_player_id = target_who.player_id,
            source_id = invoker_id,
            effect_id,
            ability_id,
            stats = ?out.applied.stats.iter().map(|s| (s.stat_id, s.requested, s.shift.cur)).collect::<Vec<_>>(),
            stat_before = ?before,
            stat_after = ?after,
            missing_stats = ?out.missing_stats,
            duration_secs = out.applied.duration_secs,
            held = out.applied.expires_at.is_none(),
            beneficial,
            replaced_effect_ids = ?out.replaced.iter().map(|b| b.effect_id).collect::<Vec<_>>(),
            "timed effect applied"
        );
        if icons > EFFECT_BAR_SLOTS_PER_SIDE {
            // The client draws at most ten per side (B-73); the entry still
            // works, its icon may not show. INFO, not WARN: content can
            // legitimately stack this many, and nothing on the server is wrong.
            tracing::info!(
                target: "abilities",
                event = "effect_bar_overflow",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = target,
                target_id = target,
                target_player_id = target_who.player_id,
                effect_id,
                ability_id,
                beneficial,
                icons,
                slots = EFFECT_BAR_SLOTS_PER_SIDE,
                "more timed effects on one side than the client's effect bar shows"
            );
        }
        Some(out)
    }

    /// Take off every entry on `target` for which `pred` holds, logging each
    /// with `why`. Returns them.
    pub fn remove_timed_effects(
        &mut self,
        target: u32,
        why: StatBuffRemoval,
        pred: impl Fn(&TimedEffect) -> bool,
    ) -> Vec<TimedEffect> {
        let target_who = self.player_identity(target);
        let invokers: Vec<u32> = match self.get_entity(target) {
            Some(e) => e
                .stat_buffs
                .entries
                .iter()
                .filter(|b| pred(b))
                .map(|b| b.invoker_id)
                .collect(),
            None => return Vec::new(),
        };
        let whos: Vec<PlayerIdentity> = invokers.iter().map(|&i| self.player_identity(i)).collect();
        let Some(entity) = self.get_entity_mut(target) else {
            return Vec::new();
        };
        let removed = entity.remove_timed_effects_where(pred);
        for (entry, who) in removed.iter().zip(whos) {
            log_removed(target, who, target_who, entry, why);
        }
        removed
    }

    /// Take off every entry on `target` whose ability carries `moniker_id`
    /// (AB-08's stance exclusivity, AB-10's cleanses).
    pub fn remove_timed_effects_by_moniker(
        &mut self,
        target: u32,
        moniker_id: i64,
    ) -> Vec<TimedEffect> {
        self.remove_timed_effects(target, StatBuffRemoval::RemovedByMoniker, |e| {
            e.has_moniker(moniker_id)
        })
    }

    /// The moniker ids of `ability_id`, for a [`TimedEffectSpec`]. Empty for
    /// an unknown ability.
    pub fn ability_moniker_ids(&self, ability_id: i32) -> Vec<i64> {
        self.ability_defs
            .get(&ability_id)
            .map(|d| d.moniker_ids.clone())
            .unwrap_or_default()
    }

    /// Every entity with an entry due to expire by `now`, sorted.
    pub fn entities_with_expired_stat_buffs(&self, now: Instant) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .all_entity_ids()
            .into_iter()
            .filter(|&eid| {
                self.get_entity(eid)
                    .is_some_and(|e| e.stat_buffs.entries.iter().any(|b| b.is_expired(now)))
            })
            .collect();
        out.sort_unstable();
        out
    }

    /// Every entity whose ledger holds an entry or owes a timer, sorted.
    pub fn entities_with_stat_buff_work(&self) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .all_entity_ids()
            .into_iter()
            .filter(|&eid| {
                self.get_entity(eid)
                    .is_some_and(|e| !e.stat_buffs.is_idle())
            })
            .collect();
        out.sort_unstable();
        out
    }
}

fn log_removed(
    target: u32,
    who: PlayerIdentity,
    target_who: PlayerIdentity,
    entry: &TimedEffect,
    why: StatBuffRemoval,
) {
    tracing::info!(
        target: "abilities",
        event = "stat_buff_removed",
        decision_outcome = "removed",
        reason = why.reason(),
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = target,
        target_id = target,
        target_player_id = target_who.player_id,
        source_id = entry.invoker_id,
        effect_id = entry.effect_id,
        ability_id = entry.ability_id,
        restored = ?entry.stats.iter().map(|s| (s.stat_id, -s.shift.cur)).collect::<Vec<_>>(),
        "timed effect removed"
    );
}
