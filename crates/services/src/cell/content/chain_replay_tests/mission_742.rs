//! Mission 742 "Giving the Walls Ears" — chains 6101-6119
//! (`harset_goauld_chains.sql`, Harset packet H41).
//!
//! 742 is the only Harset mission whose 2009 script survives, so unlike
//! the rest of the campaign these chains are a port and the regression
//! surface is *fidelity* as much as correctness. The guards below are
//! grouped by the thing that would silently break:
//!
//! 1. **Step plumbing.** Each beat resolves its own action list, in
//!    order, and stops resolving once its gate closes. `sort_order` is
//!    asserted as a sequence rather than a set wherever the 2009 graph
//!    had an ordering (the grant before the unbind, the unbind before
//!    the completion).
//! 2. **The three-basket race.** The heart of the packet. Step 2504 has
//!    three non-optional objectives, and
//!    `cell::missions::complete_objective` ends the whole mission the
//!    moment the last required objective of the current step completes
//!    (`progression.rs:176-183`). The port therefore splits each basket
//!    into a priority-0 "partial" chain and a priority-10 "final" chain,
//!    and relies on the final chain's `advance_step` running FIRST. That
//!    ordering is asserted directly, for each of the three baskets in
//!    turn, because whichever basket the player clicks last is the one
//!    that has to carry the advance.
//! 3. **Item conservation.** Three Scarabs granted, exactly one consumed
//!    per basket including the third, and the Jaffa Disguise never
//!    consumed at all. The disguise assertion is a *reversion* guard: it
//!    fails if anyone later "helpfully" adds a `remove_item 2819`.
//! 4. **The offer** (D-H20) and its archetype / 1200-completion gates,
//!    including the `available_interactions[43]` disjointness argument
//!    that gate exists to guarantee.
//!
//! **Pending H50.** Every `objective_status`-gated assertion below seeds
//! the objective params by hand, which is what a *live* dispatch does
//! only within one session: objective ids never reach the database
//! (seed-file header, "KNOWN BLOCKER"), so after a relog the production
//! context carries none of them. `objective_params_do_not_survive_the_
//! production_hydration_shape` pins that defect deliberately, and is the
//! test that must be inverted when H50 lands. The basket relog-restore
//! acceptance for step 2504 is NOT claimed by this module.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::{build_engine, load_single_chain_for_test};
use crate::test_support::require_db_or_skip;

const HARSET: i32 = 57;
const CMD_CENTER: i32 = 68;

/// `(tag, own objective, other objective, other objective)` for the
/// three listening-device baskets. The pairing is the whole point of
/// chains 6107-6109: each "final" chain names its own objective plus
/// the two it must see already completed.
const BASKETS: [(&str, i32, i32, i32); 3] = [
    ("FirstBug", 2913, 2914, 2915),
    ("SecondBug", 2914, 2913, 2915),
    ("ThirdBug", 2915, 2913, 2914),
];

/// Load one chain by id into an otherwise-empty engine.
async fn engine_with(pool: &sqlx::PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn fire(engine: &ChainEngine, trigger_type: TriggerType, ctx: &ExecutionContext) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// Actions contributed by one chain, in resolved order.
fn actions_of(resolved: &ResolvedActions, chain_id: i64) -> Vec<Action> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, a)| a.clone())
        .collect()
}

fn ctx_with_step(step_id: i32, status: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        format!("mission_742_step_{step_id}_status"),
        serde_json::json!(status),
    );
    ctx
}

/// A player standing in `world` mid-way through planting devices:
/// step 2504 active, with each objective's status given explicitly.
fn ctx_planting(world: i32, tag: &str, objectives: &[(i32, &str)]) -> ExecutionContext {
    let mut ctx = ctx_with_step(2504, "active");
    ctx.world_id = Some(world);
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    for (oid, status) in objectives {
        ctx.set_param(
            format!("mission_742_obj_{oid}_status"),
            serde_json::json!(*status),
        );
    }
    ctx
}

// ── 1. Accept and the disguise ──────────────────────────────────────

/// Chain 6101 is the `.script`'s node 1 "Started" port: bind Petbe's
/// topic (dsm 3129 on template 163) and grant three Scarabs.
///
/// The `qty: 3` is asserted explicitly. `add_item`'s loader defaults a
/// missing `qty` to 1 (`loader/action.rs:46`), so dropping the param
/// would silently grant one device for a three-device objective and
/// strand the player on the third basket with an empty bag.
#[tokio::test]
async fn chain_6101_binds_petbe_and_grants_exactly_three_scarabs() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6101).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(742));
    ctx.set_param(
        "mission_742_status".to_string(),
        serde_json::json!("active"),
    );

    let resolved = fire(&engine, TriggerType::MissionAccepted, &ctx);
    let actions = actions_of(&resolved, 6101);

    assert_eq!(
        actions.len(),
        2,
        "chain 6101 must resolve exactly the bind and the grant; got {actions:?}"
    );
    assert!(
        matches!(
            &actions[0],
            Action::AddDialogSet { dialog_set_id: 3129, slot: 163, .. }
        ),
        "first action must bind dsm 3129 to Petbe's template 163 — `slot` is \
         the ENTITY TEMPLATE id, not a UI slot; got {:?}",
        actions[0]
    );
    assert!(
        matches!(
            actions[1],
            Action::GrantItem {
                item_id: 2820,
                count: 3,
                ..
            }
        ),
        "second action must grant Scarab 2820 x3 (`.script` node 52, \
         Quantity=3) — a missing `qty` param silently defaults to 1; got {:?}",
        actions[1]
    );
}

/// Adjacent negative: the bind/grant must not resolve for a mission that
/// is not active. Guards the campaign rule that every grant chain gates
/// on mission state rather than on the dead `content_triggers.once`.
#[tokio::test]
async fn chain_6101_does_not_resolve_when_742_is_not_active() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6101).await;

    for status in ["not_active", "completed"] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("mission_id".to_string(), serde_json::json!(742));
        ctx.set_param(
            "mission_742_status".to_string(),
            serde_json::json!(status),
        );

        let resolved = fire(&engine, TriggerType::MissionAccepted, &ctx);
        assert!(
            actions_of(&resolved, 6101).is_empty(),
            "chain 6101 must resolve nothing for mission_742_status = {status}"
        );
    }
}

/// Chain 6102 — `.script` node 8 -> 9 -> {10, 11}. Order is asserted as
/// a sequence because the 2009 graph hung the unbind and the advance off
/// the grant's "Added" success port: a reader of the seed should see the
/// same order the designer drew.
#[tokio::test]
async fn chain_6102_grants_the_disguise_then_unbinds_petbe_then_advances() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6102).await;

    let ctx = ctx_with_step(2502, "active");
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    let actions = actions_of(&resolved, 6102);

    assert_eq!(actions.len(), 3, "chain 6102 resolved {actions:?}");
    assert!(
        matches!(
            actions[0],
            Action::GrantItem {
                item_id: 2819,
                count: 1,
                ..
            }
        ),
        "got {:?}",
        actions[0]
    );
    assert!(
        matches!(
            actions[1],
            Action::RemoveDialogSet {
                dialog_set_id: 3129,
                slot: 163
            }
        ),
        "got {:?}",
        actions[1]
    );
    assert!(
        matches!(
            actions[2],
            Action::AdvanceStep {
                mission_id: 742,
                step_id: 2503
            }
        ),
        "got {:?}",
        actions[2]
    );
}

/// The one-shot guard. `content_triggers.once` is dead code, so the only
/// thing stopping a second trip through Petbe's dialog from handing out
/// a second disguise is the `step_status 2502 eq active` condition that
/// the chain's own `advance_step` closes.
#[tokio::test]
async fn chain_6102_cannot_grant_a_second_disguise_after_it_advances() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6102).await;

    for status in ["completed", "not_active"] {
        let ctx = ctx_with_step(2502, status);
        let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
        assert!(
            actions_of(&resolved, 6102).is_empty(),
            "chain 6102 must not re-resolve with step 2502 {status} — \
             `once` is not enforced, the step gate is the whole guard"
        );
    }
}

/// Chain 6103 — `.script` node 12 -> 13 -> {16, 14}. Node 14's bind of
/// the NULL-dialog indicator dsm 1000000 onto template 164 is the D-H05
/// substitution: three `set_interaction_type` rows carrying the exact
/// mask that dsm row declares.
///
/// The `RemoveItem` assertion is a reversion guard, not a happy path:
/// the Jaffa Disguise is worn for the rest of the sequence and the
/// `.script` has no `Act_RemoveItems` for 2819 anywhere.
#[tokio::test]
async fn chain_6103_lights_all_three_baskets_and_never_consumes_the_disguise() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6103).await;

    let ctx = ctx_with_step(2503, "active");
    let resolved = fire(&engine, TriggerType::ItemUse, &ctx);
    let actions = actions_of(&resolved, 6103);

    assert_eq!(actions.len(), 5, "chain 6103 resolved {actions:?}");
    assert!(matches!(
        actions[0],
        Action::AdvanceStep {
            mission_id: 742,
            step_id: 2504
        }
    ));
    assert!(matches!(actions[1], Action::DisplayDialog { dialog_id: 2637 }));

    for (i, (tag, ..)) in BASKETS.iter().enumerate() {
        match &actions[2 + i] {
            Action::SetInteractionType {
                entity_tag,
                operation,
                mask,
            } => {
                assert_eq!(entity_tag, tag);
                assert_eq!(operation, "|");
                assert_eq!(
                    *mask, 1_073_741_824,
                    "the basket glow must be INT_MissionWorldObject — the same \
                     mask dsm 1000000 carries in `interaction_flags`, which is \
                     where the 2009 data put it"
                );
            }
            other => panic!("expected SetInteractionType for {tag}, got {other:?}"),
        }
    }

    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::RemoveItem { item_id: 2819, .. })),
        "the Jaffa Disguise must NOT be consumed on use — there is no \
         Act_RemoveItems for 2819 anywhere in the 2009 script, and \
         UseInventoryItem no longer auto-consumes. This assertion exists to \
         fail if a `remove_item 2819` row is ever added."
    );
}

/// Adjacent negative for the `item_use` beat: double-clicking the
/// disguise a second time must resolve nothing, or the player gets a
/// second blurb and the glows are re-painted on already-planted baskets.
#[tokio::test]
async fn chain_6103_does_not_refire_once_the_disguise_is_already_on() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6103).await;

    let ctx = ctx_with_step(2503, "completed");
    let resolved = fire(&engine, TriggerType::ItemUse, &ctx);
    assert!(actions_of(&resolved, 6103).is_empty());
}

// ── 2. The three baskets ────────────────────────────────────────────

/// Each basket's partial chain completes its OWN objective, consumes
/// exactly one Scarab, and clears only its own glow — regardless of what
/// the other two baskets have done.
#[tokio::test]
async fn each_basket_completes_its_own_objective_and_consumes_one_scarab() {
    let pool = require_db_or_skip!();

    for (chain_id, (tag, own, other_a, other_b)) in (6104i32..).zip(BASKETS) {
        let engine = engine_with(&pool, chain_id).await;

        // The other two still pending — the first-click case.
        let ctx = ctx_planting(
            HARSET,
            tag,
            &[(own, "active"), (other_a, "active"), (other_b, "active")],
        );
        let actions = actions_of(&fire(&engine, TriggerType::InteractTag, &ctx), chain_id as i64);

        assert_eq!(
            actions.len(),
            3,
            "chain {chain_id} ({tag}) resolved {actions:?}"
        );
        assert!(
            matches!(
                actions[0],
                Action::CompleteObjective {
                    mission_id: 742,
                    objective_id
                } if objective_id == own
            ),
            "chain {chain_id} must complete objective {own}, got {:?}",
            actions[0]
        );
        assert!(
            matches!(
                actions[1],
                Action::RemoveItem {
                    item_id: 2820,
                    count: 1
                }
            ),
            "chain {chain_id} must consume exactly one Scarab. The 2009 build \
             leaked all three because node 30's Player port was unwired \
             (H-B15); the removal node itself IS connected to the counter at \
             `.script` lines 468-469, so three-granted / three-consumed is the \
             recovered intent. Got {:?}",
            actions[1]
        );
        assert!(
            matches!(
                &actions[2],
                Action::SetInteractionType { entity_tag, operation, .. }
                    if entity_tag == tag && operation == "~"
            ),
            "chain {chain_id} must clear only its own glow, got {:?}",
            actions[2]
        );
    }
}

/// The spam-drain guard. Nothing stops a player right-clicking one
/// basket repeatedly — `once` is dead — so the per-basket
/// `objective_status <own> eq active` condition is what prevents a
/// single basket from draining all three Scarabs while the other two
/// stay unplanted. Reverting that condition to the shared
/// `step_status 2504 eq active` alone makes this test fail.
#[tokio::test]
async fn a_basket_whose_objective_is_already_complete_resolves_nothing() {
    let pool = require_db_or_skip!();

    for (chain_id, (tag, own, other_a, other_b)) in (6104i32..).zip(BASKETS) {
        let engine = engine_with(&pool, chain_id).await;
        let ctx = ctx_planting(
            HARSET,
            tag,
            &[(own, "completed"), (other_a, "active"), (other_b, "active")],
        );
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(
            actions_of(&resolved, chain_id as i64).is_empty(),
            "chain {chain_id} ({tag}) re-resolved after its objective completed \
             — one basket could drain every Scarab"
        );
    }
}

/// The basket chains are world-gated because `set_interaction_type`
/// resolves its tag only inside the acting player's own space, and
/// `Condition::World` fails closed on an unset `world_id`.
#[tokio::test]
async fn basket_chains_are_inert_outside_harset() {
    let pool = require_db_or_skip!();

    for (chain_id, (tag, own, other_a, other_b)) in (6104i32..).zip(BASKETS) {
        let engine = engine_with(&pool, chain_id).await;

        for world in [Some(CMD_CENTER), None] {
            let mut ctx = ctx_planting(
                HARSET,
                tag,
                &[(own, "active"), (other_a, "active"), (other_b, "active")],
            );
            ctx.world_id = world;

            let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
            assert!(
                actions_of(&resolved, chain_id as i64).is_empty(),
                "chain {chain_id} ({tag}) resolved with world_id = {world:?}"
            );
        }
    }
}

/// **The ordering guard.** On the last basket — whichever of the three
/// it is — both that basket's chains resolve, and the priority-10 final
/// chain's `advance_step` must come out AHEAD of the priority-0
/// partial's `complete_objective`.
///
/// If the order inverts, `cell::missions::complete_objective` sees every
/// non-optional objective of step 2504 complete and calls
/// `mission.complete()` (`progression.rs:176-183`) — 742 ends three
/// steps early, the player never reports to Anat and never reaches
/// Nerus. Dropping the `priority` column to 0 on 6107-6109 reproduces
/// exactly that, and fails here.
///
/// Both chains are registered into one engine so the assertion runs
/// through the same `register_chain` bucket sort production uses.
#[tokio::test]
async fn the_last_basket_advances_the_step_before_it_completes_the_objective() {
    let pool = require_db_or_skip!();

    for (offset, (tag, own, other_a, other_b)) in BASKETS.into_iter().enumerate() {
        let partial_id = 6104 + offset as i32;
        let final_id = 6107 + offset as i32;

        let mut engine = ChainEngine::new();
        for id in [partial_id, final_id] {
            let chain = load_single_chain_for_test(&pool, id)
                .await
                .expect("DB query must succeed")
                .unwrap_or_else(|| panic!("chain {id} must exist"));
            engine.register_chain(chain);
        }

        let ctx = ctx_planting(
            HARSET,
            tag,
            &[
                (own, "active"),
                (other_a, "completed"),
                (other_b, "completed"),
            ],
        );
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);

        let advance_at = resolved.actions.iter().position(|(id, a)| {
            *id == final_id as i64
                && matches!(
                    a,
                    Action::AdvanceStep {
                        mission_id: 742,
                        step_id: 2505
                    }
                )
        });
        let complete_at = resolved.actions.iter().position(|(id, a)| {
            *id == partial_id as i64
                && matches!(a, Action::CompleteObjective { mission_id: 742, objective_id } if *objective_id == own)
        });

        let advance_at = advance_at.unwrap_or_else(|| {
            panic!("chain {final_id} must resolve AdvanceStep(742, 2505) when {tag} is last; got {:?}", resolved.actions)
        });
        let complete_at = complete_at.unwrap_or_else(|| {
            panic!("chain {partial_id} must still resolve CompleteObjective({own}); got {:?}", resolved.actions)
        });

        assert!(
            advance_at < complete_at,
            "priority inversion on the last basket ({tag}): AdvanceStep at \
             index {advance_at} must precede CompleteObjective at index \
             {complete_at}. With the order reversed, completing the last \
             required objective of step 2504 ends mission 742 outright."
        );

        // The third basket still pays its Scarab.
        assert!(
            resolved.actions.iter().any(|(id, a)| *id == partial_id as i64
                && matches!(
                    a,
                    Action::RemoveItem {
                        item_id: 2820,
                        count: 1
                    }
                )),
            "the last basket must consume its Scarab like the other two"
        );

        // And Anat opens exactly once, not three times.
        let binds = resolved
            .actions
            .iter()
            .filter(|(_, a)| matches!(a, Action::AddDialogSet { dialog_set_id: 3130, .. }))
            .count();
        assert_eq!(binds, 1, "Anat's report topic must bind exactly once");
    }
}

/// Adjacent negative for the final chains: a basket clicked while either
/// of the other two is still pending must NOT advance the step.
#[tokio::test]
async fn a_basket_that_is_not_the_last_one_does_not_advance_the_step() {
    let pool = require_db_or_skip!();

    for (offset, (tag, own, other_a, other_b)) in BASKETS.into_iter().enumerate() {
        let final_id = 6107 + offset as i32;
        let engine = engine_with(&pool, final_id).await;

        for others in [
            [(other_a, "active"), (other_b, "active")],
            [(other_a, "completed"), (other_b, "active")],
            [(other_a, "active"), (other_b, "completed")],
        ] {
            let mut objectives = vec![(own, "active")];
            objectives.extend_from_slice(&others);
            let ctx = ctx_planting(HARSET, tag, &objectives);

            let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
            assert!(
                actions_of(&resolved, final_id as i64).is_empty(),
                "chain {final_id} ({tag}) advanced the step with {others:?} \
                 still outstanding"
            );
        }
    }
}

// ── 3. Report, turn-in, restore ─────────────────────────────────────

/// Chain 6110 — `.script` node 33 -> 34 -> {40, 41, 42}.
#[tokio::test]
async fn chain_6110_anat_grants_the_map_and_opens_nerus() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6110).await;

    let ctx = ctx_with_step(2505, "active");
    let actions = actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6110);

    assert_eq!(actions.len(), 4, "chain 6110 resolved {actions:?}");
    assert!(matches!(
        actions[0],
        Action::RemoveDialogSet {
            dialog_set_id: 3130,
            slot: 43
        }
    ));
    assert!(matches!(
        actions[1],
        Action::GrantItem {
            item_id: 2864,
            count: 1,
            ..
        }
    ));
    assert!(matches!(
        actions[2],
        Action::AdvanceStep {
            mission_id: 742,
            step_id: 2506
        }
    ));
    assert!(
        matches!(
            actions[3],
            Action::AddDialogSet {
                dialog_set_id: 3131,
                slot: 53,
                ..
            }
        ),
        "Nerus is template 53; binding to the wrong template leaves the \
         player holding a map with nobody to give it to. Got {:?}",
        actions[3]
    );
}

/// Anat's report topic is a one-shot: the step gate closes behind it, so
/// a second pass cannot grant a second Scarab Map. Item 2864 has
/// `max_stack_size = 1`, so a duplicate would occupy a second inventory
/// row and `RemoveInventoryItemByType` would only ever reclaim one.
#[tokio::test]
async fn chain_6110_cannot_grant_a_second_scarab_map() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6110).await;

    let ctx = ctx_with_step(2505, "completed");
    assert!(actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6110).is_empty());
}

/// Chain 6111 — `.script` node 43 -> {44, 45}. `complete_mission`, not
/// `complete_objective`: the Python called `missions.complete(742)`
/// outright, and `complete_mission_direct` force-completes step 2506's
/// objective on the way through.
#[tokio::test]
async fn chain_6111_takes_the_map_and_completes_the_mission() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6111).await;

    let ctx = ctx_with_step(2506, "active");
    let actions = actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6111);

    assert_eq!(actions.len(), 3, "chain 6111 resolved {actions:?}");
    assert!(matches!(
        actions[0],
        Action::RemoveItem {
            item_id: 2864,
            count: 1
        }
    ));
    assert!(matches!(
        actions[1],
        Action::RemoveDialogSet {
            dialog_set_id: 3131,
            slot: 53
        }
    ));
    assert!(
        matches!(actions[2], Action::CompleteMission { mission_id: 742 }),
        "got {:?}",
        actions[2]
    );
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::CompleteObjective { .. })),
        "742 completes via complete_mission, matching the Python's explicit \
         missions.complete(742) — a complete_objective here would depend on \
         step 2506's objective list instead"
    );
}

/// The two durable restore chains: Petbe (step 2502, world 57) and
/// Anat/Nerus (steps 2505/2506, world 68). These are `step_status`-gated
/// and therefore survive a relog today; the basket restores
/// (6113-6115) do not — see the module docs and H50.
#[tokio::test]
async fn step_gated_restore_chains_rebind_exactly_their_own_step() {
    let pool = require_db_or_skip!();

    // (chain, world, world name, active step, expected dsm, template)
    let cases: [(i32, i32, &str, i32, i32, i32); 3] = [
        (6112, HARSET, "Harset", 2502, 3129, 163),
        (6116, CMD_CENTER, "Harset_CmdCenter", 2505, 3130, 43),
        (6117, CMD_CENTER, "Harset_CmdCenter", 2506, 3131, 53),
    ];

    for (chain_id, world, world_name, step, dsm, template) in cases {
        let engine = engine_with(&pool, chain_id).await;

        let mut ctx = ctx_with_step(step, "active");
        ctx.world_id = Some(world);
        ctx.set_param("world_name".to_string(), serde_json::json!(world_name));

        let actions = actions_of(&fire(&engine, TriggerType::PlayerLoaded, &ctx), chain_id as i64);
        assert_eq!(
            actions.len(),
            1,
            "chain {chain_id} must restore exactly one binding; got {actions:?}"
        );
        assert!(
            matches!(
                actions[0],
                Action::AddDialogSet { dialog_set_id, slot, .. }
                    if dialog_set_id == dsm && slot == template
            ),
            "chain {chain_id} must rebind dsm {dsm} on template {template}; got {:?}",
            actions[0]
        );

        // Adjacent negative: a different active step restores nothing.
        let mut other = ctx_with_step(step, "completed");
        other.world_id = Some(world);
        other.set_param("world_name".to_string(), serde_json::json!(world_name));
        assert!(
            actions_of(&fire(&engine, TriggerType::PlayerLoaded, &other), chain_id as i64)
                .is_empty(),
            "chain {chain_id} restored a binding for a step that is no longer active"
        );
    }
}

// ── 4. The offer (D-H20) ────────────────────────────────────────────

/// Chain 6118 binds Anat's offer topic only for a Goa'uld who has
/// finished 1200 and has not already taken 742.
///
/// The `mission_status 1200 eq completed` gate is structural, not
/// flavour: Anat is template 43, `handle_interact` reads
/// `available_interactions[43].first()`, and a player holding both dsm
/// 3127 (this offer) and dsm 4751 (1200 step 3585) would lose one of the
/// two Anat beats to bind order.
#[tokio::test]
async fn chain_6118_offers_742_only_to_a_goauld_who_finished_1200() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6118).await;

    let eligible = || {
        let mut ctx = ExecutionContext::new();
        ctx.world_id = Some(CMD_CENTER);
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Harset_CmdCenter"),
        );
        ctx.set_param("archetype".to_string(), serde_json::json!(6));
        ctx.set_param(
            "mission_742_status".to_string(),
            serde_json::json!("not_active"),
        );
        ctx.set_param(
            "mission_1200_status".to_string(),
            serde_json::json!("completed"),
        );
        ctx
    };

    let actions = actions_of(&fire(&engine, TriggerType::PlayerLoaded, &eligible()), 6118);
    assert_eq!(actions.len(), 1, "chain 6118 resolved {actions:?}");
    assert!(
        matches!(
            actions[0],
            Action::AddDialogSet {
                dialog_set_id: 3127,
                slot: 43,
                ..
            }
        ),
        "the offer restores dead `entity_interactions` row 35 (template 43, \
         dsm 3127) as a chain; got {:?}",
        actions[0]
    );

    // Each gate, knocked out one at a time.
    let knockouts: [(&str, serde_json::Value); 4] = [
        // Jaffa, not Goa'uld.
        ("archetype", serde_json::json!(8)),
        // Has not met the Queen yet.
        ("mission_1200_status", serde_json::json!("active")),
        ("mission_1200_status", serde_json::json!("not_active")),
        // Already on 742.
        ("mission_742_status", serde_json::json!("active")),
    ];
    for (key, value) in knockouts {
        let mut ctx = eligible();
        ctx.set_param(key.to_string(), value.clone());
        assert!(
            actions_of(&fire(&engine, TriggerType::PlayerLoaded, &ctx), 6118).is_empty(),
            "chain 6118 offered 742 with {key} = {value}"
        );
    }

    // And the world gate.
    let mut wrong_world = eligible();
    wrong_world.world_id = Some(HARSET);
    assert!(actions_of(&fire(&engine, TriggerType::PlayerLoaded, &wrong_world), 6118).is_empty());
}

/// Chain 6119 accepts, then retires the offer topic.
#[tokio::test]
async fn chain_6119_accepts_742_and_retires_the_offer() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 6119).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "mission_742_status".to_string(),
        serde_json::json!("not_active"),
    );

    let actions = actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6119);
    assert_eq!(actions.len(), 2, "chain 6119 resolved {actions:?}");
    assert!(matches!(actions[0], Action::AcceptMission { mission_id: 742 }));
    assert!(matches!(
        actions[1],
        Action::RemoveDialogSet {
            dialog_set_id: 3127,
            slot: 43
        }
    ));

    for status in ["active", "completed"] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "mission_742_status".to_string(),
            serde_json::json!(status),
        );
        assert!(
            actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 6119).is_empty(),
            "chain 6119 re-accepted 742 with status {status}"
        );
    }
}

/// The disjointness the D-H20 gate buys, asserted against the full
/// seeded engine: for any single `player_loaded Harset_CmdCenter` event,
/// at most one dialog set is ever bound to Anat's template 43.
///
/// Chains 6116 (742 step 2505), 6118 (the 742 offer) and 6126 (1200 step
/// 3585) all bind template 43. Two of them resolving on one event would
/// push two entries into `available_interactions[43]`, and
/// `handle_interact` would silently take whichever landed first.
#[tokio::test]
async fn at_most_one_dialog_set_ever_binds_to_anat_on_one_world_entry() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // Every combination of the three mission states that can coexist.
    for m742 in ["not_active", "active", "completed"] {
        for m1200 in ["not_active", "active", "completed"] {
            let mut ctx = ExecutionContext::new();
            ctx.world_id = Some(CMD_CENTER);
            ctx.set_param(
                "world_name".to_string(),
                serde_json::json!("Harset_CmdCenter"),
            );
            ctx.set_param("archetype".to_string(), serde_json::json!(6));
            ctx.set_param("mission_742_status".to_string(), serde_json::json!(m742));
            ctx.set_param("mission_1200_status".to_string(), serde_json::json!(m1200));
            // The only steps that bind Anat, both "active" at once — the
            // worst case the gate has to rule out.
            ctx.set_param(
                "mission_742_step_2505_status".to_string(),
                serde_json::json!("active"),
            );
            ctx.set_param(
                "mission_1200_step_3585_status".to_string(),
                serde_json::json!("active"),
            );

            let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
            let anat_binds: Vec<i32> = resolved
                .actions
                .iter()
                .filter_map(|(_, a)| match a {
                    Action::AddDialogSet {
                        dialog_set_id,
                        slot: 43,
                        ..
                    } => Some(*dialog_set_id),
                    _ => None,
                })
                .collect();

            assert!(
                anat_binds.len() <= 1,
                "742 = {m742}, 1200 = {m1200} bound {anat_binds:?} to Anat's \
                 template 43 in one world entry. `handle_interact` takes \
                 `.first()`, so the second binding is unreachable. The \
                 `mission_status 1200 eq completed` gate on chain 6118 is what \
                 makes these states disjoint (D-H20)."
            );
        }
    }
}

// ── 5. H50 defect pin ───────────────────────────────────────────────

/// **Pins a known defect so the fix is forced to come back here.**
///
/// Every `objective_status` assertion above seeds
/// `mission_742_obj_<id>_status` by hand. Production can only produce
/// those keys from `MissionInstance.active_objectives` /
/// `.completed_objectives` — and after a relog those hold the STEP id,
/// not the objective ids, because `executor/mission.rs:84-85` and
/// `:241-242` persist `active_objective_ids: vec![step_id]` and
/// `player_init/mod.rs:172-188` hydrates straight back from that array.
///
/// This test reconstructs the hydrated shape exactly as `player_init`
/// builds it and asserts the objective keys are MISSING, then that chain
/// 6104 is consequently inert. That is the current, wrong behaviour.
///
/// **When H50 lands, invert this test**: the hydrated instance will carry
/// 2913/2914/2915, the params will be present, and chain 6104 will
/// resolve — at which point the basket relog-restore acceptance
/// (chains 6113-6115) can be claimed for the first time.
#[tokio::test]
async fn objective_params_do_not_survive_the_production_hydration_shape() {
    use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};

    let pool = require_db_or_skip!();

    // What `player_init` reconstructs for a player saved mid-step-2504:
    // `active_objective_ids` contains the step id and nothing else.
    let hydrated = MissionInstance::new(
        742,
        2504,
        vec![MissionObjective {
            objective_id: 2504,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    );

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(HARSET);
    ctx.set_param("entity_tag".to_string(), serde_json::json!("FirstBug"));
    ctx.set_param(
        format!("mission_742_step_{}_status", hydrated.current_step_id.unwrap()),
        serde_json::json!("active"),
    );
    // Mirror `populate_mission_context`'s objective loop over exactly the
    // hydrated instance — no hand-seeded objective ids.
    for obj in &hydrated.active_objectives {
        ctx.set_param(
            format!("mission_742_obj_{}_status", obj.objective_id),
            serde_json::json!("active"),
        );
    }

    assert!(
        ctx.params.get("mission_742_obj_2913_status").is_none(),
        "H50 has landed: objective 2913 now survives hydration. Invert this \
         test, drop the `pending H50` note from worknotes/H41.md, and claim \
         the basket relog-restore acceptance via chains 6113-6115."
    );

    let engine = engine_with(&pool, 6104).await;
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert!(
        actions_of(&resolved, 6104).is_empty(),
        "chain 6104 resolved against a hydrated context — H50 is fixed, so \
         invert this test"
    );
}
