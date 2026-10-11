//! Bandolier ammo flush summary and lost-slot rows (TG-ITM-11): the
//! dropped-slot WARN, the DEBUG `bandolier_flush` summary, and the logout
//! variant's `ammo_lost_at_logout` WARN.
//!
//! Each dirty-slot set holds one slot, because the dirty set is a `HashSet`
//! and a two-slot run would depend on iteration order.

use tokio::sync::mpsc;
use tracing::Level;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager_with_player, LogCapture};
use cimmeria_entity::cell_entity::BandolierItem;

use super::{flush_dirty_bandolier_ammo, flush_dirty_bandolier_ammo_at_logout};

const PLAYER_ID: i32 = 100;

/// A player entity with a weapon in slot 1 and `dirty` as its dirty slots.
fn dirty_mgr(dirty: &[i32]) -> SpaceManager {
    let mut mgr = make_space_manager_with_player(1);
    let e = mgr.get_entity_mut(1).unwrap();
    e.is_player = true;
    e.player_id = Some(PLAYER_ID);
    e.account_id = Some(700);
    e.log_names.player_name = Some("Tester");
    e.bandolier_items.insert(
        1,
        BandolierItem {
            instance_id: 1011,
            item_id: 11,
            clip_size: 12,
            default_ammo_type: 7,
            current_ammo: 12,
            cur_ammo_type: 7,
        },
    );
    e.bandolier_ammo_dirty.extend(dirty.iter().copied());
    mgr
}

/// A sender whose receiver is already gone, so every send fails.
fn closed_tx() -> mpsc::Sender<CellToBaseMsg> {
    let (tx, rx) = mpsc::channel(1);
    drop(rx);
    tx
}

#[tokio::test]
async fn bandolier_flush_reports_dropped_and_unsent() {
    {
        // Slot 0 has no item: it is dropped, and nothing is left unsent.
        let capture = LogCapture::install();
        let mut mgr = dirty_mgr(&[0]);
        let tx = closed_tx();
        flush_dirty_bandolier_ammo(mgr.get_entity_mut(1).unwrap(), PLAYER_ID, &tx).await;
        assert!(capture
            .find_event(Level::WARN, "no item; dropping", "no_item")
            .is_some());
        assert!(capture.all().iter().any(|c| c.has_field("dropped", "1")));
    }
    {
        // Slot 1 has an item and the channel is closed: the send fails, the
        // slot stays dirty, and the summary counts it as unsent.
        let capture = LogCapture::install();
        let mut mgr = dirty_mgr(&[1]);
        let tx = closed_tx();
        flush_dirty_bandolier_ammo(mgr.get_entity_mut(1).unwrap(), PLAYER_ID, &tx).await;
        let summary = capture
            .find_message(Level::DEBUG, "bandolier ammo flush finished")
            .expect("summary row");
        assert!(summary.has_field("sent", "0"));
        assert!(summary.has_field("dropped", "0"));
        assert!(summary.has_field("unsent", "1"));
        assert!(mgr.get_entity(1).unwrap().bandolier_ammo_dirty.contains(&1));
    }
}

#[tokio::test]
async fn bandolier_flush_at_logout_warns_ammo_lost() {
    {
        let capture = LogCapture::install();
        let mut mgr = dirty_mgr(&[1]);
        let tx = closed_tx();
        flush_dirty_bandolier_ammo_at_logout(mgr.get_entity_mut(1).unwrap(), PLAYER_ID, &tx).await;
        assert!(capture
            .find_event(Level::WARN, "leaving slot dirty", "ammo_lost_at_logout")
            .is_some());
    }
    {
        // The normal variant keeps the retry wording and carries no logout reason.
        let capture = LogCapture::install();
        let mut mgr = dirty_mgr(&[1]);
        let tx = closed_tx();
        flush_dirty_bandolier_ammo(mgr.get_entity_mut(1).unwrap(), PLAYER_ID, &tx).await;
        assert!(capture
            .find_event(Level::WARN, "leaving slot dirty", "ammo_lost_at_logout")
            .is_none());
        assert!(capture
            .find_message(Level::WARN, "leaving slot dirty")
            .is_some());
    }
}
