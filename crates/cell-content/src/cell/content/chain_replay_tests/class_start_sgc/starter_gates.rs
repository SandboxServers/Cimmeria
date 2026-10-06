//! Chains 3001, 3017 and 3018 carry `archetype neq 7` (OD-CS09): a Free
//! Jaffa visiting SGC_W1 is not pulled into the Human tutorial (M1559) or
//! into M1561. The gate excludes the Free Jaffa only: the Asgard holding
//! state, every other visitor and a player with no archetype still get the
//! chains, as before CS-05.
//!
//! Each test fires the real dispatcher for every archetype and reads a side
//! effect the chain's own actions leave, so it fails if the condition row is
//! removed (the Free Jaffa case) or widened to `eq`/`lt` (the others).

use tokio::sync::mpsc;

use super::super::super::event_dispatch::{
    fire_dialog_choice, fire_entity_death, fire_player_loaded,
};
use super::{
    drain, engine_with, interaction_flags, label, sgc_mgr, spawn_tagged, Sent, FREE_JAFFA, HUMANS,
    NON_HUMANS, PLAYER_EID, PLAYER_ID, STORY_ACTIVE,
};
use crate::test_support::require_db_or_skip;

const TAGGED_EID: u32 = 7102;

/// Every archetype, plus the player with none.
fn everyone() -> Vec<Option<i32>> {
    HUMANS
        .iter()
        .chain(NON_HUMANS.iter())
        .map(|&a| Some(a))
        .chain([None])
        .collect()
}

/// Chain 3001 (`player_loaded SGC_W1`, 1559 not active): accepts M1559 and
/// marks Gen. Hammond. Read through Hammond's marker.
#[tokio::test]
async fn live_db_chain_3001_starts_the_human_tutorial_for_everyone_but_a_free_jaffa() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3001]).await;

    for archetype in everyone() {
        let mut mgr = sgc_mgr(archetype);
        spawn_tagged(&mut mgr, TAGGED_EID, "SGCW1_GenHammond");
        let (tx, _rx) = mpsc::channel(64);

        fire_player_loaded(PLAYER_EID, PLAYER_ID, "SGC_W1", &engine, &tx, &mut mgr).await;

        let marked = interaction_flags(&mgr, TAGGED_EID) & STORY_ACTIVE != 0;
        assert_eq!(
            marked,
            archetype != Some(FREE_JAFFA),
            "chain 3001 for {}: Hammond is marked for everyone but a Free Jaffa",
            label(archetype),
        );
    }
}

/// Chain 3017 (`entity_dead_tag SGC_W1_JaffaBomb`): opens Hammond's radio
/// dialog 5359, the offer of M1561. The archetype is the killer's.
#[tokio::test]
async fn live_db_chain_3017_offers_m1561_to_everyone_but_a_free_jaffa() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3017]).await;

    for archetype in everyone() {
        let mut mgr = sgc_mgr(archetype);
        // `display_dialog` on a death has no clicked entity; it binds the
        // player's last interaction target.
        spawn_tagged(&mut mgr, TAGGED_EID, "SGC_W1_FirearmBody");
        mgr.get_entity_mut(PLAYER_EID)
            .unwrap()
            .last_interaction_target = Some(TAGGED_EID);
        let (tx, mut rx) = mpsc::channel(64);

        fire_entity_death(
            PLAYER_EID,
            PLAYER_ID,
            "SGC_W1_JaffaBomb",
            &engine,
            &tx,
            &mut mgr,
        )
        .await;

        let expected = if archetype == Some(FREE_JAFFA) {
            vec![]
        } else {
            vec![Sent::Dialog(5359)]
        };
        assert_eq!(
            drain(&mut rx),
            expected,
            "chain 3017 for {}: dialog 5359 for everyone but a Free Jaffa",
            label(archetype),
        );
    }
}

/// Chain 3018 (`dialog_choice 5359`, 1561 not active): accepts M1561 and
/// marks the Airman's body. Read through the body's marker.
#[tokio::test]
async fn live_db_chain_3018_accepts_m1561_for_everyone_but_a_free_jaffa() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[3018]).await;

    for archetype in everyone() {
        let mut mgr = sgc_mgr(archetype);
        spawn_tagged(&mut mgr, TAGGED_EID, "SGCW1_AirmanBody");
        let (tx, _rx) = mpsc::channel(64);

        fire_dialog_choice(PLAYER_EID, PLAYER_ID, 5359, -1, &engine, &tx, &mut mgr).await;

        let marked = interaction_flags(&mgr, TAGGED_EID) & STORY_ACTIVE != 0;
        assert_eq!(
            marked,
            archetype != Some(FREE_JAFFA),
            "chain 3018 for {}: the Airman's body is marked for everyone but a Free Jaffa",
            label(archetype),
        );
    }
}
