//! Same-world respawn replays the per-entity client caches that the
//! reanchor's `CREATE_BASE_PLAYER` wipes and that neither the inventory
//! push nor the region re-registration covers: the hotbar ability list,
//! the active bandolier slot, the mission journal and the preserved
//! `state_field` preference bits.
//!
//! Bug shape (colo session 2026-09-19 00:37 UTC, entity 2): after the
//! reanchor the server sent `onEndAidWait`, `onStateFieldUpdate(0)`, the
//! reanchor burst, appearance/tint and the inventory snapshot — and nothing
//! else. The client rebuilt its player entity from that, so the hotbar and
//! journal were whatever the fresh entity defaulted to, and the auto-cycle
//! preference the player had toggled on was gone until they toggled it
//! again by hand (the log shows the manual `setAutoCycle` 25 s later).

use super::super::respawn::handle_respawn;
use super::make_mgr_with_player;
use crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE;
use crate::cell::client_methods::missionary::ON_MISSION_UPDATE;
use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_DEAD, BSF_MOVEMENT_LOCK};
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx::{ON_KNOWN_ABILITIES_UPDATE, ON_STATE_FIELD_UPDATE};
use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};
use tokio::sync::mpsc;

/// Every `EntityMethodCall` for entity 1 after the reanchor, in order, as
/// `(method_index, args)`, plus the index of the reanchor itself and the
/// `onStateFieldUpdate` values sent *before* it.
struct Replay {
    reanchor_at: usize,
    pre_reanchor_state_fields: Vec<u32>,
    after: Vec<(u16, Vec<u8>)>,
}

fn collect(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Replay {
    let mut reanchor_at = None;
    let mut pre_reanchor_state_fields = Vec::new();
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
                } else if method_index == ON_STATE_FIELD_UPDATE {
                    pre_reanchor_state_fields
                        .push(u32::from_le_bytes([args[0], args[1], args[2], args[3]]));
                }
            }
            _ => {}
        }
        idx += 1;
    }
    Replay {
        reanchor_at: reanchor_at.expect("fixture sanity: same-world respawn must reanchor"),
        pre_reanchor_state_fields,
        after,
    }
}

fn i32_at(args: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        args[offset],
        args[offset + 1],
        args[offset + 2],
        args[offset + 3],
    ])
}

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

    let (tx, mut rx) = mpsc::channel(32);
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
    assert_eq!(
        replay.pre_reanchor_state_fields,
        vec![BSF_AUTO_CYCLING],
        "the pre-reanchor onStateFieldUpdate must carry the post-reset field (preference \
         bit only) rather than a hard 0 — as an XOR delta against the client's cached \
         dead|locked|auto-cycling value, 0 would flip the preference bit off"
    );
    assert!(
        replay.reanchor_at > 0,
        "reanchor must not be the first message"
    );

    let hotbar = replay
        .after
        .iter()
        .find(|(m, _)| *m == ON_KNOWN_ABILITIES_UPDATE)
        .expect(
            "respawn must re-send onKnownAbilitiesUpdate after the reanchor — the recreated \
             client entity has an empty hotbar until the next trainer visit otherwise",
        );
    let count = i32_at(&hotbar.1, 0);
    let mut ids: Vec<i32> = (0..count as usize)
        .map(|i| i32_at(&hotbar.1, 4 + i * 4))
        .collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        vec![592, 1218],
        "hotbar replay must carry every known ability"
    );

    let slot = replay
        .after
        .iter()
        .find(|(m, _)| *m == ON_ACTIVE_SLOT_UPDATE)
        .expect("respawn must re-send onActiveSlotUpdate after the reanchor");
    assert_eq!(
        (i32_at(&slot.1, 0), i32_at(&slot.1, 4)),
        (3, 3),
        "active-slot replay must be bag 3 with the 1-indexed wire slot (2 + 1)"
    );

    let journal = replay
        .after
        .iter()
        .find(|(m, _)| *m == ON_MISSION_UPDATE)
        .expect("respawn must re-send the mission journal after the reanchor");
    assert_eq!(
        i32_at(&journal.1, 0),
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

/// With no preference bit set there is nothing for the post-reanchor
/// `state_field` replay to say: the client's cache and the entity both
/// read 0, and an extra `onStateFieldUpdate(0)` would be a no-op packet.
/// The hotbar / slot / journal replays still go out.
#[tokio::test]
async fn respawn_without_preference_bits_sends_no_post_reanchor_state_field() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.add_ability(592);
        e.set_state_flag(BSF_DEAD);
    }

    let (tx, mut rx) = mpsc::channel(32);
    handle_respawn(1, -1, &tx, &mut mgr).await;

    let replay = collect(&mut rx);
    assert_eq!(
        replay.pre_reanchor_state_fields,
        vec![0],
        "no preference bit ⇒ the pre-reanchor clear is a plain 0"
    );
    assert!(
        replay
            .after
            .iter()
            .all(|(m, _)| *m != ON_STATE_FIELD_UPDATE),
        "no preference bit ⇒ no post-reanchor onStateFieldUpdate"
    );
    assert!(
        replay
            .after
            .iter()
            .any(|(m, _)| *m == ON_KNOWN_ABILITIES_UPDATE),
        "the hotbar replay is unconditional"
    );
}
