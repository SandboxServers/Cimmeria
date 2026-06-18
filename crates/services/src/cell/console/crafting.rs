//! Crafting / discipline console commands (category E): `.learndiscipline`,
//! `.forgetdiscipline`, `.allcraft`.
//!
//! These route through the existing crafting grant plumbing
//! (`CellToBaseMsg::GrantExpertise` → `base::crafting::handlers`). Discipline
//! expertise is clamped `[0, 100]` base-side, so "forget" zeroes the expertise
//! (a full row delete would need a dedicated base path, noted in feedback).
//!
//! Legacy reference: `deprecated/python/cell/commands/Crafting.py`.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(
            caller_id,
            &format!(".{name}: a player target is required."),
            tx,
        )
        .await;
        return;
    };
    let Some(player_id) = space_mgr.get_entity(target).and_then(|e| e.player_id) else {
        send_gm_feedback(caller_id, &format!(".{name}: target has no player id."), tx).await;
        return;
    };
    match name {
        "learndiscipline" => learn(caller_id, target, player_id, args, tx).await,
        "forgetdiscipline" => forget(caller_id, target, player_id, args, tx).await,
        "allcraft" => all_craft(caller_id, tx).await,
        _ => {}
    }
}

/// `.learndiscipline <disciplineId> [expertise=1]` — learn/raise a discipline.
async fn learn(
    caller_id: u32,
    target: u32,
    player_id: i32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let Some(discipline_id) = super::parse_i32(caller_id, args, 0, "disciplineId", tx).await else {
        return;
    };
    let expertise = match args.get(1) {
        Some(s) => match s.parse::<i32>() {
            Ok(v) if v > 0 => v,
            _ => {
                send_gm_feedback(caller_id, "expertise must be a positive integer.", tx).await;
                return;
            }
        },
        None => 1,
    };
    let _ = tx
        .send(CellToBaseMsg::GrantExpertise {
            entity_id: target,
            player_id,
            discipline_id,
            amount: expertise,
        })
        .await;
    send_gm_feedback(
        caller_id,
        &format!("learndiscipline [{target}] discipline {discipline_id} +{expertise}"),
        tx,
    )
    .await;
}

/// `.forgetdiscipline <disciplineId>` — zero out a discipline's expertise. (The
/// base grant path clamps to `[0, 100]`; a negative delta drives it to 0.)
async fn forget(
    caller_id: u32,
    target: u32,
    player_id: i32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let Some(discipline_id) = super::parse_i32(caller_id, args, 0, "disciplineId", tx).await else {
        return;
    };
    let _ = tx
        .send(CellToBaseMsg::GrantExpertise {
            entity_id: target,
            player_id,
            discipline_id,
            amount: -100,
        })
        .await;
    send_gm_feedback(
        caller_id,
        &format!(
            "forgetdiscipline [{target}] discipline {discipline_id} -> expertise 0 \
             (row not deleted)"
        ),
        tx,
    )
    .await;
}

/// `.allcraft` — the legacy granted every blueprint + max racial paradigm + all
/// disciplines. That needs the full blueprint/discipline/paradigm catalog, which
/// is not cached cell-side and has no consolidated base grant path, so this is a
/// pointer to the per-discipline grant rather than a silent partial unlock.
async fn all_craft(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>) {
    send_gm_feedback(
        caller_id,
        "allcraft: full blueprint unlock isn't wired cell-side. Use \
         .learndiscipline <id> 100 per discipline (and the native gm crafting grants).",
        tx,
    )
    .await;
}
