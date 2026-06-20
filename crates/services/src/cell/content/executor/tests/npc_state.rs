//! NPC-state executor coverage — aggression, threat generation
//! (appearance-before-state ordering), POI / follow-target / AI-state
//! transitions driven by content chains.

use super::*;

/// `Action::SetAggression { level: 1 }` writes the `aggression` field on
/// the tagged NPC's `CellEntity` so the AI idle tick can wake it up. Bug
/// shape: the previous implementation stored an unread `"aggression"`
/// property-bag entry; this guard pins that the canonical field is now
/// the source of truth.
#[tokio::test]
async fn set_aggression_level_one_writes_entity_field() {
    let mut mgr = make_space_mgr();
    // Tagged NPC the chain will target.
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Drone".to_string());
        assert_eq!(npc.aggression, 0, "fresh NPC must start passive");
    }
    // Player triggering the chain — must be co-located so
    // `find_entity_by_tag` resolves "Drone" against the same space.
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1032,
            Action::SetAggression {
                entity_tag: "Drone".to_string(),
                level: 1,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(101).unwrap().aggression,
        1,
        "SetAggression level=1 must write aggression=1 on the target — \
         the AI idle tick reads this field directly, no property-bag lookup",
    );
}

/// Chain-driven `Action::GenerateThreat` must broadcast `RefreshAppearance`
/// BEFORE `onStateFieldUpdate` when the player just entered combat.
/// `combat::generate_threat` flips `weapon_holstered = false` via
/// `enter_player_combat`; without the appearance refresh the client's
/// cached `ComponentList` keeps rendering the holstered/empty-hand mesh
/// while the server thinks the weapon is drawn — every subsequent fire
/// animation plays against empty hands and the in-combat pose shows no
/// weapon.
///
/// Bug shape from the play-session report on chain 1032 (Ambernol vial
/// pickup triggers drone aggro): "the players fists go into the combat
/// position even though that's not the active bandolier slot, the
/// player then shoots without having a weapon in their hands, and the
/// fists holsters when aggro drops when the drone dies." Three callers
/// of `combat::generate_threat` — `npc_ai_idle_auto_aggro` (NPC sees
/// player), `damage_apply::apply_damage_to_target` (player hits NPC),
/// and this content-action path — must all refresh appearance on
/// first-add. Pre-fix, only the first two did.
///
/// Order matters: appearance BEFORE state-field per the documented
/// "splinch" guard (the client's draw animation socket-attaches the
/// weapon mesh from the current `BeingAppearance`; flipping
/// `BSF_InCombat` first starts the draw animation against an empty
/// socket and the mesh snaps in mid-frame).
#[tokio::test]
async fn generate_threat_action_refreshes_appearance_before_state_field_on_first_aggro() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Drone".to_string());
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        // Pre-stage: player is holstered + OOC. The bug shape is
        // specifically the first-add `enter_player_combat` transition
        // flipping `weapon_holstered=false` without broadcasting
        // appearance — fixture must start at the holstered state for
        // the flip to be observable.
        p.weapon_holstered = true;
        assert!(
            p.threatened_mobs.is_empty(),
            "fixture sanity: player starts with no aggroed mobs so \
             `enter_player_combat` takes the first-add transition path \
             (returns Some(state)) — without that, the appearance + \
             state-field broadcasts are both skipped and this test \
             passes for the wrong reason"
        );
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(32);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1032,
            Action::GenerateThreat {
                entity_tag: Some("Drone".to_string()),
                threat_level: 1000,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // Drain the cell→base channel and capture the order of the two
    // load-bearing messages.
    let mut refresh_at: Option<usize> = None;
    let mut state_field_at: Option<usize> = None;
    let mut idx: usize = 0;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::RefreshAppearance {
                entity_id: 1,
                player_id: 42,
                holstered,
            } => {
                assert!(
                    !holstered,
                    "RefreshAppearance must carry holstered=false — \
                     enter_player_combat flipped the field; the wire \
                     must mirror the server-side value"
                );
                refresh_at.get_or_insert(idx);
            }
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE => {
                state_field_at.get_or_insert(idx);
            }
            _ => {}
        }
        idx += 1;
    }

    let refresh_idx = refresh_at.expect(
        "generate_threat content action MUST broadcast RefreshAppearance \
         on the first-add `enter_player_combat` transition — pre-fix the \
         action handler only sent onStateFieldUpdate and the client kept \
         the holstered/empty-hand BeingAppearance cached, producing the \
         play-session 'fists in combat pose, shoots without a weapon' bug \
         specifically from chain 1032 (Ambernol pickup triggers drone aggro)",
    );
    let state_field_idx = state_field_at.expect(
        "generate_threat must STILL broadcast onStateFieldUpdate so the \
         in-combat HUD / targeting cursor / state-bit-derived UI flips. \
         A regression that drops this packet leaves the player visually \
         drawn but UI-OOC",
    );
    assert!(
        refresh_idx < state_field_idx,
        "ordering: RefreshAppearance (#{refresh_idx}) must precede \
         onStateFieldUpdate (#{state_field_idx}). Both flow through the \
         same client-side state-machine entry point, but only the \
         appearance path triggers the socket re-attach that writes the \
         weapon-category byte. If `BSF_InCombat` flips first, the \
         unholster animation starts before the mesh is attached — the \
         documented 'splinch' bug shape from PR #395"
    );

    // Sanity: the cell-side state did flip. If this fails, the
    // assertions above passed for the wrong reason.
    let player = mgr.get_entity(1).unwrap();
    assert!(
        !player.weapon_holstered,
        "enter_player_combat must flip weapon_holstered=false on the \
         first-add transition — broadcast or not, the server-side state \
         is the source of truth for subsequent fire-path gating"
    );
    assert!(
        player.threatened_mobs.contains(&101),
        "drone must be in the player's threatened_mobs set after \
         generate_threat — this is what `enter_player_combat`'s \
         was_empty → Some(state) gate keys on"
    );
}

// ──────────────────────────────────────────────────────────────────────
// Phase 4 / 6 / 7 content actions: SetNpcPoi, SetFollowTarget, SetNpcAiState
// ──────────────────────────────────────────────────────────────────────

/// `Action::SetNpcPoi` transitions the tagged NPC into Investigating
/// with `poi` set to the given coords and `nav_path` cleared so the
/// investigate handler can re-pathfind.
#[tokio::test]
async fn set_npc_poi_transitions_target_to_investigating() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Guard".to_string());
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(99.0, 0.0, 99.0));
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1234,
            Action::SetNpcPoi {
                entity_tag: "Guard".to_string(),
                x: 50.0,
                y: 0.0,
                z: 60.0,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(101).unwrap();
    assert_eq!(npc.ai_state, AiState::Investigating);
    assert_eq!(
        npc.poi,
        Some(cimmeria_common::Vector3::new(50.0, 0.0, 60.0)),
    );
    assert!(npc.nav_path.is_empty(), "Action must clear stale nav_path");
}

/// `Action::SetFollowTarget` with a resolvable `target_tag` transitions
/// the follower into Follow with `follow_target_id` set.
#[tokio::test]
async fn set_follow_target_resolves_target_and_transitions_to_follow() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Pet".to_string());
    }
    mgr.spawn_npc(102, "Agnos", [20.0, 0.0, 20.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(102) {
        npc.tag = Some("Owner".to_string());
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            5678,
            Action::SetFollowTarget {
                entity_tag: "Pet".to_string(),
                target_tag: Some("Owner".to_string()),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let pet = mgr.get_entity(101).unwrap();
    assert_eq!(pet.ai_state, AiState::Follow);
    assert_eq!(pet.follow_target_id, Some(102));
}

/// `Action::SetFollowTarget` with `target_tag = None` clears the follow
/// state and returns the NPC to Idle.
#[tokio::test]
async fn set_follow_target_none_clears_and_returns_to_idle() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Pet".to_string());
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(999);
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            5678,
            Action::SetFollowTarget {
                entity_tag: "Pet".to_string(),
                target_tag: None,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let pet = mgr.get_entity(101).unwrap();
    assert_eq!(pet.ai_state, AiState::Idle);
    assert_eq!(pet.follow_target_id, None);
}

/// `Action::SetFollowTarget` with an unresolvable `target_tag` is
/// treated the same as `None` — the follow state clears and the NPC
/// returns to Idle. Pin: a typo or removed tag shouldn't leave the
/// follower in a half-state.
#[tokio::test]
async fn set_follow_target_unresolvable_tag_clears_follow() {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Pet".to_string());
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(999);
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            5678,
            Action::SetFollowTarget {
                entity_tag: "Pet".to_string(),
                target_tag: Some("Nonexistent".to_string()),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let pet = mgr.get_entity(101).unwrap();
    assert_eq!(
        pet.ai_state,
        AiState::Idle,
        "Unresolvable target_tag must drop to Idle (treated as clear)",
    );
    assert_eq!(pet.follow_target_id, None);
}

/// `Action::SetNpcAiState { state: Despawning }` flips the state.
#[tokio::test]
async fn set_npc_ai_state_despawning_flips_state() {
    use cimmeria_content_engine::actions::NpcAiStateAction;
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Boss".to_string());
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            999,
            Action::SetNpcAiState {
                entity_tag: "Boss".to_string(),
                state: NpcAiStateAction::Despawning,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(101).unwrap();
    assert_eq!(npc.ai_state, AiState::Despawning);
}

/// `Action::SetNpcAiState { state: Idle }` on a patrolling NPC drops
/// it back to Idle. Pin: `patrol_next_index` persists so the AI tick's
/// Idle→Patrol promotion can resume the route from where it left off.
#[tokio::test]
async fn set_npc_ai_state_idle_on_patroller_preserves_patrol_index() {
    use cimmeria_common::Vector3;
    use cimmeria_content_engine::actions::NpcAiStateAction;
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Guard".to_string());
        npc.patrol_path = vec![
            Vector3::new(10.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 10.0),
            Vector3::new(0.0, 0.0, 10.0),
        ];
        npc.ai_state = AiState::Patrol;
        npc.patrol_next_index = 2;
    }
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            999,
            Action::SetNpcAiState {
                entity_tag: "Guard".to_string(),
                state: NpcAiStateAction::Idle,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(101).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
    assert_eq!(
        npc.patrol_next_index, 2,
        "patrol_next_index must persist across SetNpcAiState(Idle) so the AI tick can resume the route",
    );
}
