//! Mission 622 — Arm Yourself!, the first pistol as the core tutorial
//! milestone (Class Start v6, CS-04; OD-CS01, OD-CS04, OD-CS08).
//!
//! Searching the NID Guard's body (dialog 3996, step 80623) grants the
//! pistol. For a Human or a Loyalist Jaffa the same chain, 1005, then
//! teaches the four CORE_TUTORIAL abilities (592 Pistol Shot, 594 Strike,
//! 597 Heal Focus, 1218 Recuperation) and shows tutorial 5882 "Equipping a
//! Weapon", in that order; tutorial 5883 "Combat" follows from the global
//! chain 7101 on the first hostile combat. Chain 1010 is the Goa'uld holding
//! state: the pistol and the step, exactly as before CS-04.
//!
//! The gates, the per-step flow and the relog chains of 622 stay in
//! [`super::mission_622`]. Everything here runs against the full seeded
//! engine (`build_engine`, which also runs the loader's unknown-ability and
//! unknown-tutorial refusals), so a refused or duplicate chain shows up as a
//! wrong action list. The base's half (the provenance row, the tutorial
//! row) is played by hand or left to its own tests.

use std::collections::HashMap;

use tokio::sync::mpsc;

use cimmeria_content_engine::ability_grant::AbilityGrant;
use cimmeria_content_engine::actions::{AbilityGrantKind, Action};
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::MissionInstance;

use super::super::engine_loader::build_engine;
use crate::cell::content::{
    apply_tutorial_recorded, fire_dialog_open, fire_pending_combat_entries,
};
use crate::cell::messages::{
    CellToBaseMsg, ContentGrantAbilities, RecordTutorialShown, TutorialRecordOutcome,
    TutorialRecorded,
};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WorldRow;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7622;
const PLAYER_ID: i32 = 4622;
const GUARD_EID: u32 = 7623;

const GUARD_BODY_DIALOG: i32 = 3996;
const PISTOL: i32 = 55;
const BACKPACK: i32 = 1;
const CORE_TUTORIAL_ABILITIES: [i32; 4] = [592, 594, 597, 1218];
/// Soldier, Commando, Scientist, Archaeologist, Loyalist Jaffa
/// (`EArchetype`, `entities/defs/enumerations.xml`).
const HUMAN_AND_LOYALIST: [i32; 5] = [1, 2, 3, 4, 8];
const GOAULD: i32 = 6;
const EQUIPPING_A_WEAPON: i32 = 5882;
const COMBAT: i32 = 5883;

const CORE_CHAIN: i64 = 1005;
const HOLDING_CHAIN: i64 = 1010;
const COMBAT_TUTORIAL_CHAIN: i64 = 7101;

/// What the Guard search did before CS-04 and still does for everyone:
/// clear the body's search bit, put the pistol in the backpack, advance to
/// the equip step.
fn guard_search(chain: i64) -> Vec<(i64, Action)> {
    vec![
        (
            chain,
            Action::SetInteractionType {
                entity_tag: "ArmYourself_GuardBody".to_string(),
                operation: "~".to_string(),
                mask: 4_194_304,
            },
        ),
        (
            chain,
            Action::GrantItem {
                item_id: PISTOL,
                count: 1,
                container_id: Some(BACKPACK),
            },
        ),
        (
            chain,
            Action::AdvanceStep {
                mission_id: 622,
                step_id: 80622,
            },
        ),
    ]
}

/// Every action the Guard-body dialog resolves at step 80623, across the
/// whole seeded engine. `archetype = None` leaves the param out (the
/// condition then reads -1).
fn search_guard(engine: &ChainEngine, archetype: Option<i32>) -> Vec<(i64, Action)> {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "dialog_id".to_string(),
        serde_json::json!(GUARD_BODY_DIALOG),
    );
    ctx.set_param(
        "mission_622_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_622_step_80623_status".to_string(),
        serde_json::json!("active"),
    );
    if let Some(archetype) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx).actions
}

/// **Guard: a Human or Loyalist Jaffa gets the pistol, then the core
/// abilities, then tutorial 5882 (OD-CS04).** For archetypes 1-4 and 8 the
/// Guard search resolves exactly chain 1005: the pre-CS-04 search, then one
/// `tutorial` grant of 592/594/597/1218 (source 622, gated on those five
/// archetypes on the grant row itself), then `show_tutorial 5882`. The whole
/// list is compared in order, so a tutorial before the abilities, abilities
/// before the pistol, a second pistol from a chain whose gate overlaps, and
/// a grant open to every archetype all fail it. Remove the two CS-04 action
/// rows and the list is three actions short.
#[tokio::test]
async fn live_db_guard_search_gives_pistol_then_core_abilities_then_tutorial_5882() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let mut expected = guard_search(CORE_CHAIN);
    expected.push((
        CORE_CHAIN,
        Action::GrantAbility(AbilityGrant {
            ability_ids: CORE_TUTORIAL_ABILITIES.to_vec(),
            source_kind: AbilityGrantKind::Tutorial,
            source_id: Some(622),
            archetypes: HUMAN_AND_LOYALIST.to_vec(),
        }),
    ));
    expected.push((
        CORE_CHAIN,
        Action::ShowTutorial {
            tutorial_id: EQUIPPING_A_WEAPON,
        },
    ));

    for archetype in HUMAN_AND_LOYALIST {
        assert_eq!(
            search_guard(&engine, Some(archetype)),
            expected,
            "archetype {archetype} searching the Guard"
        );
    }
}

/// **Guard: the Goa'uld holding state is the pre-CS-04 Guard search
/// (OD-CS08).** A Goa'uld (archetype 6, char_defs 10/19) resolves exactly
/// chain 1010: clear the search bit, the pistol, the step. No grant and no
/// tutorial. An Asgard (5) and a Shol'va (7) have no Cellblock start and get
/// the same. Add either CS-04 row to the holding chain, or widen chain
/// 1005's gate to 6, and the list is no longer equal.
#[tokio::test]
async fn live_db_guard_search_holding_state_is_the_pistol_and_the_step_only() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for archetype in [GOAULD, 5, 7] {
        assert_eq!(
            search_guard(&engine, Some(archetype)),
            guard_search(HOLDING_CHAIN),
            "archetype {archetype} keeps the pre-CS-04 Guard search"
        );
    }
}

/// **Guard: exactly one pistol and one step advance for every archetype.**
/// Chains 1005 and 1010 split one trigger on `archetype`; a gap between the
/// two gates leaves a player with no pistol and mission 622 stuck, an
/// overlap grants two. A missing archetype (-1) must still get the pistol.
#[tokio::test]
async fn live_db_guard_search_grants_one_pistol_whatever_the_archetype() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for archetype in std::iter::once(None).chain((-1..=10).map(Some)) {
        let actions = search_guard(&engine, archetype);
        let pistols = actions
            .iter()
            .filter(|(_, a)| {
                matches!(
                    a,
                    Action::GrantItem {
                        item_id: PISTOL,
                        ..
                    }
                )
            })
            .count();
        let advances = actions
            .iter()
            .filter(|(_, a)| {
                matches!(
                    a,
                    Action::AdvanceStep {
                        mission_id: 622,
                        step_id: 80622
                    }
                )
            })
            .count();
        assert_eq!(
            (pistols, advances),
            (1, 1),
            "archetype {archetype:?}: {actions:?}"
        );
    }
}

/// Castle_CellBlock with one connected player of `archetype` on step 80623
/// of mission 622 and one NPC to fight.
fn cellblock_with_player_at_the_guard(archetype: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&HashMap::from([(
        "Castle_CellBlock".to_string(),
        WorldRow::enforcing(2),
    )]));
    mgr.create_entity(PLAYER_EID, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(GUARD_EID, "Castle_CellBlock", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER_EID).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(archetype);
    p.missions
        .add_mission(MissionInstance::new(622, 80623, Vec::new()));
    mgr.connect_entity(PLAYER_EID);
    mgr
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// Index of the pistol's `GrantItem` (backpack, one, not a loot or GM
/// grant) in `msgs`, asserting there is exactly one.
fn pistol_grant_index(msgs: &[CellToBaseMsg]) -> usize {
    let hits: Vec<usize> = msgs
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            matches!(
                m,
                CellToBaseMsg::GrantItem {
                    entity_id: PLAYER_EID,
                    player_id: PLAYER_ID,
                    item_id: PISTOL,
                    container_id: BACKPACK,
                    count: 1,
                    notify_gm: false,
                    loot: None,
                }
            )
        })
        .map(|(i, _)| i)
        .collect();
    assert_eq!(hits.len(), 1, "one pistol grant to the base: {msgs:?}");
    hits[0]
}

fn tutorial_records(msgs: &[CellToBaseMsg]) -> Vec<RecordTutorialShown> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::RecordTutorialShown(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

/// **Guard: on the wire to the base, a Soldier's Guard search is pistol,
/// then abilities, then 5882, and the first combat afterwards asks for 5883
/// from the seeded chain 7101.** Through the real dispatcher and executor:
/// `fire_dialog_open(3996)` sends one `GrantItem 55`, then one
/// `ContentGrantAbilities` (592/594/597/1218, `tutorial`, source 622), then
/// one `RecordTutorialShown 5882`, in that order (the base handles them in
/// order, so `onKnownAbilitiesUpdate` reaches the client before the
/// tutorial). The base's `First` answer displays dialog 5882. Entering
/// combat then asks the base to record 5883 from chain 7101. A resolve-only
/// test cannot tell a wired executor arm from the catch-all, or see that
/// 7101's `tutorial_shown 5882` gate opens off the mark `show_tutorial`
/// leaves on a Cellblock player.
#[tokio::test]
async fn live_db_soldier_guard_search_reaches_the_base_in_order_and_first_combat_asks_for_5883() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let mut mgr = cellblock_with_player_at_the_guard(1);
    let (tx, mut rx) = mpsc::channel(64);

    fire_dialog_open(
        PLAYER_EID,
        PLAYER_ID,
        GUARD_BODY_DIALOG,
        &engine,
        &tx,
        &mut mgr,
    )
    .await;
    let msgs = drain(&mut rx);

    let pistol_at = pistol_grant_index(&msgs);
    let grants: Vec<(usize, &ContentGrantAbilities)> = msgs
        .iter()
        .enumerate()
        .filter_map(|(i, m)| match m {
            CellToBaseMsg::ContentGrantAbilities(g) => Some((i, g)),
            _ => None,
        })
        .collect();
    assert_eq!(grants.len(), 1, "one ability grant: {msgs:?}");
    let (grant_at, grant) = grants[0];
    assert_eq!(
        (
            grant.entity_id,
            grant.player_id,
            grant.chain_id,
            grant.ability_ids.as_slice(),
            grant.source_kind,
            grant.source_id,
            grant.archetypes.as_slice(),
        ),
        (
            PLAYER_EID,
            PLAYER_ID,
            CORE_CHAIN,
            CORE_TUTORIAL_ABILITIES.as_slice(),
            AbilityGrantKind::Tutorial,
            Some(622),
            HUMAN_AND_LOYALIST.as_slice(),
        )
    );
    let record_at = msgs
        .iter()
        .position(|m| matches!(m, CellToBaseMsg::RecordTutorialShown(_)))
        .expect("the tutorial is recorded");
    assert_eq!(
        tutorial_records(&msgs),
        vec![RecordTutorialShown {
            entity_id: PLAYER_EID,
            player_id: PLAYER_ID,
            chain_id: CORE_CHAIN,
            tutorial_id: EQUIPPING_A_WEAPON,
        }]
    );
    assert!(
        pistol_at < grant_at && grant_at < record_at,
        "pistol ({pistol_at}), then abilities ({grant_at}), then tutorial ({record_at})"
    );

    // The base recorded 5882 for the first time: the dialog is displayed.
    apply_tutorial_recorded(
        TutorialRecorded {
            entity_id: PLAYER_EID,
            player_id: PLAYER_ID,
            chain_id: CORE_CHAIN,
            tutorial_id: EQUIPPING_A_WEAPON,
            outcome: TutorialRecordOutcome::First,
        },
        &tx,
        &mut mgr,
    )
    .await;
    let mut frame = Vec::new();
    frame.extend_from_slice(&(PLAYER_EID as i32).to_le_bytes());
    frame.extend_from_slice(&EQUIPPING_A_WEAPON.to_le_bytes());
    frame.extend_from_slice(&0i32.to_le_bytes());
    frame.push(1);
    frame.extend_from_slice(&0i32.to_le_bytes());
    let displays: Vec<Vec<u8>> = drain(&mut rx)
        .into_iter()
        .filter_map(|m| match m {
            // onDialogDisplay, spelled out so a change to the constant
            // cannot make the assertion agree with itself.
            CellToBaseMsg::EntityMethodCall {
                method_index: 105,
                args,
                ..
            } => Some(args),
            _ => None,
        })
        .collect();
    assert_eq!(displays, vec![frame], "one onDialogDisplay for 5882");

    // First hostile combat in the Cellblock: the seeded chain 7101.
    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, GUARD_EID);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert_eq!(
        tutorial_records(&drain(&mut rx)),
        vec![RecordTutorialShown {
            entity_id: PLAYER_EID,
            player_id: PLAYER_ID,
            chain_id: COMBAT_TUTORIAL_CHAIN,
            tutorial_id: COMBAT,
        }],
        "the first combat after 5882 asks for 5883"
    );
}

/// **Guard: a Goa'uld's Guard search sends the pistol and nothing new, and
/// combat never asks for 5883 (OD-CS08).** Same path as the Soldier test
/// with archetype 6: one `GrantItem 55`, no `ContentGrantAbilities`, no
/// `RecordTutorialShown`, no tutorial mark on the entity; entering combat
/// afterwards records nothing, because chain 7101 waits for 5882.
#[tokio::test]
async fn live_db_goauld_guard_search_sends_the_pistol_only_and_combat_shows_no_tutorial() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let mut mgr = cellblock_with_player_at_the_guard(GOAULD);
    let (tx, mut rx) = mpsc::channel(64);

    fire_dialog_open(
        PLAYER_EID,
        PLAYER_ID,
        GUARD_BODY_DIALOG,
        &engine,
        &tx,
        &mut mgr,
    )
    .await;
    let msgs = drain(&mut rx);
    pistol_grant_index(&msgs);
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::ContentGrantAbilities(_) | CellToBaseMsg::RecordTutorialShown(_)
        )),
        "the holding state grants no ability and shows no tutorial: {msgs:?}"
    );
    assert!(mgr
        .get_entity(PLAYER_EID)
        .unwrap()
        .shown_tutorials
        .is_empty());

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, GUARD_EID);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert!(
        tutorial_records(&drain(&mut rx)).is_empty(),
        "5883 waits for 5882, which a Goa'uld never sees here"
    );
}
