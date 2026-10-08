//! M1559's FirearmBody pickup after CS-05: chain 3008 (the four Human
//! classes) and chain 3029 (everyone else, the Asgard holding state of
//! OD-CS09), and the combat tutorial 5883 that chain 7101 hangs off 5882.
//!
//! Both chains are registered together in every test, as they are in play,
//! so a player who matched both would show up as two pistols.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::{AbilityGrantKind, Action};

use super::super::super::engine_loader::load_single_chain_for_test;
use super::super::super::event_dispatch::{fire_interact_tag, fire_pending_combat_entries};
use super::{
    drain, engine_with, give_mission, label, mission_status, sgc_mgr, spawn_tagged, Sent, ASGARD,
    COMPLETED, HUMANS, NON_HUMANS, PLAYER_EID, PLAYER_ID,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const BODY_EID: u32 = 7102;
const MOB_EID: u32 = 7103;
const BODY_TAG: &str = "SGC_W1_FirearmBody";

const PISTOL: Sent = Sent::Item(55, 1, 1);
const CORPSE_LINE: Sent = Sent::Dialog(5358);
const EQUIPPING_A_WEAPON: Sent = Sent::Tutorial(5882);
const COMBAT: Sent = Sent::Tutorial(5883);

/// The CORE_TUTORIAL grant of the Abilities matrix (Pistol Shot, Strike,
/// Heal Focus, Recuperation), as chain 3008 must send it.
fn core_grant() -> Sent {
    Sent::Abilities(
        vec![592, 594, 597, 1218],
        AbilityGrantKind::Tutorial,
        Some(1559),
        vec![],
    )
}

/// A player of `archetype` on M1559's last step, standing at the body.
fn at_the_body(archetype: Option<i32>) -> SpaceManager {
    let mut mgr = sgc_mgr(archetype);
    spawn_tagged(&mut mgr, BODY_EID, BODY_TAG);
    give_mission(&mut mgr, 1559, 4614);
    mgr
}

/// **Guard: a Human gets the pistol, then the core abilities, then tutorial
/// 5882, once.** The order is the point: the tutorial tells the player to
/// put Pistol Shot on the bar, so `onKnownAbilitiesUpdate` has to be ahead
/// of it, and the base handles cell messages in send order. Drop the
/// `grant_ability` or `show_tutorial` row from chain 3008, or reorder them,
/// and this fails.
#[tokio::test]
async fn live_db_chain_3008_gives_a_human_the_pistol_then_the_core_grant_then_tutorial_5882() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3008, 3029]).await;

    // A player with no archetype reads -1 and takes the Human branch.
    for archetype in HUMANS.iter().map(|&a| Some(a)).chain([None]) {
        let mut mgr = at_the_body(archetype);
        let (tx, mut rx) = mpsc::channel(64);

        fire_interact_tag(
            PLAYER_EID, PLAYER_ID, BODY_TAG, BODY_EID, &engine, &tx, &mut mgr,
        )
        .await;

        assert_eq!(
            drain(&mut rx),
            vec![PISTOL, core_grant(), CORPSE_LINE, EQUIPPING_A_WEAPON],
            "chain 3008 for {}: pistol, core grant, corpse line, tutorial 5882",
            label(archetype),
        );
        assert_eq!(
            mission_status(&mgr, 1559),
            COMPLETED,
            "chain 3008 for {}: M1559 is complete",
            label(archetype),
        );

        // The gate is `mission_status 1559 eq active`: a second press on the
        // body gives nothing more.
        fire_interact_tag(
            PLAYER_EID, PLAYER_ID, BODY_TAG, BODY_EID, &engine, &tx, &mut mgr,
        )
        .await;
        assert_eq!(
            drain(&mut rx),
            vec![],
            "{}: a second press grants nothing",
            label(archetype)
        );
    }
}

/// **Guard: the Asgard holding state (OD-CS09).** An Asgard, and every
/// other non-Human with M1559 active, gets what chain 3008 gave before
/// CS-05 and nothing else: mission complete, pistol 55 to the backpack, the
/// corpse line. No ability grant and no tutorial. Loosen chain 3008's
/// `archetype lt 5`, or add a grant to chain 3029, and this fails.
#[tokio::test]
async fn live_db_asgard_and_other_non_humans_keep_the_pre_cs05_pickup() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3008, 3029]).await;

    for archetype in NON_HUMANS {
        let mut mgr = at_the_body(Some(archetype));
        let (tx, mut rx) = mpsc::channel(64);

        fire_interact_tag(
            PLAYER_EID, PLAYER_ID, BODY_TAG, BODY_EID, &engine, &tx, &mut mgr,
        )
        .await;

        assert_eq!(
            drain(&mut rx),
            vec![PISTOL, CORPSE_LINE],
            "archetype {archetype}: the pistol and the corpse line, as before CS-05",
        );
        assert_eq!(mission_status(&mgr, 1559), COMPLETED);
        assert!(
            mgr.get_entity(PLAYER_EID)
                .unwrap()
                .shown_tutorials
                .is_empty(),
            "archetype {archetype}: no tutorial is marked",
        );
    }
}

/// Chain 3029's action list is chain 3008's as it was before CS-05, row for
/// row. This is the literal half of "the Asgard keeps today's runtime": the
/// test above reads what reaches the base, this one reads the seed.
#[tokio::test]
async fn live_db_chain_3029_is_the_pre_cs05_chain_3008() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 3029)
        .await
        .expect("DB query for chain 3029 must succeed")
        .expect("chain 3029 must be seeded and must load");

    assert_eq!(
        chain.actions,
        vec![
            Action::CompleteMission { mission_id: 1559 },
            Action::GrantItem {
                item_id: 55,
                count: 1,
                container_id: Some(1),
            },
            Action::DisplayDialog { dialog_id: 5358 },
        ],
        "chain 3029 must stay the three actions chain 3008 had before CS-05",
    );
    assert!(
        chain.action_delays.iter().all(|&d| d == 0),
        "chain 3029 runs immediately, as chain 3008 did",
    );
}

/// **Guard: 5883 "Combat" reaches an SGC Human and not the Asgard.** Chain
/// 7101 is gated on 5882 having been shown, which only chain 3008 does. Run
/// the pickup, put the player in hostile combat, drain the combat queue:
/// the Human is sent 5883 once, the Asgard never.
#[tokio::test]
async fn live_db_chain_7101_shows_5883_after_the_sgc_pickup_for_a_human_only() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3008, 3029, 7101]).await;

    for (archetype, expected) in [(HUMANS[0], vec![COMBAT]), (ASGARD, vec![])] {
        let mut mgr = at_the_body(Some(archetype));
        mgr.spawn_npc(MOB_EID, "SGC_W1", [5.0, 0.0, 5.0], [0.0; 3])
            .expect("SGC_W1 must accept the mob");
        let (tx, mut rx) = mpsc::channel(64);

        fire_interact_tag(
            PLAYER_EID, PLAYER_ID, BODY_TAG, BODY_EID, &engine, &tx, &mut mgr,
        )
        .await;
        drain(&mut rx);

        let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_EID);
        fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
        assert_eq!(
            drain(&mut rx),
            expected,
            "archetype {archetype}: 5883 on the first combat after the pickup",
        );

        // Leaving and re-entering combat never asks for it again.
        let _ = crate::cell::combat::exit_player_combat(&mut mgr, PLAYER_EID, MOB_EID);
        let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_EID);
        fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
        assert_eq!(
            drain(&mut rx),
            vec![],
            "archetype {archetype}: 5883 is one-time"
        );
    }
}
