//! `/info` and `/who` — read-only queries that feed back a summary line.

use tokio::sync::mpsc;

use cimmeria_entity::stats::HEALTH;

use super::feedback::send_gm_feedback;
use super::{CellToBaseMsg, SpaceManager};

/// `/info` — dump a concise summary of the caller's current target (or self if
/// no target is selected).
pub(super) async fn handle_info(
    caller_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let subject = space_mgr
        .get_entity(caller_entity_id)
        .and_then(|e| e.current_target_id)
        .map(|id| id as u32)
        .filter(|id| space_mgr.get_entity(*id).is_some())
        .unwrap_or(caller_entity_id);

    let Some(e) = space_mgr.get_entity(subject) else {
        send_gm_feedback(caller_entity_id, "Info failed: no entity.", tx).await;
        return;
    };

    let name = e
        .npc_name
        .clone()
        .or_else(|| e.template_id.map(|t| format!("template {t}")))
        .unwrap_or_else(|| {
            if e.is_player {
                "player".into()
            } else {
                "entity".into()
            }
        });
    let (hp_cur, hp_max) = e.stats.get(HEALTH).map_or((0, 0), |s| (s.cur, s.max));
    let text = format!(
        "[{}] {} | faction {} | hp {}/{} | pos ({:.1}, {:.1}, {:.1})",
        subject, name, e.faction, hp_cur, hp_max, e.position.x, e.position.y, e.position.z
    );
    send_gm_feedback(caller_entity_id, &text, tx).await;
}

/// `/who` — list the player entities in the caller's space.
///
/// Players carry no character name on the cell, so each entry is
/// `entity_id (player_id N)`.
pub(super) async fn handle_who(
    caller_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(space_id) = space_mgr.get_entity_space_id(caller_entity_id) else {
        send_gm_feedback(caller_entity_id, "Who failed: you are not in a space.", tx).await;
        return;
    };

    // Collect (entity_id, player_id) and sort by NUMERIC entity id. Sorting
    // the formatted strings instead would order lexicographically — "10"
    // before "2" — so build the display strings only after the numeric sort.
    let mut ids: Vec<_> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&pid| space_mgr.get_entity_space_id(pid) == Some(space_id))
        .map(|pid| (pid, space_mgr.get_entity(pid).and_then(|e| e.player_id)))
        .collect();
    ids.sort_by_key(|&(pid, _)| pid);
    let entries: Vec<String> = ids
        .into_iter()
        .map(|(pid, player_id)| match player_id {
            Some(p) => format!("{pid} (player_id {p})"),
            None => format!("{pid}"),
        })
        .collect();

    let text = if entries.is_empty() {
        "No players in your space.".to_string()
    } else {
        format!("Players ({}): {}", entries.len(), entries.join(", "))
    };
    send_gm_feedback(caller_entity_id, &text, tx).await;
}

#[cfg(test)]
mod tests {
    use super::super::tests_common::*;
    use super::super::{handle_gm_command, GmCommandIntent};
    use super::*;

    #[tokio::test]
    async fn info_dumps_self_when_no_target() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(1, GmCommandIntent::Info, &tx, &mut mgr, &[]).await;
        let fb = feedback_text_to(&drain(&mut rx), 1).expect("info must feed back to caller");
        // Exact self-dump: subject id 1, name "player" (no npc_name/template
        // on a bare player), faction 0, default HEALTH 100/100, pos from the
        // fixture (5.0, 0.0, 5.0). Pins the full format/order, so a field
        // reorder or label change fails here.
        assert_eq!(fb, "[1] player | faction 0 | hp 100/100 | pos (5.0, 0.0, 5.0)");
    }

    /// **Regression guard:** `/who` entries are ordered by NUMERIC entity id,
    /// not lexicographically. With ids 1, 2, 10 the numeric order is 1, 2, 10;
    /// a string sort (`entries.sort()`) would wrongly give 1, 10, 2. The exact
    /// assertion pins both format and ordering — reverting the numeric sort
    /// trips it.
    #[tokio::test]
    async fn who_lists_players_sorted_by_numeric_id() {
        let mut mgr = mgr_with_player(); // caller = entity 1, player_id 100
        for (eid, pid) in [(10u32, 110i32), (2u32, 200i32)] {
            mgr.create_entity(eid, "Castle", [1.0, 0.0, 1.0], [0.0; 3])
                .unwrap();
            mgr.connect_entity(eid);
            if let Some(p) = mgr.get_entity_mut(eid) {
                p.player_id = Some(pid);
            }
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(1, GmCommandIntent::Who, &tx, &mut mgr, &[]).await;
        let fb = feedback_text_to(&drain(&mut rx), 1).expect("who must feed back");
        assert_eq!(
            fb,
            "Players (3): 1 (player_id 100), 2 (player_id 200), 10 (player_id 110)"
        );
    }
}
