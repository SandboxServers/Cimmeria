//! 1. Ordered progression
//!
//! Split out of the former single-file `mission_1361.rs`; the shared
//! context builders, constants and resolve helpers stay in [`super`].

use super::*;

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
        (
            6515,
            "4040",
            TriggerType::InteractTag,
            "CmdCenter_Mohkatan",
            CMD_CENTER,
        ),
        (6518, "4041", TriggerType::DialogChoice, "4459", HARSET),
        (
            6520,
            "4042",
            TriggerType::InteractTag,
            "CmdCenter_Mohkatan",
            CMD_CENTER,
        ),
        (
            6522,
            "4043",
            TriggerType::InteractTag,
            "CmdCenter_Baal",
            CMD_CENTER,
        ),
        (6525, "4693", TriggerType::DialogChoice, "4462", CMD_CENTER),
        (
            6527,
            "4694",
            TriggerType::InteractTag,
            "CmdCenter_Marsh",
            CMD_CENTER,
        ),
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
            // `TriggerType` is `Clone` but not `Copy`, and `tt` is bound
            // once by the outer `for`, so it has to be cloned per inner
            // iteration rather than moved.
            let resolved = resolve_one(&pool, chain_id, &ctx, tt.clone()).await;
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
