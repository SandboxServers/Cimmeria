//! DK-40: the label arms and helpers that waves 5 and 6 assert with.
use std::collections::HashMap;

use super::*;

#[test]
fn labels_and_delays_cover_the_wave_5_and_6_verbs() {
    let r = ResolvedActions {
        actions: vec![
            (8290, Action::DisplayRadioDialog { dialog_id: 5833 }),
            (
                8290,
                Action::CrossWorldTeleport {
                    world_name: "SGC_W1".into(),
                    position: [172.79, -10.9, 12.0],
                },
            ),
            (
                8290,
                Action::SpawnEntity {
                    template_id: 443,
                    position: [66.2, -0.12, 34.3],
                    heading: std::f32::consts::FRAC_PI_2,
                    tag: "Dakara_E1_StoryRm_BaalHologram".into(),
                    is_stationary: Some(true),
                    aggression: None,
                    allow_shared: None,
                },
            ),
            (
                8290,
                Action::DespawnEntity {
                    entity_tag: "Dakara_E1_StoryRm_BaalHologram".into(),
                },
            ),
            (8290, Action::PlaySequence { sequence_id: 1194 }),
            (9999, Action::DisplayRadioDialog { dialog_id: 1 }),
        ],
        action_delays: vec![2000, 0, 0, 0, 0, 0],
        params: HashMap::new(),
    };
    assert_eq!(
        labels_in(&r, M1651_CHAINS),
        [
            "8290:display_radio_dialog(5833)",
            "8290:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)",
            "8290:spawn_entity(443#Dakara_E1_StoryRm_BaalHologram)",
            "8290:despawn_entity(Dakara_E1_StoryRm_BaalHologram)",
            "8290:play_sequence(1194)",
        ]
    );
    assert_eq!(delays_of(&r, 8290), [2000, 0, 0, 0, 0]);
    assert_eq!(radio_count(&r), 2);
}

#[test]
fn labels_cover_the_wave_6c_verbs() {
    let r = ResolvedActions {
        actions: vec![
            (
                8231,
                Action::StartMinigame {
                    minigame_type: "Livewire".into(),
                    difficulty: 1,
                    on_victory_chains: vec![8231],
                },
            ),
            (8231, Action::GrantStargateAddress { stargate_id: 27 }),
            (9999, Action::DisplayDialog { dialog_id: 1 }),
        ],
        action_delays: vec![0, 0, 0],
        params: HashMap::new(),
    };
    assert_eq!(
        labels_in(&r, M1649_TERMINAL_CHAINS),
        [
            "8231:start_minigame(Livewire,victory=[8231],d=1)",
            "8231:grant_stargate_address(27)",
        ]
    );
}
