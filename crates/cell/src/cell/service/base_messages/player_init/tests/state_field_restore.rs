//! **#412 restore guards.** `InitPlayerState` must restore the
//! persisted preference bits of `state_field` (today:
//! `BSF_AutoCycling`), re-arm `abilities.auto_cycle`, and
//! re-broadcast `onStateFieldUpdate` so the client's button
//! highlight survives the relog — while a corrupt row carrying
//! transient combat bits must be masked out so the player never
//! spawns dead / frozen / in-combat.

use super::super::*;
use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_DEAD, BSF_IN_COMBAT, BSF_MOVEMENT_LOCK};
use cimmeria_entity::cell_entity::SystemOptions;

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
    }
    mgr.connect_entity(1);
    mgr
}

fn drain_state_field_broadcasts(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<u32> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE && args.len() == 4
            {
                out.push(u32::from_le_bytes([args[0], args[1], args[2], args[3]]));
            }
        }
    }
    out
}

/// Happy path: persisted BSF_AutoCycling restores onto the entity,
/// re-arms the loop flag, and re-broadcasts the bit to the client.
/// This is the acceptance shape from #412 — after relog, the first
/// attack enters `arm_auto_cycle` (gated on `abilities.auto_cycle`)
/// and the loop drives itself without a second button press.
#[tokio::test]
async fn restores_auto_cycle_bit_and_rearms_loop_flag() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        BSF_AUTO_CYCLING,
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert_ne!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "persisted BSF_AutoCycling must restore onto the entity"
    );
    assert!(
        e.abilities.auto_cycle,
        "auto_cycle must re-arm so the first attack of the session \
         enters arm_auto_cycle and the loop starts (the #412 symptom \
         was exactly this flag staying false)"
    );
    assert!(
        e.abilities.auto_cycle_ability_id.is_none(),
        "the ability stash stays empty until first commit, by design"
    );

    let broadcasts = drain_state_field_broadcasts(&mut rx);
    assert_eq!(
        broadcasts,
        vec![BSF_AUTO_CYCLING],
        "restore must re-broadcast onStateFieldUpdate so the client's \
         gun-icon button highlights without an in-session toggle"
    );
}

/// Mask guard: a corrupt / hand-edited row carrying transient combat
/// bits must NOT restore them — spawning dead + movement-locked is
/// the failure shape the mask exists to prevent. No broadcast either
/// (nothing legitimate was restored).
#[tokio::test]
async fn transient_bits_in_saved_row_are_not_restored() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        BSF_DEAD | BSF_IN_COMBAT | BSF_MOVEMENT_LOCK,
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.state_field, 0,
        "transient combat bits must be masked out on restore — a \
         relog is always a clean combat slate"
    );
    assert!(!e.abilities.auto_cycle, "loop flag must stay disarmed");
    assert!(
        drain_state_field_broadcasts(&mut rx).is_empty(),
        "nothing restored → no onStateFieldUpdate"
    );
}

/// Zero-state companion: the common case (player never touched the
/// button) must not emit a spurious state-field packet during the
/// already-busy world-entry burst.
#[tokio::test]
async fn zero_state_field_emits_no_broadcast() {
    let mut mgr = make_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0,
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        drain_state_field_broadcasts(&mut rx).is_empty(),
        "state_field 0 must not add a packet to the login burst"
    );
}
