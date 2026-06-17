//! `/goto` — teleport the caller to a coordinate or another player.

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::{CellToBaseMsg, SpaceManager};

/// `/goto <x> <y> <z>` — teleport the caller to an absolute coordinate in their
/// current space. Mirrors `content::executor::transport::teleport`: capture the
/// prior position, update the spatial grid, then route the authoritative snap
/// through `TeleportPlayer`.
pub(super) async fn handle_goto_coords(
    caller_entity_id: u32,
    position: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Defence-in-depth: reject non-finite coordinates before any grid write.
    // The base `/goto` parser (`parse_vector3`) already rejects nan/inf, but
    // the cell must not trust the privileged intent blindly — same discipline
    // as `/give`'s count re-validation, and matching the native `gmGotoXYZ`
    // guard (#518). A NaN/inf position poisons AoI distance math (NaN fails
    // every ordering comparison, dropping the entity out of witness calc) and
    // ships a garbage forced-position snap to the client.
    if position.iter().any(|c| !c.is_finite()) {
        send_gm_feedback(
            caller_entity_id,
            "Goto failed: coordinates must be finite numbers.",
            tx,
        )
        .await;
        return;
    }

    let Some((prev_pos, space_id)) = space_mgr.get_entity(caller_entity_id).map(|e| {
        (
            [e.position.x, e.position.y, e.position.z],
            e.space_id.0 as u32,
        )
    }) else {
        send_gm_feedback(caller_entity_id, "Goto failed: you are not in a space.", tx).await;
        return;
    };

    teleport_caller(
        caller_entity_id,
        position,
        prev_pos,
        space_id,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        caller_entity_id,
        &format!(
            "Teleported to ({:.1}, {:.1}, {:.1}).",
            position[0], position[1], position[2]
        ),
        tx,
    )
    .await;
}

/// `/goto <player>` — teleport the caller to another entity in the same space.
///
/// Players carry no character name on the cell entity (only `player_id`), so
/// `name` is resolved against (a) a numeric entity id, then (b) any entity's
/// `npc_name`. The match is restricted to player entities so `/goto` only ever
/// jumps to a player, matching the command's intent.
pub(super) async fn handle_goto_player(
    caller_entity_id: u32,
    name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some((prev_pos, space_id)) = space_mgr.get_entity(caller_entity_id).map(|e| {
        (
            [e.position.x, e.position.y, e.position.z],
            e.space_id.0 as u32,
        )
    }) else {
        send_gm_feedback(caller_entity_id, "Goto failed: you are not in a space.", tx).await;
        return;
    };

    // Resolve target among players in the caller's space.
    let numeric_id: Option<u32> = name.parse::<u32>().ok();
    let target = space_mgr.all_player_entity_ids().into_iter().find(|&pid| {
        pid != caller_entity_id
            && space_mgr.get_entity_space_id(pid) == Some(space_id)
            && (Some(pid) == numeric_id
                // Dead arm for players: player entities don't carry `npc_name`,
                // so this branch never matches a player — only numeric-id
                // resolution works for `/goto <player>` in practice.
                || space_mgr
                    .get_entity(pid)
                    .and_then(|e| e.npc_name.as_deref())
                    .is_some_and(|n| n.eq_ignore_ascii_case(name)))
    });

    let Some(target_id) = target else {
        send_gm_feedback(
            caller_entity_id,
            &format!("Goto failed: no player '{name}' in your space."),
            tx,
        )
        .await;
        return;
    };

    let Some(dest) = space_mgr
        .get_entity(target_id)
        .map(|e| [e.position.x, e.position.y, e.position.z])
    else {
        send_gm_feedback(caller_entity_id, "Goto failed: target vanished.", tx).await;
        return;
    };

    teleport_caller(caller_entity_id, dest, prev_pos, space_id, tx, space_mgr).await;
    send_gm_feedback(
        caller_entity_id,
        &format!("Teleported to entity {target_id}."),
        tx,
    )
    .await;
}

/// Shared teleport bundle: update the spatial grid in place, then emit the
/// authoritative `TeleportPlayer` (FORCED_POSITION snap). `prev_pos` must be
/// captured BEFORE the grid update — it becomes the forced-position previous
/// reference vector (see `build_forced_position`).
async fn teleport_caller(
    caller_entity_id: u32,
    position: [f32; 3],
    prev_pos: [f32; 3],
    space_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    space_mgr.update_entity_position(caller_entity_id, position, [0, 0, 0], [0.0; 3]);
    if let Err(e) = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id: caller_entity_id,
            space_id,
            position,
            prev_pos,
        })
        .await
    {
        tracing::warn!(caller_entity_id, error = %e, "gm_command: TeleportPlayer send failed");
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests_common::*;
    use super::super::{handle_gm_command, GmCommandIntent};
    use super::*;
    use cimmeria_common::math::Vector3;

    #[tokio::test]
    async fn goto_coords_teleports_caller_and_emits_teleport_player() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::GotoCoords(Vector3::new(100.0, 0.0, 200.0)),
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!([e.position.x, e.position.z], [100.0, 200.0]);

        let msgs = drain(&mut rx);
        let teleport = msgs.iter().find_map(|m| match m {
            CellToBaseMsg::TeleportPlayer {
                entity_id,
                position,
                prev_pos,
                ..
            } if *entity_id == 1 => Some((*position, *prev_pos)),
            _ => None,
        });
        let (pos, prev) = teleport.expect("must emit TeleportPlayer for the caller");
        assert_eq!(pos, [100.0, 0.0, 200.0]);
        // prev_pos captured BEFORE the grid update = the spawn position.
        assert_eq!(prev, [5.0, 0.0, 5.0]);
    }

    #[tokio::test]
    async fn goto_player_jumps_to_target_by_entity_id() {
        let mut mgr = mgr_with_player();
        // Second player at id 2.
        mgr.create_entity(2, "Castle", [50.0, 0.0, 60.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(2);
        let (tx, mut rx) = mpsc::channel(8);

        handle_gm_command(
            1,
            GmCommandIntent::GotoPlayer("2".to_string()),
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!([e.position.x, e.position.z], [50.0, 60.0]);
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("Teleported to entity 2"), "got: {fb}");
    }

    #[tokio::test]
    async fn goto_player_unknown_feeds_back_error() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::GotoPlayer("Ghost".to_string()),
            &tx,
            &mut mgr,
            &[],
        )
        .await;
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("no player 'Ghost'"), "got: {fb}");
    }

    /// **Defence-in-depth guard:** the cell `/goto` handler must reject a
    /// non-finite coordinate even if one reaches it via the intent — no grid
    /// write, no `TeleportPlayer`. The base parser already rejects nan/inf,
    /// but the cell re-validates (mirrors #518's `gmGotoXYZ`). Reverting the
    /// `is_finite` check lets a NaN corrupt the entity's grid position and the
    /// forced-position snap.
    #[tokio::test]
    async fn goto_coords_rejects_non_finite() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::GotoCoords(Vector3::new(f32::NAN, 0.0, 0.0)),
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        // Position unchanged (still the fixture spawn at [5,0,5]); no teleport.
        let e = mgr.get_entity(1).unwrap();
        assert_eq!([e.position.x, e.position.y, e.position.z], [5.0, 0.0, 5.0]);
        let msgs = drain(&mut rx);
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
            "non-finite goto must not emit TeleportPlayer; got {msgs:#?}"
        );
        let fb = feedback_text_to(&msgs, 1).unwrap();
        assert!(fb.contains("finite"), "got: {fb}");
    }
}
