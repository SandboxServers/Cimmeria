//! The special-ammo damage framework, packet AM-04 (ammo campaign, issue
//! #1026, decision D-AM07): when a player fires a weapon shot with a special
//! ammo type loaded, the server applies that type's
//! `resources.ammo_modifiers` row to the shot. There is no cast, no cooldown,
//! and the toggle abilities (715 Hollow Point, 719 Armor Piercing, ...) are
//! never launched; `toggle_ability_id` only records where the numbers came
//! from.
//!
//! # What one row does to a shot
//!
//! [`shot_ammo`] resolves the row for a shot. The combat pipeline
//! (`cimmeria-cell-combat`, `abilities::damage_apply`) then:
//!
//! - multiplies the shot's pre-armour damage by `damage_mult`, next to the
//!   cover scale ([`ShotAmmo::damage_scale`]);
//! - divides the armour mitigation by `penetration_mult`
//!   ([`ShotAmmo::penetration_mult`]; `combat::calculate_damage_penetrating`):
//!   2.0 lets half as much armour stand against the shot, 0.5 twice as much.
//!   It scales the armour term itself rather than the attacker's
//!   `PENETRATION` stat, because that stat is 0 on every player and a
//!   multiple of 0 would make the column dead;
//! - replaces the ability's damage type with `damage_type` when the row sets
//!   one ([`ShotAmmo::damage_type`]);
//! - on a hit (not a miss), runs `on_hit_effect_id` on the target through the
//!   ordinary effect machinery ([`ShotAmmo::on_hit_effect_id`]): its
//!   `script_name` dispatches through [`super::dispatch_by_name`] after the
//!   damage, and a pulsing effect registers on the target like any ability
//!   effect.
//!
//! # When a shot is modified
//!
//! Only when all of these hold, else the shot fires exactly as before:
//!
//! - `ammo.finite_special` is on. While it is off, reloads of special rounds
//!   are free (AM-02 gates the draw on it), and AM-F's widening already lets
//!   every Standard Pistol and SMG pick Hollow Point: an ungated modifier
//!   would hand every such player free extra damage the day AM-04 merges.
//!   So AM-04 ships dark with the rest of the campaign and AM-12 turns both
//!   halves on together.
//! - The attacker is a player. NPCs have no bandolier and keep their damage
//!   unchanged (NPCs fire with infinite default ammo).
//! - The ability is a weapon shot: `required_ammo > 0`, the same test
//!   `use_ability` uses for "weapon attack". A grenade or a melee swing with
//!   Hollow Point in the pistol is not a Hollow Point shot.
//! - The active bandolier slot's `cur_ammo_type` has a modifier row.
//!   Default ammo never has one (the table's CHECK), nor does a family whose
//!   packet has not shipped.
//!
//! # Extension point for the Wave-2 families (AM-08 .. AM-11c)
//!
//! A family changes a shot through data plus, when it needs behaviour the
//! existing scripts lack, one effect script:
//!
//! 1. Seed `ammo_modifiers_<family>.sql` under `db/resources/Abilities/Seed/`
//!    with the family's `ammo_modifiers` rows, and add one `\ir` line for it
//!    in `db/database.sql` after `Effects/Seed/effects.sql` (the HP/AP file's
//!    line is the model). The same file may seed the family's on-hit
//!    `resources.effects` and `effect_nvps` rows, with ids from the family's
//!    block (see `ammo_modifiers_hp_ap.sql`'s header).
//! 2. Point the row's `on_hit_effect_id` at that effect. Its `script_name`
//!    picks the behaviour: an existing script (`StatBuff`, `Stun`,
//!    `Suppression`, `MeleeDamage`, ...) or a new one.
//! 3. A new script is a zero-sized `impl super::EffectScript` in the
//!    family's own `effects/ammo_<family>.rs` in `cimmeria-cell-effect-scripts`
//!    plus one row in its `EFFECT_SCRIPTS` table. [`super::EffectContext`] gives it the shooter
//!    (`source_id`, so [`shot_ammo`]'s inputs are reachable), the target and
//!    the effect's NVPs. A pulsing effect (`pulse_count > 1`) is re-fired by
//!    the pulse tick, so a DoT needs no extra wiring.
//!
//! Nothing in this file or in the combat pipeline changes per family.

use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::abilities::{DT_PSIONIC, DT_UNTYPED};

use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::AmmoModifier;

/// The special-ammo modifier in force for one shot, from [`shot_ammo`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotAmmo {
    /// The loaded `EAmmoType` ordinal.
    pub ammo_type: i32,
    /// The ammo type's reserve item design id (`ammo_item_types`), the
    /// telemetry `item_id` correlator; `None` when the type has no item.
    pub ammo_item_id: Option<i32>,
    /// The `resources.ammo_modifiers` row.
    pub modifier: AmmoModifier,
}

impl ShotAmmo {
    /// The factor on the shot's pre-armour damage: `damage_mult`.
    pub fn damage_scale(&self) -> f64 {
        f64::from(self.modifier.damage_mult)
    }

    /// The divisor on the armour mitigation: `penetration_mult`. The table's
    /// CHECK keeps it `> 0`; a non-positive value from a bad row reads as 1.0
    /// rather than dividing by zero.
    pub fn penetration_mult(&self) -> f64 {
        let m = f64::from(self.modifier.penetration_mult);
        if m > 0.0 && m.is_finite() {
            m
        } else {
            1.0
        }
    }

    /// The shot's damage type: the row's override when it names a valid
    /// `EDamageType` ordinal, else `ability_damage_type`.
    pub fn damage_type(&self, ability_damage_type: i8) -> i8 {
        match self.modifier.damage_type {
            Some(dt) if (i32::from(DT_UNTYPED)..=i32::from(DT_PSIONIC)).contains(&dt) => dt as i8,
            _ => ability_damage_type,
        }
    }

    /// The effect to run on the target when the shot hits, if the row names
    /// one and the effect exists in `space_mgr.effect_defs`. A row naming a
    /// missing effect logs a warning and runs nothing.
    pub fn on_hit_effect_id(&self, space_mgr: &SpaceManager) -> Option<i32> {
        let id = self.modifier.on_hit_effect_id?;
        if space_mgr.effect_defs.contains_key(&id) {
            return Some(id);
        }
        tracing::warn!(
            target: "ammo",
            event = "ammo_on_hit_effect_missing",
            ammo_type = self.ammo_type,
            on_hit_effect_id = id,
            on_hit_effect_name = cimmeria_names::book().effect(id),
            toggle_ability_id = self.modifier.toggle_ability_id,
            toggle_ability_name = cimmeria_names::book().ability(self.modifier.toggle_ability_id),
            "ammo_modifiers row names an on-hit effect that is not in effect_defs; \
             the shot fires without it"
        );
        None
    }
}

/// The modifier for a shot `attacker_id` fires with `ability`, or `None` when
/// the shot fires unmodified. See the module docs for the rules.
///
/// `finite_special` is `cimmeria_entity::ammo_feature::finite_special()`,
/// read by the caller so tests never touch the process-wide switch.
pub fn shot_ammo(
    space_mgr: &SpaceManager,
    attacker_id: u32,
    ability: Option<&AbilityDef>,
    finite_special: bool,
) -> Option<ShotAmmo> {
    if !finite_special {
        return None;
    }
    if ability.is_none_or(|a| a.required_ammo <= 0) {
        return None;
    }
    let attacker = space_mgr.get_entity(attacker_id)?;
    if !attacker.is_player {
        return None;
    }
    let ammo_type = attacker.active_ammo_type();
    let modifier = *space_mgr.ammo_catalog.modifier(ammo_type)?;
    Some(ShotAmmo {
        ammo_type,
        ammo_item_id: space_mgr.ammo_catalog.item_id_for(ammo_type),
        modifier,
    })
}

/// Log `ammo_damage_applied` for one modified shot. `health_damage` is the
/// HEALTH the shot took off the target, `damage_type` the type it landed as.
pub fn log_applied(
    space_mgr: &SpaceManager,
    shot: &ShotAmmo,
    attacker_id: u32,
    target_id: u32,
    ability_id: i32,
    damage_type: i8,
    health_damage: i32,
    on_hit_effect_id: Option<i32>,
) {
    let who = space_mgr.player_identity(attacker_id);
    tracing::debug!(
        target: "ammo",
        event = "ammo_damage_applied",
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        entity_id = attacker_id,
        entity_name = space_mgr.entity_label(attacker_id),
        item_id = shot.ammo_item_id,
        item_name = crate::cell::effects::content_names::item_name(shot.ammo_item_id),
        ammo_type = shot.ammo_type,
        target_entity_id = target_id,
        target_entity_name = space_mgr.entity_label(target_id),
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        damage_mult = shot.modifier.damage_mult,
        penetration_mult = shot.modifier.penetration_mult,
        damage_type,
        toggle_ability_id = shot.modifier.toggle_ability_id,
        toggle_ability_name = super::content_names::ability_name(shot.modifier.toggle_ability_id),
        on_hit_effect_id,
        on_hit_effect_name = super::content_names::effect_name(on_hit_effect_id),
        health_damage,
        "special ammo modifier applied to a shot"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::spawner::AmmoCatalog;
    use crate::test_support::{effect_with_nvp, make_mgr_with_target};
    use cimmeria_entity::abilities::{DT_ENERGY, DT_PHYSICAL};
    use cimmeria_entity::ammo_type::{BULLET_ARMOR_PIERCING, BULLET_DEFAULT, BULLET_HOLLOW_POINT};
    use cimmeria_entity::cell_entity::BandolierItem;

    const HP_ITEM: i32 = 9001;

    fn hp_row() -> AmmoModifier {
        AmmoModifier {
            ammo_type: BULLET_HOLLOW_POINT,
            damage_mult: 1.2,
            penetration_mult: 0.5,
            damage_type: Some(i32::from(DT_PHYSICAL)),
            on_hit_effect_id: None,
            toggle_ability_id: 715,
            beneficial: false,
        }
    }

    fn shot_ability(required_ammo: i32) -> AbilityDef {
        AbilityDef {
            ability_id: 592,
            name: "Pistol Shot".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo,
            event_set_id: None,
            velocity: 0.0,
            type_id: Default::default(),
            passive: false,
        }
    }

    /// Player 1 with a pistol in slot 0 loaded with `ammo_type`, and a
    /// catalog holding `rows`.
    fn mgr_loaded(ammo_type: i32, rows: Vec<AmmoModifier>) -> SpaceManager {
        let mut mgr = make_mgr_with_target();
        mgr.ammo_catalog = AmmoCatalog::from_rows(rows, [(BULLET_HOLLOW_POINT, HP_ITEM)]);
        let e = mgr.get_entity_mut(1).unwrap();
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 1,
                item_id: 3241,
                clip_size: 15,
                default_ammo_type: BULLET_DEFAULT,
                current_ammo: 15,
                cur_ammo_type: ammo_type,
            },
        );
        mgr
    }

    #[test]
    fn hollow_point_shot_resolves_its_row() {
        let mgr = mgr_loaded(BULLET_HOLLOW_POINT, vec![hp_row()]);
        let shot = shot_ammo(&mgr, 1, Some(&shot_ability(1)), true).expect("modified");
        assert_eq!(shot.ammo_type, BULLET_HOLLOW_POINT);
        assert_eq!(shot.ammo_item_id, Some(HP_ITEM));
        assert_eq!(shot.modifier, hp_row());
        assert!((shot.damage_scale() - 1.2).abs() < 1e-6);
        assert!((shot.penetration_mult() - 0.5).abs() < 1e-6);
    }

    /// D-AM07 ships dark: with `ammo.finite_special` off, the same loaded
    /// Hollow Point shot fires unmodified.
    #[test]
    fn flag_off_fires_unmodified() {
        let mgr = mgr_loaded(BULLET_HOLLOW_POINT, vec![hp_row()]);
        assert_eq!(shot_ammo(&mgr, 1, Some(&shot_ability(1)), false), None);
    }

    #[test]
    fn default_ammo_and_unshipped_families_fire_unmodified() {
        let mgr = mgr_loaded(BULLET_DEFAULT, vec![hp_row()]);
        assert_eq!(shot_ammo(&mgr, 1, Some(&shot_ability(1)), true), None);
        // Armor Piercing loaded but no row for it in this catalog.
        let mgr = mgr_loaded(BULLET_ARMOR_PIERCING, vec![hp_row()]);
        assert_eq!(shot_ammo(&mgr, 1, Some(&shot_ability(1)), true), None);
    }

    /// A grenade, a melee swing or an unknown ability is not a shot.
    #[test]
    fn non_weapon_abilities_fire_unmodified() {
        let mgr = mgr_loaded(BULLET_HOLLOW_POINT, vec![hp_row()]);
        assert_eq!(shot_ammo(&mgr, 1, Some(&shot_ability(0)), true), None);
        assert_eq!(shot_ammo(&mgr, 1, None, true), None);
    }

    /// NPC damage is unchanged, even for an NPC that somehow carries a
    /// loaded slot.
    #[test]
    fn npc_shots_fire_unmodified() {
        let mut mgr = mgr_loaded(BULLET_HOLLOW_POINT, vec![hp_row()]);
        mgr.get_entity_mut(1).unwrap().is_player = false;
        assert_eq!(shot_ammo(&mgr, 1, Some(&shot_ability(1)), true), None);
        assert_eq!(shot_ammo(&mgr, 42, Some(&shot_ability(1)), true), None);
    }

    #[test]
    fn damage_type_override_only_for_a_valid_ordinal() {
        let mut row = hp_row();
        let shot = |row: AmmoModifier| ShotAmmo {
            ammo_type: row.ammo_type,
            ammo_item_id: None,
            modifier: row,
        };
        row.damage_type = Some(i32::from(DT_ENERGY));
        assert_eq!(shot(row).damage_type(DT_PHYSICAL), DT_ENERGY);
        row.damage_type = None;
        assert_eq!(shot(row).damage_type(DT_PHYSICAL), DT_PHYSICAL);
        row.damage_type = Some(9);
        assert_eq!(shot(row).damage_type(DT_PHYSICAL), DT_PHYSICAL);
        row.penetration_mult = 0.0;
        assert!((shot(row).penetration_mult() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn on_hit_effect_resolves_only_when_the_effect_exists() {
        let mut row = hp_row();
        row.on_hit_effect_id = Some(999);
        let mut mgr = mgr_loaded(BULLET_HOLLOW_POINT, vec![row]);
        let shot = shot_ammo(&mgr, 1, Some(&shot_ability(1)), true).unwrap();
        let logs = crate::test_support::LogCapture::install();
        assert_eq!(shot.on_hit_effect_id(&mgr), None, "effect 999 not loaded");
        let warn = logs
            .all()
            .into_iter()
            .find(|c| c.target == "ammo" && c.has_field("event", "ammo_on_hit_effect_missing"))
            .expect("a missing on-hit effect must warn");
        assert_eq!(warn.level, tracing::Level::WARN);
        assert!(warn.has_field("on_hit_effect_id", "999"), "{warn:?}");
        drop(logs);
        mgr.effect_defs
            .insert(999, effect_with_nvp("HealthDamage", "5"));
        assert_eq!(shot.on_hit_effect_id(&mgr), Some(999));
    }

    /// `ammo_modifiers_hp_ap.sql` loads the two reconstructed rows through
    /// the startup loader, with the numbers `damage_apply::ammo_tests` (in
    /// `cimmeria-cell-combat`) assumes, and their provenance abilities
    /// exist. Fails if the seed or its `\ir` line is dropped or a number
    /// drifts.
    #[tokio::test]
    async fn live_db_hp_ap_seed_rows() {
        let pool = crate::test_support::require_db_or_skip!();
        let catalog = crate::cell::spawner::load_ammo_catalog(&pool)
            .await
            .expect("ammo catalog loads");
        let physical = Some(i32::from(DT_PHYSICAL));
        assert_eq!(
            catalog.modifier(BULLET_HOLLOW_POINT),
            Some(&AmmoModifier {
                ammo_type: BULLET_HOLLOW_POINT,
                damage_mult: 1.25,
                penetration_mult: 0.5,
                damage_type: physical,
                on_hit_effect_id: None,
                toggle_ability_id: 715,
                beneficial: false,
            })
        );
        assert_eq!(
            catalog.modifier(BULLET_ARMOR_PIERCING),
            Some(&AmmoModifier {
                ammo_type: BULLET_ARMOR_PIERCING,
                damage_mult: 0.9,
                penetration_mult: 2.0,
                damage_type: physical,
                on_hit_effect_id: None,
                toggle_ability_id: 719,
                beneficial: false,
            })
        );
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM resources.abilities WHERE ability_id IN (715, 719) \
             ORDER BY ability_id",
        )
        .fetch_all(&pool)
        .await
        .expect("abilities query");
        assert_eq!(
            names,
            ["Hollow Point Ammunition", "Armor Piercing Ammunition"]
        );
    }
}
