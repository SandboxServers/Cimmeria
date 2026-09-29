//! Cell-side creation telemetry, on the `org` target: the outcome rows for
//! the refusals and successes the cell decides, and the pending-creation
//! transitions. The base writes the rows it decides (eligibility, the
//! database refusals, a created organization) on the same events and the
//! same `org_actions_total` series, so each action has exactly one row.

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::OrgType;

use crate::cell::org_creation::{count_org_action, Pending};

/// The two actions whose rows the cell writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The registrar dialog opened (after the base found the player
    /// eligible), or could not be.
    RegistrarOpen,
    /// A name (cell method 94) refused before it reached the base.
    Create,
}

impl Action {
    fn event(self) -> &'static str {
        match self {
            Action::RegistrarOpen => "org.registrar_open",
            Action::Create => "org.create",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Action::RegistrarOpen => "registrar_open",
            Action::Create => "create",
        }
    }
}

/// One outcome row.
#[derive(Debug)]
pub struct Outcome {
    pub action: Action,
    pub actor: PlayerIdentity,
    pub entity_id: u32,
    pub org_type: Option<OrgType>,
    pub npc_entity_id: Option<u32>,
    pub name_units: Option<usize>,
    pub attempts_left: Option<u8>,
    /// The D-ORG10 rule a refused name broke.
    pub text_reason: Option<&'static str>,
}

impl Outcome {
    pub fn new(action: Action, actor: PlayerIdentity, entity_id: u32) -> Self {
        Self {
            action,
            actor,
            entity_id,
            org_type: None,
            npc_entity_id: None,
            name_units: None,
            attempts_left: None,
            text_reason: None,
        }
    }

    pub fn ok(&self) {
        self.emit(None);
    }

    pub fn rejected(&self, reason: &'static str) {
        self.emit(Some(reason));
    }

    fn emit(&self, reason: Option<&'static str>) {
        let outcome = if reason.is_some() { "rejected" } else { "ok" };
        tracing::info!(
            target: "org",
            event = self.action.event(),
            outcome,
            reason,
            account_id = self.actor.account_id,
            player_id = self.actor.player_id,
            entity_id = self.entity_id,
            org_type = self.org_type.map(OrgType::name),
            npc_entity_id = self.npc_entity_id,
            name_units = self.name_units,
            attempts_left = self.attempts_left,
            text_reason = self.text_reason,
            "organization {} {}",
            self.action.label(),
            outcome
        );
        count_org_action(self.action.label(), outcome, reason.unwrap_or("none"));
    }
}

/// `pending_creation_created`: a registrar offer was recorded (or an open
/// one re-pointed, `refreshed`).
pub fn pending_created(actor: PlayerIdentity, p: &Pending, refreshed: bool) {
    tracing::debug!(
        target: "org",
        event = "pending_creation_created",
        account_id = actor.account_id,
        player_id = actor.player_id,
        org_type = p.org_type.name(),
        npc_entity_id = p.npc_entity_id,
        space_id = p.space_id,
        attempts_left = p.attempts_left,
        refreshed,
        "organization creation offer open"
    );
}

/// `pending_creation_consumed`: the organization was created.
pub fn pending_consumed(actor: PlayerIdentity, p: &Pending) {
    tracing::debug!(
        target: "org",
        event = "pending_creation_consumed",
        account_id = actor.account_id,
        player_id = actor.player_id,
        org_type = p.org_type.name(),
        npc_entity_id = p.npc_entity_id,
        attempts_left = p.attempts_left,
        "organization creation offer closed by a creation"
    );
}

/// `pending_creation_expired`: the offer ended without a creation.
/// `cause` is `ttl`, `space_changed` or `disconnect`.
pub fn pending_expired(actor: PlayerIdentity, p: &Pending, cause: &'static str) {
    tracing::debug!(
        target: "org",
        event = "pending_creation_expired",
        account_id = actor.account_id,
        player_id = actor.player_id,
        org_type = p.org_type.name(),
        npc_entity_id = p.npc_entity_id,
        attempts_left = p.attempts_left,
        cause,
        "organization creation offer ended"
    );
}

/// `pending_creation_attempt_charged`: a name was refused; one attempt
/// spent.
pub fn attempt_charged(actor: PlayerIdentity, attempts_left: Option<u8>, by: &'static str) {
    tracing::debug!(
        target: "org",
        event = "pending_creation_attempt_charged",
        account_id = actor.account_id,
        player_id = actor.player_id,
        attempts_left,
        by,
        "organization creation attempt spent"
    );
}
