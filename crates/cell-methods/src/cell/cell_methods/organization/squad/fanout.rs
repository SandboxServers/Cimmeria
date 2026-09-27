//! What each squad member's client is told when the squad changes.
//!
//! Every recipient is resolved to their **live** entity per event
//! ([`SpaceManager::player_entity_by_player_id`]), never from a cached id:
//! entity ids are recycled and gate travel re-creates the entity. A member
//! who resolves to nothing is in gate transit; bystander messages to them
//! are skipped (their world entry replays the squad, see `world_entry`), and
//! a terminal `onOrganizationLeft` owed to them is queued instead of
//! dropped.
//!
//! Message order follows ORG-E1 Q1: `onOrganizationRosterInfo` [38] stores
//! every member with id 0 ("Offline"), and only `onMemberJoinedOrganization`
//! [37] sets a member's id, so the roster always comes first and a [37] per
//! other member follows it.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::{Departure, Squad, SquadMember};

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::organization::{OrgLeaveReason, OrgRank, OrgType};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_joined_organization, build_on_member_left_organization,
    build_on_member_rank_changed_organization, build_on_organization_joined,
    build_on_organization_left, build_on_organization_roster_info, build_on_squad_loot_type,
    RosterInfo, ON_MEMBER_JOINED_ORGANIZATION, ON_MEMBER_LEFT_ORGANIZATION,
    ON_MEMBER_RANK_CHANGED_ORGANIZATION, ON_ORGANIZATION_JOINED, ON_ORGANIZATION_LEFT,
    ON_ORGANIZATION_ROSTER_INFO, ON_SQUAD_LOOT_TYPE,
};

/// Queue one client method on `entity_id`'s player.
pub(super) async fn send(
    tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
) {
    let msg = CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    };
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "squad",
            event = "squad.send_failed",
            entity_id,
            method_index,
            reason = "cell_to_base_closed",
            "squad client method could not be queued -- the client's squad view is stale"
        );
    }
}

/// The roster snapshot of a player entity, or `None` when it is not a
/// fully initialised player (no `player_id` or no cached name yet).
pub(super) fn member_snapshot(entity: &CellEntity) -> Option<SquadMember> {
    if !entity.is_player {
        return None;
    }
    Some(SquadMember {
        player_id: entity.player_id?,
        name: entity.character_name.clone()?,
        level: u8::try_from(entity.level).unwrap_or(u8::MAX),
        archetype: entity
            .archetype_id
            .and_then(|a| u8::try_from(a).ok())
            .unwrap_or(0),
    })
}

/// The member id the client stores for `player_id`: their live entity id,
/// or 0 (D-ORG11).
fn member_id(space_mgr: &SpaceManager, player_id: i32) -> i32 {
    space_mgr
        .player_entity_by_player_id(player_id)
        .map_or(0, |eid| eid as i32)
}

/// Mirror the registry onto `player_id`'s live entity.
pub(super) fn stamp_squad_id(space_mgr: &mut SpaceManager, player_id: i32, squad_id: Option<i32>) {
    if let Some(eid) = space_mgr.player_entity_by_player_id(player_id) {
        if let Some(entity) = space_mgr.get_entity_mut(eid) {
            entity.squad_id = squad_id;
        }
    }
}

fn roster(squad: &Squad) -> Vec<RosterInfo> {
    squad
        .members()
        .iter()
        .map(|m| RosterInfo {
            name: m.name.clone(),
            level: m.level,
            archetype: m.archetype,
            rank: squad.rank_of(m.player_id),
            note: String::new(),
            officer_note: String::new(),
        })
        .collect()
}

/// The whole squad, sent to one member's client: `onOrganizationJoined`
/// [35], the roster [38], one `onMemberJoinedOrganization` [37] per other
/// member with their live entity id, and the loot mode [51].
///
/// `new_member` is 1 on a fresh join and 0 on a world-entry replay; each
/// [37] carries 1 only for members in `newcomers`.
pub(super) async fn send_whole_squad(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
    squad: &Squad,
    recipient_entity: u32,
    recipient_player: i32,
    new_member: bool,
    newcomers: &[i32],
) {
    let sid = squad.id();
    send(
        tx,
        recipient_entity,
        ON_ORGANIZATION_JOINED,
        build_on_organization_joined(
            sid,
            OrgType::Squad,
            squad.rank_of(recipient_player),
            new_member,
        ),
    )
    .await;
    send(
        tx,
        recipient_entity,
        ON_ORGANIZATION_ROSTER_INFO,
        build_on_organization_roster_info(sid, &roster(squad)),
    )
    .await;
    for m in squad
        .members()
        .iter()
        .filter(|m| m.player_id != recipient_player)
    {
        send(
            tx,
            recipient_entity,
            ON_MEMBER_JOINED_ORGANIZATION,
            build_on_member_joined_organization(
                &m.name,
                member_id(space_mgr, m.player_id),
                sid,
                squad.rank_of(m.player_id),
                newcomers.contains(&m.player_id),
            ),
        )
        .await;
    }
    send(
        tx,
        recipient_entity,
        ON_SQUAD_LOOT_TYPE,
        build_on_squad_loot_type(sid, squad.loot()),
    )
    .await;
}

/// Announce `newcomers` joining `squad_id`: each newcomer gets the whole
/// squad; every other online member gets one [37] per newcomer.
pub(super) async fn announce_join(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    squad_id: i32,
    newcomers: &[i32],
) {
    for &pid in newcomers {
        stamp_squad_id(space_mgr, pid, Some(squad_id));
    }
    let Some(squad) = space_mgr.squads.squad(squad_id).cloned() else {
        return;
    };
    for m in squad.members() {
        let Some(eid) = space_mgr.player_entity_by_player_id(m.player_id) else {
            continue;
        };
        if newcomers.contains(&m.player_id) {
            send_whole_squad(tx, space_mgr, &squad, eid, m.player_id, true, newcomers).await;
            continue;
        }
        for n in squad
            .members()
            .iter()
            .filter(|n| newcomers.contains(&n.player_id))
        {
            send(
                tx,
                eid,
                ON_MEMBER_JOINED_ORGANIZATION,
                build_on_member_joined_organization(
                    &n.name,
                    member_id(space_mgr, n.player_id),
                    squad_id,
                    squad.rank_of(n.player_id),
                    true,
                ),
            )
            .await;
        }
    }
}

/// Tell `player_id` they left `squad_id`, or queue it for their next world
/// entry when they are in transit. A logout needs neither: the client is
/// gone.
async fn tell_left(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    player_id: i32,
    squad_id: i32,
    reason: OrgLeaveReason,
) {
    stamp_squad_id(space_mgr, player_id, None);
    if reason == OrgLeaveReason::Logout {
        return;
    }
    match space_mgr.player_entity_by_player_id(player_id) {
        Some(eid) => {
            send(
                tx,
                eid,
                ON_ORGANIZATION_LEFT,
                build_on_organization_left(reason, squad_id),
            )
            .await;
        }
        None => {
            tracing::debug!(
                target: "squad",
                event = "squad.left_owed",
                player_id,
                squad_id,
                reason = reason.as_u8(),
                "squad member in transit; onOrganizationLeft queued for their world entry"
            );
            space_mgr.squads.owe_left(player_id, squad_id, reason);
        }
    }
}

/// Announce a departure (leave, kick or logout) to everyone concerned:
///
/// 1. the departed member: `onOrganizationLeft` [36] with the reason;
/// 2. every remaining member: `onMemberLeftOrganization` [39];
/// 3. a promotion: `onMemberRankChangedOrganization` [40] (`Leader`) to
///    every remaining member;
/// 4. a disband: the member left alone gets [36] `Disbanded`, after the
///    [39] that removes the departed row.
pub(super) async fn announce_departure(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    d: &Departure,
) {
    let sid = d.squad_id;
    // Resolved before `tell_left` so a logging-out member's [39] still
    // names their entity: the DisconnectEntity arm runs this before the
    // teardown.
    let departed_id = member_id(space_mgr, d.departed.player_id);
    tell_left(tx, space_mgr, d.departed.player_id, sid, d.reason).await;
    for m in &d.remaining {
        let Some(eid) = space_mgr.player_entity_by_player_id(m.player_id) else {
            continue;
        };
        send(
            tx,
            eid,
            ON_MEMBER_LEFT_ORGANIZATION,
            build_on_member_left_organization(departed_id, d.reason, sid, &d.departed.name),
        )
        .await;
    }
    if let Some(leader) = d.new_leader {
        let name = d
            .remaining
            .iter()
            .find(|m| m.player_id == leader)
            .map_or("", |m| m.name.as_str());
        let leader_id = member_id(space_mgr, leader);
        for m in &d.remaining {
            let Some(eid) = space_mgr.player_entity_by_player_id(m.player_id) else {
                continue;
            };
            send(
                tx,
                eid,
                ON_MEMBER_RANK_CHANGED_ORGANIZATION,
                build_on_member_rank_changed_organization(leader_id, OrgRank::LEADER, sid, name),
            )
            .await;
        }
        tracing::info!(
            target: "squad",
            event = "squad.leader_promoted",
            squad_id = sid,
            player_id = leader,
            "squad leader left; the longest-standing member leads"
        );
    }
    if d.disbanded {
        for m in &d.remaining {
            tell_left(tx, space_mgr, m.player_id, sid, OrgLeaveReason::Disbanded).await;
        }
        tracing::info!(
            target: "squad",
            event = "squad.disbanded",
            squad_id = sid,
            "squad of one dissolved"
        );
    }
}

/// `onSquadLootType` [51] to `entity_id`.
pub(super) async fn send_loot_type(
    tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    squad: &Squad,
) {
    send(
        tx,
        entity_id,
        ON_SQUAD_LOOT_TYPE,
        build_on_squad_loot_type(squad.id(), squad.loot()),
    )
    .await;
}
