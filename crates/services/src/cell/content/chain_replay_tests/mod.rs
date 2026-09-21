//! Live-DB chain-replay regression guards.
//!
//! Each test loads a specific seeded `content_*` chain from the database
//! through the same `build_chains_from_rows` pipeline that the cell service
//! uses at startup, registers it in a fresh `ChainEngine`, and fires
//! synthetic events through `resolve_event` to assert the chain matches
//! (or doesn't) under specific `ExecutionContext` shapes.
//!
//! Loading from DB rather than hand-constructing the chain in Rust is
//! deliberate: the whole point is to catch silent drift in the SQL seed
//! (e.g. someone removes a `mission_status` condition that was added to fix
//! a bug). A pure-Rust replica would let that drift pass.
//!
//! Skip cleanly when DATABASE_URL is unset.
//!
//! Files are organised by mission family — each sibling module pins the
//! chains for one mission's branches and edges so a regression surfaces
//! near the seed it touches. Four modules are organised by *action verb*
//! instead, because the risk they guard is the executor arm rather than
//! any one mission's wiring: [`sgc_w1_move_entity`], [`grant_xp`],
//! [`npc_bark`], [`livewire_pairs`] and [`castle_702_704_executor`].
//! Those five also run
//! the resolved actions through
//! `executor::execute_actions` and assert on the resulting
//! `CellToBaseMsg` traffic — a resolve-only test cannot tell a wired
//! executor arm from the `other =>` catch-all.

mod castle_702_704_executor;
mod entity_health_below;
mod gc1_escort;
mod grant_xp;
mod harset_space;
mod harset_spawn_entity;
mod livewire_pairs;
mod mission_1200;
mod mission_1324;
mod mission_1326;
mod mission_1360;
mod mission_1360_harset;
mod mission_1361;
mod mission_1562;
mod mission_567;
mod mission_622;
mod mission_638;
mod mission_639;
mod mission_639_cover;
mod mission_640;
mod mission_641;
mod mission_680;
mod mission_681_686;
mod mission_681_686_flank;
mod mission_686_straegis;
mod mission_687;
mod mission_688;
mod mission_689;
mod mission_701;
mod mission_702;
mod mission_703;
mod mission_704;
mod mission_704_escort;
mod mission_704_restores;
mod mission_706;
mod mission_708;
mod mission_742;
mod mission_abandoned;
mod mission_relog_persistence;
mod npc_bark;
mod region8_guard_aggro;
mod region_transition_accepts;
mod sgc_w1_move_entity;
mod stargate_triggers;
mod start_minigame_difficulty;
mod world_condition;

/// Assert every action `chain_id` resolved is immediate (`delay_ms = 0`).
///
/// `execute_actions` QUEUES rather than runs any action with a non-zero
/// delay (`executor/mod.rs:96-120`). For a chain whose action list mixes a
/// state change with the step gate that closes it, that is a correctness
/// hole rather than a timing detail: a deferred `advance_step` leaves the
/// old step active across the delay window, and every event arriving
/// inside that window passes the gate again. Mission 708's Control
/// Crystal is the worked example — a deferred advance turns a
/// grant-exactly-once into a faucet.
///
/// Lives here rather than per mission module because the reasoning is not
/// mission-specific and two copies of it had already started to drift
/// apart in their panic text. Asserted per chain rather than over the
/// whole `ResolvedActions` so the failure names the offending chain.
#[cfg(test)]
fn assert_no_deferred_actions(
    resolved: &cimmeria_content_engine::chain::ResolvedActions,
    chain_id: i64,
) {
    for (i, (id, action)) in resolved.actions.iter().enumerate() {
        if *id != chain_id {
            continue;
        }
        let delay = resolved.action_delays.get(i).copied().unwrap_or(0);
        assert_eq!(
            delay, 0,
            "chain {chain_id} action {action:?} carries delay_ms = {delay}; \
             a deferred action is queued rather than run, which breaks the \
             step gate it shares an action list with",
        );
    }
}
