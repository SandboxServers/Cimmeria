//! The one telemetry row each AB-N2 ability-testing GM command writes
//! (`gmGiveAbility`, `gmSetGodMode`, `gmResetAbilities`,
//! `gmGiveAllAbilities`, `gmSetMobAbilitySet`), and each AB-N1 combat-debug
//! toggle (`gmDebugAbility`, `gmDebugCombat`, `gmDebugCombatVerbose`,
//! `gmDebugHeal`, `gmDebugAbilityOnMob`).
//!
//! Target `abilities`, `event = "gm_command"`, with the caller's
//! `account_id` / `player_id` (instrumentation-discipline Rule 5), the
//! command, its decoded `args`, and `decision_outcome`: `applied` (the cell
//! changed state), `forwarded` (the base writes and answers), or `refused`
//! with a `reason`. A refusal is DEBUG, since a GM can trigger every one at
//! will; the other outcomes are INFO. Every refusal also answers the GM with
//! a feedback line, so no press goes unanswered.

use crate::cell::space_manager::SpaceManager;

/// How a GM command ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    /// The cell changed state.
    Applied,
    /// The base persists it and answers.
    Forwarded,
    /// Nothing changed, for `reason`.
    Refused(&'static str),
}

/// Log the command's one row.
pub(super) fn log_gm_command(
    space_mgr: &SpaceManager,
    entity_id: u32,
    cmd: &'static str,
    method_index: u16,
    args: &str,
    outcome: Outcome,
) {
    let id = space_mgr.player_identity(entity_id);
    match outcome {
        Outcome::Refused(reason) => tracing::debug!(
            target: "abilities",
            event = "gm_command",
            cmd,
            method_index,
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            args,
            decision_outcome = "refused",
            reason,
            "GM ability command refused"
        ),
        Outcome::Applied | Outcome::Forwarded => tracing::info!(
            target: "abilities",
            event = "gm_command",
            cmd,
            method_index,
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            args,
            decision_outcome = if outcome == Outcome::Applied {
                "applied"
            } else {
                "forwarded"
            },
            "GM ability command"
        ),
    }
}
