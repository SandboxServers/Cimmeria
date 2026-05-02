//! Ability trainer open — `onTrainerOpen` (flat method index 113).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

/// Send `onTrainerOpen` (flat index 113) to the player.
///
/// Wire: `trainerEntityId:i32, abilities:ARRAY<{abilityId:i32, trainable:u8}>,
///        costToRespec:i32`.
pub(super) async fn send_trainer_open(
    player_id: u32,
    npc_entity_id: i32,
    archetype_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    // Get the ability tree for this archetype so we know what to offer
    let tree = crate::mercury::archetype_ability_tree(archetype_id);
    let all_abilities: Vec<i32> = tree.trees.iter().flatten().copied().collect();

    // TODO: Check which abilities the player already knows and mark as trainable
    // For now, mark all as trainable (1)
    let count = all_abilities.len() as u32;
    let mut args = Vec::with_capacity(8 + all_abilities.len() * 5);
    args.extend_from_slice(&npc_entity_id.to_le_bytes());  // TrainerID
    args.extend_from_slice(&count.to_le_bytes());           // ability count
    for ability_id in &all_abilities {
        args.extend_from_slice(&ability_id.to_le_bytes());  // abilityID
        args.push(1);                                       // trainable = true
    }
    args.extend_from_slice(&1000i32.to_le_bytes());         // CostToRespec

    tracing::debug!(player_id, npc_entity_id, archetype_id, count, "Sending onTrainerOpen");
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: player_id,
        method_index: crate::mercury::method_idx::ON_TRAINER_OPEN,
        args,
    }).await;
}

#[cfg(test)]
mod tests {
    #[test]
    fn trainer_open_args_format() {
        let npc_id: i32 = 100_002;
        let abilities = vec![597i32, 603, 604];
        let count = abilities.len() as u32;

        let mut args = Vec::new();
        args.extend_from_slice(&npc_id.to_le_bytes());
        args.extend_from_slice(&count.to_le_bytes());
        for &ab in &abilities {
            args.extend_from_slice(&ab.to_le_bytes());
            args.push(1); // trainable
        }
        args.extend_from_slice(&1000i32.to_le_bytes());

        // 4 (npc) + 4 (count) + 3*(4+1) + 4 (respec) = 27
        assert_eq!(args.len(), 27);
        assert_eq!(u32::from_le_bytes([args[4], args[5], args[6], args[7]]), 3);
        // First ability
        assert_eq!(i32::from_le_bytes([args[8], args[9], args[10], args[11]]), 597);
        assert_eq!(args[12], 1); // trainable
    }
}
