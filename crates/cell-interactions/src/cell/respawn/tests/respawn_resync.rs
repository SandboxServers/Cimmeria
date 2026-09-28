//! Same-world respawn replays the per-entity client caches that the
//! reanchor's `CREATE_BASE_PLAYER` wipes and that neither the inventory
//! push nor the region re-registration covers: level, state field, the
//! full stat set, archetype, ability tree, the hotbar ability list, the
//! active bandolier slot and the mission journal.
//!
//! Bug shapes:
//! - Colo session 2026-09-19 00:37 UTC, entity 2: after the reanchor the
//!   server sent the reanchor burst, appearance/tint and the inventory
//!   snapshot, and nothing else. The hotbar and journal were whatever the
//!   fresh entity defaulted to, and the auto-cycle preference was gone.
//! - Colo playtest 2026-09-28 (Castle, entity 2): the HEALTH/FOCUS
//!   `onStatUpdate` and the `onStateFieldUpdate` went out *before* the
//!   reanchor, to the pawn the client was about to destroy, and nothing
//!   replayed stats, archetype or the ability tree to the new pawn. The
//!   client sent no hotbar `useAbility` from 16:54 to 18:27.

use super::super::handle_respawn;
use super::make_mgr_with_player;
use crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE;
use crate::cell::client_methods::missionary::ON_MISSION_UPDATE;
use crate::cell::client_methods::{being, combatant, player};
use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_DEAD, BSF_MOVEMENT_LOCK};
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx::{ON_KNOWN_ABILITIES_UPDATE, ON_STATE_FIELD_UPDATE};
use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

/// Every `EntityMethodCall` for entity 1, split at the reanchor, as
/// `(method_index, args)` in send order.
struct Replay {
    before: Vec<(u16, Vec<u8>)>,
    after: Vec<(u16, Vec<u8>)>,
}

impl Replay {
    fn after_index_of(&self, method: u16) -> Option<usize> {
        self.after.iter().position(|(m, _)| *m == method)
    }

    fn after_args(&self, method: u16) -> Option<&[u8]> {
        self.after
            .iter()
            .find(|(m, _)| *m == method)
            .map(|(_, a)| a.as_slice())
    }
}

fn collect(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Replay {
    let mut reanchor_at = None;
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut idx = 0usize;
    while let Ok(m) = rx.try_recv() {
        match m {
            CellToBaseMsg::ReanchorPlayer { entity_id: 1, .. } if reanchor_at.is_none() => {
                reanchor_at = Some(idx);
            }
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } => {
                if reanchor_at.is_some() {
                    after.push((method_index, args));
                } else {
                    before.push((method_index, args));
                }
            }
            _ => {}
        }
        idx += 1;
    }
    reanchor_at.expect("fixture sanity: same-world respawn must reanchor");
    Replay { before, after }
}

fn i32_at(args: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        args[offset],
        args[offset + 1],
        args[offset + 2],
        args[offset + 3],
    ])
}

/// `(stat_id, min, cur, max)` entries of a `StatUpdateList` payload.
fn stat_entries(args: &[u8]) -> Vec<(i32, i32, i32, i32)> {
    let count = i32_at(args, 0) as usize;
    assert_eq!(args.len(), 4 + count * 16, "StatUpdateList length");
    (0..count)
        .map(|i| {
            let o = 4 + i * 16;
            (
                i32_at(args, o),
                i32_at(args, o + 4),
                i32_at(args, o + 8),
                i32_at(args, o + 12),
            )
        })
        .collect()
}

/// Methods that change the client's view of the player's stats or state.
/// None of them may reach the pawn the reanchor destroys.
const STAT_OR_STATE_METHODS: [u16; 6] = [
    combatant::ON_STAT_UPDATE,
    combatant::ON_STAT_BASE_UPDATE,
    combatant::ON_ARCHETYPE_UPDATE,
    being::ON_LEVEL_UPDATE,
    being::ON_STATE_FIELD_UPDATE,
    player::ON_ABILITY_TREE_INFO,
];

#[tokio::test]
async fn same_world_respawn_replays_hotbar_active_slot_journal_and_preference_bits() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.add_ability(592);
        e.abilities.add_ability(1218);
        e.active_bandolier_slot = 2;
        e.missions.add_mission(MissionInstance::new(
            622,
            2113,
            vec![MissionObjective {
                objective_id: 2452,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        ));
        // Died mid-fight with auto-cycle switched on. The combat bits go
        // through the ref-counted helpers; the preference bit is single-source.
        e.set_state_flag(BSF_DEAD);
        e.set_state_flag(BSF_MOVEMENT_LOCK);
        e.state_field |= BSF_AUTO_CYCLING;
    }

    let (tx, mut rx) = mpsc::channel(64);
    handle_respawn(1, -1, &tx, &mut mgr).await;

    let entity = mgr
        .get_entity(1)
        .expect("cell entity survives same-world respawn");
    assert_eq!(
        entity.state_field, BSF_AUTO_CYCLING,
        "respawn must drop the combat bits but keep the persisted preference bit — \
         a relog keeps it too, and the player did not ask to turn auto-cycle off"
    );
    assert!(
        entity.state_flag_counts.is_empty(),
        "the ref-counted combat flags must still be fully reset"
    );

    let replay = collect(&mut rx);
    assert!(
        replay
            .before
            .iter()
            .all(|(m, _)| *m != ON_STATE_FIELD_UPDATE),
        "no onStateFieldUpdate before the reanchor: it lands on the pawn the client destroys"
    );

    let hotbar = replay.after_args(ON_KNOWN_ABILITIES_UPDATE).expect(
        "respawn must re-send onKnownAbilitiesUpdate after the reanchor — the recreated \
         client entity has an empty hotbar until the next trainer visit otherwise",
    );
    let count = i32_at(hotbar, 0);
    let mut ids: Vec<i32> = (0..count as usize)
        .map(|i| i32_at(hotbar, 4 + i * 4))
        .collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        vec![592, 1218],
        "hotbar replay must carry every known ability"
    );

    let slot = replay
        .after_args(ON_ACTIVE_SLOT_UPDATE)
        .expect("respawn must re-send onActiveSlotUpdate after the reanchor");
    assert_eq!(
        (i32_at(slot, 0), i32_at(slot, 4)),
        (3, 3),
        "active-slot replay must be bag 3 with the 1-indexed wire slot (2 + 1)"
    );

    let journal = replay
        .after_args(ON_MISSION_UPDATE)
        .expect("respawn must re-send the mission journal after the reanchor");
    assert_eq!(
        i32_at(journal, 0),
        622,
        "journal replay must carry the active mission"
    );

    let post_state: Vec<u32> = replay
        .after
        .iter()
        .filter(|(m, _)| *m == ON_STATE_FIELD_UPDATE)
        .map(|(_, a)| u32::from_le_bytes([a[0], a[1], a[2], a[3]]))
        .collect();
    assert_eq!(
        post_state,
        vec![BSF_AUTO_CYCLING],
        "after the reanchor the client's cached state_field is 0 again, so the preserved \
         preference bit must be re-broadcast once or the auto-cycle button stays unlit"
    );
}

/// With no preference bit set the post-reanchor state field is a plain 0,
/// sent once, as the login burst does. Nothing goes before the reanchor.
#[tokio::test]
async fn respawn_without_preference_bits_replays_a_zero_state_field_after_the_reanchor() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.add_ability(592);
        e.set_state_flag(BSF_DEAD);
    }

    let (tx, mut rx) = mpsc::channel(64);
    handle_respawn(1, -1, &tx, &mut mgr).await;

    let replay = collect(&mut rx);
    assert!(
        replay
            .before
            .iter()
            .all(|(m, _)| *m != ON_STATE_FIELD_UPDATE),
        "no onStateFieldUpdate before the reanchor"
    );
    let post_state: Vec<&[u8]> = replay
        .after
        .iter()
        .filter(|(m, _)| *m == ON_STATE_FIELD_UPDATE)
        .map(|(_, a)| a.as_slice())
        .collect();
    assert_eq!(
        post_state,
        vec![&0u32.to_le_bytes()[..]],
        "one onStateFieldUpdate(0) after the reanchor: the dead bits are off on the new pawn"
    );
    assert!(
        replay.after_index_of(ON_KNOWN_ABILITIES_UPDATE).is_some(),
        "the hotbar replay is unconditional"
    );
}

/// The new pawn gets the login burst's player-state half from the live
/// entity: every stat (FOCUS at max), base stats, archetype, level and the
/// ability tree, all after the reanchor and none before it.
///
/// Fails on the pre-fix respawn, which sent only the dirty HEALTH/FOCUS
/// before the reanchor and no 20/21/23/141 after it.
#[tokio::test]
async fn same_world_respawn_replays_full_stats_after_reanchor() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    if let Some(e) = mgr.get_entity_mut(1) {
        e.level = 7;
        e.archetype_id = Some(2);
        e.abilities.add_ability(592);
        e.stats.get_mut(HEALTH).unwrap().update(0, 0, 250);
        e.stats.get_mut(FOCUS).unwrap().update(0, 3, 120);
        e.set_state_flag(BSF_DEAD);
    }

    let (tx, mut rx) = mpsc::channel(64);
    handle_respawn(1, -1, &tx, &mut mgr).await;
    let replay = collect(&mut rx);

    let stray: Vec<u16> = replay
        .before
        .iter()
        .map(|(m, _)| *m)
        .filter(|m| STAT_OR_STATE_METHODS.contains(m))
        .collect();
    assert!(
        stray.is_empty(),
        "nothing stat- or state-related may go before the reanchor (the client destroys that \
         pawn); sent {stray:?}"
    );

    // Method 20 carries every stat the entity has, FOCUS and HEALTH full.
    let entity = mgr.get_entity(1).unwrap();
    let stats = replay
        .after_args(combatant::ON_STAT_UPDATE)
        .expect("respawn must send onStatUpdate (20) after the reanchor");
    assert_eq!(
        stats,
        entity.stats.serialize_all().as_slice(),
        "method 20 must be the full stat list, not the dirty subset"
    );
    let entries = stat_entries(stats);
    let focus = entries
        .iter()
        .find(|e| e.0 == FOCUS)
        .expect("FOCUS in the stat list");
    assert_eq!((focus.2, focus.3), (120, 120), "FOCUS at max");
    let health = entries
        .iter()
        .find(|e| e.0 == HEALTH)
        .expect("HEALTH in the stat list");
    assert_eq!((health.2, health.3), (250, 250), "HEALTH at max");

    // 21, 23, 141 follow 20, in the login burst's order.
    let ix = |m: u16| {
        replay
            .after_index_of(m)
            .unwrap_or_else(|| panic!("method {m} must be replayed after the reanchor"))
    };
    let (i20, i21, i23, i141) = (
        ix(combatant::ON_STAT_UPDATE),
        ix(combatant::ON_STAT_BASE_UPDATE),
        ix(combatant::ON_ARCHETYPE_UPDATE),
        ix(player::ON_ABILITY_TREE_INFO),
    );
    assert!(
        i20 < i21 && i21 < i23 && i23 < i141,
        "order must be 20, 21, 23, 141; got {i20}, {i21}, {i23}, {i141}"
    );
    assert_eq!(
        replay.after_args(combatant::ON_STAT_BASE_UPDATE).unwrap(),
        entity.stats.serialize_all_base().as_slice(),
        "method 21 must be the full base-stat list"
    );
    assert_eq!(
        replay.after_args(combatant::ON_ARCHETYPE_UPDATE).unwrap(),
        &2i32.to_le_bytes()[..],
        "method 23 carries the archetype"
    );
    assert_eq!(
        replay.after_args(being::ON_LEVEL_UPDATE).unwrap(),
        &7i32.to_le_bytes()[..],
        "method 15 carries the level"
    );
    let tree = crate::ability_tree::tree_info(&mgr.ability_tree_catalog, 2, 100).serialize();
    assert_eq!(
        replay.after_args(player::ON_ABILITY_TREE_INFO).unwrap(),
        tree.as_slice(),
        "method 141 is the archetype's tree from the trainer's catalog"
    );
    assert!(
        i141 < ix(ON_KNOWN_ABILITIES_UPDATE),
        "the tree precedes the hotbar, as at login"
    );
}
