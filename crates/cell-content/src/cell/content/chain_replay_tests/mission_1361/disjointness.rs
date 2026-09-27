//! 2. Cross-chain disjointness
//!
//! Split out of the former single-file `mission_1361.rs`; the shared
//! context builders, constants and resolve helpers stay in [`super`].

use super::*;

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
/// and the client — which holds one non-tutorial dialog at a time —
/// would discard the first unread, so the player would see only one blurb
/// while both chains' state changes landed. Silent content loss.
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
        ctx.set_param(
            "archetype".to_string(),
            serde_json::json!(ARCHETYPE_SOLDIER),
        );
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
/// `interactions::handle_interact` scans
/// `available_interactions[template_id]` with `find_map`, taking the first
/// entry whose `dialog_id` is non-NULL — so a second live dialog-carrying
/// bind on one slot makes one of them permanently unreachable. Slot 10
/// (Col. Marsh, template 10) is the one at risk: mission 1360's letter
/// bind (dsm 5356), the Praxis offer (5254) and the Praxis turn-in (5253)
/// all target it, and all three carry a real dialog.
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
        ctx.set_param(
            "archetype".to_string(),
            serde_json::json!(ARCHETYPE_SOLDIER),
        );
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
                 dialog sets ({dsms:?}) to template slot {slot}. \
                 `interactions::handle_interact` takes the first bind carrying a \
                 non-NULL dialog and ignores the rest, so the others would be \
                 permanently unreachable — see harset_opcore_chains.sql note (D).",
                dsms.len()
            );
        }
    }
}

/// The hand-back that chain 6502's DISJOINTNESS row makes necessary.
///
/// 6502 stands down while 1361's turn-in step 4694 is active, so a player
/// holding Frost's letter through the Praxis debrief has no bind on
/// template slot 10 the moment 6527 completes 1361. `player_loaded` will
/// not fire again — they are standing still in the Command Center — and
/// template 10 ships `interaction_type = 0` with
/// `static_interaction_sets = '{}'`, so with no bind the client never
/// registers an interaction on Marsh and the right-click that would fire
/// chain 6501 is never sent. The letter turn-in would be unreachable
/// until the player crossed a world boundary.
///
/// Chain 6505 closes that on the `mission_completed 1361` event. This test
/// pins both halves: it fires for a player who still owes the letter, and
/// it stays silent for one who does not (the overwhelmingly common case,
/// where a stray bind would put a "!" on Marsh with nothing behind it).
#[tokio::test]
async fn chain_6505_hands_the_letter_indicator_back_when_1361_completes() {
    let pool = require_db_or_skip!();

    // Positive: 1360 still active on step 4038 when 1361 completes.
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    ctx.set_param("mission_id".to_string(), serde_json::json!(1361));
    ctx.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("active"),
    );
    let got = actions_of(&resolve_one(&pool, 6505, &ctx, TriggerType::MissionCompleted).await);
    assert_eq!(
        got,
        vec![Action::AddDialogSet {
            dialog_set_id: 5356,
            slot: 10,
            mission_id: Some(1360)
        }],
        "completing 1361 while Frost's letter is still undelivered must re-bind dsm \
         5356 to template slot 10, or Marsh goes unclickable and mission 1360 \
         dead-ends until the player crosses a world boundary"
    );

    // Negatives: no letter owed, and the wrong world.
    for (label, m1360, s4038, world) in [
        (
            "letter already delivered",
            "completed",
            "completed",
            CMD_CENTER,
        ),
        ("1360 never started", "not_active", "not_active", CMD_CENTER),
        ("right state, wrong world", "active", "active", HARSET),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.world_id = Some(world);
        ctx.set_param("mission_id".to_string(), serde_json::json!(1361));
        ctx.set_param("mission_1360_status".to_string(), serde_json::json!(m1360));
        ctx.set_param(
            "mission_1360_step_4038_status".to_string(),
            serde_json::json!(s4038),
        );
        assert!(
            resolve_one(&pool, 6505, &ctx, TriggerType::MissionCompleted)
                .await
                .actions
                .is_empty(),
            "chain 6505 must resolve nothing ({label}): a bind here paints a \"!\" on a \
             shared-hub NPC with no chain behind the click"
        );
    }
}

/// Chain 6505 is scoped to mission 1361's completion specifically, not to
/// "any mission completing in the Command Center".
///
/// `Trigger::OnMissionCompleted` carries a `mission_id` key, and
/// `fire_mission_completed` stamps the completing mission into
/// `ctx.params["mission_id"]`. If 6505's event_key drifted (or were left
/// NULL), completing *any* mission while the letter is owed would re-run
/// the bind — harmless once, but it would also mask a regression in
/// 6502's gate by papering over it on every unrelated completion.
#[tokio::test]
async fn chain_6505_only_answers_to_1361() {
    let pool = require_db_or_skip!();

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(CMD_CENTER);
    // A different mission completing in the same world with the same
    // letter state.
    ctx.set_param("mission_id".to_string(), serde_json::json!(1360));
    ctx.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_1360_step_4038_status".to_string(),
        serde_json::json!("active"),
    );

    assert!(
        resolve_one(&pool, 6505, &ctx, TriggerType::MissionCompleted)
            .await
            .actions
            .is_empty(),
        "chain 6505 must key on mission 1361's completion only; it fired for 1360"
    );
}
