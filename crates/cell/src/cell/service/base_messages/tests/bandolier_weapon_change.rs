//! A weapon change through the inventory (`SyncBandolierItems`, a drag-equip)
//! or a grant into the active slot (`UpdateBandolierItem`) does what a slot
//! change does for the outgoing weapon (CS-07 review finding 1): the new
//! weapon's abilities replace the old one's on the known list, the stale
//! last-fired stash and auto-cycle loop go, and a cast warming up with the
//! old weapon is interrupted.

use std::time::{Duration, Instant};

use super::*;
use cimmeria_entity::cell_entity::PendingCast;

const PISTOL: i32 = 55;
const PISTOL_RANGED: i32 = 579;
const SMG: i32 = 21;
const SMG_RANGED: i32 = 559;

fn item(instance_id: i32, item_id: i32) -> BandolierItem {
    BandolierItem {
        instance_id,
        item_id,
        clip_size: 15,
        default_ammo_type: 1,
        current_ammo: 15,
        cur_ammo_type: 1,
    }
}

/// A player with the pistol drawn in slot 0 and its 579 granted, who last
/// fired 579, has a loop armed on it, and is warming up a 579 cast.
fn fixture() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.item_event_set_abilities.insert(
        (PISTOL, crate::cell::spawner::EVENT_ITEM_RANGED),
        PISTOL_RANGED,
    );
    mgr.item_event_set_abilities
        .insert((SMG, crate::cell::spawner::EVENT_ITEM_RANGED), SMG_RANGED);
    let p = mgr.get_entity_mut(1).unwrap();
    p.is_player = true;
    p.player_id = Some(100);
    p.weapon_holstered = false;
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(0, item(1, PISTOL));
    p.abilities
        .swap_weapon_granted_abilities([PISTOL_RANGED].into_iter().collect());
    p.abilities.last_fired_ability_id = Some(PISTOL_RANGED);
    p.abilities.auto_cycle = true;
    p.abilities.auto_cycle_ability_id = Some(PISTOL_RANGED);
    p.state_field |= crate::cell::combat::BSF_AUTO_CYCLING;
    let (anchor, space_id) = (p.position, p.space_id);
    p.pending_cast = Some(PendingCast {
        ability_id: PISTOL_RANGED,
        target_id: 0,
        wire_target_id: 0,
        ground: None,
        effect_seq: 1,
        received_at: Instant::now(),
        fire_at: Instant::now() + Duration::from_secs(5),
        warmup_secs: 5.0,
        anchor,
        space_id,
        weapon_instance: Some(1),
    });
    mgr.pending_casts.insert(1);
    mgr.connect_entity(1);
    mgr
}

async fn deliver(mgr: &mut SpaceManager, msg: BaseToCellMsg) -> Vec<u16> {
    let (tx, mut rx) = mpsc::channel(256);
    handle_base_message(msg, &tx, mgr, &ChainEngine::new(), &[]).await;
    let mut methods = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = m {
            methods.push(method_index);
        }
    }
    methods
}

/// The outgoing weapon's state is gone and the new weapon's attack is known.
fn assert_weapon_changed(mgr: &SpaceManager, methods: &[u16]) {
    let p = mgr.get_entity(1).unwrap();
    assert!(
        p.abilities.has_ability(SMG_RANGED),
        "the SMG's own attack joins the known list"
    );
    assert!(
        !p.abilities.has_ability(PISTOL_RANGED),
        "the pistol's attack leaves with the pistol"
    );
    assert!(
        methods.contains(&crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE),
        "the client is told: {methods:?}"
    );
    assert_eq!(
        p.abilities.last_fired_ability_id, None,
        "setAutoCycle must not re-press the pistol's shot with the SMG out"
    );
    assert!(!p.abilities.auto_cycle, "the loop armed on 579 stops");
    assert_eq!(p.state_field & crate::cell::combat::BSF_AUTO_CYCLING, 0);
    assert!(
        p.pending_cast.is_none(),
        "the pistol's warmup never fires with the SMG"
    );
    assert!(!mgr.pending_casts.contains(&1));
}

/// **Regression guard.** A drag-equip of an SMG over the active pistol.
#[tokio::test]
async fn drag_equip_of_a_new_weapon_swaps_grants_and_drops_the_old_weapons_state() {
    let mut mgr = fixture();
    let methods = deliver(
        &mut mgr,
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(0, item(2, SMG))],
        },
    )
    .await;
    assert_weapon_changed(&mgr, &methods);
}

/// **Regression guard.** A grant that lands an SMG in the active slot.
#[tokio::test]
async fn grant_into_the_active_slot_swaps_grants_and_drops_the_old_weapons_state() {
    let mut mgr = fixture();
    let methods = deliver(
        &mut mgr,
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 0,
            item: item(2, SMG),
            make_active: false,
        },
    )
    .await;
    assert_weapon_changed(&mgr, &methods);
}

/// Dragging the active weapon out leaves the hand empty: its attack is
/// revoked and its warmup interrupted.
#[tokio::test]
async fn drag_unequip_of_the_active_weapon_revokes_its_attack() {
    let mut mgr = fixture();
    let methods = deliver(
        &mut mgr,
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(2, item(1, PISTOL))],
        },
    )
    .await;
    let p = mgr.get_entity(1).unwrap();
    assert!(!p.abilities.has_ability(PISTOL_RANGED));
    assert!(p.pending_cast.is_none());
    assert!(
        methods.contains(&crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE),
        "the client is told the shot left: {methods:?}"
    );
    assert!(!p.abilities.auto_cycle);
    assert_eq!(
        p.state_field & crate::cell::combat::BSF_AUTO_CYCLING,
        0,
        "the loop's button goes dark"
    );
    assert!(
        methods.contains(&crate::mercury::method_idx::ON_STATE_FIELD_UPDATE),
        "the BSF clear is sent: {methods:?}"
    );
}

/// **Regression guard (review S1).** A reload in flight for the outgoing
/// weapon is cancelled, as a slot change cancels it: the completion tick
/// refills the pinned slot index, so it would load the new weapon for free
/// and the fire gate would block it until the old deadline. A reload queued
/// behind the draw (`pending_reload_at`) goes too.
#[tokio::test]
async fn a_weapon_change_cancels_a_reload_in_flight() {
    for msg in [
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(0, item(2, SMG))],
        },
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 0,
            item: item(2, SMG),
            make_active: false,
        },
    ] {
        let mut mgr = fixture();
        {
            let p = mgr.get_entity_mut(1).unwrap();
            p.reload_complete_at = Some(Instant::now() + Duration::from_secs(2));
            p.reload_slot_id = Some(0);
            p.pending_reload_at = Some(Instant::now() + Duration::from_secs(1));
        }
        deliver(&mut mgr, msg).await;
        let p = mgr.get_entity(1).unwrap();
        assert_eq!(
            p.reload_complete_at, None,
            "the pistol's reload is cancelled"
        );
        assert_eq!(p.reload_slot_id, None);
        assert_eq!(p.pending_reload_at, None, "a queued reload goes too");
    }
}

/// A resync that leaves the same weapon in the active slot (an ammo or
/// other-slot change) touches none of it: no interrupt, no stash clear, no
/// known-list send.
#[tokio::test]
async fn a_same_weapon_resync_changes_nothing() {
    let mut mgr = fixture();
    let methods = deliver(
        &mut mgr,
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(0, item(1, PISTOL)), (1, item(2, SMG))],
        },
    )
    .await;
    let p = mgr.get_entity(1).unwrap();
    assert!(p.pending_cast.is_some(), "the warmup survives");
    assert!(
        p.reload_complete_at.is_none(),
        "the fixture has no reload; this resync must not start one"
    );
    assert_eq!(p.abilities.last_fired_ability_id, Some(PISTOL_RANGED));
    assert!(p.abilities.auto_cycle);
    assert!(p.abilities.has_ability(PISTOL_RANGED));
    assert!(!methods.contains(&crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE));

    // Same for an UpdateBandolierItem into another slot.
    deliver(
        &mut mgr,
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 2,
            item: item(3, SMG),
            make_active: false,
        },
    )
    .await;
    assert!(mgr.get_entity(1).unwrap().pending_cast.is_some());
}
