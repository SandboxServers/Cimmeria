//! Counter action handlers: `IncrementCounter` and `ResetCounter`.
//!
//! Counter values live on the cell entity and are read back by
//! `Condition::Counter` via `populate_mission_context` writing
//! `counter_<name>` into the chain context.

use crate::cell::space_manager::SpaceManager;

/// `Action::IncrementCounter` — bump the named counter by `amount`,
/// initializing missing entries at 0.
///
/// Resolve/execute ordering gotcha: chains are resolved (conditions
/// evaluated against ctx) before any actions run, so a sibling
/// completion chain on the same trigger event reads the PRE-increment
/// counter value. For "kill N targets to complete" the completion
/// condition must be `counter >= N - 1` so it fires on the kill that
/// brings the counter to N. Documented at each call site that uses this
/// pattern (e.g., castle_cellblock_chains.sql chain 1087 / 1094).
pub(super) fn increment(
    counter_name: String,
    amount: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        let entry = entity.counters.entry(counter_name.clone()).or_insert(0);
        *entry = entry.saturating_add(amount);
        let new_value = *entry;
        tracing::debug!(
            entity_id,
            entity_name = crate::cell::space_manager::EntityNames::of(entity).entity_name,
            player_id,
            player_name = entity.identity().player_name,
            %counter_name,
            amount,
            new_value,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "Content: incremented counter"
        );
    } else {
        tracing::warn!(
            entity_id, // nt:id-only the entity is gone, so there is no name to resolve
            %counter_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "Content: increment_counter source entity missing — counter not updated"
        );
    }
}

/// `Action::ResetCounter` — drop the entry entirely so subsequent
/// `Condition::Counter` reads see the missing-key default of 0 rather
/// than a residual value.
pub(super) fn reset(
    counter_name: String,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        let removed = entity.counters.remove(&counter_name);
        tracing::debug!(
            entity_id,
            entity_name = crate::cell::space_manager::EntityNames::of(entity).entity_name,
            player_id,
            player_name = entity.identity().player_name,
            %counter_name,
            ?removed,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            "Content: reset counter"
        );
    }
}
