//! Deployable fixtures (deployables Phase 0): the seeded 1012 Microwave
//! Emitter shape, so the world tests and the combat tests start from the
//! same numbers the seed carries.

use cimmeria_entity::abilities::{AbilityDef, EffectDef};

use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{DeployableCatalog, DeployableSpec, SpawnRecord};

use super::pets::pet_template_record;

/// 1012 Deployable: Microwave Emitter.
pub const DEPLOYABLE_ABILITY: i32 = 1012;
/// Template 400, the emitter.
pub const DEPLOYABLE_TEMPLATE: i32 = 400;
/// 5065 "Pulser": 30 pulses x 1 s.
pub const DEPLOYABLE_LIFETIME_EFFECT: i32 = 5065;
/// 5066 "Damage": Medium radius, -100F, `RangedPhysicalDamage`.
pub const DEPLOYABLE_PULSE_EFFECT: i32 = 5066;
/// 1012's seeded `max_range`, in metres as the loader leaves it (500 UE3
/// units).
pub const DEPLOYABLE_MAX_RANGE: f32 = 5.0;
/// 1012's seeded warmup, seconds.
pub const DEPLOYABLE_WARMUP: f32 = 2.0;
/// 1012's seeded flags: SpeedDeploy | Deactivate_AutoCycle |
/// DoNotActivate_AutoCycle | ForceStanding | DeploymentBar.
pub const DEPLOYABLE_FLAGS: u32 = 5890;

/// The seeded `resources.deployables` row for 1012.
pub const DEPLOYABLE_SPEC: DeployableSpec = DeployableSpec {
    ability_id: DEPLOYABLE_ABILITY,
    template_id: DEPLOYABLE_TEMPLATE,
    lifetime_effect_id: DEPLOYABLE_LIFETIME_EFFECT,
    pulse_effect_id: DEPLOYABLE_PULSE_EFFECT,
    max_active: 1,
};

/// 1012's ability row (`abilities.sql`): cooldown 30, warmup 2, range 500
/// UE3 units (5 m),
/// target type Ground, no event set, effects 5066 and 5065.
pub fn deployable_ability_def() -> AbilityDef {
    AbilityDef {
        ability_id: DEPLOYABLE_ABILITY,
        name: "Deployable: Microwave Emitter".to_string(),
        cooldown: 30.0,
        warmup: DEPLOYABLE_WARMUP,
        flags: DEPLOYABLE_FLAGS,
        is_ranged: false,
        min_range: 0.0,
        max_range: DEPLOYABLE_MAX_RANGE,
        target_type_id: 3,
        effect_ids: vec![DEPLOYABLE_PULSE_EFFECT, DEPLOYABLE_LIFETIME_EFFECT],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 100.0,
        type_id: Default::default(),
        passive: false,
    }
}

/// 5065 and 5066 as the seed loads them.
pub fn deployable_effect_defs() -> [EffectDef; 2] {
    let pulser = EffectDef {
        effect_id: DEPLOYABLE_LIFETIME_EFFECT,
        ability_id: DEPLOYABLE_ABILITY,
        delay: 2,
        pulse_count: 30,
        pulse_duration: 1.0,
        tcm_param1: "Medium".to_string(),
        ..Default::default()
    };
    let mut damage = EffectDef {
        effect_id: DEPLOYABLE_PULSE_EFFECT,
        ability_id: DEPLOYABLE_ABILITY,
        script_name: Some("RangedPhysicalDamage".to_string()),
        target_collection_method: "TCM_AERadius".to_string(),
        tcm_param1: "Medium".to_string(),
        flags: 144,
        ..Default::default()
    };
    damage
        .params
        .insert("FocusDamage".to_string(), "100".to_string());
    [pulser, damage]
}

/// Template 400 as the seed carries it, loaded the way the template cache
/// loads every row (a pet-shaped record reshaped to the seeded columns).
pub fn deployable_template_record() -> SpawnRecord {
    let mut r = pet_template_record(DEPLOYABLE_TEMPLATE);
    r.template_name = "Deployable: Microwave Emitter".to_string();
    r.class = "being".to_string();
    r.body_set = "WP-Human.BS_DeployableLow".to_string();
    r.components = Some(vec!["WP-Human.Dp_Standard100".to_string()]);
    r.level = Some(1);
    r.faction = Some(1);
    r.name_id = Some(5463);
    r.ability_ids = Vec::new();
    r
}

/// Cache the template, the effects, the ability and the deployables row
/// on `mgr`.
pub fn seed_deployable(mgr: &mut SpaceManager) {
    mgr.spawn_templates
        .insert(DEPLOYABLE_TEMPLATE, deployable_template_record());
    for e in deployable_effect_defs() {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    mgr.ability_defs
        .insert(DEPLOYABLE_ABILITY, deployable_ability_def());
    mgr.deployable_specs = DeployableCatalog::from_rows([DEPLOYABLE_SPEC]);
}
