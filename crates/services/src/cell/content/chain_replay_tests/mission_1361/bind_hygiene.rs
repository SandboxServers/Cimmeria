//! 4. Bind hygiene
//!
//! Split out of the former single-file `mission_1361.rs`; the shared
//! context builders, constants and resolve helpers stay in [`super`].

use super::*;

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
/// for Hansen and Anat. Adding one would dead-end the mission:
/// `fire_interact_tag` short-circuits `interactions::handle_interact`,
/// which is the only code that opens a *bound* dsm's dialog. Hansen's
/// 4459 and Anat's 4462 are exactly those bound dialogs, and each carries
/// the button (`Convince Hansen.` / `Flatter Anat.`) whose `dialog_choice`
/// drives chains 6518 and 6525. Suppress the dialog and the button never
/// renders, so the step can never advance.
///
/// (An earlier version of this note blamed `last_interaction_target`
/// instead. That is no longer the mechanism — the pin now happens in
/// `cell_methods/player/interaction/interact.rs` ahead of all chain
/// dispatch — but the guard itself stands on the stronger reason above.)
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
             use the BIND path deliberately. `fire_interact_tag` short-circuits \
             `interactions::handle_interact`, which is the only code that opens a \
             BOUND dsm's dialog — and dialogs 4459 and 4462 are exactly those bound \
             dialogs, each carrying the button whose `dialog_choice` drives the step. \
             Adding this chain suppresses the dialog, so the button never renders and \
             the step can never advance. See harset_opcore_chains.sql note (C)."
        );
    }
}
