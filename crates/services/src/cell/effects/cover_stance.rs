//! Cover Stance: the buff/unbuff pair behind ability 1451 (NA22).
//!
//! Seeded ability 1451 "Cover Stance" ("+200 Cover Defense",
//! `db/resources/Abilities/Seed/abilities.sql`) owns two effects:
//!
//! | Effect | Seed name | Script |
//! |---|---|---|
//! | 4565 | "Buff", "Single Target +100 CoverDefense" | [`CoverStance`] |
//! | 1742 | "Remove Stance", "Remove Stance Moniker" | [`RemoveCoverStance`] |
//!
//! Both rows are single-shot (`pulse_count = 1`) and shipped with no
//! `script_name` and no NVPs, so the effect pipeline had nothing to run for
//! them. NA22 names the scripts on the two rows. The server grants the
//! stance when an NPC reaches its reserved cover slot and removes it when
//! the NPC leaves the slot, leashes or dies ([`crate::cell::cover::stance`]).
//!
//! The magnitude is 100, from effect 4565's own description. The ability's
//! "+200" is not used (D-NA15): the effect row is the thing that applies,
//! and the ability tooltips disagree with their own effects elsewhere too
//! (1452 "Duck and Cover" says "+100 Crouching Defense", its only effect
//! 1743 says "+200 CoverDefense"). Effects 1746, 1747, 2003 and 4565 all
//! say "+100". A `CoverDefense` NVP on the effect row overrides it, so the
//! magnitude is tunable in the seed.
//!
//! **What the stat does (NA32).** The QR roll reads `COVER_DEFENSE` at
//! -0.01 QR per point (client `alias.xml:235`) for a defender that stands
//! at its cover node facing the attacker, so the stance is -1.0 QR, and
//! nothing when the NPC is flanked ([`crate::cell::combat::cover_shift`],
//! `abilities/damage_apply/cover_roll.rs`).
//!
//! **Pose:** no server-to-client movement-type or pose message exists
//! (D-NA10). Whether the client crouches an NPC that stands at a cover
//! marker is the owner experiment in
//! `docs/reverse-engineering/findings/cover-world-placement.md` Q4.

use super::{EffectContext, EffectScript};
use cimmeria_entity::stats::COVER_DEFENSE;

/// `COVER_DEFENSE` added by the stance when the effect row carries no
/// `CoverDefense` NVP: effect 4565's "+100 CoverDefense".
pub const COVER_STANCE_DEFENSE: i32 = 100;

/// The stance magnitude for `ctx.effect`: its `CoverDefense` NVP, or
/// [`COVER_STANCE_DEFENSE`].
fn magnitude(ctx: &EffectContext) -> i32 {
    match ctx.effect.param_i32("CoverDefense") {
        v if v > 0 => v,
        _ => COVER_STANCE_DEFENSE,
    }
}

/// Raise the target's `COVER_DEFENSE` (current and max) by the stance
/// magnitude. Effect 4565.
pub struct CoverStance;

impl EffectScript for CoverStance {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let delta = magnitude(ctx);
        let Some(stat) = ctx
            .space_mgr
            .get_entity_mut(ctx.target_id)
            .and_then(|t| t.stats.get_mut(COVER_DEFENSE))
        else {
            return;
        };
        let (min, cur, max) = (stat.min, stat.cur, stat.max);
        stat.update(min, cur + delta, max + delta);
        tracing::debug!(
            target: "abilities",
            event = "cover_stance_applied",
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            delta,
            cover_defense = cur + delta,
            "Cover Stance applied"
        );
    }
}

/// Lower the target's `COVER_DEFENSE` (current and max) by the stance
/// magnitude, never below the stat's minimum. Effect 1742.
pub struct RemoveCoverStance;

impl EffectScript for RemoveCoverStance {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let delta = magnitude(ctx);
        let Some(stat) = ctx
            .space_mgr
            .get_entity_mut(ctx.target_id)
            .and_then(|t| t.stats.get_mut(COVER_DEFENSE))
        else {
            return;
        };
        let (min, cur, max) = (stat.min, stat.cur, stat.max);
        let new_max = (max - delta).max(min);
        let new_cur = (cur - delta).clamp(min, new_max);
        stat.update(min, new_cur, new_max);
        tracing::debug!(
            target: "abilities",
            event = "cover_stance_removed",
            target_id = ctx.target_id,
            effect_id = ctx.effect.effect_id,
            delta,
            cover_defense = new_cur,
            "Cover Stance removed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::abilities::EffectDef;

    fn cover_defense(mgr: &SpaceManager, id: u32) -> (i32, i32) {
        let s = mgr
            .get_entity(id)
            .unwrap()
            .stats
            .get(COVER_DEFENSE)
            .unwrap();
        (s.cur, s.max)
    }

    fn run(mgr: &mut SpaceManager, script: &dyn EffectScript, effect: &EffectDef) {
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect,
            space_mgr: mgr,
        };
        script.on_apply(&mut ctx);
    }

    fn mgr_with_entity() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr
    }

    /// Apply then remove leaves the stat where it started; apply alone
    /// raises it by the seeded +100.
    #[test]
    fn stance_pair_raises_then_restores_cover_defense() {
        let mut mgr = mgr_with_entity();
        let effect = EffectDef::default();
        assert_eq!(cover_defense(&mgr, 1), (0, 0));
        run(&mut mgr, &CoverStance, &effect);
        assert_eq!(cover_defense(&mgr, 1), (100, 100));
        run(&mut mgr, &RemoveCoverStance, &effect);
        assert_eq!(cover_defense(&mgr, 1), (0, 0));
        // A stray remove never drives the stat negative.
        run(&mut mgr, &RemoveCoverStance, &effect);
        assert_eq!(cover_defense(&mgr, 1), (0, 0));
    }

    #[test]
    fn cover_defense_nvp_overrides_the_default_magnitude() {
        let mut mgr = mgr_with_entity();
        let mut effect = EffectDef::default();
        effect
            .params
            .insert("CoverDefense".to_string(), "40".to_string());
        run(&mut mgr, &CoverStance, &effect);
        assert_eq!(cover_defense(&mgr, 1), (40, 40));
    }
}
