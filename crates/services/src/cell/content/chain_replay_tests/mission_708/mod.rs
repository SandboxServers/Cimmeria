//! Mission 708 — "Secure the Stargate" (Castle rebuild packet CA09).
//! Pins chains 1341-1365 in
//! `db/resources/Content/Seed/castle_706_708_chains.sql`.
//!
//! Split by step family rather than kept as one file: 708 has six steps,
//! two archetype splits, five kill sources and five restore chains, and
//! a single module would blow past the 700-line hard cap.
//!
//! - [`diagnosis`] — step 2415, the two mutually-alternative routes
//!   (chains 1341-1345).
//! - [`crystal`] — step 2416, the Control Crystal grant and its
//!   single-grant guard (chains 1346-1351).
//! - [`report`] — step 2417, the archetype-split report to Checkpoint
//!   Alpha (chains 1352-1355).
//! - [`stargate`] — steps 2418 / 4462 / 4469, the DHD minigame, the dial
//!   and the crossing (chains 1356-1360).
//! - [`restore`] — the `player_loaded Castle` repaints (chains
//!   1361-1365).
//!
//! Two of these go past `resolve_event` and push the resolved actions
//! through `execute_actions`, asserting the `CellToBaseMsg` traffic
//! (TESTING.md type 6, PR #618's extension): the crystal grant
//! (`Action::GrantItem` → `CellToBaseMsg::GrantItem`) and the DHD
//! minigame launcher (`Action::StartMinigame` →
//! `CellToBaseMsg::StartMinigame`). Those are the two verbs in this
//! mission whose failure mode is a silently-dropped executor arm rather
//! than a mis-authored condition.

mod crystal;
mod diagnosis;
mod report;
mod restore;
mod stargate;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::{load_chain_expansions_for_test, load_single_chain_for_test};

/// The "!" main-story-active cue.
const BANG: i64 = cimmeria_entity::interaction_flags::INT_A_STORY_MISSION_ACTIVE;
/// Quest-object glow.
const GLOW: i64 = cimmeria_entity::interaction_flags::INT_MISSION_WORLD_OBJECT;
/// Hackable-console cue on the DHD.
const LIVEWIRE: i64 = cimmeria_entity::interaction_flags::INT_MINIGAME_LIVEWIRE;

/// Jaffa. Every other starting archetype takes the Tau'ri branch, which
/// is why the seed writes `neq 8` rather than enumerating the others.
const JAFFA: i32 = 8;
/// A concrete non-Jaffa archetype for the negative half of each split.
const TAURI: i32 = 1;

/// Load one seeded chain and register it in a fresh engine.
async fn engine_for(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains — a None here \
                 means the row is missing from castle_706_708_chains.sql or its \
                 trigger/action rows failed to convert"
            )
        });
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Load a MULTI-TRIGGER chain and register every expansion. The loader
/// materializes one in-memory `Chain` per trigger row
/// (`loader/mod.rs:170-205`), and `load_single_chain_for_test` returns
/// only the first — registering just that one would silently mask drift
/// in trigger rows 2..N. Chains 1350/1351 each carry four
/// `entity_dead_tag` rows.
async fn engine_for_all_expansions(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chains = load_chain_expansions_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"));
    assert!(
        !chains.is_empty(),
        "chain {chain_id} must exist in seeded content_chains with at least one \
         convertible trigger row"
    );
    let mut engine = ChainEngine::new();
    for chain in chains {
        engine.register_chain(chain);
    }
    engine
}

fn fire(
    engine: &ChainEngine,
    trigger_type: TriggerType,
    ctx: &ExecutionContext,
) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

fn actions_of(resolved: &ResolvedActions, chain_id: i64) -> Vec<&Action> {
    resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == chain_id { Some(a) } else { None })
        .collect()
}

/// Assert every resolved action for `chain_id` is immediate.
///
/// This is the guard the server-authority review made a release
/// condition for the crystal grant: `execute_actions` QUEUES rather than
/// runs any action with `delay_ms > 0` (`executor/mod.rs:96-120`), so a
/// deferred `advance_step` would leave step 2416 active across the delay
/// window and every officer killed inside it would grant another
/// crystal. Applied to all of 708's chains, not just the crystal ones —
/// the same reasoning covers every step gate in the mission.
fn assert_no_deferred_actions(resolved: &ResolvedActions, chain_id: i64) {
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

/// Context for a step-gated 708 event: the named step active, the
/// mission active, and an archetype (populated by every dispatcher this
/// mission uses EXCEPT `fire_dialog_choice` — see `dialog_ctx`).
fn step_ctx(step_id: i32, archetype: i32) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "mission_708_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        format!("mission_708_step_{step_id}_status"),
        serde_json::json!("active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Context for a `dialog_choice` event, faithfully reproducing what
/// `fire_dialog_choice` actually populates: `dialog_id`, `button_id` and
/// mission state — and **no `archetype`**
/// (`event_dispatch/dialog.rs:87-93`).
///
/// `button_id` is -1 because none of 708's dialogs has a
/// `dialog_screen_buttons` row: the client emits its choice from the
/// close path with `ButtonId = 0xFFFFFFFF` when a dialog's total button
/// count is zero. Nothing in the seed branches on `button_id`, but
/// writing the real value keeps the fixture honest.
fn dialog_ctx(dialog_id: i32, step_id: i32) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx.set_param("button_id".to_string(), serde_json::json!(-1));
    ctx.set_param(
        "mission_708_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        format!("mission_708_step_{step_id}_status"),
        serde_json::json!("active"),
    );
    ctx
}

/// A one-space `SpaceManager` named for the world these chains run in.
/// Bounds are wide enough to hold Checkpoint Alpha (~810, 55, 515) and
/// the Throne Room (~331-396 x, 617-683 z) so the spatial grid accepts
/// a staged entity anywhere the mission actually happens.
fn make_castle_space_mgr() -> crate::cell::space_manager::SpaceManager {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1200" MinY="0" MaxY="1200" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Count `SetInteractionType` actions matching a tag/op/mask triple.
fn count_flag_ops(actions: &[&Action], tag: &str, op: &str, expected_mask: i64) -> usize {
    actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::SetInteractionType { entity_tag, operation, mask }
                    if entity_tag == tag && operation == op && *mask == expected_mask
            )
        })
        .count()
}
