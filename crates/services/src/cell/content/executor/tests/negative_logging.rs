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
        params: std::collections::HashMap::new(),
        actions: vec![(
            9002,
            Action::StartMinigame {
                minigame_type: "livewire".to_string(),
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
            .push((dialog_set_id, /* dialog_id */ 7, /* flags */ 0x10));
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
                dialog_id: 7,
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
