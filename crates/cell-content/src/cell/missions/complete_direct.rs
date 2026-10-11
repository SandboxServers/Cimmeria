//! Direct mission completion: a chain action completes the whole mission
//! without stepping through its objectives.

use tokio::sync::mpsc;

use cimmeria_entity::missions::STATUS_COMPLETED;

use super::{
    send_mission_frame, suppress_hidden_mission_frames, ON_MISSION_UPDATE, ON_OBJECTIVE_UPDATE,
    ON_STEP_UPDATE,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{EntityNames, SpaceManager};

/// Complete a mission directly (all objectives + step + mission update).
///
/// Used by the content engine when a chain action completes a mission
/// without stepping through individual objectives.
#[tracing::instrument(
    name = "mission.complete_direct",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, player_id = tracing::field::Empty)
)]
pub async fn complete_mission_direct(
    entity_id: u32,
    mission_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if super::state_lock::refuse_if_state_unknown(entity_id, mission_id, "complete", tx, space_mgr)
        .await
    {
        return;
    }
    let who = space_mgr.player_identity(entity_id);
    let world = space_mgr.get_entity_world_name(entity_id);
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(
                reason = "entity_missing",
                entity_id, // nt:id-only the entity is gone, nothing left to name
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                "complete_mission_direct: entity not found — mission not completed"
            );
            return;
        }
    };
    if let Some(pid) = entity.player_id {
        tracing::Span::current().record("player_id", pid);
    }
    let player_id = entity.player_id;
    // Snapshot for the log lines below, which run while `mission` borrows
    // the entity mutably. A player's name is already interned: no lookup.
    let entity_name = EntityNames::of(entity).entity_name;

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => {
            tracing::warn!(
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                world = world.as_deref(),
                entity_id,
                entity_name,
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                "complete_mission_direct: mission not found"
            );
            return;
        }
    };

    // Objectives still open here are force-completed below. The stuck-player
    // report on that ("objective_never_completed") is made by the content
    // executor before it calls in here, because only it holds the chain
    // engine that tells a turn-in objective from one whose trigger never
    // fired -- see `content::executor::mission::report_objectives_left_open`.

    // Complete all objectives. The flags ride along so the wire frames
    // below can report them (the client renders optional objectives
    // differently); they are captured before the mutation because
    // `complete_objective` borrows the instance mutably.
    let objectives: Vec<(i32, bool, bool)> = mission
        .active_objectives
        .iter()
        .map(|o| (o.objective_id, o.hidden, o.optional))
        .collect();
    for (oid, _, _) in &objectives {
        mission.complete_objective(*oid);
    }
    mission.complete();

    let step_id = mission.completed_steps.last().copied();
    let is_hidden = mission.is_hidden;

    tracing::info!(
        event = "mission_completed",
        path = "direct",
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        world = world.as_deref(),
        entity_id,
        entity_name,
        mission_id,
        mission_name = cimmeria_names::book().mission(mission_id),
        "Mission completed directly"
    );
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::MISSION_COMPLETE,
        format!("mission={mission_id} direct"),
    );

    // Hidden missions complete normally; only the frames are skipped (#715).
    if suppress_hidden_mission_frames(
        is_hidden,
        entity_id,
        player_id,
        mission_id,
        "complete_mission_direct",
    ) {
        return;
    }

    // Send objective updates
    for (oid, hidden, optional) in &objectives {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&oid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        args.push(u8::from(*hidden));
        args.push(u8::from(*optional));
        send_mission_frame(
            tx,
            entity_id,
            ON_OBJECTIVE_UPDATE,
            args,
            "onObjectiveUpdate",
            mission_id,
            &who,
        )
        .await;
    }

    // Send step completed
    if let Some(sid) = step_id {
        let mut args = Vec::with_capacity(5);
        args.extend_from_slice(&sid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        send_mission_frame(
            tx,
            entity_id,
            ON_STEP_UPDATE,
            args,
            "onStepUpdate",
            mission_id,
            &who,
        )
        .await;
    }

    // Send mission completed
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&mission_id.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    send_mission_frame(
        tx,
        entity_id,
        ON_MISSION_UPDATE,
        args,
        "onMissionUpdate",
        mission_id,
        &who,
    )
    .await;
}
