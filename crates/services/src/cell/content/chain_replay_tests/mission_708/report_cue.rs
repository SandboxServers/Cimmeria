//! Step 2416's report cue — chains 1350/1351.
//!
//! Split out of [`super::crystal`] (which owns the grant chains
//! 1346-1349) when that file crossed the 500-line soft cap. The seam is
//! real rather than arithmetic: the grant chains are a
//! server-authority problem (exactly one item, ever) while these two are
//! a presentation problem (which NPC lights up, for whom).
//!
//! Whichever corpse yielded the crystal, exactly one report NPC is
//! marked and the KILLER's archetype chooses it. `entity_dead_tag`
//! populates `archetype` from the killer (`lifecycle.rs:78-83`), so
//! unlike on a `dialog_choice` chain the gate here is real.
//!
//! What these chains do NOT provide is per-player cueing. The bit they
//! set is written to the shared `CellEntity` and broadcast to every
//! witness (seed engine fact 7), so a Jaffa's kill lights Moh'katan for
//! nearby Tau'ri too. The faction split is enforced one step later, on
//! the DIALOG (chains 1352/1354) — see
//! [`super::report::the_report_dialogs_are_archetype_exclusive`]. These
//! tests therefore assert RESOLVE-level exclusivity, which is the
//! property that actually holds.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::triggers::TriggerType;

use super::super::super::engine_loader::{
    load_chain_expansions_for_test, load_single_chain_for_test,
};
use super::{
    actions_of, assert_no_deferred_actions, count_flag_ops, engine_for_all_expansions, fire,
    step_ctx, BANG, JAFFA, SOURCES, TAURI,
};
use crate::test_support::require_db_or_skip;

/// Exactly one report NPC is marked per death, chosen by the killer's
/// archetype.
///
/// Both chains carry four `entity_dead_tag` trigger rows, so every
/// expansion is registered — a regression that dropped rows 2..4 would
/// leave a player who killed Muelbach with no cue at all.
#[tokio::test]
async fn crystal_death_marks_exactly_one_report_npc_by_archetype() {
    let pool = require_db_or_skip!();
    let marsh = engine_for_all_expansions(&pool, 1350).await;
    let mohkatan = engine_for_all_expansions(&pool, 1351).await;

    for (_, tag, _) in SOURCES {
        let mut tauri = step_ctx(2416, TAURI);
        tauri.set_param("entity_tag".to_string(), serde_json::json!(tag));
        let marsh_resolved = fire(&marsh, TriggerType::EntityDeath, &tauri);
        assert_no_deferred_actions(&marsh_resolved, 1350);
        let marsh_actions = actions_of(&marsh_resolved, 1350);
        assert_eq!(
            count_flag_ops(&marsh_actions, "Castle_ColMarsh", "|", BANG),
            1,
            "a Tau'ri killing {tag} must get exactly one '!' on Col. Marsh; \
             got {marsh_actions:?}",
        );
        assert!(
            actions_of(&fire(&mohkatan, TriggerType::EntityDeath, &tauri), 1351).is_empty(),
            "a Tau'ri's kill must not RESOLVE the Moh'katan cue chain (the bit \
             chain 1351 would set is zone-wide, so this is about which chain \
             fires, not about who can see the icon)",
        );

        let mut jaffa = step_ctx(2416, JAFFA);
        jaffa.set_param("entity_tag".to_string(), serde_json::json!(tag));
        let moh_resolved = fire(&mohkatan, TriggerType::EntityDeath, &jaffa);
        assert_no_deferred_actions(&moh_resolved, 1351);
        let moh_actions = actions_of(&moh_resolved, 1351);
        assert_eq!(
            count_flag_ops(&moh_actions, "Castle_Mohkatan", "|", BANG),
            1,
            "a Jaffa killing {tag} must get exactly one '!' on Moh'katan; \
             got {moh_actions:?}",
        );
        assert!(
            actions_of(&fire(&marsh, TriggerType::EntityDeath, &jaffa), 1350).is_empty(),
            "a Jaffa's kill must not RESOLVE the Col. Marsh cue chain",
        );
    }
}

/// The cue chains must be step-gated too: a later kill (the officers
/// respawn) must not re-light a report NPC the player has already
/// reported to.
#[tokio::test]
async fn report_cue_chains_do_not_fire_outside_step_2416() {
    let pool = require_db_or_skip!();
    let marsh = engine_for_all_expansions(&pool, 1350).await;

    for step in [2415, 2417, 2418, 4469] {
        let mut ctx = step_ctx(step, TAURI);
        ctx.set_param(
            "entity_tag".to_string(),
            serde_json::json!("Castle_BravoOfficer1"),
        );
        assert!(
            actions_of(&fire(&marsh, TriggerType::EntityDeath, &ctx), 1350).is_empty(),
            "chain 1350 must not re-light Col. Marsh on a kill at step {step}",
        );
    }
}

/// Seed engine fact 3, pinned: the grant chain and its report cue must
/// resolve out of ONE event, in one batch.
///
/// `ChainEngine::resolve_event` evaluates every chain's conditions
/// against a single pre-action `ExecutionContext` snapshot and then
/// concatenates the matched action lists (`chain/mod.rs:288-325`). Chain
/// 1346 advances the player out of step 2416; chain 1350's
/// `step_status 708 2416 eq active` gate is evaluated against the
/// snapshot, so it still passes.
///
/// If the engine ever re-evaluated conditions per chain, or interleaved
/// resolution with execution, 1346 would advance first and 1350 would
/// find step 2417 — the killer would reach step 2417 with NO cue on
/// either report NPC. That is not cosmetic: templates 10 and 54 ship
/// `interaction_type = 0` (`entity_templates.sql:25/63`), so with the
/// cue bit never set the report NPC has flags 0 and the client offers no
/// right-click at all. The mission would hard-stall at 2417 with no
/// recovery but a relog into restore chain 1362. That combination makes
/// this the highest-consequence path in mission 708.
///
/// Asserted at resolve level deliberately: the fixture does not spawn
/// Marsh, so `execute_actions` would only log a tag-miss for the
/// `set_interaction_type`. The property under test is which chains land
/// in one `ResolvedActions`, which is exactly what `resolve_event`
/// decides.
#[tokio::test]
async fn the_crystal_grant_and_its_report_cue_resolve_in_one_batch() {
    let pool = require_db_or_skip!();

    // One engine holding the grant chain AND every expansion of the cue
    // chain — the whole point is that they are resolved together.
    let mut engine = ChainEngine::new();
    let grant = load_single_chain_for_test(&pool, 1346)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain 1346 must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain 1346 must exist in seeded content_chains"));
    engine.register_chain(grant);
    let cues = load_chain_expansions_for_test(&pool, 1350)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain 1350 must succeed: {e}"));
    assert!(
        !cues.is_empty(),
        "chain 1350 must expand to at least one registrable chain",
    );
    for cue in cues {
        engine.register_chain(cue);
    }

    let mut ctx = step_ctx(2416, TAURI);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_BravoOfficer1"),
    );

    let resolved = fire(&engine, TriggerType::EntityDeath, &ctx);

    // Assert against the COMBINED list. Filtering per chain first would
    // lose the point: what matters is that both chains contributed to the
    // same resolution.
    let all: Vec<&Action> = resolved.actions.iter().map(|(_, a)| a).collect();

    assert!(
        all.iter().any(|a| matches!(
            a,
            Action::GrantItem {
                item_id: 2790,
                count: 1,
                ..
            }
        )),
        "the batch must carry chain 1346's Control Crystal grant; got {all:?}",
    );
    assert!(
        all.iter().any(|a| matches!(
            a,
            Action::AdvanceStep {
                mission_id: 708,
                step_id: 2417
            }
        )),
        "the batch must carry chain 1346's advance to 2417; got {all:?}",
    );
    assert_eq!(
        count_flag_ops(&all, "Castle_ColMarsh", "|", BANG),
        1,
        "the batch must ALSO carry chain 1350's '!' on Col. Marsh, even though \
         chain 1346 advances out of step 2416 in the same batch — one context \
         snapshot, conditions evaluated before any action runs. Zero here means \
         the engine started re-evaluating per chain and the killer now reaches \
         step 2417 with an unclickable report NPC; got {all:?}",
    );

    // And the other faction's cue must not ride along.
    assert_eq!(
        count_flag_ops(&all, "Castle_Mohkatan", "|", BANG),
        0,
        "a Tau'ri's kill must not also resolve the Moh'katan cue; got {all:?}",
    );
}
