//! Crafting / discipline console commands (category E): `.learndiscipline`,
//! `.forgetdiscipline`, `.allcraft`, `.craftkit`, `.learnblueprint`.
//!
//! The discipline commands route through the existing crafting grant
//! plumbing (a `GmGrantExpertise` in the `CellToBaseMsg::Plugin` envelope →
//! `cimmeria-base-crafting`'s `base::crafting::handlers`).
//! Discipline expertise is clamped `[0, 100]` base-side, so "forget" zeroes
//! the expertise (a full row delete would need a dedicated base path, noted
//! in feedback). `.allcraft` sends a `GmAllCraft` to
//! `base::crafting::allcraft`. `.craftkit` and `.learnblueprint` send a
//! `GmCraftGrant` to `base::crafting::gm_grant`, both in the envelope (#962
//! step 5); the base holds
//! the catalog, re-checks the caller's access level and answers the GM.
//!
//! Legacy reference: `deprecated/python/cell/commands/Crafting.py`.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::{
    CellToBaseMsg, GmAllCraft, GmCraftGrant, GmCraftGrantKind, GmGrantExpertise, PluginMsg,
};
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
    let account_id = space_mgr.player_identity(target).account_id;
    match name {
        "learndiscipline" => learn(caller_id, target, player_id, args, tx).await,
        "forgetdiscipline" => forget(caller_id, target, player_id, args, tx).await,
        "allcraft" => all_craft(caller_id, target, player_id, tx).await,
        "craftkit" => craft_kit(caller_id, (target, account_id, player_id), args, tx).await,
        "learnblueprint" => {
            learn_blueprint(caller_id, (target, account_id, player_id), args, tx).await
        }
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
    // Discipline ids are positive keys; reject 0/negative before sending the
    // grant so an invalid key can't be written to the player's expertise.
    if discipline_id <= 0 {
        send_gm_feedback(caller_id, "disciplineId must be a positive integer.", tx).await;
        return;
    }
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
        .send(CellToBaseMsg::Plugin(PluginMsg::new(GmGrantExpertise {
            entity_id: target,
            player_id,
            discipline_id,
            amount: expertise,
        })))
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
    if discipline_id <= 0 {
        send_gm_feedback(caller_id, "disciplineId must be a positive integer.", tx).await;
        return;
    }
    let _ = tx
        .send(CellToBaseMsg::Plugin(PluginMsg::new(GmGrantExpertise {
            entity_id: target,
            player_id,
            discipline_id,
            amount: -100,
        })))
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

/// `.allcraft` — every paradigm at 7, every discipline at 100, every
/// blueprint, and "craft anywhere" for the target's session. The
/// base holds the catalog and the persistence, so the cell only forwards;
/// the base re-checks the caller's access level and sends the result lines.
async fn all_craft(caller_id: u32, target: u32, player_id: i32, tx: &mpsc::Sender<CellToBaseMsg>) {
    let grant = GmAllCraft {
        entity_id: target,
        player_id,
        gm_entity_id: caller_id,
    };
    if let Err(e) = tx.send(CellToBaseMsg::Plugin(PluginMsg::new(grant))).await {
        tracing::warn!(
            target: "crafting",
            event = "allcraft_send_failed",
            caller_id,
            target,
            error = %e,
            "allcraft could not be queued (base channel closed)"
        );
    }
}

/// `.craftkit <blueprintId> [count=1]`: the target gets the blueprint's
/// component set 1, `count` times over. The base checks the blueprint and
/// the count range, so the cell only parses.
async fn craft_kit(
    caller_id: u32,
    target: GrantTarget,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let Some(blueprint_id) = super::parse_i32(caller_id, args, 0, "blueprintId", tx).await else {
        return;
    };
    let count = match args.get(1) {
        Some(_) => match super::parse_i32(caller_id, args, 1, "count", tx).await {
            Some(count) => count,
            None => return,
        },
        None => 1,
    };
    send_grant(
        caller_id,
        target,
        GmCraftGrantKind::Kit {
            blueprint_id,
            count,
        },
        tx,
    )
    .await;
}

/// `.learnblueprint <blueprintId>`: teach the target one blueprint.
async fn learn_blueprint(
    caller_id: u32,
    target: GrantTarget,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let Some(blueprint_id) = super::parse_i32(caller_id, args, 0, "blueprintId", tx).await else {
        return;
    };
    send_grant(
        caller_id,
        target,
        GmCraftGrantKind::LearnBlueprint { blueprint_id },
        tx,
    )
    .await;
}

/// The target of a GM crafting grant: cell entity id, account id (from
/// the cell's player identity; `None` when not threaded in) and player id.
type GrantTarget = (u32, Option<u32>, i32);

/// Forward a GM crafting grant; the base answers the GM.
async fn send_grant(
    caller_id: u32,
    (entity_id, account_id, player_id): GrantTarget,
    grant: GmCraftGrantKind,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let msg = GmCraftGrant {
        entity_id,
        player_id,
        gm_entity_id: caller_id,
        grant,
    };
    if let Err(e) = tx.send(CellToBaseMsg::Plugin(PluginMsg::new(msg))).await {
        tracing::warn!(
            target: "crafting",
            event = "forward_failed",
            kind = "gm_craft_grant",
            account_id,
            player_id,
            entity_id,
            gm_entity_id = caller_id,
            error = %e,
            "GM crafting grant could not be queued (base channel closed)"
        );
    }
}
