//! NPC aggro and threat list management.
//!
//! Threat is accumulated per attacker on each NPC. The NPC AI picks its
//! current target as the entity with the highest threat. First hit also
//! transitions the NPC from `Idle` to `Fighting`.

/// Leash distance in world units — if an NPC's target moves further than this
/// from the NPC's spawn position, the NPC resets and walks home.
pub const LEASH_DISTANCE: f32 = 50.0;

/// Maximum attack range in world units for NPC ranged attacks.
/// NPCs won't fire until the target is within this distance.
pub const NPC_ATTACK_RANGE: f32 = 30.0;

/// Default NPC attack ability ID: "Pistol Shot" (ability 592, ranged DD).
/// Was incorrectly 597 ("Heal Focus") — a self-heal, not an attack.
pub const NPC_DEFAULT_ABILITY: i32 = 592;

/// Generate threat on an NPC target from a player attacker.
///
/// Transitions the NPC from Idle to Fighting on first hit, and accumulates
/// threat so the NPC knows who to attack back.
pub fn generate_threat(
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    attacker_id: u32,
    target_id: u32,
    threat_amount: f32,
) {
    use cimmeria_entity::cell_entity::AiState;

    if let Some(target) = space_mgr.get_entity_mut(target_id) {
        if !target.is_player {
            if target.ai_state == AiState::Idle {
                target.ai_state = AiState::Fighting;
                tracing::info!(
                    npc_id = target_id, attacker = attacker_id,
                    "NPC aggro: Idle -> Fighting"
                );
            }
            *target.threat_list.entry(attacker_id).or_insert(0.0) += threat_amount;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_default_ability_is_pistol_shot() {
        // 592 = Pistol Shot (ranged DD). Was previously 597 (Heal Focus, self-heal).
        assert_eq!(NPC_DEFAULT_ABILITY, 592);
    }

    #[test]
    fn leash_distance_is_50() {
        assert_eq!(LEASH_DISTANCE, 50.0);
    }

    fn make_test_space_mgr_with_npc() -> crate::cell::space_manager::SpaceManager {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Create a player entity
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();

        // Create an NPC entity
        mgr.spawn_npc(100, "Agnos", [15.0, 0.0, 15.0], [0.0; 3]).unwrap();

        mgr
    }

    #[test]
    fn generate_threat_transitions_idle_to_fighting() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        // NPC should start Idle
        assert_eq!(mgr.get_entity(100).unwrap().ai_state, AiState::Idle);

        generate_threat(&mut mgr, 1, 100, 50.0);

        // NPC should now be Fighting
        let npc = mgr.get_entity(100).unwrap();
        assert_eq!(npc.ai_state, AiState::Fighting);
        assert_eq!(npc.threat_list[&1], 50.0);
    }

    #[test]
    fn generate_threat_accumulates() {
        let mut mgr = make_test_space_mgr_with_npc();

        generate_threat(&mut mgr, 1, 100, 50.0);
        generate_threat(&mut mgr, 1, 100, 30.0);

        assert_eq!(mgr.get_entity(100).unwrap().threat_list[&1], 80.0);
    }

    #[test]
    fn generate_threat_multiple_attackers() {
        let mut mgr = make_test_space_mgr_with_npc();
        // Add a second player
        mgr.create_entity(2, "Agnos", [20.0, 0.0, 10.0], [0.0; 3]).unwrap();

        generate_threat(&mut mgr, 1, 100, 50.0);
        generate_threat(&mut mgr, 2, 100, 100.0);

        let npc = mgr.get_entity(100).unwrap();
        assert_eq!(npc.threat_list.len(), 2);
        assert_eq!(npc.threat_list[&1], 50.0);
        assert_eq!(npc.threat_list[&2], 100.0);

        // Top threat should be entity 2
        let top = npc.threat_list.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(&id, _)| id);
        assert_eq!(top, Some(2));
    }

    #[test]
    fn generate_threat_ignores_player_targets() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();
        // Mark entity 1 as a player
        mgr.get_entity_mut(1).unwrap().is_player = true;

        // Try to generate threat on a player entity — should be ignored
        generate_threat(&mut mgr, 100, 1, 50.0);

        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.ai_state, AiState::Idle);
        assert!(player.threat_list.is_empty());
    }

    #[test]
    fn generate_threat_stays_fighting_on_second_hit() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        generate_threat(&mut mgr, 1, 100, 50.0);
        assert_eq!(mgr.get_entity(100).unwrap().ai_state, AiState::Fighting);

        // Second hit should stay Fighting (not re-trigger transition)
        generate_threat(&mut mgr, 1, 100, 25.0);
        assert_eq!(mgr.get_entity(100).unwrap().ai_state, AiState::Fighting);
    }
}
