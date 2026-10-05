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
//! Its Health then goes back to the mark's `rest_health` and its witnesses
//! see the bar refill, so damage never builds up across testers into a kill
//! (DA-02 review F2).

use std::time::Instant;

use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::release_npc_from_player_combat;
use crate::cell::abilities::send_entity_method_to_self_and_witnesses;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{SpaceManager, TrainingDummy};
use crate::mercury::method_idx::ON_STAT_UPDATE;

/// The `reason` of the release row.
pub const TRAINING_DUMMY_RELEASE_REASON: &str = "training_dummy_quiet";

/// Release every training dummy whose fight has gone quiet by `now`, and
/// put its Health back. Returns how many players left combat.
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
        refill_health(dummy_id, tx, space_mgr).await;
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

/// Put a released dummy's Health back to its rest value and show it to the
/// dummy's witnesses. A dead dummy is left to the respawn tick.
async fn refill_health(
    dummy_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let update = {
        let Some(e) = space_mgr.get_entity_mut(dummy_id) else {
            return;
        };
        if crate::cell::combat::is_dead_state(e.state_field) {
            return;
        }
        let Some(rest) = e.extensions.get::<TrainingDummy>().map(|m| m.rest_health) else {
            return;
        };
        let Some(hp) = e.stats.get_mut(HEALTH) else {
            return;
        };
        let target = rest.min(hp.max);
        if hp.cur == target {
            return;
        }
        hp.set_current(target);
        let update = e.stats.serialize_dirty();
        e.stats.clear_dirty();
        update
    };
    if !update.is_empty() {
        send_entity_method_to_self_and_witnesses(dummy_id, ON_STAT_UPDATE, update, tx, space_mgr)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::cell::combat::{generate_threat, AggroCause, BSF_IN_COMBAT};
    use crate::cell::space_manager::TRAINING_DUMMY_COMBAT_TIMEOUT;

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
            let e = mgr.get_entity_mut(npc).unwrap();
            e.extensions
                .insert(TrainingDummy::new(t0).with_rest_health(1_000));
            e.stats.get_mut(HEALTH).unwrap().update(0, 1_000, 1_000);
            e.stats.clear_dirty();
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

    /// F2: damage on a dummy does not build up across fights. The release
    /// puts its Health back to the rest value and the clients are told.
    /// Revert proof: drop the `refill_health` call and the dummy stays at
    /// 400.
    #[tokio::test]
    async fn a_released_dummy_is_back_at_its_rest_health() {
        let (mut mgr, npc, t0) = world(true);
        let hp = mgr
            .get_entity_mut(npc)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap();
        hp.update(0, 400, 1_000);
        // The player sees the dummy, so the refill has a witness to reach.
        let _ = mgr.compute_aoi_changes();
        let (tx, mut rx) = mpsc::channel(64);
        training_dummy_combat_tick_at(t0, &tx, &mut mgr).await;
        training_dummy_combat_tick_at(t0 + TRAINING_DUMMY_COMBAT_TIMEOUT, &tx, &mut mgr).await;
        assert_eq!(
            mgr.get_entity(npc).unwrap().stats.get(HEALTH).unwrap().cur,
            1_000
        );
        let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(
            sent.iter().any(|m| matches!(
                m,
                CellToBaseMsg::EntityMethodCall { entity_id, method_index: ON_STAT_UPDATE, .. }
                    | CellToBaseMsg::WitnessEntityMethod { entity_id, method_index: ON_STAT_UPDATE, .. }
                    if *entity_id == npc
            )),
            "the refill reaches the clients: {sent:#?}"
        );
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
