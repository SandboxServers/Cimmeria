//! The logged `SpaceManager` face of the stat-buff ledger. The arithmetic
//! is `CellEntity::apply_stat_buff` / `remove_stat_buffs_where` in
//! `cimmeria-entity`; this layer adds the telemetry rows (`stat_buff_applied`,
//! `stat_buff_removed` with a `reason`) that make a buff debuggable from
//! SigNoz alone.

use std::time::Instant;

use cimmeria_entity::cell_entity::{ActiveStatBuff, StatBuffApplied, StatBuffSpec};

use crate::cell::space_manager::SpaceManager;

/// Why a buff came off: the `reason` of its `stat_buff_removed` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatBuffRemoval {
    /// Its duration ran out (the stat-buff tick).
    Expired,
    /// A buff on the same stat replaced it.
    Replaced,
    /// Its effect script's `on_remove`.
    Removed,
    /// Its entity died and the effect carries `EF_ClearOnDeath`.
    Died,
}

impl StatBuffRemoval {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Expired => "expired",
            Self::Replaced => "replaced",
            Self::Removed => "removed",
            Self::Died => "died",
        }
    }
}

impl SpaceManager {
    /// Apply one stat buff to `target` (replacing a buff on the same stat).
    /// Returns `None`, logging why, when the target or the stat is missing.
    pub fn apply_stat_buff(
        &mut self,
        target: u32,
        spec: StatBuffSpec,
        now: Instant,
    ) -> Option<StatBuffApplied> {
        let id = self.player_identity(target);
        let Some(entity) = self.get_entity_mut(target) else {
            tracing::warn!(
                target: "abilities",
                event = "stat_buff_skipped",
                reason = "target_missing",
                entity_id = target,
                effect_id = spec.effect_id,
                ability_id = spec.ability_id,
                stat_id = spec.stat_id,
                "stat buff target entity is gone; nothing applied"
            );
            return None;
        };
        let before = entity.stats.get(spec.stat_id).map(|s| s.cur);
        let Some(out) = entity.apply_stat_buff(spec, now) else {
            tracing::warn!(
                target: "abilities",
                event = "stat_buff_skipped",
                reason = "stat_missing",
                entity_id = target,
                account_id = id.account_id,
                player_id = id.player_id,
                effect_id = spec.effect_id,
                ability_id = spec.ability_id,
                stat_id = spec.stat_id,
                "stat buff names a stat the entity does not have; nothing applied"
            );
            return None;
        };
        let after = entity.stats.get(spec.stat_id).map(|s| s.cur);
        if let Some(old) = &out.replaced {
            log_removed(target, id, old, StatBuffRemoval::Replaced, before, None);
        }
        tracing::info!(
            target: "abilities",
            event = "stat_buff_applied",
            decision_outcome = if out.replaced.is_some() { "replaced" } else { "applied" },
            entity_id = target,
            account_id = id.account_id,
            player_id = id.player_id,
            source_id = spec.invoker_id,
            effect_id = spec.effect_id,
            ability_id = spec.ability_id,
            stat_id = spec.stat_id,
            requested = spec.delta,
            applied = out.applied.shift.cur,
            max_widened = out.applied.shift.max,
            stat_before = before,
            stat_after = after,
            duration_secs = spec.duration_secs,
            replaced_effect_id = out.replaced.as_ref().map(|b| b.effect_id),
            "stat buff applied"
        );
        Some(out)
    }

    /// Take off every buff on `target` for which `pred` holds, logging each
    /// with `why`. Returns them.
    pub fn remove_stat_buffs(
        &mut self,
        target: u32,
        why: StatBuffRemoval,
        pred: impl Fn(&ActiveStatBuff) -> bool,
    ) -> Vec<ActiveStatBuff> {
        let id = self.player_identity(target);
        let Some(entity) = self.get_entity_mut(target) else {
            return Vec::new();
        };
        let befores: Vec<(i32, Option<i32>)> = entity
            .stat_buffs
            .buffs
            .iter()
            .filter(|b| pred(b))
            .map(|b| (b.stat_id, entity.stats.get(b.stat_id).map(|s| s.cur)))
            .collect();
        let removed = entity.remove_stat_buffs_where(pred);
        for (buff, (_, before)) in removed.iter().zip(befores) {
            let after = entity.stats.get(buff.stat_id).map(|s| s.cur);
            log_removed(target, id, buff, why, before, after);
        }
        removed
    }

    /// Every entity with a buff due to expire by `now`, sorted.
    pub fn entities_with_expired_stat_buffs(&self, now: Instant) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .all_entity_ids()
            .into_iter()
            .filter(|&eid| {
                self.get_entity(eid)
                    .is_some_and(|e| e.stat_buffs.buffs.iter().any(|b| b.expires_at <= now))
            })
            .collect();
        out.sort_unstable();
        out
    }

    /// Every entity whose ledger holds a buff or owes a timer, sorted.
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
    id: cimmeria_entity::cell_entity::PlayerIdentity,
    buff: &ActiveStatBuff,
    why: StatBuffRemoval,
    before: Option<i32>,
    after: Option<i32>,
) {
    tracing::info!(
        target: "abilities",
        event = "stat_buff_removed",
        decision_outcome = "removed",
        reason = why.reason(),
        entity_id = target,
        account_id = id.account_id,
        player_id = id.player_id,
        effect_id = buff.effect_id,
        ability_id = buff.ability_id,
        stat_id = buff.stat_id,
        restored = -buff.shift.cur,
        stat_before = before,
        stat_after = after,
        "stat buff removed"
    );
}
