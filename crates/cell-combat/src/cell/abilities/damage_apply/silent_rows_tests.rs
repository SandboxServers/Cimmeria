//! AB-T2: the hit plan's quiet exits (`silent_rows`). Each guard drives one
//! and asserts its row; without the row the find fails.

use cimmeria_entity::abilities::{AbilityDef, RC_HIT};
use cimmeria_entity::cell_entity::PlayerIdentity;
use tracing::Level;

use super::super::effect_scripts::plan_hit_effects;
use super::super::HitIds;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture};

const ABILITY: i32 = 9201;

fn ids() -> HitIds {
    HitIds {
        entity_id: 1,
        target_eid: 2,
        ability_id: ABILITY,
        actor: PlayerIdentity::new(Some(901), Some(101)),
        target: PlayerIdentity::UNKNOWN,
        cast_id: None,
        god_mode: false,
        world: "unknown",
    }
}

fn ability(effect_ids: Vec<i32>) -> AbilityDef {
    AbilityDef {
        ability_id: ABILITY,
        name: "test".to_string(),
        cooldown: 0.5,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0.0,
        max_range: 10.0,
        target_type_id: 0,
        effect_ids,
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    }
}

fn row(all: &[Captured], event: &str) -> Captured {
    all.iter()
        .find(|c| c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no `{event}` row in {all:#?}"))
}

/// **Regression guard (AB-T2, effect family).** An effect id the ability
/// names but the effect table lacks was a bare `continue`. It now WARNs
/// `effect_def_missing` under `abilities.effect`, with the cast's id.
#[test]
fn an_effect_id_with_no_definition_warns() {
    let mut mgr = SpaceManager::new(1);
    let outer = mgr.enter_cast_scope(Some(77));
    let logs = LogCapture::install();

    let plan = plan_hit_effects(&mgr, Some(&ability(vec![4040])), None, true, RC_HIT, ids());

    mgr.exit_cast_scope(outer);
    assert!(plan.nvp.is_empty() && plan.after_scripts.is_empty());
    let r = row(&logs.all(), "effect_def_missing");
    assert_eq!(r.target, "abilities.effect");
    assert_eq!(r.level, Level::WARN);
    for (k, v) in [
        ("effect_id", "4040"),
        ("site", "plan"),
        ("cast_id", "77"),
        ("player_id", "101"),
    ] {
        assert!(r.has_field(k, v), "{k} = {v}: {r:?}");
    }
}

/// **Regression guard (AB-T2, effect family).** A hit with no ability
/// definition deals the generic 15-HP fallback; it now WARNs so the made-up
/// number is visible.
#[test]
fn the_unknown_ability_fallback_warns() {
    let mgr = SpaceManager::new(1);
    let logs = LogCapture::install();

    let plan = plan_hit_effects(&mgr, None, None, true, RC_HIT, ids());

    assert_eq!(plan.nvp.len(), 1, "the fallback swing is still dealt");
    let r = row(&logs.all(), "unknown_ability_fallback_damage");
    assert_eq!(r.level, Level::WARN);
    assert!(r.has_field("health_damage", "15"), "{r:?}");
}
