//! Special-ammo effect scripts, packet AM-09 (ammo campaign, issue #1026):
//! EMP rounds (toggle ability 1445), the on-hit effect.
//!
//! Created empty by AM-F so each family packet adds its script here and one
//! `EFFECT_SCRIPTS` row in `registry.rs`, and never edits `effects/mod.rs`.
//!
//! # What an EMP round does on a hit
//!
//! [`EmpDisrupt`] (effect 9120, `ammo_modifiers_emp.sql`) splits on the
//! target, the way the EMP Grenade (ability 2864) does in the cooked data:
//!
//! - a living target loses `FocusDamage` Focus and no Health (grenade effect
//!   4202, "Non-Mechanical Target Damage -100F / -0H");
//! - a mechanical target loses `MechanicalHealthDamage` Health and no Focus
//!   (grenade effect 4200, "Mechanical Target Damage -0F / -225H").
//!
//! The grenade also disorients a mechanical target for 20 s (effect 4201).
//! The rounds do not: the server has no disorient, and a timed disable on
//! the on-hit effect would need `pulse_count > 1`, which re-fires `on_apply`
//! and drains again every pulse. See `worknotes/AM-09.md`.
//!
//! Since ability mechanics AB-09c a hit also breaks the target's warmup and
//! channels at the row's `InterruptChance` (25 %, DESIGN), resisted by its
//! `interruptRes`: the script queues the request and combat resolves it.
//!
//! # Which targets are mechanical
//!
//! There is no mechanical flag on an entity, a template or a faction
//! (`EEntityFlags`, `entity_templates` and `EFaction` have none), and the
//! grenade's "Mechanical Type Check" effects carry no data. The body set is
//! the one signal the server has, so [`is_mechanical`] matches it against
//! [`MECHANICAL_BODY_SETS`]. A player is never mechanical.

use cimmeria_entity::stats::{FOCUS, HEALTH};

use cimmeria_cell_world::cell::space_manager::EntityNames;

use super::{EffectContext, EffectScript};

/// Body-set prefixes (`entity_templates.body_set`, the package and the start
/// of the set name) of the targets EMP treats as machines. RECONSTRUCTION:
/// chosen by name, one line each.
///
/// Left out on purpose: the Straegis (`MOB_Straegis*`). Their lore line
/// (dialog screen text: "They lack biology, yet move and act. They also lack
/// the characteristic signs of machine...") says they are neither, so they
/// stay non-mechanical until someone finds the original rule.
pub const MECHANICAL_BODY_SETS: &[&str] = &[
    // Prisoner Retrieval Unit (templates 4 and 145).
    "MOB_CA_DroneTank.",
    // "Malfunctioning Drone".
    "MOB_Goauld_Drone.",
    // "Ancient Drone".
    "MOB_AncientDrone.",
    // "BattleWalker".
    "MOB_BattleWalker.",
    // Deployables (turrets, emitters): `WP-Human.BS_DeployableLow` and kin.
    "WP-Human.BS_Deployable",
];

/// Whether EMP treats a target with this body set as a machine.
pub fn is_mechanical(body_set: Option<&str>) -> bool {
    body_set.is_some_and(|bs| MECHANICAL_BODY_SETS.iter().any(|p| bs.starts_with(p)))
}

/// What one EMP hit takes from a target: `(focus, health)`, each clamped to
/// what the target has. `focus_damage` applies only to a living target and
/// `mechanical_health_damage` only to a mechanical one; negatives read as 0.
pub fn emp_hit(
    mechanical: bool,
    focus_cur: i32,
    health_cur: i32,
    focus_damage: i32,
    mechanical_health_damage: i32,
) -> (i32, i32) {
    if mechanical {
        (0, mechanical_health_damage.max(0).min(health_cur.max(0)))
    } else {
        (focus_damage.max(0).min(focus_cur.max(0)), 0)
    }
}

/// The EMP rounds' on-hit effect. See the module docs.
///
/// NVPs:
///   - `FocusDamage` (i32): Focus drained from a living target.
///   - `MechanicalHealthDamage` (i32): Health taken from a mechanical target.
///
/// Health it takes can kill: `damage_apply`'s effect-driven death sweep runs
/// after the on-hit script.
pub struct EmpDisrupt;

impl EffectScript for EmpDisrupt {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let focus_damage = ctx.effect.param_i32("FocusDamage");
        let mech_damage = ctx.effect.param_i32("MechanicalHealthDamage");
        // Snapshot the shooter before the target is borrowed for the hit:
        // the rows below can't look anything up while it is. An EMP hit is
        // rare, so naming it up front costs nothing that matters.
        let shooter = ctx.space_mgr.player_identity(ctx.source_id);
        let shooter_name = ctx.space_mgr.entity_names(ctx.source_id).entity_name;

        let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
            tracing::debug!(
                target: "ammo",
                event = "ammo_emp_disrupt",
                entity_id = ctx.source_id,
                entity_name = shooter_name,
                account_id = shooter.account_id,
            account_name = shooter.account_name,
            player_id = shooter.player_id,
            player_name = shooter.player_name,
                target_entity_id = ctx.target_id,
                target_entity_name = ctx.space_mgr.entity_label(ctx.target_id),
                effect_id = ctx.effect.effect_id,
                effect_name = cimmeria_names::book().effect(ctx.effect.effect_id),
                reason = "target_missing",
                "EMP round: target gone before the on-hit effect ran"
            );
            return;
        };

        let mechanical = !target.is_player && is_mechanical(target.body_set.as_deref());
        let focus_cur = target.stats.get(FOCUS).map_or(0, |s| s.cur);
        let health_cur = target.stats.get(HEALTH).map_or(0, |s| s.cur);
        let (focus_drained, health_damage) =
            emp_hit(mechanical, focus_cur, health_cur, focus_damage, mech_damage);

        if focus_drained > 0 {
            if let Some(stat) = target.stats.get_mut(FOCUS) {
                stat.update(stat.min, stat.cur - focus_drained, stat.max);
            }
        }
        if health_damage > 0 {
            if let Some(stat) = target.stats.get_mut(HEALTH) {
                stat.update(stat.min, stat.cur - health_damage, stat.max);
            }
        }

        tracing::debug!(
            target: "ammo",
            event = "ammo_emp_disrupt",
            entity_id = ctx.source_id,
            entity_name = shooter_name,
            account_id = shooter.account_id,
            account_name = shooter.account_name,
            player_id = shooter.player_id,
            player_name = shooter.player_name,
            target_entity_id = ctx.target_id,
            target_entity_name = EntityNames::of(target).entity_name,
            target_template_id = target.template_id,
            target_template_name = cimmeria_cell_world::cell::effects::content_names::template_name(target.template_id),
            effect_id = ctx.effect.effect_id,
            effect_name = cimmeria_names::book().effect(ctx.effect.effect_id),
            mechanical,
            focus_before = focus_cur,
            focus_drained,
            health_before = health_cur,
            health_damage,
            "EMP round disrupted the target"
        );
        // The disruption also breaks a cast, at the row's InterruptChance
        // (ability mechanics AB-09c); combat rolls it against interruptRes.
        if let Some(chance) = super::crowd_control::stated_interrupt_chance(ctx.effect) {
            super::crowd_control::queue_interrupt(ctx, chance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::effects::registry;
    use crate::cell::effects::test_fixtures::make_mgr_with_target;
    use cimmeria_entity::abilities::EffectDef;
    use cimmeria_entity::ammo_type::BULLET_EMP;
    use std::collections::HashMap;

    /// The numbers `ammo_modifiers_emp.sql` seeds on effect 9120.
    const FOCUS_DAMAGE: i32 = 10;
    const MECH_DAMAGE: i32 = 5;

    fn emp_effect() -> EffectDef {
        let mut params = HashMap::new();
        params.insert("FocusDamage".to_string(), FOCUS_DAMAGE.to_string());
        params.insert(
            "MechanicalHealthDamage".to_string(),
            MECH_DAMAGE.to_string(),
        );
        EffectDef {
            effect_id: 9120,
            ability_id: 1445,
            script_name: Some("EmpDisrupt".to_string()),
            params,
            ..Default::default()
        }
    }

    /// Entity 1 of `make_mgr_with_target` (HEALTH 50, FOCUS 200) as an NPC
    /// with `body_set`, hit by one EMP round from entity 1 itself. Returns
    /// `(focus, health)` after.
    fn hit(body_set: Option<&str>, is_player: bool) -> (i32, i32) {
        let mut mgr = make_mgr_with_target();
        let e = mgr.get_entity_mut(1).unwrap();
        e.is_player = is_player;
        e.body_set = body_set.map(str::to_string);
        let effect = emp_effect();
        let script = registry::lookup("EmpDisrupt").expect("EmpDisrupt is registered");
        script.on_apply(&mut EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        });
        let e = mgr.get_entity(1).unwrap();
        (
            e.stats.get(FOCUS).unwrap().cur,
            e.stats.get(HEALTH).unwrap().cur,
        )
    }

    #[test]
    fn mechanical_body_sets_match_the_named_machines_only() {
        for bs in [
            "MOB_CA_DroneTank.BS_MOB_DroneFlyer",
            "MOB_Goauld_Drone.MOB_Goauld_Drone",
            "MOB_AncientDrone.MOB_AncientDrone_BS",
            "MOB_BattleWalker.BS_MOB_BattleWalker",
            "WP-Human.BS_DeployableLow",
        ] {
            assert!(is_mechanical(Some(bs)), "{bs}");
        }
        for bs in [
            "BS_HumanMale.BS_HumanMale",
            "BS_JaffaMale.BS_JaffaMale",
            "MOB_StraegisTitan.BS_MOB_StraegisTitan",
            "MOB_ScavDog.BS_MOB_ScavDog",
            // A weapon mesh in the same package is not a deployable.
            "WP-Human.WP_SMG_1A",
            "",
        ] {
            assert!(!is_mechanical(Some(bs)), "{bs}");
        }
        assert!(!is_mechanical(None));
    }

    #[test]
    fn emp_hit_splits_on_the_target_and_clamps() {
        // Living: Focus only.
        assert_eq!(emp_hit(false, 200, 50, 10, 5), (10, 0));
        // Mechanical: Health only.
        assert_eq!(emp_hit(true, 200, 50, 10, 5), (0, 5));
        // Clamped to what the target has.
        assert_eq!(emp_hit(false, 4, 50, 10, 5), (4, 0));
        assert_eq!(emp_hit(true, 200, 3, 10, 5), (0, 3));
        // Bad NVPs or empty pools take nothing.
        assert_eq!(emp_hit(false, 200, 50, -10, 5), (0, 0));
        assert_eq!(emp_hit(true, 200, 50, 10, -5), (0, 0));
        assert_eq!(emp_hit(false, 0, 50, 10, 5), (0, 0));
        assert_eq!(emp_hit(true, 200, 0, 10, 5), (0, 0));
    }

    /// A living NPC loses the seeded Focus and keeps its Health.
    #[test]
    fn living_target_loses_focus() {
        assert_eq!(
            hit(Some("BS_JaffaMale.BS_JaffaMale"), false),
            (200 - FOCUS_DAMAGE, 50)
        );
    }

    /// A drone loses the seeded Health and keeps its Focus.
    #[test]
    fn mechanical_target_loses_health() {
        assert_eq!(
            hit(Some("MOB_CA_DroneTank.BS_MOB_DroneFlyer"), false),
            (200, 50 - MECH_DAMAGE)
        );
    }

    /// A player is never mechanical, whatever body set it carries.
    #[test]
    fn player_is_never_mechanical() {
        assert_eq!(
            hit(Some("MOB_CA_DroneTank.BS_MOB_DroneFlyer"), true),
            (200 - FOCUS_DAMAGE, 50)
        );
    }

    /// A missing target is a no-op, not a panic.
    #[test]
    fn missing_target_is_a_no_op() {
        let mut mgr = make_mgr_with_target();
        let effect = emp_effect();
        EmpDisrupt.on_apply(&mut EffectContext {
            source_id: 1,
            target_id: 999,
            effect: &effect,
            space_mgr: &mut mgr,
        });
        assert_eq!(
            mgr.get_entity(1).unwrap().stats.get(FOCUS).unwrap().cur,
            200
        );
    }

    /// Type 12 guard (AM-12 close-out): every EMP hit logs one DEBUG
    /// `ammo_emp_disrupt` on target `ammo` with the split it took, and a
    /// missing target logs the same event with `reason=target_missing`.
    #[test]
    fn emp_hit_logs_ammo_emp_disrupt() {
        let logs = crate::test_support::LogCapture::install();
        let rows = |logs: &crate::test_support::LogCaptureGuard| {
            logs.all()
                .into_iter()
                .filter(|c| c.target == "ammo" && c.has_field("event", "ammo_emp_disrupt"))
                .collect::<Vec<_>>()
        };

        hit(Some("MOB_CA_DroneTank.BS_MOB_DroneFlyer"), false);
        let hit_rows = rows(&logs);
        assert_eq!(hit_rows.len(), 1, "{:#?}", logs.all());
        let row = &hit_rows[0];
        assert_eq!(row.level, tracing::Level::DEBUG);
        assert!(row.has_field("mechanical", "true"), "{row:#?}");
        assert!(row.has_field("health_damage", &MECH_DAMAGE.to_string()));
        assert!(row.has_field("focus_drained", "0"));
        assert!(row.has_field("effect_id", "9120"));

        let mut mgr = make_mgr_with_target();
        let effect = emp_effect();
        EmpDisrupt.on_apply(&mut EffectContext {
            source_id: 1,
            target_id: 999,
            effect: &effect,
            space_mgr: &mut mgr,
        });
        assert!(
            rows(&logs)
                .iter()
                .any(|c| c.has_field("reason", "target_missing")),
            "{:#?}",
            logs.all()
        );
    }

    /// `ammo_modifiers_emp.sql` loads through the startup loaders: the EMP
    /// row with its reconstructed numbers and on-hit effect 9120, and effect
    /// 9120 with script `EmpDisrupt` and the two NVPs the unit tests above
    /// and `damage_apply::ammo_emp_tests` (in `cimmeria-cell-combat`) assume.
    /// Fails if the seed or its `\ir` line is dropped or a number drifts.
    #[tokio::test]
    async fn live_db_emp_seed_rows() {
        let pool = crate::test_support::require_db_or_skip!();
        let catalog = crate::cell::spawner::load_ammo_catalog(&pool)
            .await
            .expect("ammo catalog loads");
        let row = catalog.modifier(BULLET_EMP).expect("Bullet_EMP row");
        assert_eq!(row.damage_mult, 1.1);
        assert_eq!(row.penetration_mult, 0.75);
        assert_eq!(
            row.damage_type,
            Some(i32::from(cimmeria_entity::abilities::DT_PHYSICAL))
        );
        assert_eq!(row.on_hit_effect_id, Some(9120));
        assert_eq!(row.toggle_ability_id, 1445);

        let effects = crate::cell::spawner::load_effect_defs(&pool)
            .await
            .expect("effect defs load");
        let effect = effects.get(&9120).expect("effect 9120");
        assert_eq!(effect.script_name.as_deref(), Some("EmpDisrupt"));
        assert_eq!(
            effect.pulse_count, 1,
            "single-shot: no re-fire, no stacking"
        );
        assert_eq!(effect.param_i32("FocusDamage"), FOCUS_DAMAGE);
        assert_eq!(effect.param_i32("MechanicalHealthDamage"), MECH_DAMAGE);
        // AB-09c: a quarter of EMP hits break a cast (DESIGN).
        assert_eq!(effect.param_i32("InterruptChance"), 25);

        let name: String =
            sqlx::query_scalar("SELECT name FROM resources.abilities WHERE ability_id = 1445")
                .fetch_one(&pool)
                .await
                .expect("ability 1445");
        assert_eq!(name, "EMP Ammunition");
    }
}
