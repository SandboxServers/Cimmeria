//! Cell half of the AM-02 switch return (D-AM05). The database half (the
//! partial return into full bags) is `ammo_reserve::requests_live_db_tests`
//! in `cimmeria-base-methods`.

use cimmeria_entity::ammo_type::{BULLET_ARMOR_PIERCING, BULLET_DEFAULT, BULLET_HOLLOW_POINT};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use super::*;
use crate::cell::messages::ReserveRefusal;

const ENTITY: u32 = 1;
const PLAYER: i32 = 9201;
const SLOT: i32 = 0;
const INSTANCE: i32 = 5151;

fn mgr(ammo_type: i32, current: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(ENTITY, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(ENTITY).unwrap();
    e.is_player = true;
    e.player_id = Some(PLAYER);
    e.active_bandolier_slot = SLOT;
    e.bandolier_items.insert(
        SLOT,
        BandolierItem {
            instance_id: INSTANCE,
            item_id: 1,
            clip_size: 30,
            default_ammo_type: BULLET_DEFAULT,
            current_ammo: current,
            cur_ammo_type: ammo_type,
        },
    );
    mgr.connect_entity(ENTITY);
    mgr
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

fn requests(sent: &[CellToBaseMsg]) -> Vec<&AmmoReserveRequest> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::AmmoReserve(r) => Some(r),
            _ => None,
        })
        .collect()
}

fn slot(mgr: &SpaceManager) -> (i32, i32) {
    let i = &mgr.get_entity(ENTITY).unwrap().bandolier_items[&SLOT];
    (i.current_ammo, i.cur_ammo_type)
}

fn returned(
    returned: i32,
    remainder: i32,
    result: Result<(), ReserveRefusal>,
) -> AmmoReserveAnswer {
    AmmoReserveAnswer::SwitchReturned {
        entity_id: ENTITY,
        player_id: PLAYER,
        slot_id: SLOT,
        instance_id: INSTANCE,
        from_ammo_type: BULLET_HOLLOW_POINT,
        to_ammo_type: BULLET_ARMOR_PIERCING,
        rounds: 20,
        returned,
        remainder,
        result,
    }
}

fn sent_line(sent: &[CellToBaseMsg], text: &str) -> bool {
    let line = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    sent.iter().any(|m| {
        matches!(m, CellToBaseMsg::EntityMethodCall { method_index, args, .. }
            if *method_index == crate::mercury::method_idx::ON_PLAYER_COMMUNICATION && *args == line)
    })
}

/// Flag off: the switch proceeds untouched, nothing is sent.
#[tokio::test]
async fn flag_off_proceeds_unchanged() {
    let mut mgr = mgr(BULLET_HOLLOW_POINT, 20);
    let (tx, mut rx) = mpsc::channel(16);
    let r =
        begin_switch_return_with(ENTITY, SLOT, BULLET_ARMOR_PIERCING, false, &tx, &mut mgr).await;
    assert_eq!(r, SwitchReturn::Proceed);
    assert!(drain(&mut rx).is_empty());
    assert_eq!(slot(&mgr), (20, BULLET_HOLLOW_POINT));
}

/// Special clip: the switch defers, the clip is flushed and then emptied
/// (rounds in transit), and one return request carries the 20 rounds. A
/// second switch while it is in flight sends nothing.
#[tokio::test]
async fn special_clip_defers_and_asks_for_the_return() {
    let mut mgr = mgr(BULLET_HOLLOW_POINT, 20);
    let (tx, mut rx) = mpsc::channel(16);
    let r =
        begin_switch_return_with(ENTITY, SLOT, BULLET_ARMOR_PIERCING, true, &tx, &mut mgr).await;
    assert_eq!(r, SwitchReturn::Deferred);
    let sent = drain(&mut rx);
    assert!(matches!(
        sent.first(),
        Some(CellToBaseMsg::BandolierAmmoUpdate {
            current_ammo: 20,
            cur_ammo_type: BULLET_HOLLOW_POINT,
            ..
        })
    ));
    assert_eq!(
        requests(&sent),
        vec![&AmmoReserveRequest::SwitchReturn {
            entity_id: ENTITY,
            player_id: PLAYER,
            slot_id: SLOT,
            instance_id: INSTANCE,
            from_ammo_type: BULLET_HOLLOW_POINT,
            to_ammo_type: BULLET_ARMOR_PIERCING,
            rounds: 20,
        }]
    );
    assert_eq!(slot(&mgr), (0, BULLET_HOLLOW_POINT));
    assert!(!mgr
        .get_entity(ENTITY)
        .unwrap()
        .bandolier_ammo_dirty
        .contains(&SLOT));

    let again = begin_switch_return_with(ENTITY, SLOT, BULLET_DEFAULT, true, &tx, &mut mgr).await;
    assert_eq!(again, SwitchReturn::Deferred);
    assert!(requests(&drain(&mut rx)).is_empty());
}

/// Default clip to a special type: the free default rounds leave the clip
/// and the switch proceeds (no free special rounds).
#[tokio::test]
async fn default_to_special_empties_the_clip() {
    let mut mgr = mgr(BULLET_DEFAULT, 30);
    let (tx, mut rx) = mpsc::channel(16);
    let r = begin_switch_return_with(ENTITY, SLOT, BULLET_HOLLOW_POINT, true, &tx, &mut mgr).await;
    assert_eq!(r, SwitchReturn::Proceed);
    assert!(requests(&drain(&mut rx)).is_empty());
    assert_eq!(slot(&mgr).0, 0);
}

/// An empty special clip has nothing to return: the switch proceeds.
#[tokio::test]
async fn empty_special_clip_proceeds() {
    let mut mgr = mgr(BULLET_HOLLOW_POINT, 0);
    let (tx, mut rx) = mpsc::channel(16);
    let r = begin_switch_return_with(ENTITY, SLOT, BULLET_DEFAULT, true, &tx, &mut mgr).await;
    assert_eq!(r, SwitchReturn::Proceed);
    assert!(drain(&mut rx).is_empty());
}

/// Everything fit: the slot takes the new type with an empty clip and the
/// client's indicator is told the new type.
#[tokio::test]
async fn all_returned_finishes_the_switch() {
    let mut mgr = mgr(BULLET_HOLLOW_POINT, 20);
    let (tx, mut rx) = mpsc::channel(32);
    begin_switch_return_with(ENTITY, SLOT, BULLET_ARMOR_PIERCING, true, &tx, &mut mgr).await;
    drain(&mut rx);
    handle_switch_returned(returned(20, 0, Ok(())), &tx, &mut mgr).await;
    assert_eq!(slot(&mgr), (0, BULLET_ARMOR_PIERCING));
    let sent = drain(&mut rx);
    let prop = build_entity_property_args(GENERICPROPERTY_AMMO_TYPE_ID, BULLET_ARMOR_PIERCING);
    assert!(sent
        .iter()
        .any(|m| matches!(m, CellToBaseMsg::EntityMethodCall { args, .. } if *args == prop)));
}

/// Bags full: 13 of 20 fit, the 7 left stay loaded as Hollow Point, the
/// switch does not happen, and the player is told why.
#[tokio::test]
async fn bags_full_keeps_the_remainder_as_the_old_type() {
    let mut mgr = mgr(BULLET_HOLLOW_POINT, 20);
    let (tx, mut rx) = mpsc::channel(32);
    begin_switch_return_with(ENTITY, SLOT, BULLET_ARMOR_PIERCING, true, &tx, &mut mgr).await;
    drain(&mut rx);
    handle_switch_returned(returned(13, 7, Ok(())), &tx, &mut mgr).await;
    assert_eq!(slot(&mgr), (7, BULLET_HOLLOW_POINT));
    let sent = drain(&mut rx);
    assert!(sent_line(
        &sent,
        "Your bags are full: 7 Hollow Point rounds stay loaded. Make room and switch again."
    ));
    let prop = build_entity_property_args(GENERICPROPERTY_AMMO_TYPE_ID, BULLET_HOLLOW_POINT);
    assert!(sent
        .iter()
        .any(|m| matches!(m, CellToBaseMsg::EntityMethodCall { args, .. } if *args == prop)));
}

/// A refused return moved nothing: the rounds come back to the clip.
#[tokio::test]
async fn refused_return_restores_the_clip() {
    let mut mgr = mgr(BULLET_HOLLOW_POINT, 20);
    let (tx, mut rx) = mpsc::channel(32);
    begin_switch_return_with(ENTITY, SLOT, BULLET_ARMOR_PIERCING, true, &tx, &mut mgr).await;
    drain(&mut rx);
    handle_switch_returned(returned(0, 0, Err(ReserveRefusal::DbError)), &tx, &mut mgr).await;
    assert_eq!(slot(&mgr), (20, BULLET_HOLLOW_POINT));
    assert!(sent_line(
        &drain(&mut rx),
        "The ammo switch failed. Try again."
    ));
}

/// Type 12 guard (AM-12 close-out): a special switch logs
/// `ammo_switch_return_requested` (DEBUG), a bags-full answer logs
/// `ammo_switch_refused` (INFO, `reason=bags_full`, with the remainder), and
/// a default clip switched to a special type logs
/// `ammo_switch_default_emptied` (DEBUG), all on target `ammo`.
#[tokio::test]
async fn switch_return_logs_request_refusal_and_default_emptied() {
    use crate::test_support::LogCapture;
    let logs = LogCapture::install();
    let row = |event: &str| {
        logs.all()
            .into_iter()
            .find(|c| c.target == "ammo" && c.has_field("event", event))
            .unwrap_or_else(|| panic!("no {event} row: {:#?}", logs.all()))
    };

    let mut special = mgr(BULLET_HOLLOW_POINT, 20);
    let (tx, mut rx) = mpsc::channel(32);
    begin_switch_return_with(ENTITY, SLOT, BULLET_ARMOR_PIERCING, true, &tx, &mut special).await;
    drain(&mut rx);
    let requested = row("ammo_switch_return_requested");
    assert_eq!(requested.level, tracing::Level::DEBUG);
    assert!(requested.has_field("player_id", &PLAYER.to_string()));

    handle_switch_returned(returned(13, 7, Ok(())), &tx, &mut special).await;
    drain(&mut rx);
    let refused = row("ammo_switch_refused");
    assert_eq!(refused.level, tracing::Level::INFO);
    assert!(refused.has_field("reason", "bags_full"));
    assert!(refused.has_field("remainder", "7"));

    let mut default = mgr(BULLET_DEFAULT, 30);
    begin_switch_return_with(ENTITY, SLOT, BULLET_HOLLOW_POINT, true, &tx, &mut default).await;
    assert_eq!(
        row("ammo_switch_default_emptied").level,
        tracing::Level::DEBUG
    );
}
