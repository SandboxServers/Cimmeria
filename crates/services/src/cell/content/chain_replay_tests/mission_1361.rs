//! Mission 1361 — Meet The Praxis (packet H31,
//! `db/resources/Content/Seed/harset_opcore_chains.sql` chains 6511-6527).
//!
//! Six strictly ordered talk steps across two worlds:
//!
//! | Step | Beat | World | Dialog | Chains |
//! |---|---|---|---|---|
//! | (offer) | Marsh briefing | 68 | 4457 | 6511, 6512, 6513 |
//! | 4040 | Moh'katan asks for weapon samples | 68 | 4458 | 6514, 6515 |
//! | 4041 | Convince Hansen | **57** | 4459 -> 4460 | 6516, 6518 |
//! | 4042 | Deliver the samples | 68 | 4466 | 6519, 6520 |
//! | 4043 | Ba'al | 68 | 4461 | 6521, 6522 |
//! | 4693 | Anat | 68 | 4462 -> 4463 | 6523, 6525 |
//! | 4694 | Return to Marsh | 68 | 4465 | 6526, 6527 |
//!
//! Chain ids 6517 and 6524 are deliberately unused — Hansen's and Anat's
//! beats use the *bind path* (no `interact_tag` chain) because their
//! outcome dialogs 4460/4463 are displayed from a `dialog_choice` trigger,
//! which stamps no `target_entity_id`; only `handle_interact` pins
//! `last_interaction_target`, and an `interact_tag` chain short-circuits
//! it. See the seed file's note (C).
//!
//! The acceptance trio (6511-6513) ships **disabled**: step 4041 is in
//! world 57 and its neighbours are in world 68, so the mission requires a
//! working 68 -> 57 crossing, and the only one — chain 6007 in
//! `harset_space_chains.sql` — is itself disabled pending an M0
//! coordinate pin. Shipping acceptance live against a dark return leg
//! would soft-stick every player at step 4041 with no recovery (there is
//! no `fail_objective` executor arm and no chain-authorable abandon).
//!
//! Four guard families live here:
//!
//! 1. **Ordered progression.** Each step chain resolves its exact action
//!    list on its own step and nothing on any other step.
//! 2. **Cross-chain disjointness.** Three chains key on
//!    `interact_tag 'CmdCenter_Marsh'` and `resolve_event` APPENDS every
//!    matching chain's actions with no first-match break, so one
//!    right-click could otherwise run two of them. Asserted through
//!    `build_engine`, against the whole seeded DB, because a per-chain
//!    test cannot see a collision by construction.
//! 3. **The parked acceptance path**, including the biconditional with
//!    chain 6007 so M0 cannot flip one without the other.
//! 4. **Bind hygiene** — no template slot ever holds two live binds, and
//!    every indicator a step sets is cleared and re-paintable on relog.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::{build_engine, load_single_chain_for_test};
use crate::test_support::require_db_or_skip;

/// `resources.worlds.world_id` for `Harset_CmdCenter` (Marsh, Moh'katan,
/// Ba'al, Anat) and `Harset` (Hansen).
const CMD_CENTER: i32 = 68;
const HARSET: i32 = 57;

/// `EArchetype` values from `entities/defs/enumerations.xml:364,366`.
/// 1361 is the Human/OP-CORE arrival mission, so both of these are
/// excluded by the offer chains.
const ARCHETYPE_JAFFA: i64 = 8;
const ARCHETYPE_GOAULD: i64 = 6;
/// Any Human archetype — `ARCHETYPE_Soldier`. The gate is two `neq` rows,
/// not an `eq`, because "Human" is four archetypes.
const ARCHETYPE_SOLDIER: i64 = 5;

/// Every step id in 1361, in play order. Used to prove each step chain is
/// silent on every step but its own.
const STEPS: [&str; 6] = ["4040", "4041", "4042", "4043", "4693", "4694"];

/// Build a context with 1361 on `current_step`, in `world_id`.
///
/// Mirrors `populate_mission_context`: the current step is written
/// `active` and every other step is left absent, which the evaluator reads
/// through its `unwrap_or("not_active")` fallback.
fn praxis_ctx(world_id: i32, current_step: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(world_id);
    ctx.set_param(
        "mission_1361_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        format!("mission_1361_step_{current_step}_status"),
        serde_json::json!("active"),
    );
    ctx
}

fn with_tag(mut ctx: ExecutionContext, tag: &str) -> ExecutionContext {
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx
}

fn with_dialog(mut ctx: ExecutionContext, dialog_id: i32) -> ExecutionContext {
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx
}

fn with_world_name(mut ctx: ExecutionContext, name: &str) -> ExecutionContext {
    ctx.set_param("world_name".to_string(), serde_json::json!(name));
    ctx
}

/// Load one chain, or fail with a message that separates "row missing"
/// from "row present but the loader rejected it".
async fn load(pool: &sqlx::PgPool, chain_id: i32) -> Chain {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains AND load cleanly \
                 (an unknown trigger/condition/action type is dropped with a warn, which \
                 looks identical to a missing row from here)"
            )
        })
}

/// Resolve one event against a single loaded chain.
async fn resolve_one(
    pool: &sqlx::PgPool,
    chain_id: i32,
    ctx: &ExecutionContext,
    tt: TriggerType,
) -> ResolvedActions {
    let mut engine = ChainEngine::new();
    engine.register_chain(load(pool, chain_id).await);
    let event = TriggerEvent {
        trigger_type: tt,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// The actions a chain resolves, stripped of chain ids.
fn actions_of(resolved: &ResolvedActions) -> Vec<Action> {
    resolved.actions.iter().map(|(_, a)| a.clone()).collect()
}

// ── 1. Ordered progression ────────────────────────────────────────────

/// Step 4040: Moh'katan asks for samples of Earth weaponry and the step
/// advances to the Hansen leg.
///
/// Note what is **absent**: no `add_dialog_set` for Hansen. Hansen is in
/// world 57 and this chain runs in world 68, and `cross_world_teleport`
/// destroys the cell entity — so a bind made here would not survive the
/// crossing. Chain 6516 makes it on the far side. A future edit that
/// "helpfully" adds the bind here would be a silent no-op, so the exact
/// action count is what guards it.
#[tokio::test]
async fn chain_6515_advances_from_mohkatan_to_hansen() {
    let pool = require_db_or_skip!();
    let ctx = with_tag(praxis_ctx(CMD_CENTER, "4040"), "CmdCenter_Mohkatan");
    let got = actions_of(&resolve_one(&pool, 6515, &ctx, TriggerType::InteractTag).await);

    assert_eq!(
        got,
        vec![
            Action::DisplayDialog { dialog_id: 4458 },
            Action::RemoveDialogSet {
                dialog_set_id: 6397,
                slot: 54
            },
            Action::AdvanceStep {
                mission_id: 1361,
                step_id: 4041
            },
        ],
        "chain 6515 must play 4458, clear Moh'katan's \"!\", and advance to 4041 — and \
         must NOT bind Hansen (world 57; the bind would not survive the door crossing)"
    );
}

/// Step 4041: the player clicks "Convince Hansen." on dialog 4459.
///
/// `display_dialog 4460` is first for a reason worth stating: it resolves
/// Hansen through `last_interaction_target`, which `handle_interact`
/// pinned when it opened 4459 from the dsm 6399 bind. That only holds
/// because there is no `interact_tag` chain for Hansen — see the module
/// header and the seed's note (C).
///
/// No `add_item`: the "weapon samples" mission item does not exist
/// anywhere in `resources.items` (decision H31-D1), so nothing is granted
/// here and nothing is removed at step 4042.
#[tokio::test]
async fn chain_6518_advances_from_hansen_to_the_delivery() {
    let pool = require_db_or_skip!();
    let ctx = with_dialog(praxis_ctx(HARSET, "4041"), 4459);
    let got = actions_of(&resolve_one(&pool, 6518, &ctx, TriggerType::DialogChoice).await);

    assert_eq!(
        got,
        vec![
            Action::DisplayDialog { dialog_id: 4460 },
            Action::RemoveDialogSet {
                dialog_set_id: 6399,
                slot: 212
            },
            Action::AdvanceStep {
                mission_id: 1361,
                step_id: 4042
            },
        ],
        "chain 6518 must show Hansen relenting (4460), clear his \"!\", and advance to \
         4042. If an `add_item` appears here, the weapon-samples item was invented — \
         H31-D1 says there isn't one."
    );
}

/// Step 4042: deliver the samples. Also hands off to Ba'al in-chain,
/// which is legitimate here because Ba'al is in the same world.
#[tokio::test]
async fn chain_6520_delivers_and_hands_off_to_baal() {
    let pool = require_db_or_skip!();
    let ctx = with_tag(praxis_ctx(CMD_CENTER, "4042"), "CmdCenter_Mohkatan");
    let got = actions_of(&resolve_one(&pool, 6520, &ctx, TriggerType::InteractTag).await);

    assert_eq!(
        got,
        vec![
            Action::DisplayDialog { dialog_id: 4466 },
            Action::RemoveDialogSet {
                dialog_set_id: 6398,
                slot: 54
            },
            Action::AdvanceStep {
                mission_id: 1361,
                step_id: 4043
            },
            Action::AddDialogSet {
                dialog_set_id: 6395,
                slot: 42,
                mission_id: Some(1361)
            },
        ],
        "chain 6520 must play 4466, clear Moh'katan's \"!\", advance to 4043 and light \
         up Ba'al (same world, so the in-chain bind survives)"
    );
}

/// Step 4043: Ba'al, handing off to Anat.
#[tokio::test]
async fn chain_6522_advances_from_baal_to_anat() {
    let pool = require_db_or_skip!();
    let ctx = with_tag(praxis_ctx(CMD_CENTER, "4043"), "CmdCenter_Baal");
    let got = actions_of(&resolve_one(&pool, 6522, &ctx, TriggerType::InteractTag).await);

    assert_eq!(
        got,
        vec![
            Action::DisplayDialog { dialog_id: 4461 },
            Action::RemoveDialogSet {
                dialog_set_id: 6395,
                slot: 42
            },
            Action::AdvanceStep {
                mission_id: 1361,
                step_id: 4693
            },
            Action::AddDialogSet {
                dialog_set_id: 6396,
                slot: 43,
                mission_id: Some(1361)
            },
        ],
        "chain 6522 must play 4461, clear Ba'al's \"!\", advance to 4693 and bind Anat"
    );
}

/// Step 4693: the player clicks "Flatter Anat." on dialog 4462, and Marsh
/// lights up for the turn-in.
#[tokio::test]
async fn chain_6525_advances_from_anat_to_the_marsh_turn_in() {
    let pool = require_db_or_skip!();
    let ctx = with_dialog(praxis_ctx(CMD_CENTER, "4693"), 4462);
    let got = actions_of(&resolve_one(&pool, 6525, &ctx, TriggerType::DialogChoice).await);

    assert_eq!(
        got,
        vec![
            Action::DisplayDialog { dialog_id: 4463 },
            Action::RemoveDialogSet {
                dialog_set_id: 6396,
                slot: 43
            },
            Action::AdvanceStep {
                mission_id: 1361,
                step_id: 4694
            },
            Action::AddDialogSet {
                dialog_set_id: 5253,
                slot: 10,
                mission_id: Some(1361)
            },
        ],
        "chain 6525 must show Anat's reply (4463), clear her \"!\", advance to 4694 and \
         put the turn-in \"?\" on Marsh"
    );
}

/// Step 4694: Marsh debriefs and the mission ends.
///
/// `complete_mission`, not `complete_objective`: 4694 is the terminal
/// step, and `complete_mission_direct` closes objective 5573 with it.
/// Using `complete_objective` on a MID-mission step's last required
/// objective would end the whole mission early, which is why no chain in
/// this file emits one.
#[tokio::test]
async fn chain_6527_completes_the_praxis_at_marsh() {
    let pool = require_db_or_skip!();
    let ctx = with_tag(praxis_ctx(CMD_CENTER, "4694"), "CmdCenter_Marsh");
    let got = actions_of(&resolve_one(&pool, 6527, &ctx, TriggerType::InteractTag).await);

    assert_eq!(
        got,
        vec![
            Action::DisplayDialog { dialog_id: 4465 },
            Action::RemoveDialogSet {
                dialog_set_id: 5253,
                slot: 10
            },
            Action::CompleteMission { mission_id: 1361 },
        ],
        "chain 6527 must play 4465, clear the turn-in \"?\" and COMPLETE 1361"
    );
}

/// Strict ordering: every step chain is silent on every step but its own.
///
/// This is the guard that makes "strictly ordered" a tested property
/// rather than an authoring intention. A mission has exactly one
/// `current_step_id`, so the six step chains are mutually exclusive by
/// construction — but only as long as each one actually carries its
/// `step_status ... eq active` gate. Dropping one would let the player
/// skip ahead by clicking the wrong NPC.
#[tokio::test]
async fn every_step_chain_is_silent_on_every_other_step() {
    let pool = require_db_or_skip!();

    // (chain, its own step, trigger, tag-or-dialog key, world)
    let cases: [(i32, &str, TriggerType, &str, i32); 6] = [
        (6515, "4040", TriggerType::InteractTag, "CmdCenter_Mohkatan", CMD_CENTER),
        (6518, "4041", TriggerType::DialogChoice, "4459", HARSET),
        (6520, "4042", TriggerType::InteractTag, "CmdCenter_Mohkatan", CMD_CENTER),
        (6522, "4043", TriggerType::InteractTag, "CmdCenter_Baal", CMD_CENTER),
        (6525, "4693", TriggerType::DialogChoice, "4462", CMD_CENTER),
        (6527, "4694", TriggerType::InteractTag, "CmdCenter_Marsh", CMD_CENTER),
    ];

    for (chain_id, own_step, tt, key, world) in cases {
        for step in STEPS {
            if step == own_step {
                continue;
            }
            let base = praxis_ctx(world, step);
            let ctx = match tt {
                TriggerType::DialogChoice => with_dialog(base, key.parse().unwrap()),
                _ => with_tag(base, key),
            };
            let resolved = resolve_one(&pool, chain_id, &ctx, tt).await;
            assert!(
                resolved.actions.is_empty(),
                "chain {chain_id} (step {own_step}) must resolve nothing while step \
                 {step} is the current step; got {:?}",
                resolved.actions
            );
        }
    }
}

/// Completed mission: no step chain re-fires. Covers the "already
/// completed" adjacent-negative required by the packet's acceptance.
#[tokio::test]
async fn no_step_chain_fires_once_1361_is_completed() {
    let pool = require_db_or_skip!();

    for (chain_id, step, tag) in [
        (6515, "4040", "CmdCenter_Mohkatan"),
        (6520, "4042", "CmdCenter_Mohkatan"),
        (6522, "4043", "CmdCenter_Baal"),
        (6527, "4694", "CmdCenter_Marsh"),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.world_id = Some(CMD_CENTER);
        ctx.set_param(
            "mission_1361_status".to_string(),
            serde_json::json!("completed"),
        );
        // `MissionInstance::complete()` moves the current step into
        // `completed_steps`, so the terminal step reads `completed`.
        ctx.set_param(
            format!("mission_1361_step_{step}_status"),
            serde_json::json!("completed"),
        );
        ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));

        let resolved = resolve_one(&pool, chain_id, &ctx, TriggerType::InteractTag).await;
        assert!(
            resolved.actions.is_empty(),
            "chain {chain_id} must not re-fire after 1361 completes; got {:?}",
            resolved.actions
        );
    }
}

/// Wrong world: every world-68 step chain is silent for a player standing
/// in Harset, and the one world-57 chain is silent in the Command Center.
///
/// `OnInteractTag` and `OnDialogChoice` do not filter by world, so the
/// `world` condition is the only thing scoping these chains. It also
/// fails **closed** on an unset `world_id`, which the last case pins.
#[tokio::test]
async fn step_chains_are_scoped_to_their_own_world() {
    let pool = require_db_or_skip!();

    for (chain_id, step, tag) in [
        (6515, "4040", "CmdCenter_Mohkatan"),
        (6520, "4042", "CmdCenter_Mohkatan"),
        (6522, "4043", "CmdCenter_Baal"),
        (6527, "4694", "CmdCenter_Marsh"),
    ] {
        let ctx = with_tag(praxis_ctx(HARSET, step), tag);
        assert!(
            resolve_one(&pool, chain_id, &ctx, TriggerType::InteractTag)
                .await
                .actions
                .is_empty(),
            "chain {chain_id} is a world-68 beat and must not fire from Harset (57)"
        );

        let mut unset = praxis_ctx(CMD_CENTER, step);
        unset.world_id = None;
        let ctx = with_tag(unset, tag);
        assert!(
            resolve_one(&pool, chain_id, &ctx, TriggerType::InteractTag)
                .await
                .actions
                .is_empty(),
            "chain {chain_id} must fail closed when the dispatch site left world_id unset"
        );
    }

    // The mirror case: Hansen's beat is the only world-57 chain.
    let ctx = with_dialog(praxis_ctx(CMD_CENTER, "4041"), 4459);
    assert!(
        resolve_one(&pool, 6518, &ctx, TriggerType::DialogChoice)
            .await
            .actions
            .is_empty(),
        "chain 6518 is the world-57 Hansen beat and must not fire from the Command Center"
    );
}

// ── 2. Cross-chain disjointness ───────────────────────────────────────

/// **The multi-chain dispatch guard.** Three chains key on
/// `interact_tag 'CmdCenter_Marsh'` — 6501 (mission 1360's letter
/// turn-in), 6512 (the Praxis offer) and 6527 (the Praxis turn-in).
///
/// `ChainEngine::resolve_event` loops every registered chain for the
/// trigger type and APPENDS the actions of each one whose conditions
/// pass; there is no first-match break, and `priority` only orders the
/// bucket. Conditions are all evaluated *before* any action runs, so a
/// `complete_mission` in one chain cannot gate another in the same event.
///
/// If two of these ever co-fired, both `display_dialog` actions would run
/// and the second `send_dialog_display` would re-pin `open_dialog_id` —
/// the player would see only one blurb while both chains' state changes
/// landed. Silent content loss.
///
/// This runs against `build_engine`, i.e. the whole seeded DB, because a
/// per-chain test cannot observe a collision by construction. It also
/// means a *future* chain that keys on Marsh without gating itself will
/// fail here rather than in a player's log.
#[tokio::test]
async fn marsh_interact_chains_are_pairwise_disjoint() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // Every combination of the three mission states that can hold at a
    // Marsh right-click, including the impossible-looking ones — the
    // point is that no combination produces two chains.
    let states = [
        ("not_active", "not_active", "not_active"), // nothing in progress
        ("active", "active", "not_active"),         // carrying the letter (first visit)
        ("completed", "completed", "not_active"),   // letter delivered
        ("completed", "completed", "active"),       // Praxis turn-in ready
        ("active", "active", "active"),             // both at once — the trap
    ];

    for (m1360, s4038, s4694) in states {
        let mut ctx = ExecutionContext::new();
        ctx.world_id = Some(CMD_CENTER);
        ctx.set_param(
            "entity_tag".to_string(),
            serde_json::json!("CmdCenter_Marsh"),
        );
        ctx.set_param("archetype".to_string(), serde_json::json!(ARCHETYPE_SOLDIER));
        ctx.set_param("mission_1360_status".to_string(), serde_json::json!(m1360));
        ctx.set_param(
            "mission_1360_step_4038_status".to_string(),
            serde_json::json!(s4038),
        );
        ctx.set_param(
            "mission_1361_status".to_string(),
            serde_json::json!(if s4694 == "active" {
                "active"
            } else {
                "not_active"
            }),
        );
        ctx.set_param(
            "mission_1361_step_4694_status".to_string(),
            serde_json::json!(s4694),
        );

        let event = TriggerEvent {
            trigger_type: TriggerType::InteractTag,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);

        let firing: Vec<i64> = {
            let mut ids: Vec<i64> = resolved.actions.iter().map(|(id, _)| *id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };
        assert!(
            firing.len() <= 1,
            "a single right-click on Col. Marsh resolved {} chains ({firing:?}) with \
             1360={m1360}, step4038={s4038}, step4694={s4694}. Exactly one chain may \
             claim a click: resolve_event appends every match, so two `display_dialog` \
             actions mean the player sees only the last one while BOTH chains' item \
             removals and mission completions still run. Re-check the DISJOINTNESS \
             conditions in harset_opcore_chains.sql note (A).",
            firing.len()
        );
    }
}

/// The same property for Moh'katan, who is claimed by two chains (6515 on
/// step 4040 and 6520 on step 4042). These are disjoint for free — a
/// mission has one current step — but "for free" is exactly the kind of
/// reasoning that stops being true when someone adds a third chain.
#[tokio::test]
async fn mohkatan_interact_chains_are_pairwise_disjoint() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for step in STEPS {
        let ctx = with_tag(praxis_ctx(CMD_CENTER, step), "CmdCenter_Mohkatan");
        let event = TriggerEvent {
            trigger_type: TriggerType::InteractTag,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);
        let mut ids: Vec<i64> = resolved.actions.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert!(
            ids.len() <= 1,
            "a right-click on Moh'katan at step {step} resolved {ids:?}; at most one \
             chain may claim it"
        );
    }
}

/// No template slot ever holds two live binds at once.
///
/// `handle_interact` picks `available_interactions[template_id].first()`
/// — a single insertion-ordered Vec — so a second live bind on one slot
/// makes one of them permanently unreachable. Slot 10 (Col. Marsh,
/// template 10) is the one at risk: mission 1360's letter bind (dsm 5356),
/// the Praxis offer (5254) and the Praxis turn-in (5253) all target it.
///
/// This walks every `player_loaded` bind chain in the file and asserts
/// that for any reachable mission state, at most one of them fires per
/// slot.
#[tokio::test]
async fn no_template_slot_ever_holds_two_binds_at_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // (1360 status, 4038 status, 1361 status, 1361 current step)
    let states: [(&str, &str, &str, Option<&str>); 6] = [
        ("not_active", "not_active", "not_active", None),
        ("active", "active", "not_active", None),
        ("completed", "completed", "not_active", None),
        ("completed", "completed", "active", Some("4040")),
        ("completed", "completed", "active", Some("4694")),
        ("active", "active", "active", Some("4694")),
    ];

    for (m1360, s4038, m1361, step) in states {
        let mut ctx = ExecutionContext::new();
        ctx.world_id = Some(CMD_CENTER);
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Harset_CmdCenter"),
        );
        ctx.set_param("archetype".to_string(), serde_json::json!(ARCHETYPE_SOLDIER));
        ctx.set_param("mission_1360_status".to_string(), serde_json::json!(m1360));
        ctx.set_param(
            "mission_1360_step_4038_status".to_string(),
            serde_json::json!(s4038),
        );
        ctx.set_param("mission_1361_status".to_string(), serde_json::json!(m1361));
        if let Some(s) = step {
            ctx.set_param(
                format!("mission_1361_step_{s}_status"),
                serde_json::json!("active"),
            );
        }

        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);

        let mut per_slot: HashMap<i32, Vec<i32>> = HashMap::new();
        for (_, action) in &resolved.actions {
            if let Action::AddDialogSet {
                dialog_set_id,
                slot,
                ..
            } = action
            {
                per_slot.entry(*slot).or_default().push(*dialog_set_id);
            }
        }

        for (slot, dsms) in &per_slot {
            assert!(
                dsms.len() <= 1,
                "world entry with 1360={m1360}/{s4038}, 1361={m1361}/{step:?} bound {} \
                 dialog sets ({dsms:?}) to template slot {slot}. `handle_interact` only \
                 ever opens `.first()`, so the rest would be unreachable — see \
                 harset_opcore_chains.sql note (D).",
                dsms.len()
            );
        }
    }
}

// ── 3. The parked acceptance path ─────────────────────────────────────

/// The acceptance trio is disabled, and its enablement is tied to chain
/// 6007 (the 68 -> 57 Command Center return door) as a **biconditional**.
///
/// 1361 sends the player from world 68 to Hansen in world 57 and back. If
/// acceptance were live while the return door is dark, every player who
/// accepted would soft-stick at step 4041 with no recovery: there is no
/// `fail_objective` executor arm and no chain-authorable abandon. If the
/// door is later pinned and opened without flipping these three, the
/// mission becomes silently unreachable instead.
///
/// Both drifts fail here. M0 flips all four rows in one change.
#[tokio::test]
async fn praxis_acceptance_is_enabled_iff_the_return_door_is() {
    let pool = require_db_or_skip!();

    let door = load(&pool, 6007).await;
    for chain_id in [6511, 6512, 6513] {
        let chain = load(&pool, chain_id).await;
        assert_eq!(
            chain.enabled,
            door.enabled,
            "chain {chain_id} (1361 acceptance) is enabled={} but the 68->57 return \
             door chain 6007 is enabled={}. These must move together: step 4041 is at \
             Hansen in world 57 while 4040/4042 are in world 68, so accepting 1361 \
             without a working return leg soft-sticks the player at 4041 forever, and \
             opening the door without enabling acceptance leaves the mission \
             unreachable. M0 flips 6007 (harset_space_chains.sql, after pinning its \
             arrival coordinate) and 6511/6512/6513 in the same change.",
            chain.enabled,
            door.enabled,
        );
    }
}

/// While parked, the acceptance chains resolve nothing even under the
/// context that satisfies every one of their conditions.
#[tokio::test]
async fn acceptance_chains_resolve_nothing_while_parked() {
    let pool = require_db_or_skip!();

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Harset_CmdCenter"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(ARCHETYPE_SOLDIER));
    ctx.set_param(
        "mission_1361_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("CmdCenter_Marsh"),
    );
    ctx.set_param("dialog_id".to_string(), serde_json::json!(4457));

    for (chain_id, tt) in [
        (6511, TriggerType::PlayerLoaded),
        (6512, TriggerType::InteractTag),
        (6513, TriggerType::DialogChoice),
    ] {
        let resolved = resolve_one(&pool, chain_id, &ctx, tt).await;
        assert!(
            resolved.actions.is_empty(),
            "chain {chain_id} is parked and must resolve nothing; got {:?}",
            resolved.actions
        );
    }
}

/// Chains 6511 and 6512 must carry **identical** condition lists.
///
/// 6511 binds dsm 5254 (the offer "?") and 6512 is the `interact_tag`
/// chain that displays the briefing. If the bind could happen while the
/// display chain could not fire, `fire_interact_tag` would match nothing,
/// `handled` would stay false, and `handle_interact` would fall through to
/// the bound dsm — rendering dialog **4456**, whose two buttons ("Accept"
/// id 8 type 2 and "More Info" id 9 type 1) are indistinguishable at the
/// `dialog_choice` trigger because no condition type can read `button_id`.
/// Clicking "More Info" would then accept the mission.
///
/// 6512 displays 4457 instead precisely to avoid that, so the invariant
/// that keeps 4456 unreachable is this equality.
#[tokio::test]
async fn offer_bind_and_offer_dialog_carry_identical_conditions() {
    let pool = require_db_or_skip!();
    let bind = load(&pool, 6511).await;
    let dialog = load(&pool, 6512).await;

    assert_eq!(
        format!("{:?}", bind.conditions),
        format!("{:?}", dialog.conditions),
        "chains 6511 (offer bind) and 6512 (offer dialog) must carry identical \
         conditions. If the bind can fire while the dialog chain cannot, \
         `handle_interact` falls through to dsm 5254 and renders dialog 4456, whose \
         \"More Info\" button is indistinguishable from \"Accept\" at the trigger — so \
         reading more about the mission would accept it. See harset_opcore_chains.sql, \
         the 4456 note."
    );
}

/// Even though the acceptance chains are parked, their **gates** are
/// tested directly: `Condition::evaluate` is public, so the archetype
/// filter can be exercised without enabling the chain.
///
/// This is the "wrong archetype" adjacent negative. 1361 is the Human
/// arrival mission; Jaffa get 1324 (H20) and Goa'uld get 1200 (H40). The
/// gate is two `neq` rows rather than one `eq` because "Human" is four
/// archetypes, so a regression that collapsed them into `eq 5` would
/// silently lock out Commandos, Scientists and Engineers.
#[tokio::test]
async fn the_praxis_offer_is_human_only() {
    let pool = require_db_or_skip!();

    for chain_id in [6511, 6512] {
        let chain = load(&pool, chain_id).await;

        let eval = |archetype: i64| {
            let mut ctx = ExecutionContext::new();
            ctx.world_id = Some(CMD_CENTER);
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
            ctx.set_param(
                "mission_1361_status".to_string(),
                serde_json::json!("not_active"),
            );
            ctx.set_param(
                "mission_1360_step_4038_status".to_string(),
                serde_json::json!("completed"),
            );
            chain.conditions.iter().all(|c| c.evaluate(&ctx))
        };

        assert!(
            eval(ARCHETYPE_SOLDIER),
            "chain {chain_id}'s conditions must pass for a Human archetype \
             ({ARCHETYPE_SOLDIER})"
        );
        assert!(
            !eval(ARCHETYPE_JAFFA),
            "chain {chain_id} must not offer 1361 to a Jaffa (archetype \
             {ARCHETYPE_JAFFA}) — their arrival mission is 1324"
        );
        assert!(
            !eval(ARCHETYPE_GOAULD),
            "chain {chain_id} must not offer 1361 to a Goa'uld (archetype \
             {ARCHETYPE_GOAULD}) — their arrival mission is 1200"
        );
    }
}

/// The offer stands down while the player is still carrying Frost's
/// letter — the other half of the Marsh disjointness contract, tested at
/// the condition level because the chains are parked.
#[tokio::test]
async fn the_praxis_offer_waits_for_the_letter_to_be_delivered() {
    let pool = require_db_or_skip!();

    for chain_id in [6511, 6512] {
        let chain = load(&pool, chain_id).await;
        let mut ctx = ExecutionContext::new();
        ctx.world_id = Some(CMD_CENTER);
        ctx.set_param("archetype".to_string(), serde_json::json!(ARCHETYPE_SOLDIER));
        ctx.set_param(
            "mission_1361_status".to_string(),
            serde_json::json!("not_active"),
        );
        ctx.set_param(
            "mission_1360_step_4038_status".to_string(),
            serde_json::json!("active"),
        );

        assert!(
            !chain.conditions.iter().all(|c| c.evaluate(&ctx)),
            "chain {chain_id} must stand down while mission 1360's step 4038 is active, \
             so the Praxis offer cannot co-fire with the letter turn-in (chain 6501) on \
             one right-click"
        );
    }
}

/// The accept action list, checked on the parked chain so it cannot rot
/// while unreachable.
#[tokio::test]
async fn chain_6513_is_authored_to_accept_and_point_at_mohkatan() {
    let pool = require_db_or_skip!();
    let chain = load(&pool, 6513).await;

    assert_eq!(
        chain.actions,
        vec![
            Action::AcceptMission { mission_id: 1361 },
            Action::RemoveDialogSet {
                dialog_set_id: 5254,
                slot: 10
            },
            Action::AddDialogSet {
                dialog_set_id: 6397,
                slot: 54,
                mission_id: Some(1361)
            },
        ],
        "chain 6513 must accept 1361, drop Marsh's offer \"?\" and light up Moh'katan"
    );
}

// ── 4. Bind hygiene ───────────────────────────────────────────────────

/// Every step's relog-restore chain re-paints exactly the binding its
/// step needs, and nothing on any other step.
///
/// These are not merely relog safety. `available_interactions` live on the
/// cell entity and `cross_world_teleport` destroys it, so the 68 <-> 57
/// round trip that step 4041 forces would otherwise arrive with an empty
/// binding table — chain 6516 is the *only* thing that makes Hansen
/// clickable at all, and 6519 the only thing that makes Moh'katan
/// clickable on the way back.
#[tokio::test]
async fn every_restore_chain_repaints_exactly_its_own_step() {
    let pool = require_db_or_skip!();

    // (chain, step, world, world_name, dsm, template slot)
    let cases: [(i32, &str, i32, &str, i32, i32); 6] = [
        (6514, "4040", CMD_CENTER, "Harset_CmdCenter", 6397, 54),
        (6516, "4041", HARSET, "Harset", 6399, 212),
        (6519, "4042", CMD_CENTER, "Harset_CmdCenter", 6398, 54),
        (6521, "4043", CMD_CENTER, "Harset_CmdCenter", 6395, 42),
        (6523, "4693", CMD_CENTER, "Harset_CmdCenter", 6396, 43),
        (6526, "4694", CMD_CENTER, "Harset_CmdCenter", 5253, 10),
    ];

    for (chain_id, own_step, world, world_name, dsm, slot) in cases {
        // Positive: on its own step, exactly one bind.
        let ctx = with_world_name(praxis_ctx(world, own_step), world_name);
        let got = actions_of(&resolve_one(&pool, chain_id, &ctx, TriggerType::PlayerLoaded).await);
        assert_eq!(
            got,
            vec![Action::AddDialogSet {
                dialog_set_id: dsm,
                slot,
                mission_id: Some(1361)
            }],
            "restore chain {chain_id} must re-bind dsm {dsm} to template slot {slot} on \
             entry to {world_name} while step {own_step} is active"
        );

        // Negative: silent on every other step.
        for step in STEPS {
            if step == own_step {
                continue;
            }
            let ctx = with_world_name(praxis_ctx(world, step), world_name);
            assert!(
                resolve_one(&pool, chain_id, &ctx, TriggerType::PlayerLoaded)
                    .await
                    .actions
                    .is_empty(),
                "restore chain {chain_id} must not paint an indicator while step {step} \
                 is active — a stale \"!\" on a shared-hub NPC is visible to that player \
                 on every visit with nothing behind the click"
            );
        }
    }
}

/// Hansen's restore chain is world-scoped to Harset, and Moh'katan's to
/// the Command Center. This is the guard for the cross-world hand-off
/// rule: a bind made in the wrong world is destroyed by the crossing, so
/// a restore chain that fired in both worlds would paint an indicator on
/// a template that isn't there.
#[tokio::test]
async fn cross_world_restore_chains_do_not_fire_in_the_wrong_world() {
    let pool = require_db_or_skip!();

    // Hansen (world 57) must not bind while the player is in world 68.
    let ctx = with_world_name(praxis_ctx(CMD_CENTER, "4041"), "Harset_CmdCenter");
    assert!(
        resolve_one(&pool, 6516, &ctx, TriggerType::PlayerLoaded)
            .await
            .actions
            .is_empty(),
        "chain 6516 binds Hansen (template 212, world 57) and must not fire on a \
         Command Center load"
    );

    // Moh'katan (world 68) must not bind while the player is in world 57.
    let ctx = with_world_name(praxis_ctx(HARSET, "4042"), "Harset");
    assert!(
        resolve_one(&pool, 6519, &ctx, TriggerType::PlayerLoaded)
            .await
            .actions
            .is_empty(),
        "chain 6519 binds Moh'katan (template 54, world 68) and must not fire on a \
         Harset load"
    );
}

/// Chain ids 6517 and 6524 must stay **absent**.
///
/// They are the `interact_tag` chains a future author would naturally add
/// for Hansen and Anat. Adding one would break chains 6518 and 6525:
/// `fire_interact_tag` short-circuits `handle_interact`, which is the only
/// place `last_interaction_target` is pinned, and the follow-up
/// `display_dialog 4460` / `4463` fired from a `dialog_choice` trigger has
/// no other way to resolve the NPC — so the outcome dialog would abort
/// with a warn or speak from a stale NPC's portrait.
///
/// This test is the tripwire for that edit.
#[tokio::test]
async fn hansen_and_anat_have_no_interact_tag_chain() {
    let pool = require_db_or_skip!();

    for chain_id in [6517, 6524] {
        let found = load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"));
        assert!(
            found.is_none(),
            "chain {chain_id} must not exist. Hansen (6516/6518) and Anat (6523/6525) \
             use the BIND path deliberately: their outcome dialogs 4460 and 4463 are \
             displayed from a `dialog_choice` trigger, which stamps no \
             `target_entity_id`, so they depend on `last_interaction_target` — and that \
             is pinned only inside `handle_interact`, which an `interact_tag` chain \
             short-circuits. Adding this chain silently breaks the outcome dialog. See \
             harset_opcore_chains.sql note (C)."
        );
    }
}
