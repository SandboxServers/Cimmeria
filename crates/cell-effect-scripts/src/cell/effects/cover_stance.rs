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
//! the NPC leaves the slot, leashes or dies
//! ([`cimmeria_cell_world::cell::cover::stance`]).
//!
//! The magnitude is 100, from effect 4565's own description. The ability's
//! "+200" is not used (D-NA15): the effect row is the thing that applies,
//! and the ability tooltips disagree with their own effects elsewhere too
//! (1452 "Duck and Cover" says "+100 Crouching Defense", its only effect
//! 1743 says "+200 CoverDefense"). Effects 1746, 1747, 2003 and 4565 all
//! say "+100". A `CoverDefense` NVP on the effect row overrides it, so the
//! magnitude is tunable in the seed.
//!
//! **What the stat does (NA32, D-NA15a).** Cover is a damage reduction
//! rated by the node (10-60%). `COVER_DEFENSE` adds 0.1 percentage points
//! per point inside that band, so the stance is +10 points, for a defender
//! at its cover node facing the attacker, and nothing when the NPC is
//! flanked (`cimmeria_services::cell::combat::cover_reduction`,
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
            cast_id = ctx.row_ids().cast_id,
            account_id = ctx.row_ids().account_id,
            player_id = ctx.row_ids().player_id,
            target_player_id = ctx.row_ids().target_player_id,
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
            cast_id = ctx.row_ids().cast_id,
            account_id = ctx.row_ids().account_id,
            player_id = ctx.row_ids().player_id,
            target_player_id = ctx.row_ids().target_player_id,
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

/// Live-DB guard, moved from `cimmeria-cell-world`'s `live_db_use_cover`
/// with the scripts it checks.
#[cfg(test)]
mod live_db_tests {
    use cimmeria_cell_world::cell::cover::{self, COVER_STANCE_EFFECT, COVER_STANCE_REMOVE_EFFECT};

    use crate::cell::spawner::load_effect_defs;
    use crate::test_support::require_db_or_skip;

    /// Effects 4565 and 1742 name the Cover Stance scripts. Without them the
    /// stance is tracked but no stat changes (`cover.stance event=effect_missing`).
    #[tokio::test]
    async fn cover_stance_effect_rows_name_their_scripts() {
        let pool = require_db_or_skip!();
        let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
        for (id, script) in [
            (COVER_STANCE_EFFECT, "CoverStance"),
            (COVER_STANCE_REMOVE_EFFECT, "RemoveCoverStance"),
        ] {
            let def = defs
                .get(&id)
                .unwrap_or_else(|| panic!("effect {id} seeded"));
            assert_eq!(def.ability_id, cover::COVER_STANCE_ABILITY);
            assert_eq!(def.script_name.as_deref(), Some(script), "effect {id}");
            assert!(
                crate::cell::effects::registry::lookup(script).is_some(),
                "{script} must be registered"
            );
        }
    }
}
