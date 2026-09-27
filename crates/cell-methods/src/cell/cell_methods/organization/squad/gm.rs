//! The GM `.`-console's squad commands (ORG-04): `.squad_invite <name>`,
//! `.squad_join <name>` and `.squad_info [name]`. The console
//! (`cimmeria-cell-console`) parses and GM-gates the line and calls these.
//!
//! - [`gm_invite`] is the player's `/squadinvite`: the same checks, limits
//!   and outcome row (`squad.invite`), with the GM as the inviter.
//! - [`gm_join`] puts the GM into the named player's squad with no invite
//!   and no answer, founding one with that player as leader if they have
//!   none, so a single tester can build a squad with a sentinel character.
//!   It skips the handshake and the leader check, never the membership
//!   rules (`SquadRegistry::force_join`).
//! - [`gm_info`] prints a squad to the GM.
//!
//! Each ends in one INFO `org.gm_action` row on the `org` target with the
//! GM's identity, the named player's as target, `action`, `outcome` and a
//! `reason` on a refusal, and counts on `squad_actions_total` with
//! `action` = `gm_squad_invite` | `gm_squad_join` | `gm_squad_info`.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::SquadLootType;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{PlayerNameLookup, SpaceManager};
use crate::cell::squad::{count_action, Squad};

use super::telemetry::{self as tm, Reason};
use super::{actor, confirm, fanout, feedback, invite, reject};

/// A GM squad command, for the `org.gm_action` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GmAction {
    Invite,
    Join,
    Info,
}

impl GmAction {
    fn label(self) -> &'static str {
        match self {
            GmAction::Invite => "gm_squad_invite",
            GmAction::Join => "gm_squad_join",
            GmAction::Info => "gm_squad_info",
        }
    }
}

/// The one `org.gm_action` row of a GM squad command, before it is emitted.
struct GmRow {
    action: GmAction,
    entity_id: u32,
    gm: PlayerIdentity,
    target: Option<PlayerIdentity>,
    squad_id: Option<i32>,
}

impl GmRow {
    fn emit(self, result: Result<(), Reason>) {
        let reason = result.err().map(Reason::as_str);
        let outcome = if reason.is_some() { "rejected" } else { "ok" };
        let target = self.target.unwrap_or(PlayerIdentity::UNKNOWN);
        tracing::info!(
            target: "org",
            event = "org.gm_action",
            action = self.action.label(),
            outcome,
            reason,
            account_id = self.gm.account_id,
            player_id = self.gm.player_id,
            entity_id = self.entity_id,
            target_account_id = target.account_id,
            target_player_id = target.player_id,
            squad_id = self.squad_id,
            "GM squad command {}",
            outcome
        );
        count_action(self.action.label(), outcome, reason.unwrap_or("none"));
    }
}

/// The live entity of the online player called `name`, or the refusal and
/// the line to show the GM.
fn resolve(space_mgr: &SpaceManager, name: &str) -> Result<u32, (Reason, String)> {
    match space_mgr.find_online_player_by_name(name) {
        PlayerNameLookup::Found { entity_id, .. } => Ok(entity_id),
        PlayerNameLookup::InTransition { .. } => Err((
            Reason::TargetInTransition,
            feedback::target_travelling(name),
        )),
        PlayerNameLookup::NotFound => {
            Err((Reason::TargetNotFound, feedback::target_not_found(name)))
        }
        PlayerNameLookup::Ambiguous { .. } => {
            Err((Reason::TargetAmbiguous, feedback::target_ambiguous(name)))
        }
    }
}

/// `.squad_invite <name>`: invite `name` into the GM's squad (or a new one)
/// exactly as `/squadinvite` does. The invite handler writes the
/// `squad.invite` row and the feedback; this adds the GM audit row.
pub async fn gm_invite(
    entity_id: u32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let gm = tm::of_entity(space_mgr, entity_id);
    let target = resolve(space_mgr, target_name)
        .ok()
        .map(|t| tm::of_entity(space_mgr, t));
    let mut row = GmRow {
        action: GmAction::Invite,
        entity_id,
        gm,
        target,
        squad_id: None,
    };
    let Some(player_id) = gm.player_id else {
        row.emit(Err(Reason::NotReady));
        return reject(tx, entity_id, 0, feedback::NOT_READY).await;
    };
    let result = invite::issue(player_id, entity_id, target_name, tx, space_mgr).await;
    row.squad_id = space_mgr.squads.squad_of(player_id);
    row.emit(result);
}

/// `.squad_join <name>`: the GM joins `name`'s squad with no handshake.
pub async fn gm_join(
    entity_id: u32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut row = GmRow {
        action: GmAction::Join,
        entity_id,
        gm: tm::of_entity(space_mgr, entity_id),
        target: None,
        squad_id: None,
    };
    let Some(joiner) = actor(space_mgr, entity_id) else {
        row.emit(Err(Reason::NotReady));
        return reject(tx, entity_id, 0, feedback::NOT_READY).await;
    };
    let host_entity = match resolve(space_mgr, target_name) {
        Ok(t) => t,
        Err((reason, text)) => {
            row.emit(Err(reason));
            return reject(tx, entity_id, 0, &text).await;
        }
    };
    row.target = Some(tm::of_entity(space_mgr, host_entity));
    let Some(host) = actor(space_mgr, host_entity) else {
        row.emit(Err(Reason::NotAPlayer));
        let text = feedback::target_not_found(target_name);
        return reject(tx, entity_id, 0, &text).await;
    };
    let host_squad = space_mgr.squads.squad_of(host.player_id);
    let (joiner_pid, host_pid, host_name) = (joiner.player_id, host.player_id, host.name.clone());
    match space_mgr.squads.force_join(joiner, host) {
        Ok(joined) => {
            row.squad_id = Some(joined.squad_id);
            row.emit(Ok(()));
            space_mgr.squads.note_entity(joiner_pid, entity_id);
            space_mgr.squads.note_entity(host_pid, host_entity);
            let newcomers: &[i32] = if joined.created {
                &[host_pid, joiner_pid]
            } else {
                &[joiner_pid]
            };
            fanout::announce_join(tx, space_mgr, joined.squad_id, joined.created, newcomers).await;
            confirm(tx, entity_id, &feedback::gm_joined(&host_name)).await;
        }
        Err(r) => {
            row.squad_id = host_squad;
            row.emit(Err(r.into()));
            let text = feedback::gm_join_rejected(r, &host_name);
            reject(tx, entity_id, host_squad.unwrap_or(0), &text).await;
        }
    }
}

/// `.squad_info [name]`: the squad of `name`, or the GM's own.
pub async fn gm_info(
    entity_id: u32,
    target_name: Option<&str>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut row = GmRow {
        action: GmAction::Info,
        entity_id,
        gm: tm::of_entity(space_mgr, entity_id),
        target: None,
        squad_id: None,
    };
    let subject_entity = match target_name {
        None => entity_id,
        Some(name) => match resolve(space_mgr, name) {
            Ok(t) => t,
            Err((reason, text)) => {
                row.emit(Err(reason));
                return reject(tx, entity_id, 0, &text).await;
            }
        },
    };
    row.target = target_name.map(|_| tm::of_entity(space_mgr, subject_entity));
    let Some(subject) = actor(space_mgr, subject_entity) else {
        let reason = if target_name.is_some() {
            Reason::NotAPlayer
        } else {
            Reason::NotReady
        };
        row.emit(Err(reason));
        return reject(tx, entity_id, 0, feedback::NOT_READY).await;
    };
    let lines = match space_mgr.squads.squad_for(subject.player_id) {
        Some(squad) => info_lines(space_mgr, squad),
        None => vec![format!("{} is not in a squad.", subject.name)],
    };
    row.squad_id = space_mgr.squads.squad_of(subject.player_id);
    row.emit(Ok(()));
    for line in lines {
        confirm(tx, entity_id, &line).await;
    }
}

/// One header line, then one line per member in join order: rank, level,
/// and where they are (their live entity id, or in transit).
fn info_lines(space_mgr: &SpaceManager, squad: &Squad) -> Vec<String> {
    let loot = match squad.loot() {
        SquadLootType::RoundRobin => "round robin",
        SquadLootType::FreeForAll => "free for all",
    };
    let mut lines = vec![format!(
        "Squad {}: {} members, loot {loot}.",
        squad.id(),
        squad.members().len()
    )];
    for m in squad.members() {
        let rank = if m.player_id == squad.leader_player_id() {
            "leader"
        } else {
            "member"
        };
        let place = match space_mgr.player_entity_by_player_id(m.player_id) {
            Some(eid) => format!("entity {eid}"),
            None => "in transit".to_owned(),
        };
        lines.push(format!(
            "  {} ({rank}, level {}, player {}): {place}",
            m.name, m.level, m.player_id
        ));
    }
    lines
}
