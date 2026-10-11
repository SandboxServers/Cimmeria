//! `gmGoto` and `gmSummon` — same-space teleports by numeric entity id.

use cimmeria_cell_world::cell::plugin::EntityHookPoint;
use tokio::sync::mpsc;

use super::super::feedback::send_gm_feedback;
use super::super::forward_to_base;
use super::parse_target_id;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `gmGoto(WSTRING aNameOrID)` — teleport the calling GM to the target entity's
/// position. Numeric-id form only (name→id resolution isn't wired in the cell);
/// same-space only (cross-space requires `gmGotoLocation`).
pub(in crate::cell::console::gm) async fn handle_goto(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // The calling GM's Rule 5 identity, carried on every row below.
    let who = space_mgr.player_identity(entity_id);
    let Some(target_eid) = parse_target_id(entity_id, args, "gmGoto", space_mgr) else {
        send_gm_feedback(
            entity_id,
            "gmGoto: NameOrID must be a positive numeric id",
            tx,
        )
        .await;
        return true;
    };
    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let dest = match space_mgr.get_entity(target_eid) {
        Some(t) if Some(t.space_id.0) == caller_space => [t.position.x, t.position.y, t.position.z],
        Some(_) => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                target_entity_id = target_eid,
                target_entity_name = space_mgr.entity_label(target_eid),
                "gmGoto: target in a different space — refused"
            );
            send_gm_feedback(
                entity_id,
                &format!("gmGoto: target {target_eid} is in a different space"),
                tx,
            )
            .await;
            return true;
        }
        None => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                target_entity_id = target_eid, // nt:id-only the target is gone, so it has no name
                "gmGoto: target not found"
            );
            send_gm_feedback(
                entity_id,
                &format!("gmGoto: target {target_eid} not found"),
                tx,
            )
            .await;
            return true;
        }
    };
    let (space_id, prev_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.space_id.0 as u32,
            [e.position.x, e.position.y, e.position.z],
        ),
        None => {
            tracing::warn!(
                entity_id, // nt:id-only the caller is gone, so it has no name
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                reason = "entity_missing",
                "gmGoto: caller entity not found"
            );
            send_gm_feedback(entity_id, "gmGoto: caller entity not found", tx).await;
            return true;
        }
    };
    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        target_entity_id = target_eid,
        target_entity_name = space_mgr.entity_label(target_eid),
        subject_player_id = space_mgr.player_identity(target_eid).player_id,
        subject_player_name = space_mgr.player_identity(target_eid).player_name,
        ?dest,
        "gmGoto: teleporting GM to target"
    );
    // Position-only: `gmGoto` puts the caller *at* the target, it does not
    // point them at it.
    space_mgr.update_position_preserving_facing(entity_id, dest, [0.0; 3]);
    space_mgr.note_authorized_teleport(entity_id); // reseed validator clock

    // SS-D3, #962: the travel hook. The duel plugin ends the traveller's duel
    // (`EDUEL_DEFEAT_Teleport`) here; `every_travel_site_fires_the_travel_hook`.
    space_mgr
        .fire_entity_hook(EntityHookPoint::BeforeTravelSend, entity_id, tx)
        .await;
    if !forward_to_base(
        tx,
        CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position: dest,
            prev_pos,
        },
        "gmGoto",
    )
    .await
    {
        return true; // base channel closed — don't claim a snap that never sent.
    }
    // A pet out with the moved player comes along (pets PT-02).
    cimmeria_cell_world::cell::pets::on_owner_teleported(
        entity_id,
        cimmeria_cell_world::cell::pets::OwnerPath::GmTravel,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        entity_id,
        &format!("gmGoto: teleported to entity {target_eid}"),
        tx,
    )
    .await;
    true
}

/// `gmSummon(WSTRING aNameOrID)` — move the target entity to the caller's
/// position. A player target is snapped via `TeleportPlayer`; an NPC via the
/// spatial-grid update (witnesses pick it up on the next AoI tick). Numeric-id
/// form, same-space only.
pub(in crate::cell::console::gm) async fn handle_summon(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // The calling GM's Rule 5 identity, carried on every row below.
    let who = space_mgr.player_identity(entity_id);
    let Some(target_eid) = parse_target_id(entity_id, args, "gmSummon", space_mgr) else {
        send_gm_feedback(
            entity_id,
            "gmSummon: NameOrID must be a positive numeric id",
            tx,
        )
        .await;
        return true;
    };
    if target_eid == entity_id {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            "gmSummon: cannot summon yourself"
        );
        send_gm_feedback(entity_id, "gmSummon: cannot summon yourself", tx).await;
        return true;
    }
    let (caller_space, caller_pos) = match space_mgr.get_entity(entity_id) {
        Some(e) => (e.space_id.0, [e.position.x, e.position.y, e.position.z]),
        None => {
            send_gm_feedback(entity_id, "gmSummon: caller entity not found", tx).await;
            return true;
        }
    };
    let (is_player, target_prev) = match space_mgr.get_entity(target_eid) {
        Some(t) if t.space_id.0 == caller_space => {
            (t.is_player, [t.position.x, t.position.y, t.position.z])
        }
        Some(_) => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                target_entity_id = target_eid,
                target_entity_name = space_mgr.entity_label(target_eid),
                "gmSummon: target in a different space — refused"
            );
            send_gm_feedback(
                entity_id,
                &format!("gmSummon: target {target_eid} is in a different space"),
                tx,
            )
            .await;
            return true;
        }
        None => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                target_entity_id = target_eid, // nt:id-only the target is gone, so it has no name
                "gmSummon: target not found"
            );
            send_gm_feedback(
                entity_id,
                &format!("gmSummon: target {target_eid} not found"),
                tx,
            )
            .await;
            return true;
        }
    };
    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        target_entity_id = target_eid,
        target_entity_name = space_mgr.entity_label(target_eid),
        subject_player_id = space_mgr.player_identity(target_eid).player_id,
        subject_player_name = space_mgr.player_identity(target_eid).player_name,
        is_player,
        "gmSummon: moving target to caller"
    );
    // Position-only: the summoned entity arrives at the caller still facing
    // whichever way it was.
    space_mgr.update_position_preserving_facing(target_eid, caller_pos, [0.0; 3]);
    space_mgr.note_authorized_teleport(target_eid); // reseed validator clock

    // SS-D3, #962: the travel hook. The duel plugin ends the traveller's duel
    // (`EDUEL_DEFEAT_Teleport`) here; `every_travel_site_fires_the_travel_hook`.
    space_mgr
        .fire_entity_hook(EntityHookPoint::BeforeTravelSend, target_eid, tx)
        .await;
    if is_player
        && !forward_to_base(
            tx,
            // Players need the authoritative forced-position snap to their client.
            CellToBaseMsg::TeleportPlayer {
                entity_id: target_eid,
                space_id: caller_space as u32,
                position: caller_pos,
                prev_pos: target_prev,
            },
            "gmSummon",
        )
        .await
    {
        return true; // base channel closed — don't claim a snap that never sent.
    }
    // A pet out with the moved player comes along (pets PT-02).
    cimmeria_cell_world::cell::pets::on_owner_teleported(
        target_eid,
        cimmeria_cell_world::cell::pets::OwnerPath::GmTravel,
        tx,
        space_mgr,
    )
    .await;
    send_gm_feedback(
        entity_id,
        &format!("gmSummon: summoned entity {target_eid} to you"),
        tx,
    )
    .await;
    true
}
