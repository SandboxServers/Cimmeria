//! Beta Site E1 Incendiary (UB-34, mission 443): the exclusivity guards.
//! Every Human race at every 443 state matches no 443 chain, a whole-walk
//! check proves one 443 chain per Asgard state, and every positive event for
//! a non-Asgard or unstamped archetype matches none.

use super::*;

/// Every positive 443 event, at every step, for every non-Asgard race and for
/// an unstamped archetype (`None`, in `non_asgard`), matches no 443 chain.
/// Revert: drop the `archetype eq 5` row from any one chain.
#[tokio::test]
async fn no_443_chain_fires_without_asgard() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut events = Vec::new();
        let mut login_offer = login_ctx(BETA, archetype);
        set_stage(&mut login_offer, Stage::Offer);
        events.push((TriggerType::PlayerLoaded, login_offer));
        let mut blurb = completed(1147, archetype);
        set_stage(&mut blurb, Stage::Offer);
        events.push((TriggerType::MissionCompleted, blurb));
        let mut accept = choice(OFFER_DIALOG, archetype);
        set_stage(&mut accept, Stage::Offer);
        events.push((TriggerType::DialogChoice, accept));
        for (step, tag) in STEPS {
            let mut restore = login_ctx(BETA, archetype);
            set_stage(&mut restore, Stage::Step(step));
            events.push((TriggerType::PlayerLoaded, restore));
            let mut click_step = click(tag, archetype);
            set_stage(&mut click_step, Stage::Step(step));
            events.push((TriggerType::InteractTag, click_step));
        }
        let mut walk_in = region(FIRE_SITE, archetype);
        set_stage(&mut walk_in, Stage::Step(1484));
        events.push((TriggerType::RegionEnter, walk_in));
        for (trigger, ctx) in events {
            let resolved = fire(&engine, trigger, &ctx);
            assert!(m443_matched(&resolved).is_empty(), "{archetype:?}");
        }
    }
}

/// Every Human race, at every 443 state, matches no 443 chain: the Asgard
/// gate is `eq 5`, so the Human range never reaches Carter, Satterfield or
/// the fire. Revert: change 23411's gate to a Human range.
#[tokio::test]
async fn satterfield_is_asgard_443_only() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in HUMANS {
        for stage in [Stage::Offer, Stage::Done]
            .into_iter()
            .chain(STEPS.iter().map(|(step, _)| Stage::Step(*step)))
        {
            for tag in [CARTER_TAG, SATTERFIELD_TAG, MANAGER_TAG, HUB_TAG, MIMIR] {
                expect(
                    &engine,
                    stage,
                    TriggerType::InteractTag,
                    click(tag, Some(archetype)),
                    &[],
                );
            }
        }
    }
}

/// The whole 443 walk, from the offer to Done, with a fresh engine state at
/// each step: one 443 chain per Asgard state on each click, and the relog
/// restore at each step. Revert: any of the 23405-23417 chains.
#[tokio::test]
async fn carter_and_mimir_one_dialog_chain_per_asgard_state() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let a = Some(ASGARD);
    expect(
        &engine,
        Stage::Offer,
        TriggerType::InteractTag,
        click(MIMIR, a),
        &[OFFER],
    );
    expect(
        &engine,
        Stage::Offer,
        TriggerType::InteractTag,
        click(CARTER_TAG, a),
        &[],
    );
    expect(
        &engine,
        Stage::Offer,
        TriggerType::PlayerLoaded,
        login_ctx(BETA, a),
        &[BLURB],
    );
    expect(
        &engine,
        Stage::Step(1482),
        TriggerType::InteractTag,
        click(MIMIR, a),
        &[],
    );
    expect(
        &engine,
        Stage::Step(1482),
        TriggerType::InteractTag,
        click(CARTER_TAG, a),
        &[CARTER],
    );
    expect(
        &engine,
        Stage::Step(1482),
        TriggerType::PlayerLoaded,
        login_ctx(BETA, a),
        &[CARTER_RESTORE],
    );
    expect(
        &engine,
        Stage::Step(1483),
        TriggerType::InteractTag,
        click(CARTER_TAG, a),
        &[],
    );
    expect(
        &engine,
        Stage::Step(1483),
        TriggerType::InteractTag,
        click(SATTERFIELD_TAG, a),
        &[SATTERFIELD],
    );
    expect(
        &engine,
        Stage::Step(1483),
        TriggerType::PlayerLoaded,
        login_ctx(BETA, a),
        &[SATTERFIELD_RESTORE],
    );
    expect(
        &engine,
        Stage::Step(1484),
        TriggerType::InteractTag,
        click(SATTERFIELD_TAG, a),
        &[],
    );
    expect(
        &engine,
        Stage::Step(1484),
        TriggerType::RegionEnter,
        region(FIRE_SITE, a),
        &[REGION],
    );
    expect(
        &engine,
        Stage::Step(1484),
        TriggerType::InteractTag,
        click(MANAGER_TAG, a),
        &[RADIO],
    );
    expect(
        &engine,
        Stage::Step(1484),
        TriggerType::PlayerLoaded,
        login_ctx(BETA, a),
        &[],
    );
    expect(
        &engine,
        Stage::Step(1485),
        TriggerType::InteractTag,
        click(MANAGER_TAG, a),
        &[MANAGER],
    );
    expect(
        &engine,
        Stage::Step(1485),
        TriggerType::RegionEnter,
        region(FIRE_SITE, a),
        &[],
    );
    expect(
        &engine,
        Stage::Step(1486),
        TriggerType::InteractTag,
        click(MANAGER_TAG, a),
        &[],
    );
    expect(
        &engine,
        Stage::Step(1486),
        TriggerType::InteractTag,
        click(HUB_TAG, a),
        &[HUB],
    );
    expect(
        &engine,
        Stage::Step(3455),
        TriggerType::InteractTag,
        click(HUB_TAG, a),
        &[],
    );
    expect(
        &engine,
        Stage::Step(3455),
        TriggerType::InteractTag,
        click(MIMIR, a),
        &[TURN_IN],
    );
    expect(
        &engine,
        Stage::Step(3455),
        TriggerType::PlayerLoaded,
        login_ctx(BETA, a),
        &[TURN_IN_RESTORE],
    );
    expect(
        &engine,
        Stage::Done,
        TriggerType::InteractTag,
        click(MIMIR, a),
        &[],
    );
    expect(
        &engine,
        Stage::Done,
        TriggerType::PlayerLoaded,
        login_ctx(BETA, a),
        &[],
    );
    expect(
        &engine,
        Stage::Done,
        TriggerType::InteractTag,
        click(CARTER_TAG, a),
        &[],
    );
}
