//! Step 2417 — "Take the Control Crystal to Checkpoint Alpha."
//! Chains 1352-1355.
//!
//! The archetype split. A Tau'ri reports to Col. Marsh (dialog 5008,
//! objective 5185); a Jaffa reports to Moh'katan (dialog 5009, objective
//! 5186). Neither may ever see the other's dialog, and the two report
//! objectives must never both be completed by hand.
//!
//! Where the gate actually lives is the subtle part. `fire_dialog_choice`
//! does NOT put `archetype` into the chain context
//! (`event_dispatch/dialog.rs:87-93`) and `Condition::Archetype` reads a
//! missing key as `-1` (`conditions.rs:225-230`), so on a
//! `dialog_choice` chain `archetype eq 8` is permanently false and
//! `archetype neq 8` is permanently true — a condition that reads like a
//! guard and fails open. The seed therefore gates the INTERACT halves
//! (1352/1354), where `archetype` IS populated, and lets dialog-id
//! reachability carry the choice halves: the only way to reach
//! `display_dialog 5008` is chain 1352, and `dialogButtonChoice` is
//! rejected outright unless the server displayed that dialog to that
//! player (#479, `interaction/dialog.rs:36-49`).
//!
//! [`archetype_conditions_are_never_placed_on_the_dialog_halves`] pins
//! that reasoning directly against the seed, because the failure it
//! prevents is invisible: adding the "obvious" `archetype neq 8` row to
//! chain 1353 would change nothing observable while silently making the
//! Jaffa branch unreachable if someone later copied the `eq` form onto
//! 1355.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::triggers::TriggerType;
use cimmeria_entity::missions::{MissionInstance, MissionObjective, MISSION_ACTIVE, STATUS_ACTIVE};
use tokio::sync::mpsc;

use super::super::super::executor::execute_actions;
use super::{
    actions_of, assert_no_deferred_actions, count_flag_ops, dialog_ctx, engine_for, fire,
    make_castle_space_mgr, step_ctx, BANG, JAFFA, LIVEWIRE, TAURI,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7183;
const PLAYER_ID: i32 = 7184;

/// Stage a player mid-708 on step 2417 with the step's REAL objective
/// set: 5184, 5185 and 5186, all three `is_optional = false`
/// (`mission_objectives.sql:6631/6633/6635`). Reproducing those flags is
/// the whole point of [`assert_report_advances_without_completing`]: if
/// 5184 and 5186 were optional, completing 5185 would trip
/// `all_required_complete` and end mission 708 three steps early.
///
/// Same scope caveat as `diagnosis.rs::stage_player_on_step_2415` — the
/// flags are HARD-CODED here, not read back from
/// `mission_objectives.sql`, so the guards catch a chain growing an extra
/// `complete_objective` row or an executor regression, NOT an
/// `is_optional` flip in the objectives seed.
fn stage_player_on_step_2417(mgr: &mut SpaceManager, archetype: i32) {
    mgr.create_entity(PLAYER_EID, "Castle", [810.0, 55.0, 515.0], [0.0; 3])
        .expect("Castle startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(archetype);
    p.missions.add_mission(MissionInstance::new(
        708,
        2417,
        [5184, 5185, 5186]
            .into_iter()
            .map(|objective_id| MissionObjective {
                objective_id,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            })
            .collect(),
    ));
    mgr.connect_entity(PLAYER_EID);
}

/// Chain 1352 is Tau'ri-only; chain 1354 is Jaffa-only. Both directions
/// of the cross-archetype negative are asserted, because the two failure
/// modes are different: a Jaffa seeing 5008 would report to the wrong
/// officer, while a Tau'ri seeing 5009 would be addressed as "Jaffa!"
/// by an ally.
#[tokio::test]
async fn the_report_dialogs_are_archetype_exclusive() {
    let pool = require_db_or_skip!();
    let marsh = engine_for(&pool, 1352).await;
    let mohkatan = engine_for(&pool, 1354).await;

    let mut tauri = step_ctx(2417, TAURI);
    tauri.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_ColMarsh"),
    );
    let resolved = fire(&marsh, TriggerType::InteractTag, &tauri);
    assert_no_deferred_actions(&resolved, 1352);
    let actions = actions_of(&resolved, 1352);
    assert_eq!(actions.len(), 1, "chain 1352 resolves one action");
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 5008 }),
        "a Tau'ri interacting with Col. Marsh at 2417 must see 5008; got {:?}",
        actions[0],
    );

    let mut jaffa_at_marsh = step_ctx(2417, JAFFA);
    jaffa_at_marsh.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_ColMarsh"),
    );
    assert!(
        actions_of(
            &fire(&marsh, TriggerType::InteractTag, &jaffa_at_marsh),
            1352
        )
        .is_empty(),
        "a Jaffa must never be shown dialog 5008 — the archetype gate on the \
         INTERACT chain is what makes the 5008/5009 dialog-id discrimination \
         server-authoritative",
    );

    let mut jaffa = step_ctx(2417, JAFFA);
    jaffa.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_Mohkatan"),
    );
    let resolved = fire(&mohkatan, TriggerType::InteractTag, &jaffa);
    assert_no_deferred_actions(&resolved, 1354);
    let actions = actions_of(&resolved, 1354);
    assert_eq!(actions.len(), 1, "chain 1354 resolves one action");
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 5009 }),
        "a Jaffa interacting with Moh'katan at 2417 must see 5009; got {:?}",
        actions[0],
    );

    let mut tauri_at_mohkatan = step_ctx(2417, TAURI);
    tauri_at_mohkatan.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_Mohkatan"),
    );
    assert!(
        actions_of(
            &fire(&mohkatan, TriggerType::InteractTag, &tauri_at_mohkatan),
            1354
        )
        .is_empty(),
        "a Tau'ri must never be shown dialog 5009",
    );
}

/// Neither report chain may fire outside step 2417. Marsh and Moh'katan
/// are permanent Checkpoint Alpha NPCs, so without the gate every walk
/// past them would replay the briefing and re-advance the mission.
#[tokio::test]
async fn the_report_dialogs_do_not_fire_outside_step_2417() {
    let pool = require_db_or_skip!();

    for (chain_id, tag, archetype) in [
        (1352, "Castle_ColMarsh", TAURI),
        (1354, "Castle_Mohkatan", JAFFA),
    ] {
        let engine = engine_for(&pool, chain_id).await;
        for step in [2415, 2416, 2418, 4462, 4469] {
            let mut ctx = step_ctx(step, archetype);
            ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
            assert!(
                actions_of(
                    &fire(&engine, TriggerType::InteractTag, &ctx),
                    chain_id as i64
                )
                .is_empty(),
                "chain {chain_id} must not resolve at step {step}",
            );
        }
    }
}

/// Chains 1353/1355: closing the briefing ticks exactly ONE report
/// objective, advances, clears that NPC's cue and arms the DHD.
///
/// "Exactly one" is the assertion that matters. Both 5185 and 5186 are
/// required, so completing both by hand would satisfy
/// `all_required_complete` the moment 5184 was also done and end 708 at
/// step 2417. The untaken branch is closed by `advance_step`'s
/// force-completion instead (`missions/progression.rs:57-66`), which
/// never runs the all-required check.
#[tokio::test]
async fn each_report_completes_only_its_own_objective_and_arms_the_dhd() {
    let pool = require_db_or_skip!();

    for (chain_id, dialog_id, mine, theirs, npc_tag) in [
        (1353, 5008, 5185, 5186, "Castle_ColMarsh"),
        (1355, 5009, 5186, 5185, "Castle_Mohkatan"),
    ] {
        let engine = engine_for(&pool, chain_id).await;
        let ctx = dialog_ctx(dialog_id, 2417);
        let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
        assert_no_deferred_actions(&resolved, chain_id as i64);
        let actions = actions_of(&resolved, chain_id as i64);

        assert_eq!(
            actions.len(),
            3,
            "chain {chain_id} must resolve three actions (objective, advance, \
             arm DHD); got {actions:?}",
        );
        assert!(
            matches!(
                actions[0],
                Action::CompleteObjective { mission_id: 708, objective_id: o } if *o == mine
            ),
            "chain {chain_id} must complete objective {mine} first; got {:?}",
            actions[0],
        );
        assert!(
            !actions.iter().any(|a| matches!(
                a,
                Action::CompleteObjective { objective_id: o, .. } if *o == theirs
            )),
            "chain {chain_id} must NEVER complete the other faction's objective \
             {theirs} — both are required, so completing both ends mission 708 \
             at step 2417; got {actions:?}",
        );
        assert!(
            matches!(
                actions[1],
                Action::AdvanceStep {
                    mission_id: 708,
                    step_id: 2418
                }
            ),
            "chain {chain_id} must advance to 2418; got {:?}",
            actions[1],
        );
        // The report NPC's '!' must SURVIVE. Templates 10 (Marsh) and 54
        // (Moh'katan) both ship `interaction_type = 0`
        // (`entity_templates.sql:25/63`), which is the spawn-time value of
        // `interaction_type_flags` (`space_manager/spawn.rs:127`), so
        // clearing the cue drops the NPC to flags 0 — no right-click
        // affordance for a second player still on step 2417, and no
        // recovery short of a relog into chain 1362. Packet CA09 asks for
        // the clear; the zero-baseline rule in the seed's engine fact (7)
        // overrides it for the same reason the Access Panel glow is never
        // cleared.
        assert_eq!(
            count_flag_ops(&actions, npc_tag, "~", BANG),
            0,
            "chain {chain_id} must NOT clear {npc_tag}'s '!' cue — {npc_tag}'s \
             template baseline is 0, so the clear would strip its only \
             clickable bit zone-wide; got {actions:?}",
        );
        assert_eq!(
            count_flag_ops(&actions, "Castle_DHD", "|", LIVEWIRE),
            1,
            "chain {chain_id} must arm the DHD's Livewire affordance — step 2418 \
             is 'Repair the DHD' and chain 1356 is otherwise unreachable; \
             got {actions:?}",
        );
    }
}

/// Neither report chain may answer the other's dialog, and neither may
/// fire outside step 2417.
#[tokio::test]
async fn report_dialog_choices_are_step_gated_and_do_not_cross_over() {
    let pool = require_db_or_skip!();
    let marsh = engine_for(&pool, 1353).await;
    let mohkatan = engine_for(&pool, 1355).await;

    assert!(
        actions_of(
            &fire(
                &mohkatan,
                TriggerType::DialogChoice,
                &dialog_ctx(5008, 2417)
            ),
            1355
        )
        .is_empty(),
        "chain 1355 must not answer dialog 5008",
    );
    assert!(
        actions_of(
            &fire(&marsh, TriggerType::DialogChoice, &dialog_ctx(5009, 2417)),
            1353
        )
        .is_empty(),
        "chain 1353 must not answer dialog 5009",
    );

    for step in [2416, 2418, 4462] {
        assert!(
            actions_of(
                &fire(&marsh, TriggerType::DialogChoice, &dialog_ctx(5008, step)),
                1353
            )
            .is_empty(),
            "chain 1353 must not resolve at step {step}",
        );
    }
}

/// Execute one report route end to end and assert mission 708 is still
/// ACTIVE on step 2418 with `objective_id` recorded complete.
///
/// This is the regression guard for the data-dependent hazard both
/// advisory passes flagged. `complete_objective`'s auto-complete branch
/// fires when every NON-OPTIONAL objective of the current step is
/// complete (`missions/progression.rs:176-183`). Today step 2417 carries
/// three required objectives, so ticking one leaves two active and the
/// branch is not reached. Flip any of 5184/5185/5186 to
/// `is_optional = true` in `mission_objectives.sql` and this fails with
/// the mission completed at step 2417 — three steps before the player
/// ever reaches the Stargate.
///
/// Run for BOTH factions. The two routes tick different objectives out
/// of the same required trio, so a flip of 5184 alone would be caught by
/// either, but a flip of 5185 is only visible on the Tau'ri route and a
/// flip of 5186 only on the Jaffa one.
async fn assert_report_advances_without_completing(
    pool: &sqlx::PgPool,
    chain_id: i32,
    dialog_id: i32,
    archetype: i32,
    objective_id: i32,
) {
    let engine = engine_for(pool, chain_id).await;

    let mut mgr = make_castle_space_mgr();
    stage_player_on_step_2417(&mut mgr, archetype);
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();

    let resolved = fire(
        &engine,
        TriggerType::DialogChoice,
        &dialog_ctx(dialog_id, 2417),
    );
    assert!(
        !resolved.actions.is_empty(),
        "chain {chain_id} must resolve — an empty list would make the assertions \
         below vacuously true",
    );
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let mission = mgr
        .get_entity(PLAYER_EID)
        .and_then(|e| e.missions.get_mission(708))
        .cloned()
        .expect("mission 708 must still be tracked on the player");

    assert_eq!(
        mission.status, MISSION_ACTIVE,
        "mission 708 must still be ACTIVE after reporting in via chain \
         {chain_id}. A `completed` here means a step-2417 objective was flipped \
         to optional and `complete_objective`'s all-required check now fires on \
         {objective_id}.",
    );
    assert_eq!(
        mission.current_step_id,
        Some(2418),
        "the player must be on step 2418 (Repair the DHD) after reporting in",
    );
    assert!(
        mission.completed_objectives.contains(&objective_id),
        "objective {objective_id} must be recorded completed; got {:?}",
        mission.completed_objectives,
    );

    // Drain so a full channel can't mask a send failure in a later run.
    while rx.try_recv().is_ok() {}
}

/// Executed guard for the Tau'ri report (chain 1353, objective 5185).
#[tokio::test]
async fn reporting_in_advances_the_step_without_completing_the_mission() {
    let pool = require_db_or_skip!();
    assert_report_advances_without_completing(&pool, 1353, 5008, TAURI, 5185).await;
}

/// Executed guard for the Jaffa report (chain 1355, objective 5186).
/// The Jaffa branch is the one nothing else executes, and it is the
/// branch a mis-copied `archetype eq 8` condition would silently kill —
/// see [`archetype_conditions_are_never_placed_on_the_dialog_halves`].
#[tokio::test]
async fn the_jaffa_report_advances_without_completing_the_mission() {
    let pool = require_db_or_skip!();
    assert_report_advances_without_completing(&pool, 1355, 5009, JAFFA, 5186).await;
}

/// Guard for the invisible failure described in this module's docs: the
/// `dialog_choice` halves of the split must carry NO `archetype`
/// condition. Read straight off the seed rows the loader produced,
/// because a wrong condition here would not change any observable
/// behaviour for a Tau'ri — it would only silently kill the Jaffa
/// branch if the `eq` form were used.
#[tokio::test]
async fn archetype_conditions_are_never_placed_on_the_dialog_halves() {
    use cimmeria_content_engine::conditions::Condition;

    let pool = require_db_or_skip!();

    for chain_id in [1343, 1345, 1353, 1355] {
        let chain = super::super::super::engine_loader::load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

        assert!(
            !chain
                .conditions
                .iter()
                .any(|c| matches!(c, Condition::Archetype { .. })),
            "chain {chain_id} is a `dialog_choice` chain and must carry NO \
             archetype condition: `fire_dialog_choice` never populates \
             `archetype`, so `eq` is permanently false (branch dead) and `neq` \
             is permanently true (guard fails open). The archetype gate belongs \
             on the interact chain that displays the dialog.",
        );
    }

    // ...and the interact halves MUST carry one, or the dialog-id
    // discrimination the choice halves rely on has nothing behind it.
    for chain_id in [1352, 1354] {
        let chain = super::super::super::engine_loader::load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

        assert!(
            chain
                .conditions
                .iter()
                .any(|c| matches!(c, Condition::Archetype { .. })),
            "chain {chain_id} displays a faction-specific report dialog and MUST \
             carry an archetype condition — it is the only place the split can \
             be enforced",
        );
    }
}
