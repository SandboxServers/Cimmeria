//! `Action::StartMinigame` reads the player's level and Intelligence from the
//! live cell entity (MG-02 / Contract C3).

use super::*;

/// Revert: send the literals `(1, 0)` again and the message carries 1 and 0.
#[tokio::test]
async fn start_minigame_carries_cell_level_and_intelligence() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(8, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(8) {
        p.is_player = true;
        p.player_id = Some(42);
        p.level = 17;
        p.stats
            .get_mut(cimmeria_entity::stats::INTELLIGENCE)
            .unwrap()
            .update(0, 9, 9);
    }

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            9003,
            Action::StartMinigame {
                minigame_type: "livewire".to_string(),
                difficulty: 1,
                on_victory_chains: vec![],
                on_failure_chains: vec![200],
            },
        )],
    };

    execute_actions(resolved, 8, 42, &tx, &mut mgr, &engine).await;

    match rx.try_recv() {
        Ok(CellToBaseMsg::StartMinigame {
            player_level: 17,
            intelligence: 9,
            tech_competency: 1,
            abilities_mask: 0,
            on_failure_chains,
            ..
        }) => assert_eq!(on_failure_chains, vec![200]),
        Ok(_) => panic!("StartMinigame must carry level 17 and Intelligence 9"),
        Err(e) => panic!("no StartMinigame was sent: {e}"),
    }
}
