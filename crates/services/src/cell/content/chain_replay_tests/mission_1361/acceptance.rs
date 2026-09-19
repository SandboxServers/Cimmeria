//! 3. The parked acceptance path
//!
//! Split out of the former single-file `mission_1361.rs`; the shared
//! context builders, constants and resolve helpers stay in [`super`].

use super::*;

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
            chain.enabled, door.enabled,
            "chain {chain_id} (1361 acceptance) is enabled={} but the 68->57 return \
             door chain 6007 is enabled={}. These must move together: step 4041 is at \
             Hansen in world 57 while 4040/4042 are in world 68, so accepting 1361 \
             without a working return leg soft-sticks the player at 4041 forever, and \
             opening the door without enabling acceptance leaves the mission \
             unreachable. M0 flips 6007 (harset_space_chains.sql, after pinning its \
             arrival coordinate) and 6511/6512/6513 in the same change.",
            chain.enabled, door.enabled,
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
