//! ORG-05 smoke: a right-click on an organization registrar, driven through
//! the player dispatcher from the wire's method index, on an NPC spawned
//! from a template record, so the `INT_Organization` + registrar-set
//! recognition, the range gate and the forward to the base all run as in a
//! live cell.

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::interaction_flags::INT_ORGANIZATION;
use cimmeria_entity::organization::OrgType;

use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, npc_spawn_record, LogCapture};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 4100;
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// The player at the origin and an NPC at `at` with `bits` and `sets`.
fn stage(at: [f32; 3], bits: i64, sets: &[i32]) -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(PLAYER);
    mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(PLAYER_ID);
    let npc = mgr.allocate_npc_id();
    let mut record = npc_spawn_record("Agnos", at, bits, VaultScope::Personal);
    record.static_interaction_sets = sets.to_vec();
    mgr.spawn_npc_from_record(npc, &record).unwrap();
    (mgr, npc)
}

/// Right-click `npc`; the client calls and org messages it produced.
async fn click(mgr: &mut SpaceManager, npc: u32) -> (Vec<u16>, Vec<OrgCellToBase>) {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(npc as i32).to_le_bytes(),
            &tx,
            mgr,
            &ChainEngine::new()
        )
        .await
    );
    let (mut calls, mut org) = (Vec::new(), Vec::new());
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => calls.push(method_index),
            CellToBaseMsg::Org(o) => org.push(o),
            _ => {}
        }
    }
    (calls, org)
}

/// In range: the click asks the base about exactly the registrar's type
/// and sends the client nothing yet (the dialog waits for eligibility).
#[tokio::test]
async fn registrar_click_in_range_asks_the_base_with_the_seeded_type() {
    for (sets, org_type) in [
        (vec![7447], OrgType::Team),
        (vec![3, 7448], OrgType::Command),
    ] {
        let (mut mgr, npc) = stage([3.0, 0.0, 0.0], INT_ORGANIZATION, &sets);
        let (calls, org) = click(&mut mgr, npc).await;
        assert!(calls.is_empty(), "{calls:?}");
        assert_eq!(
            org,
            vec![OrgCellToBase::RegistrarOpen {
                player_id: PLAYER_ID,
                entity_id: PLAYER,
                npc_entity_id: npc,
                org_type,
            }]
        );
    }
}

/// Out of range: no request, only the line, and one `too_far` row with the
/// distance.
#[tokio::test]
async fn registrar_click_out_of_range_asks_nothing_and_says_why() {
    let capture = LogCapture::install();
    let (mut mgr, npc) = stage([30.0, 0.0, 0.0], INT_ORGANIZATION, &[7448]);

    let (calls, org) = click(&mut mgr, npc).await;

    assert!(org.is_empty(), "{org:?}");
    assert_eq!(calls, vec![ON_PLAYER_COMMUNICATION]);
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "org.registrar_open") && c.level == Level::INFO)
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].has_field("reason", "too_far"));
    assert!(rows[0].has_field("org_type", "command"));
    assert!(rows[0].fields.contains_key("distance"));
}

/// Seed data decides: the organization bit with no registrar set, or a
/// registrar set with no bit, is not a registrar, and the click falls
/// through to the generic dispatch.
#[tokio::test]
async fn half_a_registrar_is_not_a_registrar() {
    for (bits, sets) in [(INT_ORGANIZATION, vec![]), (0, vec![7447])] {
        let (mut mgr, npc) = stage([3.0, 0.0, 0.0], bits, &sets);
        let (_, org) = click(&mut mgr, npc).await;
        assert!(org.is_empty(), "bits {bits}, sets {sets:?}: {org:?}");
    }
}

/// Negative seam: a click whose request cannot reach the base (the channel
/// is closed) is WARN `org.registrar_forward_failed`
/// (`cell_to_base_closed`) and one `base_unreachable` row.
#[tokio::test]
async fn registrar_click_with_the_base_gone_warns() {
    let capture = LogCapture::install();
    let (mut mgr, npc) = stage([3.0, 0.0, 0.0], INT_ORGANIZATION, &[7447]);
    let (tx, rx) = mpsc::channel(4);
    drop(rx);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(npc as i32).to_le_bytes(),
            &tx,
            &mut mgr,
            &ChainEngine::new()
        )
        .await
    );
    let all = capture.all();
    let warn = all
        .iter()
        .find(|c| c.level == Level::WARN && c.has_field("event", "org.registrar_forward_failed"))
        .expect("the dropped request must WARN");
    assert!(warn.has_field("reason", "cell_to_base_closed"));
    assert!(all.iter().any(|c| c.level == Level::INFO
        && c.has_field("event", "org.registrar_open")
        && c.has_field("reason", "base_unreachable")));
}
