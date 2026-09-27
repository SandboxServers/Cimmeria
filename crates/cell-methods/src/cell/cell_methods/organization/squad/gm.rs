//! The squad half of the GM `.`-console's `.squad_invite` and
//! `.squad_join` (ORG-04). The console (`cimmeria-cell-console`,
//! `console/squad.rs`) does the GM check, resolves names, lists squads for
//! `.squad_info` and writes the `org.gm_action` audit row; these entry
//! points only reuse the squad handlers' checks and fanout, and return a
//! [`GmOutcome`] for that row.
//!
//! - [`gm_invite`] is the player's `/squadinvite`: the same checks, limits,
//!   feedback and `squad.invite` row, with the GM as the inviter.
//! - [`gm_join`] puts the GM into the host's squad with no invite and no
//!   answer, founding one the host leads if they have none. It skips the
//!   handshake and the leader check, never the membership rules
//!   (`SquadRegistry::force_join`).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::telemetry::Reason;
use super::{actor, confirm, fanout, feedback, invite, reject};

/// What a GM squad command did: the squad it left the GM (or the host) in,
/// and the closed refusal reason, `None` on success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmOutcome {
    pub squad_id: Option<i32>,
    pub reason: Option<&'static str>,
}

impl GmOutcome {
    fn new(squad_id: Option<i32>, result: Result<(), Reason>) -> Self {
        Self {
            squad_id,
            reason: result.err().map(Reason::as_str),
        }
    }
}

/// `.squad_invite <name>` for the GM character `gm_player_id` at
/// `gm_entity`: the `/squadinvite` path, which resolves `target_name`,
/// answers the GM and writes the `squad.invite` row itself.
pub async fn gm_invite(
    gm_player_id: i32,
    gm_entity: u32,
    target_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> GmOutcome {
    let result = invite::issue(gm_player_id, gm_entity, target_name, tx, space_mgr).await;
    GmOutcome::new(space_mgr.squads.squad_of(gm_player_id), result)
}

/// `.squad_join`: the GM at `gm_entity` joins the squad of the player at
/// `host_entity` (already resolved by the console) with no handshake. On
/// success both get the ordinary join fanout and the GM a line; a refusal
/// sends `onErrorCode` and a line.
pub async fn gm_join(
    gm_entity: u32,
    host_entity: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> GmOutcome {
    let Some(joiner) = actor(space_mgr, gm_entity) else {
        reject(tx, gm_entity, 0, feedback::NOT_READY).await;
        return GmOutcome::new(None, Err(Reason::NotReady));
    };
    let Some(host) = actor(space_mgr, host_entity) else {
        reject(tx, gm_entity, 0, feedback::NOT_READY).await;
        return GmOutcome::new(None, Err(Reason::NotAPlayer));
    };
    let host_squad = space_mgr.squads.squad_of(host.player_id);
    let (joiner_pid, host_pid, host_name) = (joiner.player_id, host.player_id, host.name.clone());
    match space_mgr.squads.force_join(joiner, host) {
        Ok(joined) => {
            space_mgr.squads.note_entity(joiner_pid, gm_entity);
            space_mgr.squads.note_entity(host_pid, host_entity);
            let newcomers: &[i32] = if joined.created {
                &[host_pid, joiner_pid]
            } else {
                &[joiner_pid]
            };
            fanout::announce_join(tx, space_mgr, joined.squad_id, joined.created, newcomers).await;
            confirm(tx, gm_entity, &feedback::gm_joined(&host_name)).await;
            GmOutcome::new(Some(joined.squad_id), Ok(()))
        }
        Err(r) => {
            let text = feedback::gm_join_rejected(r, &host_name);
            reject(tx, gm_entity, host_squad.unwrap_or(0), &text).await;
            GmOutcome::new(host_squad, Err(r.into()))
        }
    }
}
