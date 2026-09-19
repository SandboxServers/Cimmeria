//! Negative-logging regression guards — content executor.
//!
//! Each test drops the mpsc receiver before invoking the executor, so
//! the cell→base channel returns SendError on every `.send()`. The
//! guards assert the new WARN fires with the chain_id field — reverting
//! the `if let Err` change back to `let _` would silence the log and
//! trip the test.

use super::*;

#[tokio::test]
async fn play_sequence_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(7) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx); // close the cell→base channel
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(9001, Action::PlaySequence { sequence_id: 512 })],
    };

    execute_actions(resolved, 7, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "PlaySequence: cell→base send failed")
            .is_some(),
        "negative-logging convention: PlaySequence must WARN when cell→base channel is closed; \
         reverting to `let _ = tx.send` breaks chain-stall diagnosability. \
         Captured events: {:#?}",
        capture.all()
    );
}

#[tokio::test]
async fn start_minigame_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(8, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(8) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            9002,
            Action::StartMinigame {
                minigame_type: "livewire".to_string(),
                difficulty: 1,
                on_victory_chains: vec![],
            },
        )],
    };

    execute_actions(resolved, 8, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "StartMinigame: cell→base send failed")
            .is_some(),
        "negative-logging convention: StartMinigame must WARN when cell→base channel is closed"
    );
}

#[tokio::test]
async fn set_active_slot_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(9, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(9) {
        p.is_player = true;
        p.player_id = Some(42);
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(9003, Action::SetActiveSlot { bag_id: 3, slot: 0 })],
    };

    execute_actions(resolved, 9, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "SetActiveSlot: cell→base send failed")
            .is_some(),
        "negative-logging convention: SetActiveSlot must WARN when cell→base channel is closed"
    );
}

/// Stage a player + an NPC with matching template_id, with the NPC
/// already in the player's witness set, with one `available_interactions`
/// entry on the player keyed on the template slot. Returns the slot id
/// and the dialog_set_id that's been stashed there.
///
/// Setting both `witnesses` and `available_interactions` is what makes
/// `RemoveDialogSet` exercise the `send_interaction_update_if_visible`
/// branch where the WitnessEntityMethod is actually dispatched.
fn stage_dialog_set_witness(
    mgr: &mut SpaceManager,
    player_id: u32,
    npc_id: u32,
    template_id: i32,
    dialog_set_id: i32,
) {
    use cimmeria_common::EntityId;

    mgr.create_entity(player_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(npc_id, "Agnos", [1.0; 3], [0.0; 3])
        .unwrap();

    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.is_player = true;
        p.player_id = Some(42);
        p.witnesses.insert(EntityId(npc_id as i32));
        p.available_interactions
            .entry(template_id)
            .or_default()
            .push((
                dialog_set_id,
                /* dialog_id */ Some(7),
                /* flags */ 0x10,
            ));
    }
    if let Some(n) = mgr.get_entity_mut(npc_id) {
        n.template_id = Some(template_id);
        n.interaction_type_flags = 0x01;
    }
}

#[tokio::test]
async fn remove_dialog_set_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    stage_dialog_set_witness(
        &mut mgr, /* player */ 11, /* npc */ 111, /* template_id */ 555,
        /* dialog_set_id */ 88,
    );

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            9004,
            Action::RemoveDialogSet {
                dialog_set_id: 88,
                slot: 555,
            },
        )],
    };

    execute_actions(resolved, 11, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(
                Level::WARN,
                "RemoveDialogSet: cell→base interaction-type send failed"
            )
            .is_some(),
        "negative-logging convention: RemoveDialogSet must WARN when its InteractionType push fails; \
         reverting to `let _ = tx.send` breaks stale-prompt diagnosability. \
         Captured: {:#?}",
        capture.all()
    );
}

#[tokio::test]
async fn add_dialog_set_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();

    // Pre-populate `dialog_set_maps` so `add_dialog_set` resolves the
    // entry and reaches the WitnessEntityMethod send. The cache key is
    // the dialog_set_id; value is a (dialog_id, interaction_flags).
    {
        use crate::cell::spawner::DialogSetMapEntry;
        mgr.dialog_set_maps.insert(
            88,
            DialogSetMapEntry {
                dialog_id: Some(7),
                interaction_flags: 0x10,
            },
        );
    }

    stage_dialog_set_witness(
        &mut mgr, /* player */ 12, /* npc */ 112, /* template_id */ 555,
        /* dialog_set_id */ 88,
    );

    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            9005,
            Action::AddDialogSet {
                dialog_set_id: 88,
                slot: 555,
                mission_id: None,
            },
        )],
    };

    execute_actions(resolved, 12, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(Level::WARN, "interaction-type send failed")
            .is_some(),
        "negative-logging convention: AddDialogSet's send_interaction_update_if_visible helper \
         must WARN when cell→base is closed (covers the :247 path). Captured: {:#?}",
        capture.all()
    );
}

// ── Harset H03: spawn refusals and the set_visible hide path ─────────────
//
// All four `spawn_entity` refusals share one side effect — no entity
// appears — so the behavioural tests in `executor::spawn::tests` cannot
// tell them apart: a bug that refused *everything* for a single reason
// keeps every one of them green. The structured `reason` field is the only
// discriminator, which makes it load-bearing API rather than log
// decoration. `find_event` pins level and reason together, so a level
// demotion and a renamed reason both trip these.

/// Stage a connected player in the shared (non-instanced) `Agnos` fixture.
fn stage_player_in_agnos(mgr: &mut SpaceManager, entity_id: u32) {
    mgr.create_entity(entity_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(entity_id) {
        p.is_player = true;
        p.player_id = Some(42);
    }
    mgr.connect_entity(entity_id);
}

/// Run one `Action::SpawnEntity` through the real dispatch.
async fn run_spawn(
    mgr: &mut SpaceManager,
    actor: u32,
    template_id: i32,
    tag: &str,
    allow_shared: Option<bool>,
) {
    let (tx, _rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            6301,
            Action::SpawnEntity {
                template_id,
                position: [1.0, 2.0, 3.0],
                heading: 0.0,
                tag: tag.to_string(),
                is_stationary: None,
                aggression: None,
                allow_shared,
            },
        )],
    };
    execute_actions(resolved, actor, 42, &tx, mgr, &engine).await;
}

/// Refusing a spawn into a non-instanced world must be attributable. An
/// author whose mission NPC never appears otherwise cannot tell "wrong
/// world" from "bad template id".
#[tokio::test]
async fn spawn_entity_shared_world_refusal_warns_with_reason() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    stage_player_in_agnos(&mut mgr, 7401);
    // Cache the template so the refusal can only be the world guard.
    let fixture = super::super::spawn::tests::template(10);
    let template_id = super::super::spawn::tests::TEMPLATE_ID;
    mgr.spawn_templates.insert(template_id, fixture);

    run_spawn(&mut mgr, 7401, template_id, "H03_NegLog_Shared", None).await;

    assert!(
        capture
            .find_event(Level::WARN, "refusing to spawn", "shared_world_refused")
            .is_some(),
        "the shared-world refusal must be attributable by reason -- every \
         spawn refusal has the same side effect (no entity appears), so the \
         reason field is the only discriminator. Captured: {:#?}",
        capture.all()
    );
}

/// A cache miss must be distinguishable from the shared-world refusal, and
/// `allow_shared` must genuinely lift the world guard rather than mask it.
#[tokio::test]
async fn spawn_entity_uncached_template_warns_with_its_own_reason() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    stage_player_in_agnos(&mut mgr, 7402);

    run_spawn(&mut mgr, 7402, 999_001, "H03_NegLog_NoTemplate", Some(true)).await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "not in the entity_templates cache",
                "template_not_cached"
            )
            .is_some(),
        "an uncached template must warn with its own reason. Captured: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_event(Level::WARN, "refusing to spawn", "shared_world_refused")
            .is_none(),
        "allow_shared=true must lift the world guard, so the shared-world \
         reason must NOT also fire -- if it does, the two refusals are being \
         conflated and neither is diagnosable"
    );
}

/// The hide half of `set_visible` sends one message per witness. A dropped
/// send leaves that witness still seeing an entity content just hid — a
/// desync the player can see — so it must not be a silent `let _`.
#[tokio::test]
async fn set_visible_hide_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use cimmeria_common::EntityId;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(7403, "Agnos", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    if let Some(n) = mgr.get_entity_mut(7403) {
        n.tag = Some("H03_NegLog_Vis".to_string());
    }
    stage_player_in_agnos(&mut mgr, 7404);
    if let Some(p) = mgr.get_entity_mut(7404) {
        p.witnesses.insert(EntityId(7403));
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx); // close the cell→base channel
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            6302,
            Action::SetVisible {
                entity_tag: "H03_NegLog_Vis".to_string(),
                visible: false,
            },
        )],
    };

    execute_actions(resolved, 7404, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "cell→base send failed",
                "set_visible_send_failed"
            )
            .is_some(),
        "negative-logging convention: the set_visible hide path must WARN per \
         failed witness send; reverting to `let _ = tx.send(...)` hides a \
         player-visible desync. Captured: {:#?}",
        capture.all()
    );
}

#[tokio::test]
async fn move_waypoint_warns_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    use cimmeria_common::EntityId;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(7413, "Agnos", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    if let Some(n) = mgr.get_entity_mut(7413) {
        n.tag = Some("H03_NegLog_MoveWp".to_string());
    }
    stage_player_in_agnos(&mut mgr, 7414);
    if let Some(p) = mgr.get_entity_mut(7414) {
        p.witnesses.insert(EntityId(7413));
    }

    let (tx, rx) = mpsc::channel(8);
    drop(rx); // close the cell→base channel
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            6303,
            Action::MoveWaypoint {
                entity_tag: "H03_NegLog_MoveWp".to_string(),
                destination: [10.0, 0.0, 10.0],
                speed: 1.0,
            },
        )],
    };

    execute_actions(resolved, 7414, 42, &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "cell→base send failed",
                "move_waypoint_send_failed"
            )
            .is_some(),
        "negative-logging convention: move_waypoint must WARN per failed \
         witness send; reverting to `let _ = tx.send(...)` hides the \
         stale-position window until the next AoI tick. Captured: {:#?}",
        capture.all()
    );
}
