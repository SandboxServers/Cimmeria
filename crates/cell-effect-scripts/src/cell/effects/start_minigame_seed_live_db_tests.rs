//! Live-DB guards on the Converse abilities (MG-62): effect 1643 (ability
//! 1327) and effect 2813 (ability 2089) both launch the conversation
//! minigame. Each carries `script_name = StartMinigame` and the NVPs
//! `minigame_type = ConverseBasicHumanoid` and `difficulty = 1`
//! (`effect_nvps.sql`, ids 25501-25504).
//!
//! They load the real seed through the cell's own loader. A NULL script
//! name, a dropped NVP or a renamed script fails here.

use super::registry::lookup;
use crate::cell::spawner::load_effect_defs;
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn converse_abilities_carry_start_minigame_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");

    for effect_id in [1643, 2813] {
        let effect = defs
            .get(&effect_id)
            .unwrap_or_else(|| panic!("effect {effect_id} is seeded"));
        assert_eq!(
            effect.script_name.as_deref(),
            Some("StartMinigame"),
            "effect {effect_id} script"
        );
        assert_eq!(
            effect.params.get("minigame_type").map(String::as_str),
            Some("ConverseBasicHumanoid"),
            "effect {effect_id} minigame_type"
        );
        assert_eq!(
            effect.params.get("difficulty").map(String::as_str),
            Some("1"),
            "effect {effect_id} difficulty"
        );
    }

    assert!(lookup("StartMinigame").is_some());
}
