//! 3. The acceptance path and its abandon twin
//!
//! Split out of the former single-file `mission_1361.rs`; the shared
//! context builders, constants and resolve helpers stay in [`super`].
//!
//! H31 shipped the acceptance trio parked behind the 68 -> 57 return door;
//! placement PL-A-06/07 opened that door and unparked them, and added the
//! abandon twin H54 could not write while the mission was unacceptable.

use super::*;

/// The acceptance trio's enablement is tied to chain 6007 (the 68 -> 57
/// Command Center return door) as a **biconditional**, and since placement
/// PL-A-07 the abandon twin 6528 is inside it too.
///
/// 1361 sends the player from world 68 to Hansen in world 57 and back. If
/// acceptance were live while the return door is dark, every player who
/// accepted would soft-stick at step 4041 with no recovery: there is still no
/// `fail_objective` executor arm. If the door were opened without enabling
/// acceptance, the mission would be silently unreachable instead. And an
/// abandon twin that is live while the mission cannot be accepted is
/// unreachable code, while one that is dark while the mission *can* be
/// abandoned strands whatever bind the abandon left behind — which is the
/// gap H54 recorded and could not close.
///
/// Every one of those drifts fails here. All four chains move in one change.
#[tokio::test]
async fn praxis_acceptance_is_enabled_iff_the_return_door_is() {
    let pool = require_db_or_skip!();

    let door = load(&pool, 6007).await;
    for chain_id in [6511, 6512, 6513, 6528] {
        let chain = load(&pool, chain_id).await;
        assert_eq!(
            chain.enabled, door.enabled,
            "chain {chain_id} (1361 acceptance / abandon) is enabled={} but the \
             68->57 return door chain 6007 is enabled={}. These must move together: \
             step 4041 is at Hansen in world 57 while 4040/4042 are in world 68, so \
             accepting 1361 without a working return leg soft-sticks the player at \
             4041 forever; opening the door without enabling acceptance leaves the \
             mission unreachable; and the abandon twin 6528 only has anything to do \
             while the mission can be accepted. Flip 6007 \
             (harset_space_chains.sql) and 6511/6512/6513/6528 \
             (harset_opcore_chains.sql) in the same change.",
            chain.enabled, door.enabled,
        );
    }
}

/// Now the door is open (placement PL-A-06/07), each acceptance chain
/// resolves its own action under the context that satisfies every one of its
/// conditions.
///
/// This replaces H31's `acceptance_chains_resolve_nothing_while_parked`,
/// which asserted the opposite. Inverting it rather than deleting it matters:
/// "resolves nothing" was satisfied by a *structurally dead* chain just as
/// well as by a disabled one, so the flip from `false` to `true` would have
/// left three chains that still did nothing with the suite green. The
/// per-chain action assertions below are what close that.
#[tokio::test]
async fn acceptance_chains_resolve_their_actions_now_the_door_is_open() {
    let pool = require_db_or_skip!();

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Harset_CmdCenter"),
    );
    ctx.set_param(
        "archetype".to_string(),
        serde_json::json!(ARCHETYPE_SOLDIER),
    );
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

    // 6511: the offer bind on Marsh.
    let resolved = resolve_one(&pool, 6511, &ctx, TriggerType::PlayerLoaded).await;
    assert!(
        resolved.actions.iter().any(|(id, a)| *id == 6511
            && matches!(
                a,
                Action::AddDialogSet {
                    dialog_set_id: 5254,
                    slot: 10,
                    ..
                }
            )),
        "chain 6511 must bind dsm 5254 on slot 10 (Marsh's offer \"?\"); got {:?}",
        resolved.actions
    );

    // 6512: the briefing dialog. Asserted as 4457 specifically — 4456 is the
    // version whose "More Info" button is indistinguishable from "Accept" at
    // the `dialog_choice` trigger, so displaying it would let a player accept
    // by asking a question.
    let resolved = resolve_one(&pool, 6512, &ctx, TriggerType::InteractTag).await;
    assert!(
        resolved.actions.iter().any(|(id, a)| *id == 6512
            && matches!(
                a,
                Action::DisplayDialog {
                    dialog_id: 4457,
                    ..
                }
            )),
        "chain 6512 must display dialog 4457, not 4456; got {:?}",
        resolved.actions
    );

    // 6513: accept, and hand the indicator from Marsh to Moh'katan.
    let resolved = resolve_one(&pool, 6513, &ctx, TriggerType::DialogChoice).await;
    let own: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 6513)
        .map(|(_, a)| a)
        .collect();
    assert!(
        own.iter().any(|a| matches!(
            a,
            Action::AcceptMission {
                mission_id: 1361,
                ..
            }
        )),
        "chain 6513 must accept mission 1361; got {own:?}"
    );
    assert!(
        own.iter().any(|a| matches!(
            a,
            Action::RemoveDialogSet {
                dialog_set_id: 5254,
                slot: 10,
                ..
            }
        )),
        "chain 6513 must drop the offer bind from Marsh; got {own:?}"
    );
    assert!(
        own.iter().any(|a| matches!(
            a,
            Action::AddDialogSet {
                dialog_set_id: 6397,
                slot: 54,
                ..
            }
        )),
        "chain 6513 must bind dsm 6397 on slot 54 (Moh'katan's \"!\"); got {own:?}"
    );
}

/// The abandon twin (chain 6528, placement PL-A-07) clears every world-68
/// bind mission 1361 can hold and repaints Marsh's offer — with the repaint
/// **last**.
///
/// Ordering is the load-bearing part, not the membership.
/// `interactions/dispatch/interact.rs` takes the FIRST bound entry on a
/// template slot that carries a dialog, so if `add_dialog_set 5254` ran
/// before `remove_dialog_set 5253`, slot 10 would keep replaying the turn-in
/// debrief and the mission could never be re-offered. Asserting the
/// `sort_order` of the add against every remove on the same slot is what
/// catches a re-ordering that leaves the action set unchanged.
#[tokio::test]
async fn the_1361_abandon_twin_clears_before_it_repaints() {
    let pool = require_db_or_skip!();

    let rows: Vec<(String, i32, serde_json::Value, i32)> = sqlx::query_as(
        "SELECT action_type, target_id, params, sort_order \
         FROM resources.content_actions WHERE chain_id = 6528 ORDER BY sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("chain 6528 must have action rows — placement PL-A-07 seeded them");

    let slot = |p: &serde_json::Value| p.get("slot").and_then(|s| s.as_i64()).unwrap_or(-1);

    // Every dsm 1361 binds on a world-68 template must be cleared. Slot 212
    // (Hansen) is deliberately absent: he is in world 57 and this chain is
    // gated on 68, so `available_interactions` has already dropped it.
    let cleared: Vec<(i32, i64)> = rows
        .iter()
        .filter(|(t, ..)| t == "remove_dialog_set")
        .map(|(_, id, p, _)| (*id, slot(p)))
        .collect();
    for want in [(5253, 10), (6397, 54), (6398, 54), (6395, 42), (6396, 43)] {
        assert!(
            cleared.contains(&want),
            "chain 6528 must clear dsm {} from slot {} — an abandon leaves the \
             step unknowable, so every possible bind is cleared \
             unconditionally. Cleared: {cleared:?}",
            want.0,
            want.1
        );
    }

    let repaint = rows
        .iter()
        .find(|(t, id, p, _)| t == "add_dialog_set" && *id == 5254 && slot(p) == 10)
        .expect("chain 6528 must repaint Marsh's offer (dsm 5254 on slot 10)");
    let last_clear_on_slot_10 = rows
        .iter()
        .filter(|(t, _, p, _)| t == "remove_dialog_set" && slot(p) == 10)
        .map(|(.., so)| *so)
        .max()
        .expect("chain 6528 must clear slot 10 before repainting it");
    assert!(
        repaint.3 > last_clear_on_slot_10,
        "chain 6528's `add_dialog_set 5254` runs at sort_order {} but a \
         `remove_dialog_set` on slot 10 runs at {}. The repaint must come \
         LAST: `handle_interact` takes the first dialog-carrying bind on the \
         slot, so a surviving 5253 would keep replaying the turn-in debrief \
         and Marsh could never re-offer the mission.",
        repaint.3,
        last_clear_on_slot_10
    );
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
        ctx.set_param(
            "archetype".to_string(),
            serde_json::json!(ARCHETYPE_SOLDIER),
        );
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
