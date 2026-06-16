//! `/spawn` — template-driven NPC spawn at the caller's position.

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::{CellToBaseMsg, SpaceManager, SpawnRecord};

/// Hard cap on `/spawn` count — a GM typo (`/spawn drone 99999`) must not be
/// able to DoS the cell by flooding a space with entities + AoI work.
const SPAWN_COUNT_CAP: u32 = 20;

/// `/spawn <moniker> [count]` — spawn `count` NPCs of the template whose
/// `template_name` matches `moniker` (case-insensitive), placed at the caller's
/// position. The moniker → template map is the loaded spawn-record set; we copy
/// a matching record's template fields and re-point its position to the caller.
pub(super) async fn handle_spawn(
    caller_entity_id: u32,
    moniker: &str,
    count: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    spawn_records: &[SpawnRecord],
) {
    // Caller position + space — spawn near the GM.
    let (pos, dir, space_id) = match space_mgr.get_entity(caller_entity_id) {
        Some(e) => (
            [e.position.x, e.position.y, e.position.z],
            [e.direction.x, e.direction.y, e.direction.z],
            e.space_id.0 as u32,
        ),
        None => {
            send_gm_feedback(
                caller_entity_id,
                "Spawn failed: you are not in a space.",
                tx,
            )
            .await;
            return;
        }
    };

    // Resolve the moniker against a loaded template (by template_name).
    let Some(template) = spawn_records
        .iter()
        .find(|r| r.template_name.eq_ignore_ascii_case(moniker))
    else {
        send_gm_feedback(
            caller_entity_id,
            &format!("Spawn failed: no template named '{moniker}'."),
            tx,
        )
        .await;
        return;
    };

    let n = count.clamp(1, SPAWN_COUNT_CAP);
    let mut spawned = 0u32;
    for _ in 0..n {
        let npc_id = space_mgr.allocate_npc_id();
        // Clone the resolved template record and re-point it at the caller's
        // position so the NPC spawns where the GM is standing, not at the
        // template's authored spawnlist coordinate.
        let mut record = template.clone();
        record.x = pos[0];
        record.y = pos[1];
        record.z = pos[2];
        record.heading = dir[1]; // heading is yaw (rotation.y)
        match space_mgr.spawn_npc_from_record_in_space(npc_id, &record, space_id) {
            Ok(_) => spawned += 1,
            Err(e) => {
                tracing::warn!(caller_entity_id, npc_id, moniker, "GM spawn failed: {e}");
            }
        }
    }

    if spawned == 0 {
        send_gm_feedback(
            caller_entity_id,
            &format!("Spawn failed: could not place '{moniker}'."),
            tx,
        )
        .await;
    } else {
        send_gm_feedback(
            caller_entity_id,
            &format!("Spawned {spawned}x '{}'.", template.template_name),
            tx,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests_common::*;
    use super::super::{handle_gm_command, GmCommandIntent};
    use super::*;

    #[tokio::test]
    async fn spawn_creates_entities_near_caller() {
        let mut mgr = mgr_with_player();
        let records = vec![template("Jaffa Guard")];
        let (tx, mut rx) = mpsc::channel(32);

        let before = mgr.all_npc_entity_ids().len();
        handle_gm_command(
            1,
            GmCommandIntent::Spawn {
                moniker: "jaffa guard".to_string(), // case-insensitive match
                count: 3,
            },
            &tx,
            &mut mgr,
            &records,
        )
        .await;

        let npcs = mgr.all_npc_entity_ids();
        assert_eq!(npcs.len() - before, 3, "should spawn exactly 3 NPCs");
        // Spawned at the caller's position, not the template's authored coord.
        for nid in &npcs {
            let e = mgr.get_entity(*nid).unwrap();
            assert_eq!([e.position.x, e.position.z], [5.0, 5.0]);
        }
        let fb = feedback_text_to(&drain(&mut rx), 1).expect("spawn must feed back");
        assert!(fb.contains("Spawned 3x"), "got: {fb}");
    }

    #[tokio::test]
    async fn spawn_caps_count() {
        let mut mgr = mgr_with_player();
        let records = vec![template("Drone")];
        let (tx, mut _rx) = mpsc::channel(64);
        handle_gm_command(
            1,
            GmCommandIntent::Spawn {
                moniker: "Drone".to_string(),
                count: 9999,
            },
            &tx,
            &mut mgr,
            &records,
        )
        .await;
        assert_eq!(
            mgr.all_npc_entity_ids().len(),
            SPAWN_COUNT_CAP as usize,
            "count must be clamped to SPAWN_COUNT_CAP"
        );
    }

    #[tokio::test]
    async fn spawn_unknown_moniker_feeds_back_error() {
        let mut mgr = mgr_with_player();
        let records = vec![template("Jaffa Guard")];
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Spawn {
                moniker: "Nonexistent".to_string(),
                count: 1,
            },
            &tx,
            &mut mgr,
            &records,
        )
        .await;
        assert!(mgr.all_npc_entity_ids().is_empty());
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("no template named"), "got: {fb}");
    }
}
