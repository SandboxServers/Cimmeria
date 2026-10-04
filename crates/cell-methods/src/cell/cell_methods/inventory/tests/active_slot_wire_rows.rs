//! AB-C7 full accounting: the weapon swap's `onKnownAbilitiesUpdate` goes
//! through the wire ledger with `origin = weapon_swap`. Sending it raw again
//! (`send_entity_method`) fails this.

use tokio::sync::mpsc;

use crate::test_support::LogCapture;

use super::super::dispatch::dispatch;
use super::super::REQUEST_ACTIVE_SLOT_CHANGE;
use super::make_test_space_mgr;

#[tokio::test]
async fn a_weapon_swap_hotbar_replace_writes_a_wire_row_naming_the_swap() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    const PISTOL_ITEM_ID: i32 = 55;
    const PISTOL_RANGED: i32 = 579;
    const EVENT_RANGED: i32 = 7;

    let capture = LogCapture::install();
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_RANGED), PISTOL_RANGED);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: PISTOL_ITEM_ID,
                clip_size: 12,
                default_ammo_type: 1,
                current_ammo: 12,
                cur_ammo_type: 1,
            },
        );
        e.active_bandolier_slot = 0;
        e.abilities
            .swap_weapon_granted_abilities([PISTOL_RANGED].into_iter().collect());
        e.pending_slot_swap_at = Some(std::time::Instant::now());
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(64);
    let engine = build_engine(None).await;
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes()); // wire slot 2 = server slot 1 (empty)
    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;
    while rx.try_recv().is_ok() {}

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| {
            c.target == "abilities.wire"
                && c.has_field("event", "wire_sent")
                && c.has_field("method", "onKnownAbilitiesUpdate")
        })
        .collect();
    assert_eq!(rows.len(), 1, "{:#?}", capture.all());
    assert!(rows[0].has_field("origin", "weapon_swap"), "{:#?}", rows[0]);
    assert!(rows[0].has_field("player_id", "100"), "{:#?}", rows[0]);
    // The pistol's ability is gone after the unequip.
    assert!(rows[0].has_field("ability_count", "0"), "{:#?}", rows[0]);
}
