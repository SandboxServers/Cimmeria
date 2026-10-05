//! AB-08 trust review: a passive ability is never cast. Its effects run
//! through `apply_passives`; a forged `useAbility` of one is refused at
//! launch with feedback and no cooldown.
//!
//! The fixture is 1574 Create Density: Basic as the seed holds it after
//! AB-08: `passive_yn`, cooldown 0, effect 4782 `TimedStat` `Defense` 100
//! with flags 524304 (`EF_AlwaysPersist`, no beneficial bit). Cast at a
//! hostile, it would take the attack path: a QR roll, threat and in-combat
//! on the target, for free and as often as the client likes.

use std::collections::HashMap;

use cimmeria_entity::abilities::{AbilityType, EffectDef, TARGET_SELF};
use cimmeria_wire::state_field::BSF_IN_COMBAT;

use super::duel_gate::{duel_mgr, A, MOB};
use super::warmup::calls;
use super::*;
use crate::cell::abilities::use_ability::no_mechanics::{NO_MECHANICS_ERROR_CODE, PASSIVE_TEXT};

const CREATE_DENSITY: i32 = 1574;

fn passive_mgr(passive_yn: bool) -> SpaceManager {
    let mut mgr = duel_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.effect_defs.insert(
        4782,
        EffectDef {
            effect_id: 4782,
            ability_id: CREATE_DENSITY,
            flags: 524_304,
            pulse_count: 1,
            pulse_duration: 0.0,
            script_name: Some("TimedStat".to_string()),
            params: HashMap::from([("Defense".to_string(), "100".to_string())]),
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        CREATE_DENSITY,
        AbilityDef {
            cooldown: 0.0,
            target_type_id: TARGET_SELF,
            effect_ids: vec![4782],
            type_id: AbilityType::Buff,
            passive: passive_yn,
            ..make_ability(CREATE_DENSITY, 0, 0)
        },
    );
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .add_ability(CREATE_DENSITY);
    mgr
}

/// **Regression guard (trust review).** 1574 cast at the mob is refused:
/// no threat either way, no in-combat flag on either side, no ledger entry,
/// no cooldown, and the player reads why. On revert the cast runs the
/// attack path and the mob's threat list gains A.
#[tokio::test]
async fn a_passive_cast_at_a_hostile_is_refused() {
    // `passive_yn` set, and (second pass) only the all-`EF_AlwaysPersist` rule.
    for passive_yn in [true, false] {
        let mut mgr = passive_mgr(passive_yn);
        let (tx, mut rx) = mpsc::channel(256);
        let fired = handle_use_ability(A, CREATE_DENSITY, MOB as i32, &tx, &mut mgr).await;
        let a = mgr.get_entity(A).unwrap();
        let mob = mgr.get_entity(MOB).unwrap();
        assert!(
            mob.threat_list.is_empty(),
            "no threat on the mob ({passive_yn})"
        );
        assert!(
            a.threatened_mobs.is_empty(),
            "A threatens nobody ({passive_yn})"
        );
        assert_eq!(
            a.state_field & BSF_IN_COMBAT,
            0,
            "A not in combat ({passive_yn})"
        );
        assert_eq!(
            mob.state_field & BSF_IN_COMBAT,
            0,
            "mob not in combat ({passive_yn})"
        );
        assert!(mob.stat_buffs.entries.is_empty() && a.stat_buffs.entries.is_empty());
        assert!(!a.abilities.is_on_cooldown(CREATE_DENSITY), "no cooldown");
        assert!(!fired, "refused (passive_yn {passive_yn})");

        let msgs = drain(&mut rx);
        let sent = calls(&msgs);
        assert!(
            sent.iter().any(|(e, m, args)| *e == A
                && *m == method_idx::ON_ERROR_CODE
                && args[5..7] == NO_MECHANICS_ERROR_CODE.to_le_bytes()),
            "onErrorCode 167: {msgs:?}"
        );
        assert!(
            sent.iter().any(|(e, m, args)| *e == A
                && *m == method_idx::ON_PLAYER_COMMUNICATION
                && *args
                    == cimmeria_wire::cell::chat::serialize_on_player_communication(
                        "SYSTEM",
                        0,
                        cimmeria_wire::cell::chat::CHAN_FEEDBACK,
                        PASSIVE_TEXT,
                    )),
            "the passive feedback line ({PASSIVE_TEXT})"
        );
    }
}

/// The passive still applies through its own path (login, purchase).
#[tokio::test]
async fn the_refused_passive_still_applies_through_apply_passives() {
    use cimmeria_cell_world::cell::effects::passives::{apply_passives, PassiveChange};
    use cimmeria_entity::stats::DEFENSE;
    let mut mgr = passive_mgr(true);
    let before = mgr.get_entity(A).unwrap().stats.get(DEFENSE).unwrap().cur;
    assert_eq!(
        apply_passives(&mut mgr, A, &[CREATE_DENSITY], PassiveChange::Learned),
        1
    );
    let after = mgr.get_entity(A).unwrap().stats.get(DEFENSE).unwrap().cur;
    assert_eq!(after, before + 100);
}
