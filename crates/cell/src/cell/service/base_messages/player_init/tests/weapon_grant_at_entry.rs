//! World entry puts the active weapon's own abilities on the known list
//! (CS-07 review finding 1).
//!
//! Weapon-granted abilities are transient: never persisted, added only when
//! the weapon becomes active. Before the fix only a slot-change request did
//! that, so a player who logged in holding an SMG was never told about 559
//! and patch 015's bar slot had nothing to follow.

use super::super::*;
use cimmeria_entity::cell_entity::{BandolierItem, SystemOptions};

const SMG: i32 = 21;
const SMG_RANGED: i32 = 559;
const SMG_MELEE: i32 = 595;
const PISTOL_SHOT: i32 = 592;

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
    mgr.item_event_set_abilities
        .insert((SMG, crate::cell::spawner::EVENT_ITEM_RANGED), SMG_RANGED);
    mgr.item_event_set_abilities
        .insert((SMG, crate::cell::spawner::EVENT_ITEM_MELEE), SMG_MELEE);
    mgr
}

fn smg() -> BandolierItem {
    BandolierItem {
        instance_id: 9,
        item_id: SMG,
        clip_size: 30,
        default_ammo_type: 1,
        current_ammo: 30,
        cur_ammo_type: 1,
    }
}

/// The ids of every `onKnownAbilitiesUpdate` the player was sent.
fn known_updates(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<Vec<i32>> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } = msg
        {
            if method_index == crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE {
                out.push(
                    args[4..]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| i32::from_le_bytes(*c))
                        .collect(),
                );
            }
        }
    }
    out
}

/// **Regression guard.** Logging in with an SMG in the active slot: the
/// world-entry known list carries 559 and 595, tagged weapon-granted so the
/// next swap revokes them; the persisted 592 is untouched.
#[tokio::test]
async fn login_with_an_smg_sends_its_granted_abilities() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![PISTOL_SHOT],
        0,
        vec![(0, smg())],
        SystemOptions::default(),
        0,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    for id in [PISTOL_SHOT, SMG_RANGED, SMG_MELEE] {
        assert!(e.abilities.has_ability(id), "{id} is known after login");
    }
    let mut tagged = e.abilities.weapon_granted_ability_ids();
    tagged.sort();
    assert_eq!(
        tagged,
        vec![SMG_RANGED, SMG_MELEE],
        "the weapon's abilities are weapon-granted"
    );

    let updates = known_updates(&mut rx);
    let last = updates
        .last()
        .expect("world entry sends onKnownAbilitiesUpdate");
    assert!(
        last.contains(&SMG_RANGED) && last.contains(&SMG_MELEE),
        "the hotbar seed carries the active weapon's abilities: {last:?}"
    );
}

/// An empty active slot grants nothing and the login still seeds the list.
#[tokio::test]
async fn login_unarmed_grants_no_weapon_abilities() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![PISTOL_SHOT],
        0,
        vec![(1, smg())],
        SystemOptions::default(),
        0,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(e.abilities.weapon_granted_ability_ids().is_empty());
    assert!(!e.abilities.has_ability(SMG_RANGED));
    assert_eq!(known_updates(&mut rx).last(), Some(&vec![PISTOL_SHOT]));
}
