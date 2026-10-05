//! Squad console commands (organizations campaign ORG-04): `.squad_invite
//! <name>`, `.squad_join <name>` and `.squad_info [name]`, so one tester
//! can build and inspect a squad with a sentinel character.
//!
//! The console owns the GM check, the name resolution, the `.squad_info`
//! listing and the audit row. The squad work itself is
//! `cimmeria_cell_interactions::cell::organization::squad::{gm_invite, gm_join}`,
//! beside the ORG-03 handlers whose checks and join fanout it reuses; those
//! return a `GmOutcome` for the row.
//!
//! # Telemetry
//!
//! Each command ends in exactly one INFO `event = org.gm_action` on the
//! `org` target: `action` (`gm_squad_invite` | `gm_squad_join` |
//! `gm_squad_info`), `outcome`, a closed `reason` on a refusal, the GM's
//! `account_id` / `player_id` / `entity_id`, the named player's
//! `target_account_id` / `target_player_id`, and `squad_id`. Each counts on
//! `squad_actions_total` under the same `action`.

use cimmeria_cell_world::cell::squad::SquadResources;
use tokio::sync::mpsc;

use cimmeria_cell_interactions::cell::organization::squad::{self, GmOutcome};
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::SquadLootType;

use super::send_gm_feedback;
use crate::cell::dispatch::is_gm;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{PlayerNameLookup, SpaceManager};
use crate::cell::squad::{count_action, Squad};

/// The one `org.gm_action` row of a squad command.
struct GmRow {
    action: &'static str,
    entity_id: u32,
    gm: PlayerIdentity,
    target: Option<PlayerIdentity>,
}

impl GmRow {
    fn emit(self, out: GmOutcome) {
        let outcome = if out.reason.is_some() {
            "rejected"
        } else {
            "ok"
        };
        let target = self.target.unwrap_or(PlayerIdentity::UNKNOWN);
        tracing::info!(
            target: "org",
            event = "org.gm_action",
            action = self.action,
            outcome,
            reason = out.reason,
            account_id = self.gm.account_id,
            account_name = self.gm.account_name,
            player_id = self.gm.player_id,
            player_name = self.gm.player_name,
            entity_id = self.entity_id,
            entity_name = self.gm.player_name,
            target_account_id = target.account_id,
            target_account_name = target.account_name,
            target_player_id = target.player_id,
            target_player_name = target.player_name,
            squad_id = out.squad_id, // nt:id-only squads carry no name, only their members do
            "GM squad command {}",
            outcome,
        );
        count_action(self.action, outcome, out.reason.unwrap_or("none"));
    }
}

fn refused(reason: &'static str) -> GmOutcome {
    GmOutcome {
        squad_id: None,
        reason: Some(reason),
    }
}

/// The live entity of the online player called `name`, or the refusal
/// reason and the line to show the GM.
fn resolve(space_mgr: &SpaceManager, name: &str) -> Result<u32, (&'static str, String)> {
    match space_mgr.find_online_player_by_name(name) {
        PlayerNameLookup::Found { entity_id, .. } => Ok(entity_id),
        PlayerNameLookup::InTransition { .. } => Err((
            "target_in_transition",
            format!("{name} is travelling. Try again in a moment."),
        )),
        PlayerNameLookup::NotFound => Err((
            "target_not_found",
            format!("No player named {name} is online."),
        )),
        PlayerNameLookup::Ambiguous { .. } => Err((
            "target_ambiguous",
            format!("More than one player is called {name}."),
        )),
    }
}

/// Route one `.squad_*` command. `args` has passed the registry's count.
pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let action = match name {
        "squad_invite" => "gm_squad_invite",
        "squad_join" => "gm_squad_join",
        _ => "gm_squad_info",
    };
    let mut row = GmRow {
        action,
        entity_id: caller_id,
        gm: space_mgr.player_identity(caller_id),
        target: None,
    };
    // The chat gate admits only GMs; checked again here because these
    // commands change another player's squad.
    let access_level = space_mgr
        .get_entity(caller_id)
        .map_or(0, |e| e.access_level);
    if !is_gm(access_level) {
        row.emit(refused("not_gm"));
        send_gm_feedback(caller_id, &format!(".{name} is a GM command."), tx).await;
        return;
    }
    let out = match name {
        "squad_invite" => invite(&mut row, args[0], tx, space_mgr).await,
        "squad_join" => join(&mut row, args[0], tx, space_mgr).await,
        _ => info(&mut row, args.first().copied(), tx, space_mgr).await,
    };
    row.emit(out);
}

/// `.squad_invite <name>`: the `/squadinvite` path from the GM. A name
/// that does not resolve is refused by that path, with its own feedback.
async fn invite(
    row: &mut GmRow,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> GmOutcome {
    row.target = resolve(space_mgr, target_name)
        .ok()
        .map(|t| space_mgr.player_identity(t));
    let Some(player_id) = row.gm.player_id else {
        send_gm_feedback(
            row.entity_id,
            "Squads are not available until you have entered the world.",
            tx,
        )
        .await;
        return refused("not_ready");
    };
    squad::gm_invite(player_id, row.entity_id, target_name, tx, space_mgr).await
}

/// `.squad_join <name>`.
async fn join(
    row: &mut GmRow,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> GmOutcome {
    let host = match resolve(space_mgr, target_name) {
        Ok(t) => t,
        Err((reason, line)) => {
            send_gm_feedback(row.entity_id, &line, tx).await;
            return refused(reason);
        }
    };
    row.target = Some(space_mgr.player_identity(host));
    squad::gm_join(row.entity_id, host, tx, space_mgr).await
}

/// `.squad_info [name]`: the named player's squad, or the GM's own.
async fn info(
    row: &mut GmRow,
    target_name: Option<&str>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> GmOutcome {
    let subject = match target_name {
        None => row.entity_id,
        Some(name) => match resolve(space_mgr, name) {
            Ok(t) => {
                row.target = Some(space_mgr.player_identity(t));
                t
            }
            Err((reason, line)) => {
                send_gm_feedback(row.entity_id, &line, tx).await;
                return refused(reason);
            }
        },
    };
    let entity = space_mgr.get_entity(subject);
    let (Some(player_id), Some(name)) = (
        entity.and_then(|e| e.player_id),
        entity.and_then(|e| e.character_name.clone()),
    ) else {
        send_gm_feedback(
            row.entity_id,
            "That player has not entered the world yet.",
            tx,
        )
        .await;
        return refused("not_ready");
    };
    let squad_id = space_mgr.resources.squads().squad_of(player_id);
    let lines = match space_mgr.resources.squads().squad_for(player_id) {
        Some(squad) => info_lines(space_mgr, squad),
        None => vec![format!("{name} is not in a squad.")],
    };
    for line in lines {
        send_gm_feedback(row.entity_id, &line, tx).await;
    }
    GmOutcome {
        squad_id,
        reason: None,
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
