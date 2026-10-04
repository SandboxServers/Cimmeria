//! AB-N1 guards for the landing notes: only a landing whose plan can land
//! prints `landed`. A skipped plan (`not_reachable`, `no_script`) prints
//! its plan, never a landing.

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::combat_debug::commands::{toggle, Toggle};
use cimmeria_cell_world::cell::combat_debug::deliver::prepare;
use cimmeria_entity::abilities::EffectDef;

use super::{land_effects, Landing, LandingRoute};
use crate::cell::space_manager::SpaceManager;

const CASTER: u32 = 1;
const GONE: u32 = 99;
const ABILITY: i32 = 597;

fn effect(id: i32, script: Option<&str>) -> EffectDef {
    EffectDef {
        effect_id: id,
        script_name: script.map(str::to_string),
        ..Default::default()
    }
}

/// Land `landings` with combat debug on for the caster; the lines it sent.
async fn lines(landings: Vec<Landing>) -> Vec<String> {
    let mut mgr: SpaceManager = crate::test_support::make_mgr_with_target();
    crate::test_support::seed_ability_defs(&mut mgr, &[ABILITY]);
    toggle(&mut mgr, CASTER, Toggle::Combat).unwrap();
    let (tx, _rx) = mpsc::channel(64);
    land_effects(CASTER, ABILITY, &landings, &tx, &mut mgr).await;
    prepare(&mut mgr, std::time::Instant::now())
        .into_iter()
        .map(|o| o.text)
        .collect()
}

fn user(effect: &EffectDef, recipient: u32) -> Landing {
    Landing::new(effect, recipient, LandingRoute::User("self_ability"))
}

/// Positive control: a scripted effect on a present recipient lands.
#[tokio::test]
async fn a_landing_that_can_land_prints_landed() {
    let out = lines(vec![user(&effect(10, Some("NoSuchScript")), CASTER)]).await;
    assert!(out.iter().any(|l| l.contains(": landed;")), "{out:?}");
}

/// **Guard.** A recipient that is gone (`not_reachable`) is not a landing.
#[tokio::test]
async fn a_not_reachable_landing_is_not_reported_landed() {
    let out = lines(vec![user(&effect(11, Some("NoSuchScript")), GONE)]).await;
    assert!(!out.iter().any(|l| l.contains("landed")), "{out:?}");
    assert!(
        out.iter().any(|l| l.contains("fired, nothing resolved")),
        "{out:?}"
    );
}

/// **Guard.** An effect with nothing to run (`no_script`) is not a landing.
#[tokio::test]
async fn a_no_script_landing_is_not_reported_landed() {
    let out = lines(vec![user(&effect(12, None), CASTER)]).await;
    assert!(!out.iter().any(|l| l.contains("landed")), "{out:?}");
}
