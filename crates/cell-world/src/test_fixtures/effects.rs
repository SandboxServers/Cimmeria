//! The effect-script target world: one space with a player whose pools have
//! room to move, and an effect with one NVP. For the effect tests here and in
//! `cimmeria-cell-effect-scripts` (which installs the script registry on it).
//!
//! Also the mechanic fixture ([`seed_mechanic_effect`]): a player's press of
//! an ability with no mechanic is refused at launch (ability-mechanics
//! AB-12), so a test ability that stands for "some attack" needs one.

use std::collections::HashMap;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::{FOCUS, HEALTH};

use crate::cell::effects::registry::EffectScripts;
use crate::cell::effects::{EffectContext, EffectScript};
use crate::cell::space_manager::SpaceManager;

/// The effect [`seed_mechanic_effect`] seeds. Put it in a test ability's
/// `effect_ids` to give the ability a mechanic.
pub const MECHANIC_FIXTURE_EFFECT: i32 = 9_990_001;

/// The script [`MECHANIC_FIXTURE_EFFECT`] runs: registered, and a no-op.
pub const MECHANIC_FIXTURE_SCRIPT: &str = "TestFixtureNoop";

/// A script that does nothing: the fixture casts change no stat, as the
/// effectless fixtures did before the AB-12 gate.
struct NoopScript;

impl EffectScript for NoopScript {
    fn on_apply(&self, _ctx: &mut EffectContext) {}
}

/// Seed [`MECHANIC_FIXTURE_EFFECT`] on `mgr` and add its no-op script to the
/// installed registry (keeping every script already there).
pub fn seed_mechanic_effect(mgr: &mut SpaceManager) {
    mgr.effect_defs.insert(
        MECHANIC_FIXTURE_EFFECT,
        EffectDef {
            effect_id: MECHANIC_FIXTURE_EFFECT,
            script_name: Some(MECHANIC_FIXTURE_SCRIPT.to_string()),
            ..Default::default()
        },
    );
    let installed = mgr.effect_scripts().clone();
    if installed.contains(MECHANIC_FIXTURE_SCRIPT) {
        return;
    }
    let rows: Vec<(&'static str, &'static dyn EffectScript)> = installed
        .names()
        .filter_map(|n| installed.lookup(n).map(|s| (n, s)))
        .chain([(
            MECHANIC_FIXTURE_SCRIPT,
            &NoopScript as &'static dyn EffectScript,
        )])
        .collect();
    mgr.install_effect_scripts(EffectScripts::build(rows).expect("fixture registry builds"));
}

/// One space `W` with player entity 1 (player id 100) at HEALTH 50/100 and
/// FOCUS 200/1000, so a heal has room to show a delta.
pub fn make_mgr_with_target() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        if let Some(s) = e.stats.get_mut(HEALTH) {
            s.update(0, 50, 100);
        }
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 200, 1000);
        }
    }
    mgr
}

/// Effect 999 of ability 597 with one NVP and no script.
pub fn effect_with_nvp(name: &str, value: &str) -> EffectDef {
    let mut params = HashMap::new();
    params.insert(name.to_string(), value.to_string());
    EffectDef {
        effect_id: 999,
        ability_id: 597,
        delay: 0,
        effect_sequence: 0,
        event_set_id: None,
        script_name: None,
        params,
        ..Default::default()
    }
}
