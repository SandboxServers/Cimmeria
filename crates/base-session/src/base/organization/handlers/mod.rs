//! Team and Command handlers on the base (ORG-06): login restore, presence,
//! leave and disband.
//!
//! - [`push`]: the organization state a member's client needs, in the
//!   ORG-E1 Q1 order ([`push_org_state`], ORG-05 reuses it after a
//!   creation), and the per-world-entry [`restore_on_login`].
//! - [`presence`]: `onMemberJoinedOrganization` [37] to the online members
//!   on login (the entity id) and on every session end (id 0,
//!   [`announce_offline`]).
//! - [`leave`]: `organizationLeave` (CM 9) for a Team or Command id,
//!   forwarded by the cell ([`handle_leave`]).
//! - [`disband`]: the disband fanout and `.org_disband` ([`gm_disband`]).
//! - [`fanout`]: online members as a view over the connected-client map,
//!   and the sends.
//! - [`telemetry`]: the outcome row and `org_actions_total`.
//!
//! ORG-07 adds invite, the invite response, kick and rank change, the GM
//! `.org_join` and `.org_rank`, and the ORG-API fanout:
//!
//! - [`invite`]: `organizationInvite` (0xCF) and `organizationInviteByType`
//!   (0xD0) for Teams and Commands ([`handle_invite`]).
//! - [`invite_response`]: `organizationInviteResponse` (CM 8) for a base
//!   request id ([`handle_invite_response`]).
//! - [`kick`]: `organizationKick` (0xD1) ([`handle_kick`]).
//! - [`rank`]: `organizationRankChange` (0xD2) ([`handle_rank_change`]).
//! - [`gm`]: `.org_join` and `.org_rank` ([`gm_join`], [`gm_rank`]).
//! - [`broadcast`]: [`broadcast_to_org`], one client method to the online
//!   members, optionally filtered by a permission.
//! - [`targets`]: resolving the second player by name; [`answer`]: the
//!   refusal lines.
//!
//! Every mutation follows ORG-LOCK (D-ORG04): one transaction, the
//! organization row locked first, authorization read inside it. Fanout runs
//! after the commit. Campaign ledger: `docs/analysis/organizations/`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

pub mod answer;
pub mod broadcast;
pub mod disband;
pub mod fanout;
pub mod gm;
pub mod invite;
pub mod invite_response;
pub mod kick;
pub mod leave;
pub mod presence;
pub mod push;
pub mod rank;
pub mod targets;
pub mod telemetry;

pub use broadcast::broadcast_to_org;
pub use disband::{gm_disband, GmCaller, GM_ACCESS_LEVEL};
pub use gm::{gm_join, gm_rank};
pub use invite::{handle_invite, InviteInto};
pub use invite_response::{handle_invite_response, InviteAnswer};
pub use kick::handle_kick;
pub use leave::{handle_leave, LeaveOutcome};
pub use presence::announce_offline;
pub use push::{org_state_messages, push_org_state, restore_on_login, PushError, PushSummary};
pub use rank::handle_rank_change;
pub use telemetry::OrgReject;

#[cfg(test)]
mod tests;

/// What every organization handler needs from the base.
#[derive(Clone, Copy)]
pub struct OrgCtx<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    /// For `OrgMembershipEnded` (the Bank's vault-session hook). `None` in
    /// paths that never end a membership (presence) and in tests.
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
}

/// The acting member, from the base's own session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrgPlayer {
    pub account_id: Option<u32>,
    pub player_id: i32,
    pub entity_id: u32,
}

/// The session behind a forwarded call: the one that owns `entity_id` and
/// is playing `player_id` in the world. `None` when that is no longer true
/// (the character logged off, or the entity id was reused between the
/// cell's send and now); the caller drops the call with WARN
/// `org.actor_mismatch`.
pub fn resolve_actor(ctx: &OrgCtx<'_>, player_id: i32, entity_id: u32) -> Option<OrgPlayer> {
    let addr = ctx.entity_to_addr.lock().ok()?.get(&entity_id).copied()?;
    let clients = ctx.connected.lock().ok()?;
    let c = clients.get(&addr)?;
    (c.listed_online
        && c.active_player_id == Some(player_id)
        && c.player_entity_id == Some(entity_id))
    .then_some(OrgPlayer {
        account_id: Some(c.account_id),
        player_id,
        entity_id,
    })
}
