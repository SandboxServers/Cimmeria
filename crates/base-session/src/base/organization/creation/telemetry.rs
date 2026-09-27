//! Base-side creation telemetry, on the `org` target (organizations
//! campaign telemetry rule; `docs/architecture/observability.md`, `org` /
//! `squad` row).
//!
//! One INFO outcome row per action: `event` = `org.registrar_open`,
//! `org.create` or `org.gm_action`, `outcome` (`ok` / `rejected`), a closed
//! `reason` on a refusal, and the actor's `account_id` / `player_id` as
//! `Option`s from the session (never 0 for "unknown"). Each row counts on
//! `org_actions_total{action, outcome, reason}`. The cell writes the rows
//! for the refusals it decides itself (`cimmeria-cell-methods`,
//! `organization::creation`), on the same series.

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::OrgType;

/// Count one Team or Command action on `org_actions_total`. Every label is
/// from a closed set (the action, `ok` / `rejected`, the refusal reason or
/// `none`); never an id.
pub fn count_org_action(action: &'static str, outcome: &'static str, reason: &'static str) {
    cimmeria_observability::counter!(
        "org_actions_total",
        "action" => action,
        "outcome" => outcome,
        "reason" => reason,
    );
}

/// The action a row reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    /// A registrar right-click reached the base's eligibility check.
    RegistrarOpen,
    /// A named creation (cell method 94) reached the base.
    Create,
    /// GM `.org_create`.
    GmCreate,
}

impl Action {
    fn event(self) -> &'static str {
        match self {
            Action::RegistrarOpen => "org.registrar_open",
            Action::Create => "org.create",
            Action::GmCreate => "org.gm_action",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Action::RegistrarOpen => "registrar_open",
            Action::Create => "create",
            Action::GmCreate => "gm_org_create",
        }
    }
}

/// One outcome row, before it is emitted.
#[derive(Debug)]
pub(super) struct Outcome {
    pub action: Action,
    pub actor: PlayerIdentity,
    pub entity_id: u32,
    pub org_type: OrgType,
    pub npc_entity_id: Option<u32>,
    pub name_units: Option<usize>,
    pub org_id: Option<i32>,
    /// The founder's naquadah before and after, only when a cost was paid.
    pub cash: Option<(i32, i32)>,
}

impl Outcome {
    pub(super) fn new(
        action: Action,
        actor: PlayerIdentity,
        entity_id: u32,
        org_type: OrgType,
    ) -> Self {
        Self {
            action,
            actor,
            entity_id,
            org_type,
            npc_entity_id: None,
            name_units: None,
            org_id: None,
            cash: None,
        }
    }

    pub(super) fn ok(&self) {
        self.emit(None);
    }

    pub(super) fn rejected(&self, reason: &'static str) {
        self.emit(Some(reason));
    }

    fn emit(&self, reason: Option<&'static str>) {
        let outcome = if reason.is_some() { "rejected" } else { "ok" };
        let action = (self.action == Action::GmCreate).then_some(self.action.label());
        tracing::info!(
            target: "org",
            event = self.action.event(),
            action,
            outcome,
            reason,
            account_id = self.actor.account_id,
            player_id = self.actor.player_id,
            entity_id = self.entity_id,
            org_type = self.org_type.name(),
            npc_entity_id = self.npc_entity_id,
            name_units = self.name_units,
            org_id = self.org_id,
            cost_before = self.cash.map(|c| c.0),
            cost_after = self.cash.map(|c| c.1),
            "organization {} {}",
            self.action.label(),
            outcome
        );
        count_org_action(self.action.label(), outcome, reason.unwrap_or("none"));
    }
}
