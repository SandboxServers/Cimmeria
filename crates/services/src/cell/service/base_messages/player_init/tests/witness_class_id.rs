//! `InitPlayerState` decides the class OTHER players are shown for this
//! player: `SGWGmPlayer` for a GM, `SGWPlayer` otherwise — the same split the
//! owning client gets from `CREATE_BASE_PLAYER`.
//!
//! Every current account is a GM, so GM-meets-GM is the population the
//! shared worlds actually have; these drive the production message order
//! (`CreateEntity` identity stamp, `connect_entity`, `InitPlayerState`)
//! through to the `EnteredAoI` a witness receives.

use super::super::*;
use crate::mercury::{SGWGMPLAYER_CLASS_ID, SGWPLAYER_CLASS_ID};
use cimmeria_entity::cell_entity::SystemOptions;

const GM_A: u32 = 1;
const GM_B: u32 = 2;
const PLAIN: u32 = 3;

fn shared_world() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

/// `CreateEntity` (identity stamped) then `ConnectEntity`, as the base sends
/// them. The entity is NOT yet introducible — that needs `init`.
fn create_and_connect(mgr: &mut SpaceManager, id: u32) {
    mgr.create_entity(id, "Castle", [id as f32, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(id).unwrap();
    e.account_id = Some(id);
    e.player_id = Some(id as i32);
    mgr.connect_entity(id);
}

async fn init(mgr: &mut SpaceManager, id: u32, access_level: u32) {
    let (tx, _rx) = mpsc::channel(64);
    handle_init_player_state(
        id,
        id as i32,
        "Castle".into(),
        1, // archetype_id = Soldier
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0, // state_field
        access_level,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
}

/// `class_id` each `(witness, entity)` introduction carried.
fn introduced_classes(events: &[CellToBaseMsg]) -> Vec<(u32, u32, u8)> {
    let mut out: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            CellToBaseMsg::EnteredAoI {
                witness_id,
                entity_id,
                class_id,
                ..
            } => Some((*witness_id, *entity_id, *class_id)),
            _ => None,
        })
        .collect();
    out.sort_unstable();
    out
}

/// Regression guard for "a GM is introduced to other players as a plain
/// SGWPlayer": `connect_entity` stamped 0x02 on every player and nothing ever
/// corrected it. Two GMs and a regular player in one shared space — each
/// witness must be shown every observee's real class.
#[tokio::test]
async fn witnesses_are_shown_a_gm_as_sgwgmplayer_and_a_player_as_sgwplayer() {
    let mut mgr = shared_world();
    for id in [GM_A, GM_B, PLAIN] {
        create_and_connect(&mut mgr, id);
    }
    init(&mut mgr, GM_A, 4).await; // Developer
    init(&mut mgr, GM_B, 2).await; // GameMaster
    init(&mut mgr, PLAIN, 0).await;

    let got = introduced_classes(&mgr.compute_aoi_changes());

    let gm = SGWGMPLAYER_CLASS_ID;
    let plain = SGWPLAYER_CLASS_ID;
    assert_eq!(
        got,
        vec![
            (GM_A, GM_B, gm),
            (GM_A, PLAIN, plain),
            (GM_B, GM_A, gm),
            (GM_B, PLAIN, plain),
            (PLAIN, GM_A, gm),
            (PLAIN, GM_B, gm),
        ]
    );
}

/// The pre-init placeholder class must be unobservable: a connected GM whose
/// `InitPlayerState` has not landed is not introduced at all, so no witness
/// can be sent 0x02 for what turns out to be an SGWGmPlayer. (The client
/// binds an entity's description at CREATE_ENTITY; there is no later message
/// that could correct the class.)
#[tokio::test]
async fn a_gm_is_never_introduced_under_the_placeholder_class() {
    let mut mgr = shared_world();
    create_and_connect(&mut mgr, GM_A);
    init(&mut mgr, GM_A, 2).await;
    create_and_connect(&mut mgr, GM_B); // connected, InitPlayerState pending

    let before = introduced_classes(&mgr.compute_aoi_changes());
    assert!(
        !before.iter().any(|&(_, entity, _)| entity == GM_B),
        "uninitialised GM must not be introduced: {before:?}"
    );

    init(&mut mgr, GM_B, 2).await;
    let after = introduced_classes(&mgr.compute_aoi_changes());
    assert_eq!(after, vec![(GM_A, GM_B, SGWGMPLAYER_CLASS_ID)]);
}
