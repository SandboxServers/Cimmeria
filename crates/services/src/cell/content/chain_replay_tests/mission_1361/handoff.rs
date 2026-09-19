//! 5. Playtest finding H9, in its `player_loaded` form
//!
//! Split out of the former single-file `mission_1361.rs`; the shared
//! context builders, constants and resolve helpers stay in [`super`].

use super::*;

/// Resolve one event against a **specific trigger expansion** of a chain,
/// with `enabled` forced true.
///
/// Two reasons this cannot use [`resolve_one`]. First,
/// `load_single_chain_for_test` returns only the first expansion, so a
/// chain with two trigger rows would silently go half-unasserted. Second,
/// the acceptance trio ships `enabled = false` pending M0, and
/// `resolve_event` filters on `enabled` before it evaluates anything — so
/// a parked chain's *logic* can only be exercised by flipping the flag the
/// way M0 will. `acceptance_chains_resolve_nothing_while_parked` is the
/// test that the shipped rows really are inert; this one is the test that
/// they will be correct when they are not.
async fn resolve_expansion_as_if_enabled(
    pool: &sqlx::PgPool,
    chain_id: i32,
    tt: TriggerType,
    ctx: &ExecutionContext,
) -> ResolvedActions {
    let expansions = load_chain_expansions_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"));
    assert!(
        !expansions.is_empty(),
        "chain {chain_id} must exist in seeded content_chains AND load cleanly"
    );

    let mut engine = ChainEngine::new();
    let mut registered = 0;
    for mut chain in expansions {
        if chain.trigger.trigger_type() != tt {
            continue;
        }
        chain.enabled = true;
        engine.register_chain(chain);
        registered += 1;
    }
    assert_eq!(
        registered, 1,
        "chain {chain_id} must have exactly one {tt:?} trigger row; found {registered}"
    );

    let event = TriggerEvent {
        trigger_type: tt,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// Chain 6511 carries **two** trigger rows, and the second is what makes
/// Meet The Praxis reachable at all on the normal first visit.
///
/// This is playtest finding H9 in its `player_loaded` form: a chain keyed
/// on an edge never fires when its gate opens while the player is already
/// past that edge. 6511's gate includes
/// `step_status 1360/4038 neq active`, which opens the instant chain 6501
/// completes mission 1360 — inside world 68, with no second world entry
/// coming. And 6501's own `remove_dialog_set 5356` clears the only bind on
/// template slot 10 in the same click, so Marsh would go completely
/// unclickable: `entity_templates` row 10 ships `interaction_type = 0` and
/// `static_interaction_sets = '{}'`, meaning the client stops registering
/// an interaction and the right-click that fires 6512 is never sent. The
/// offer would be unreachable until some unrelated world crossing.
///
/// Both expansions are asserted to produce the identical bind: they share
/// one condition and action list by construction
/// (`build_chains_from_rows` clones both per trigger row), and an edit
/// that split them would be a silent divergence.
#[tokio::test]
async fn chain_6511_offers_the_praxis_the_moment_the_letter_is_handed_over() {
    let pool = require_db_or_skip!();

    let expansions = load_chain_expansions_for_test(&pool, 6511)
        .await
        .expect("DB query for chain 6511 must succeed");
    assert_eq!(
        expansions.len(),
        2,
        "chain 6511 must carry two trigger rows — `player_loaded` for the world entry \
         and `mission_completed 1360` for the in-place case. Got: {:?}",
        expansions.iter().map(|c| &c.trigger).collect::<Vec<_>>()
    );

    let expected = vec![Action::AddDialogSet {
        dialog_set_id: 5254,
        slot: 10,
        mission_id: Some(1361),
    }];

    // The in-place case. This is the context `fire_mission_completed`
    // builds: it calls `populate_mission_context` AFTER
    // `complete_mission_direct`, so 1360 reads `completed` and step 4038
    // has moved into `completed_steps`.
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param("mission_id".to_string(), serde_json::json!(1360));
    ctx.set_param(
        "archetype".to_string(),
        serde_json::json!(ARCHETYPE_SOLDIER),
    );
    ctx.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("completed"),
    );
    let got = actions_of(
        &resolve_expansion_as_if_enabled(&pool, 6511, TriggerType::MissionCompleted, &ctx).await,
    );
    assert_eq!(
        got, expected,
        "handing Frost's letter to Marsh must put the Praxis offer on him in the same \
         click — chain 6501 cleared slot 10 and no second `player_loaded` is coming"
    );

    // The world-entry case still works and binds the same thing.
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
    let got = actions_of(
        &resolve_expansion_as_if_enabled(&pool, 6511, TriggerType::PlayerLoaded, &ctx).await,
    );
    assert_eq!(
        got, expected,
        "the `player_loaded` expansion must bind the same dsm as the \
         `mission_completed` one — they share one action list by construction"
    );
}

/// The `mission_completed` expansion of 6511 is gated exactly as tightly
/// as the `player_loaded` one.
///
/// A second trigger row widens a chain's reach, and the risk is that it
/// reaches somewhere the author did not mean: another mission's
/// completion, a player who already has 1361, a Jaffa or a Goa'uld, or a
/// player standing in Harset. Each is asserted silent.
#[tokio::test]
async fn chain_6511_second_trigger_is_gated_as_tightly_as_the_first() {
    let pool = require_db_or_skip!();

    // (label, completing mission, 1361 status, archetype, world)
    let cases: [(&str, i64, &str, i64, i32); 6] = [
        (
            "a different mission completed",
            1362,
            "not_active",
            ARCHETYPE_SOLDIER,
            CMD_CENTER,
        ),
        (
            "1361 already active",
            1360,
            "active",
            ARCHETYPE_SOLDIER,
            CMD_CENTER,
        ),
        (
            "1361 already completed",
            1360,
            "completed",
            ARCHETYPE_SOLDIER,
            CMD_CENTER,
        ),
        ("Jaffa", 1360, "not_active", ARCHETYPE_JAFFA, CMD_CENTER),
        ("Goa'uld", 1360, "not_active", ARCHETYPE_GOAULD, CMD_CENTER),
        (
            "right state, wrong world",
            1360,
            "not_active",
            ARCHETYPE_SOLDIER,
            HARSET,
        ),
    ];

    for (label, completing, m1361, archetype, world) in cases {
        let mut ctx = ExecutionContext::new();
        ctx.world_id = Some(world);
        ctx.set_param("mission_id".to_string(), serde_json::json!(completing));
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
        ctx.set_param("mission_1361_status".to_string(), serde_json::json!(m1361));
        ctx.set_param(
            "mission_1360_step_4038_status".to_string(),
            serde_json::json!("completed"),
        );

        let resolved =
            resolve_expansion_as_if_enabled(&pool, 6511, TriggerType::MissionCompleted, &ctx).await;
        assert!(
            resolved.actions.is_empty(),
            "chain 6511's `mission_completed` expansion must resolve nothing ({label}); \
             got {:?}",
            resolved.actions
        );
    }
}

/// Every other bind, offer and restore chain in this packet was walked for
/// the same H9 shape — "the gate opens in the world the edge fires in" —
/// and this test pins the answer so a future edit cannot quietly
/// reintroduce it.
///
/// For each `player_loaded` chain, the state that opens its gate is
/// produced by some *other* chain. If that other chain runs in a
/// **different** world, the crossing itself re-fires `player_loaded` and
/// the restore chain is sufficient. If it runs in the **same** world, the
/// restore chain can never fire in time and the handing chain must make
/// the bind itself (or a second trigger row must exist).
///
/// | Restore | Gate opened by | Same world? | Covered by |
/// |---|---|---|---|
/// | 6514 (4040) | 6513 `accept_mission`, world 68 | yes | 6513's in-chain `add_dialog_set 6397` |
/// | 6516 (4041) | 6515, world 68 → Hansen is in 57 | no | the 68→57 crossing |
/// | 6519 (4042) | 6518, world 57 → Moh'katan is in 68 | no | the 57→68 crossing |
/// | 6521 (4043) | 6520, world 68 | yes | 6520's in-chain `add_dialog_set 6395` |
/// | 6523 (4693) | 6522, world 68 | yes | 6522's in-chain `add_dialog_set 6396` |
/// | 6526 (4694) | 6525, world 68 | yes | 6525's in-chain `add_dialog_set 5253` |
///
/// The two cross-world rows are the ones that must **not** bind in-chain
/// (the bind dies with the cell entity on `cross_world_teleport`), and
/// `chain_6515_advances_from_mohkatan_to_hansen` /
/// `chain_6518_advances_from_hansen_to_the_delivery` already pin their
/// exact action lists. This test pins the other four: the chain that opens
/// each same-world gate must carry the matching bind.
#[tokio::test]
async fn every_same_world_step_handoff_binds_in_chain() {
    let pool = require_db_or_skip!();

    // (handing chain, trigger, step it is fired on, key, dsm it must bind, slot)
    let cases: [(i32, TriggerType, &str, &str, i32, i32); 3] = [
        (
            6520,
            TriggerType::InteractTag,
            "4042",
            "CmdCenter_Mohkatan",
            6395,
            42,
        ),
        (
            6522,
            TriggerType::InteractTag,
            "4043",
            "CmdCenter_Baal",
            6396,
            43,
        ),
        (6525, TriggerType::DialogChoice, "4693", "4462", 5253, 10),
    ];

    for (chain_id, tt, step, key, dsm, slot) in cases {
        let base = praxis_ctx(CMD_CENTER, step);
        let ctx = match tt {
            TriggerType::DialogChoice => with_dialog(base, key.parse().unwrap()),
            _ => with_tag(base, key),
        };
        let got = actions_of(&resolve_one(&pool, chain_id, &ctx, tt).await);
        assert!(
            got.contains(&Action::AddDialogSet {
                dialog_set_id: dsm,
                slot,
                mission_id: Some(1361)
            }),
            "chain {chain_id} advances to a step whose NPC is in the SAME world (68), so \
             the `player_loaded` restore chain can never fire in time — this chain must \
             bind dsm {dsm} to slot {slot} itself. Got: {got:?}"
        );
    }

    // 6513 is the fourth same-world hand-off, but it ships disabled with
    // the acceptance trio, so it needs the enabled-forcing helper.
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param("dialog_id".to_string(), serde_json::json!(4457));
    let got = actions_of(
        &resolve_expansion_as_if_enabled(&pool, 6513, TriggerType::DialogChoice, &ctx).await,
    );
    assert!(
        got.contains(&Action::AddDialogSet {
            dialog_set_id: 6397,
            slot: 54,
            mission_id: Some(1361)
        }),
        "chain 6513 accepts 1361 and points at Moh'katan, who is in the same world — it \
         must bind dsm 6397 in-chain, not leave it to restore chain 6514. Got: {got:?}"
    );
}

/// **Anti-vacuity guard for every negative in this file.**
///
/// `resolve_event` checks `chain.trigger.matches(event)` before it
/// evaluates a single condition, and the keyed triggers read their key out
/// of the event params: `OnInteractTag` wants `entity_tag`,
/// `OnDialogChoice` wants `dialog_id`, `OnMissionCompleted` wants
/// `mission_id`. A negative test that builds its context without the right
/// key therefore resolves nothing for a reason that has nothing to do with
/// the gate it claims to test — it passes, it keeps passing when the gate
/// is deleted, and it is worth nothing.
///
/// This asserts that a context carrying the key each chain expects, and
/// satisfying its gate, really does resolve actions. If a key name drifts
/// or a trigger is rewritten, the chain stops firing here rather than
/// quietly turning every negative in the file green.
#[tokio::test]
async fn every_keyed_trigger_matches_the_context_the_negatives_build() {
    let pool = require_db_or_skip!();

    // (chain, trigger, step, the key its negatives build the context with)
    let cases: [(i32, TriggerType, &str, &str); 6] = [
        (6515, TriggerType::InteractTag, "4040", "CmdCenter_Mohkatan"),
        (6518, TriggerType::DialogChoice, "4041", "4459"),
        (6520, TriggerType::InteractTag, "4042", "CmdCenter_Mohkatan"),
        (6522, TriggerType::InteractTag, "4043", "CmdCenter_Baal"),
        (6525, TriggerType::DialogChoice, "4693", "4462"),
        (6527, TriggerType::InteractTag, "4694", "CmdCenter_Marsh"),
    ];

    for (chain_id, tt, step, key) in cases {
        let world = if chain_id == 6518 { HARSET } else { CMD_CENTER };
        let base = praxis_ctx(world, step);
        let ctx = match tt {
            TriggerType::DialogChoice => with_dialog(base, key.parse().unwrap()),
            _ => with_tag(base, key),
        };

        let chain = load(&pool, chain_id).await;
        let event = TriggerEvent {
            trigger_type: tt.clone(),
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        assert!(
            chain.trigger.matches(&event),
            "chain {chain_id}'s trigger does not match a context built with key \
             '{key}'. Every negative test for this chain builds its context the same \
             way, so they are all passing vacuously — failing on the trigger, never \
             reaching the condition they claim to guard."
        );

        assert!(
            !resolve_one(&pool, chain_id, &ctx, tt)
                .await
                .actions
                .is_empty(),
            "chain {chain_id} must resolve actions for a context that satisfies its \
             gate; if it does not, its negatives prove nothing"
        );
    }
}
