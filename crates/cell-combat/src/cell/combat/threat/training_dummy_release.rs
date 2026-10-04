//! Combat ends on its own at a training dummy (Debug Area D-DA7).
//!
//! A training dummy (`TrainingDummy` mark: a seeded dummy or a `.dummy`)
//! gets no AI turn, so nothing ever leashes it, and the leash is the drain
//! that takes a mob out of its attackers' combat sets when a fight goes
//! quiet. Without this sweep a player who shot a seeded dummy kept
//! `BSF_InCombat` (no regen, no out-of-combat holster) until they died or
//! relogged, because a seeded dummy never despawns.
//!
//! [`training_dummy_combat_tick`] runs at 1 Hz from the cell loop. A dummy
//! whose threat list has not changed for `TRAINING_DUMMY_COMBAT_TIMEOUT`
//! (no hit landed in that time) is released through
//! [`release_npc_from_player_combat`], the drain every non-death despawn
//! uses: each player whose last threat it was leaves combat and is told.

use std::time::Instant;

use tokio::sync::mpsc;

use super::release_npc_from_player_combat;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The `reason` of the release row.
pub const TRAINING_DUMMY_RELEASE_REASON: &str = "training_dummy_quiet";

/// Release every training dummy whose fight has gone quiet by `now`.
/// Returns how many players left combat.
pub async fn training_dummy_combat_tick_at(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let mut exits = 0;
    for dummy_id in space_mgr.quiet_training_dummies(now) {
        exits +=
            release_npc_from_player_combat(dummy_id, TRAINING_DUMMY_RELEASE_REASON, tx, space_mgr)
                .await;
    }
    exits
}

/// [`training_dummy_combat_tick_at`] on the real clock.
pub async fn training_dummy_combat_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    training_dummy_combat_tick_at(Instant::now(), tx, space_mgr).await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::cell::combat::{generate_threat, AggroCause, BSF_IN_COMBAT};
    use crate::cell::space_manager::{TrainingDummy, TRAINING_DUMMY_COMBAT_TIMEOUT};

    const PLAYER: u32 = 1;

    fn world(marked: bool) -> (SpaceManager, u32, Instant) {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(PLAYER, "Castle", [0.0; 3], [0.0; 3])
            .unwrap();
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.is_player = true;
        p.player_id = Some(101);
        mgr.connect_entity(PLAYER);
        let npc = mgr.allocate_npc_id();
        mgr.spawn_npc(npc, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let t0 = Instant::now();
        if marked {
            mgr.get_entity_mut(npc)
                .unwrap()
                .extensions
                .insert(TrainingDummy::new(t0));
        }
        let _ = generate_threat(&mut mgr, PLAYER, npc, 10.0, AggroCause::Damage);
        (mgr, npc, t0)
    }

    fn in_combat(mgr: &SpaceManager) -> bool {
        mgr.get_entity(PLAYER).unwrap().state_field & BSF_IN_COMBAT != 0
    }

    /// A shot dummy holds its attacker in combat while hits keep coming,
    /// then lets go once its threat has stood still for the timeout.
    /// Revert proof: make the sweep return nothing and the player stays in
    /// combat for good.
    #[tokio::test]
    async fn a_quiet_training_dummy_takes_its_attacker_out_of_combat() {
        let (mut mgr, npc, t0) = world(true);
        let (tx, _rx) = mpsc::channel(64);
        assert!(in_combat(&mgr), "fixture: the hit put the player in combat");

        // First sweep sees the new threat and starts the quiet window.
        assert_eq!(training_dummy_combat_tick_at(t0, &tx, &mut mgr).await, 0);
        // Another hit inside the window restarts it.
        let mid = t0 + TRAINING_DUMMY_COMBAT_TIMEOUT / 2;
        let _ = generate_threat(&mut mgr, PLAYER, npc, 10.0, AggroCause::Damage);
        assert_eq!(training_dummy_combat_tick_at(mid, &tx, &mut mgr).await, 0);
        let almost = mid + TRAINING_DUMMY_COMBAT_TIMEOUT - Duration::from_millis(1);
        assert_eq!(
            training_dummy_combat_tick_at(almost, &tx, &mut mgr).await,
            0,
            "still inside the window after the second hit"
        );
        assert!(in_combat(&mgr));

        let quiet = mid + TRAINING_DUMMY_COMBAT_TIMEOUT;
        assert_eq!(training_dummy_combat_tick_at(quiet, &tx, &mut mgr).await, 1);
        assert!(!in_combat(&mgr), "the player left combat with the dummy");
        assert!(mgr.get_entity(npc).unwrap().threat_list.is_empty());
        assert!(mgr.get_entity(PLAYER).unwrap().threatened_mobs.is_empty());
    }

    /// An ordinary NPC is not touched: its leash does that job.
    #[tokio::test]
    async fn an_unmarked_npc_is_left_to_its_own_leash() {
        let (mut mgr, npc, t0) = world(false);
        let (tx, _rx) = mpsc::channel(64);
        let later = t0 + TRAINING_DUMMY_COMBAT_TIMEOUT * 3;
        training_dummy_combat_tick_at(t0, &tx, &mut mgr).await;
        assert_eq!(training_dummy_combat_tick_at(later, &tx, &mut mgr).await, 0);
        assert!(in_combat(&mgr));
        assert!(!mgr.get_entity(npc).unwrap().threat_list.is_empty());
    }
}
