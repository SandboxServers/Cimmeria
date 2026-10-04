//! `.org_create <team|command> <name>` (organizations campaign ORG-05):
//! found a Team or Command directly, skipping the registrar NPC, so one
//! tester can set up an organization without walking to the debug hub.
//!
//! The GM founds it for themselves. The console checks the GM's access
//! level and the arguments, then forwards `OrgCellToBase::GmCreate`; the
//! base re-reads the access level from its own session (D-ORG13) and applies
//! every creation rule the registrar path does (D-ORG10 name, D-ORG18 one
//! per type, the D-ORG15 cost), with no pending creation.
//!
//! # Telemetry
//!
//! The command ends in exactly one INFO `event = org.gm_action`, `action =
//! gm_org_create`, on the `org` target. The console writes it for the
//! refusals it decides (`not_gm`, `usage`, `not_ready`, `base_unreachable`);
//! the base writes it for everything after the forward (`ok`, or the
//! creation reasons). Both count on `org_actions_total`.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::OrgType;

use super::send_gm_feedback;
use crate::cell::dispatch::is_gm;
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::org_creation::count_org_action;
use crate::cell::space_manager::SpaceManager;

/// The usage line.
pub(super) const USAGE: &str = "Usage: .org_create <team|command> <name>";

/// `team` / `command` (any case) as an organization type.
fn parse_type(s: &str) -> Option<OrgType> {
    match s.to_ascii_lowercase().as_str() {
        "team" => Some(OrgType::Team),
        "command" => Some(OrgType::Command),
        _ => None,
    }
}

fn refused(gm: PlayerIdentity, entity_id: u32, org_type: Option<OrgType>, reason: &'static str) {
    tracing::info!(
        target: "org",
        event = "org.gm_action",
        action = "gm_org_create",
        outcome = "rejected",
        reason,
        account_id = gm.account_id,
        account_name = gm.account_name,
        player_id = gm.player_id,
        player_name = gm.player_name,
        entity_id,
        entity_name = gm.player_name,
        org_type = org_type.map(OrgType::name),
        "GM organization command rejected",
    );
    count_org_action("gm_org_create", "rejected", reason);
}

/// `.org_create <team|command> <name...>`. `args` has passed the registry's
/// count (at least two); the name is every argument after the type, joined
/// by single spaces, as the base's D-ORG10 normaliser would collapse them.
pub(super) async fn org_create(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let gm = space_mgr.player_identity(caller_id);
    let access_level = space_mgr
        .get_entity(caller_id)
        .map_or(0, |e| e.access_level);
    if !is_gm(access_level) {
        refused(gm, caller_id, None, "not_gm");
        send_gm_feedback(caller_id, ".org_create is a GM command.", tx).await;
        return;
    }
    let Some(org_type) = args.first().copied().and_then(parse_type) else {
        refused(gm, caller_id, None, "usage");
        send_gm_feedback(caller_id, USAGE, tx).await;
        return;
    };
    let name = args[1..].join(" ");
    let Some(player_id) = gm.player_id else {
        refused(gm, caller_id, Some(org_type), "not_ready");
        send_gm_feedback(
            caller_id,
            "Organizations are not available until you have entered the world.",
            tx,
        )
        .await;
        return;
    };
    let msg = CellToBaseMsg::Org(OrgCellToBase::GmCreate {
        player_id,
        entity_id: caller_id,
        org_type,
        name,
    });
    if tx.send(msg).await.is_err() {
        refused(gm, caller_id, Some(org_type), "base_unreachable");
        return;
    }
    tracing::debug!(
        target: "org",
        event = "org.gm_create_forwarded",
        account_id = gm.account_id,
        account_name = gm.account_name,
        player_id,
        player_name = gm.player_name,
        entity_id = caller_id,
        entity_name = gm.player_name,
        org_type = org_type.name(),
        "GM organization creation forwarded to the base",
    );
}
