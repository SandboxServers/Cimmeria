//! Pets PT-08: a passive ability's `EF_AlwaysPersist` effect (2852 Heed Our
//! Calling -> 4968, owner `speedPet` +100) holds while the ability is known.
//!
//! The three seams where the cell's known set changes each apply it: world
//! entry (`InitPlayerState`), a trainer purchase (`AbilityGranted`) and a
//! respec (`AbilitiesReset`, which removes it). Each test fails when its
//! seam's `apply_passives` call is removed.

use super::*;
use crate::ability_tree::RespecOutcome;
use cimmeria_entity::abilities::{EffectDef, EF_ALWAYS_PERSIST};
use cimmeria_entity::cell_entity::TreeProgress;
use cimmeria_entity::stats::SPEED_PET;

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 100;
const HEED_OUR_CALLING: i32 = 2852;

fn fixture() -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(6);
        p.level = 25;
    }
    crate::test_support::seed_ability_defs(&mut mgr, &[HEED_OUR_CALLING]);
    mgr.ability_defs
        .get_mut(&HEED_OUR_CALLING)
        .unwrap()
        .effect_ids = vec![4968];
    mgr.effect_defs.insert(
        4968,
        EffectDef {
            effect_id: 4968,
            ability_id: HEED_OUR_CALLING,
            flags: EF_ALWAYS_PERSIST,
            script_name: Some("PetSummonSpeed".to_string()),
            params: [("SpeedPet".to_string(), "100".to_string())].into(),
            ..Default::default()
        },
    );
    mgr
}

fn speed_pet(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(PLAYER)
        .unwrap()
        .stats
        .get(SPEED_PET)
        .unwrap()
        .cur
}

async fn deliver(mgr: &mut SpaceManager, msg: BaseToCellMsg) {
    let (tx, _rx) = mpsc::channel(256);
    let engine = ChainEngine::new();
    handle_base_message(msg, &tx, mgr, &engine, &[]).await;
}

#[tokio::test]
async fn world_entry_applies_a_known_passive() {
    let mut mgr = fixture();
    mgr.connect_entity(PLAYER);
    deliver(
        &mut mgr,
        BaseToCellMsg::InitPlayerState {
            entity_id: PLAYER,
            player_id: PLAYER_ID,
            account_id: 6,
            world_name: "Agnos".into(),
            archetype_id: 6,
            saved_missions: vec![],
            abilities: vec![HEED_OUR_CALLING],
            active_bandolier_slot: 0,
            bandolier_items: vec![],
            system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
            access_level: 0,
            known_stargates: vec![],
            tree_progress: TreeProgress::default(),
            level: 25,
            character_name: None,
            body_set: None,
            looted_containers: Vec::new(),
        },
    )
    .await;
    assert_eq!(speed_pet(&mgr), 100);
}

/// **Regression guard (UAT path).** `.giveability 2852` persists on the base,
/// which answers with `GmAbilityGranted`; that mirror must apply the passive
/// at once, so the tester's next summon is instant. Fails when the mirror's
/// `apply_passives` call is removed.
#[tokio::test]
async fn a_gm_grant_applies_the_passive_at_once() {
    let mut mgr = fixture();
    assert_eq!(speed_pet(&mgr), 0);
    deliver(
        &mut mgr,
        BaseToCellMsg::GmAbilityGranted {
            entity_id: PLAYER,
            player_id: PLAYER_ID,
            ability_id: HEED_OUR_CALLING,
        },
    )
    .await;
    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .abilities
        .has_ability(HEED_OUR_CALLING));
    assert_eq!(
        speed_pet(&mgr),
        100,
        ".giveability 2852: the summon is instant"
    );
}

/// A GM grant addressed to a character the entity no longer plays changes
/// nothing, passive included.
#[tokio::test]
async fn a_stale_gm_grant_applies_no_passive() {
    let mut mgr = fixture();
    deliver(
        &mut mgr,
        BaseToCellMsg::GmAbilityGranted {
            entity_id: PLAYER,
            player_id: PLAYER_ID + 1,
            ability_id: HEED_OUR_CALLING,
        },
    )
    .await;
    assert_eq!(speed_pet(&mgr), 0);
}

#[tokio::test]
async fn a_trainer_purchase_applies_the_passive_and_a_respec_removes_it() {
    let mut mgr = fixture();
    deliver(
        &mut mgr,
        BaseToCellMsg::AbilityGranted {
            entity_id: PLAYER,
            ability_id: HEED_OUR_CALLING,
            training_points: 0,
            tree_points_spent: 1,
        },
    )
    .await;
    assert_eq!(speed_pet(&mgr), 100, "learned: the next summon is instant");

    deliver(
        &mut mgr,
        BaseToCellMsg::AbilitiesReset {
            entity_id: PLAYER,
            player_id: PLAYER_ID,
            outcome: RespecOutcome::Reset {
                refunded: vec![HEED_OUR_CALLING],
                training_points: 1,
                naquadah: 0,
            },
        },
    )
    .await;
    assert!(!mgr
        .get_entity(PLAYER)
        .unwrap()
        .abilities
        .has_ability(HEED_OUR_CALLING));
    assert_eq!(speed_pet(&mgr), 0, "refunded: the passive is gone");
}
