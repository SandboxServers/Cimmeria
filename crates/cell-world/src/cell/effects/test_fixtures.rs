//! Shared fixtures for the effect-script unit tests (`scripts`, `heal`,
//! `stat_buff`).

use std::collections::HashMap;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::{FOCUS, HEALTH};

use crate::cell::space_manager::SpaceManager;

/// One space `W` with player entity 1 (player id 100) at HEALTH 50/100 and
/// FOCUS 200/1000, so a heal has room to show a delta.
pub(crate) fn make_mgr_with_target() -> SpaceManager {
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
pub(crate) fn effect_with_nvp(name: &str, value: &str) -> EffectDef {
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
