//! Beta Site E1 Incendiary (UB-34, mission 443): the hand-off and restore
//! chains (23408-23416) and the Mimir turn-in (23417). Seeded in
//! `db/resources/Content/Seed/beta_e1_m443_chains.sql`. The blurb, offer and
//! Accept are in the parent module.

use super::*;

/// Revert: drop 23408. Then a relog at S1482 leaves Carter unbound and silent.
#[tokio::test]
async fn relog_at_s1482_restores_carter_bind() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(1482));
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_eq!(
        summarized(&resolved, CARTER_RESTORE),
        vec!["add_dialog_set(3728, slot=2300)".to_string()],
    );
    assert_no_deferred_actions(&resolved, CARTER_RESTORE);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([CARTER_RESTORE]));
    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_stage(&mut ctx, Stage::Step(1482));
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(summarized(&resolved, CARTER_RESTORE).is_empty());
    }
    let mut later = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut later, Stage::Step(1483));
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &later);
    assert!(summarized(&resolved, CARTER_RESTORE).is_empty());
}

/// Revert: drop 23409's `remove_dialog_set 3728` or its `advance_step`. Then
/// Carter keeps the bind into S1483, or S1483 never starts.
#[tokio::test]
async fn carter_hands_off_to_satterfield() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(CARTER_TAG, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(1482));
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, CARTER),
        vec![
            "display_dialog(3895)".to_string(),
            "remove_dialog_set(3728, slot=2300)".to_string(),
            "advance_step(443, 1483)".to_string(),
            "add_dialog_set(3780, slot=2302)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, CARTER);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([CARTER]));
    for archetype in non_asgard() {
        expect(
            &engine,
            Stage::Step(1482),
            TriggerType::InteractTag,
            click(CARTER_TAG, archetype),
            &[],
        );
    }
    expect(
        &engine,
        Stage::Step(1483),
        TriggerType::InteractTag,
        click(CARTER_TAG, Some(ASGARD)),
        &[],
    );
    expect(
        &engine,
        Stage::Offer,
        TriggerType::InteractTag,
        click(CARTER_TAG, Some(ASGARD)),
        &[],
    );
}

/// Revert: drop 23410. Then a relog at S1483 leaves Satterfield unbound.
#[tokio::test]
async fn relog_at_s1483_restores_satterfield_bind() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(1483));
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_eq!(
        summarized(&resolved, SATTERFIELD_RESTORE),
        vec!["add_dialog_set(3780, slot=2302)".to_string()],
    );
    assert_no_deferred_actions(&resolved, SATTERFIELD_RESTORE);
    assert_eq!(
        m443_matched(&resolved),
        BTreeSet::from([SATTERFIELD_RESTORE])
    );
    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_stage(&mut ctx, Stage::Step(1483));
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(summarized(&resolved, SATTERFIELD_RESTORE).is_empty());
    }
    let mut earlier = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut earlier, Stage::Step(1482));
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &earlier);
    assert!(summarized(&resolved, SATTERFIELD_RESTORE).is_empty());
}

/// Revert: drop 23411's `remove_dialog_set 3780` or its `advance_step 1484`.
/// Then Satterfield keeps his bind, or the radio step never starts.
#[tokio::test]
async fn satterfield_sends_you_to_the_fire() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(SATTERFIELD_TAG, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(1483));
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, SATTERFIELD),
        vec![
            "display_dialog(3161)".to_string(),
            "remove_dialog_set(3780, slot=2302)".to_string(),
            "advance_step(443, 1484)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, SATTERFIELD);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([SATTERFIELD]));
    for archetype in non_asgard() {
        expect(
            &engine,
            Stage::Step(1483),
            TriggerType::InteractTag,
            click(SATTERFIELD_TAG, archetype),
            &[],
        );
    }
    expect(
        &engine,
        Stage::Step(1484),
        TriggerType::InteractTag,
        click(SATTERFIELD_TAG, Some(ASGARD)),
        &[],
    );
}

/// Revert: drop 23412's `S1484 active` gate or its `advance_step`. Then a
/// player who walks into FireSite at the wrong step, or never, moves on.
/// The 23413 fallback covers a player who already stands inside the region.
#[tokio::test]
async fn entering_fire_site_advances_to_1485() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = region(FIRE_SITE, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(1484));
    let resolved = fire(&engine, TriggerType::RegionEnter, &ctx);
    assert_eq!(
        summarized(&resolved, REGION),
        vec!["advance_step(443, 1485)".to_string()],
    );
    assert_no_deferred_actions(&resolved, REGION);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([REGION]));
    for archetype in non_asgard() {
        let mut ctx = region(FIRE_SITE, archetype);
        set_stage(&mut ctx, Stage::Step(1484));
        let resolved = fire(&engine, TriggerType::RegionEnter, &ctx);
        assert!(summarized(&resolved, REGION).is_empty(), "{archetype:?}");
    }
    let mut later = region(FIRE_SITE, Some(ASGARD));
    set_stage(&mut later, Stage::Step(1485));
    let resolved = fire(&engine, TriggerType::RegionEnter, &later);
    assert!(summarized(&resolved, REGION).is_empty());
    let mut elsewhere = region("Beta_Site_Evo_1.Hub", Some(ASGARD));
    set_stage(&mut elsewhere, Stage::Step(1484));
    let resolved = fire(&engine, TriggerType::RegionEnter, &elsewhere);
    assert!(summarized(&resolved, REGION).is_empty());
}

/// Revert: drop 23413's `S1484 active` gate, or 23414's `S1485 active` gate.
/// Then one manager click fires both chains, or the step does not advance.
#[tokio::test]
async fn manager_states_are_disjoint() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut at_1484 = click(MANAGER_TAG, Some(ASGARD));
    set_stage(&mut at_1484, Stage::Step(1484));
    let resolved = fire(&engine, TriggerType::InteractTag, &at_1484);
    assert_eq!(
        summarized(&resolved, RADIO),
        vec!["advance_step(443, 1485)".to_string()],
    );
    assert_no_deferred_actions(&resolved, RADIO);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([RADIO]));
    let mut at_1485 = click(MANAGER_TAG, Some(ASGARD));
    set_stage(&mut at_1485, Stage::Step(1485));
    let resolved = fire(&engine, TriggerType::InteractTag, &at_1485);
    assert_eq!(
        summarized(&resolved, MANAGER),
        vec!["advance_step(443, 1486)".to_string()],
    );
    assert_no_deferred_actions(&resolved, MANAGER);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([MANAGER]));
    expect(
        &engine,
        Stage::Step(1486),
        TriggerType::InteractTag,
        click(MANAGER_TAG, Some(ASGARD)),
        &[],
    );
    for archetype in non_asgard() {
        expect(
            &engine,
            Stage::Step(1484),
            TriggerType::InteractTag,
            click(MANAGER_TAG, archetype),
            &[],
        );
    }
}

/// Revert: drop 23415's `S1486 active` gate or its `advance_step 3455`. Then
/// the fire control hub does not reach the turn-in, or the bind is missing.
#[tokio::test]
async fn fire_control_hub_advances_to_turn_in() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(HUB_TAG, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(1486));
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, HUB),
        vec![
            "advance_step(443, 3455)".to_string(),
            "add_dialog_set(4579, slot=2305)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, HUB);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([HUB]));
    for archetype in non_asgard() {
        expect(
            &engine,
            Stage::Step(1486),
            TriggerType::InteractTag,
            click(HUB_TAG, archetype),
            &[],
        );
    }
    expect(
        &engine,
        Stage::Step(1485),
        TriggerType::InteractTag,
        click(HUB_TAG, Some(ASGARD)),
        &[],
    );
}

/// Revert: drop 23416. Then a relog at S3455 leaves Mimir's turn-in unbound.
#[tokio::test]
async fn relog_at_s3455_restores_turn_in_bind() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(3455));
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_eq!(
        summarized(&resolved, TURN_IN_RESTORE),
        vec!["add_dialog_set(4579, slot=2305)".to_string()],
    );
    assert_no_deferred_actions(&resolved, TURN_IN_RESTORE);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([TURN_IN_RESTORE]));
    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_stage(&mut ctx, Stage::Step(3455));
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(summarized(&resolved, TURN_IN_RESTORE).is_empty());
    }
    let mut earlier = login_ctx(BETA, Some(ASGARD));
    set_stage(&mut earlier, Stage::Step(1486));
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &earlier);
    assert!(summarized(&resolved, TURN_IN_RESTORE).is_empty());
}

/// Revert: drop 23417's `S3455 active` gate or its `complete_mission`. Then
/// the turn-in completes early, or 443 stays open after Mimir's turn-in.
#[tokio::test]
async fn mimir_turn_in_completes_443() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_stage(&mut ctx, Stage::Step(3455));
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, TURN_IN),
        vec![
            "display_dialog(1917)".to_string(),
            "remove_dialog_set(4579, slot=2305)".to_string(),
            "complete_mission(443)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, TURN_IN);
    assert_eq!(m443_matched(&resolved), BTreeSet::from([TURN_IN]));
    for archetype in non_asgard() {
        expect(
            &engine,
            Stage::Step(3455),
            TriggerType::InteractTag,
            click(MIMIR, archetype),
            &[],
        );
    }
    expect(
        &engine,
        Stage::Step(1486),
        TriggerType::InteractTag,
        click(MIMIR, Some(ASGARD)),
        &[],
    );
    expect(
        &engine,
        Stage::Done,
        TriggerType::InteractTag,
        click(MIMIR, Some(ASGARD)),
        &[],
    );
}
