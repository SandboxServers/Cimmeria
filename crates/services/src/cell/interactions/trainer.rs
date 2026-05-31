//! Ability trainer open — `onTrainerOpen` (flat method index 113).
//!
//! Trainer NPC interaction: detect a trainer at interact time, build the
//! per-player trainable ability list, and send `onTrainerOpen` so the client
//! opens the trainer UI.
//!
//! A trainer is any NPC whose `entity_templates.trainer_ability_list_id` is
//! non-NULL. The seed has exactly one (template_id=25, "Interaction
//! Debug NPC", `trainer_ability_list_id=1`). More are Phase 7 content.
//!
//! Per-archetype filtering: `trainer_abilities` is keyed by
//! `(list_id, archetype_id)` so a single trainer can offer different abilities
//! to a Soldier vs a Commando vs an Asgard. Today the only seeded list has
//! Commando-only entries — Soldiers/Asgard/etc. interacting with the debug
//! trainer get an empty list back, which the client handles gracefully (no
//! abilities shown but UI still opens).
//!
//! Trainable filter (mirrors Python `AbilityTrainer.canTrainAbility`):
//! - Player level meets the tree entry's `level` requirement
//! - All `prerequisite_abilities` are in the player's known set
//! - The ability isn't already known
//!
//! Wire (issue #55 deep dive, verified):
//!   `INT32 TrainerID, UINT32 count, [N × (INT32 abilityID + UINT8 trainable)], INT32 CostToRespec`
//!
//! References:
//! - `deprecated/python/cell/interactions/AbilityTrainer.py`
//! - `docs/protocol/client-method-dispatch-table.md:248` (method 113)
//! - `entities/defs/SGWPlayer.def:1194-1198`, `entities/defs/alias.xml:417-422`
//! - issue #55 deep dive (2026-05-27)

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Default respec cost in Naquadah. The Python TODO at
/// `AbilityTrainer.py:42` shipped `1000` as a placeholder; we preserve it.
/// Future content-design pass should compute this from total training
/// points already spent.
const DEFAULT_RESPEC_COST: i32 = 1000;

/// If the target entity is a trainer NPC, build the per-player ability list
/// and send `onTrainerOpen`. Returns `true` when the trainer flow handled the
/// interact (i.e., the caller should NOT fall through to the dialog/template
/// chain dispatcher). Returns `false` when:
/// - The target's template_id isn't in `template_trainer_lists`
/// - The interacting entity isn't a player with a known archetype
/// - The target entity doesn't exist
///
/// This is the canonical single-source-of-truth path. The previous
/// `dispatch.rs` `NpcInteractionType::Trainer { archetype_id }` arm was
/// dead code (never assigned anywhere in production) that called a stub
/// fabricating a list from `archetype_ability_tree` directly with no
/// per-known-ability filtering. Consolidated in issue #55.
pub(crate) async fn try_open_trainer(
    player_entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    // Identify the trainer's list_id via template lookup.
    let trainer_template_id = match space_mgr
        .get_entity(target_entity_id)
        .and_then(|t| t.template_id)
    {
        Some(t) => t,
        None => return false,
    };
    let list_id = match space_mgr.template_trainer_lists.get(&trainer_template_id) {
        Some(&id) => id,
        None => return false, // not a trainer — fall through to other interact paths
    };

    // Player must have an archetype to query the right ability list.
    let (player_archetype, player_level, known_abilities) = {
        let player = match space_mgr.get_entity(player_entity_id) {
            Some(p) => p,
            None => return false,
        };
        let arch = match player.archetype_id {
            Some(a) => a,
            None => {
                tracing::warn!(
                    player_entity_id,
                    target_entity_id,
                    "trainer interact: player has no archetype_id — skipping"
                );
                return false;
            }
        };
        (
            arch,
            player.level as i32,
            player.abilities.known_ability_ids(),
        )
    };

    // Lookup the abilities this trainer offers to this archetype.
    let offered: Vec<i32> = space_mgr
        .trainer_abilities
        .get(&(list_id, player_archetype))
        .cloned()
        .unwrap_or_default();
    if offered.is_empty() {
        tracing::warn!(
            target: "abilities",
            event = "trainer_empty_offering",
            player_entity_id,
            trainer_entity_id = target_entity_id,
            trainer_template_id,
            list_id,
            archetype_id = player_archetype,
            "Trainer has no abilities for this archetype — content gap"
        );
    }

    // For each offered ability, compute `trainable` (1 if the player can
    // train it now, 0 otherwise). Mirrors AbilityTrainer.canTrainAbility:
    // ability must be in player's archetype tree, level requirement met,
    // every prereq known, not already known.
    let known_set: std::collections::HashSet<i32> = known_abilities.into_iter().collect();
    let tree = space_mgr
        .archetype_ability_trees
        .get(&player_archetype)
        .cloned()
        .unwrap_or_default();

    let entries: Vec<(i32, u8)> = offered
        .iter()
        .map(|&ability_id| {
            let trainable = if known_set.contains(&ability_id) {
                0
            } else if let Some(entry) = tree.iter().find(|e| e.ability_id == ability_id) {
                let level_ok = player_level >= entry.level;
                let prereqs_ok = entry
                    .prerequisite_abilities
                    .iter()
                    .all(|p| known_set.contains(p));
                if level_ok && prereqs_ok {
                    1
                } else {
                    0
                }
            } else {
                tracing::warn!(
                    target: "abilities",
                    event = "trainer_offered_unbound",
                    player_entity_id,
                    trainer_entity_id = target_entity_id,
                    list_id,
                    archetype_id = player_archetype,
                    ability_id,
                    "Trainer offers ability not in player's archetype tree — \
                     content gap (trainer_abilities row without matching \
                     archetype_ability_tree entry)"
                );
                0
            };
            (ability_id, trainable)
        })
        .collect();

    // Wire layout: INT32 TrainerID, ARRAY<TrainerAbility> Abilities,
    // INT32 CostToRespec. TrainerAbility = FIXED_DICT { INT32 abilityID,
    // UINT8 trainable } — serialized as 5 bytes per entry, no marker.
    let mut args = Vec::with_capacity(4 + 4 + entries.len() * 5 + 4);
    args.extend_from_slice(&(target_entity_id as i32).to_le_bytes()); // TrainerID
    args.extend_from_slice(&(entries.len() as u32).to_le_bytes()); // ARRAY count
    for (ability_id, trainable) in &entries {
        args.extend_from_slice(&ability_id.to_le_bytes());
        args.push(*trainable);
    }
    args.extend_from_slice(&DEFAULT_RESPEC_COST.to_le_bytes()); // CostToRespec

    let send_result = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: player_entity_id,
            method_index: crate::mercury::method_idx::ON_TRAINER_OPEN,
            args,
        })
        .await;

    match send_result {
        Ok(()) => {
            tracing::info!(
                target: "abilities",
                event = "trainer_open",
                player_entity_id,
                trainer_entity_id = target_entity_id,
                trainer_template_id,
                list_id,
                archetype_id = player_archetype,
                offered = entries.len(),
                trainable_count = entries.iter().filter(|(_, t)| *t == 1).count(),
                "Sent onTrainerOpen"
            );
        }
        Err(e) => {
            tracing::error!(
                target: "abilities",
                event = "trainer_open_send_failed",
                player_entity_id,
                trainer_entity_id = target_entity_id,
                trainer_template_id,
                error = %e,
                "Failed to send onTrainerOpen — cell→base channel closed"
            );
        }
    }

    true
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::ArchetypeAbilityTreeEntry;
    use tokio::sync::mpsc;

    pub(crate) fn make_mgr_with_trainer_and_player() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
            p.archetype_id = Some(2);
            p.level = 1;
        }

        mgr.spawn_npc(200, "W", [5.0; 3], [0.0; 3]).unwrap();
        if let Some(t) = mgr.get_entity_mut(200) {
            t.template_id = Some(25);
        }
        mgr.template_trainer_lists.insert(25, 1);
        mgr.trainer_abilities.insert((1, 2), vec![597, 646, 641]);
        mgr.archetype_ability_trees.insert(
            2,
            vec![
                ArchetypeAbilityTreeEntry {
                    ability_id: 597,
                    tree_index: 1,
                    level: 1,
                    prerequisite_abilities: vec![],
                },
                ArchetypeAbilityTreeEntry {
                    ability_id: 646,
                    tree_index: 1,
                    level: 1,
                    prerequisite_abilities: vec![],
                },
                ArchetypeAbilityTreeEntry {
                    ability_id: 641,
                    tree_index: 1,
                    level: 5,
                    prerequisite_abilities: vec![],
                },
            ],
        );

        mgr
    }

    #[tokio::test]
    async fn non_trainer_target_returns_false() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        mgr.spawn_npc(200, "W", [5.0; 3], [0.0; 3]).unwrap();
        if let Some(t) = mgr.get_entity_mut(200) {
            t.template_id = Some(99);
        }
        let (tx, _rx) = mpsc::channel(8);
        let handled = try_open_trainer(1, 200, &tx, &mgr).await;
        assert!(!handled, "non-trainer template must fall through");
    }

    #[tokio::test]
    async fn trainer_sends_on_trainer_open_with_offered_abilities() {
        let mgr = make_mgr_with_trainer_and_player();
        let (tx, mut rx) = mpsc::channel(8);
        let handled = try_open_trainer(1, 200, &tx, &mgr).await;
        assert!(handled, "trainer interact must be handled");

        let msg = rx.try_recv().expect("must send onTrainerOpen");
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(entity_id, 1, "method targets the player entity");
                assert_eq!(method_index, crate::mercury::method_idx::ON_TRAINER_OPEN);
                assert_eq!(
                    i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
                    200
                );
                assert_eq!(u32::from_le_bytes([args[4], args[5], args[6], args[7]]), 3);
                assert_eq!(
                    i32::from_le_bytes([args[8], args[9], args[10], args[11]]),
                    597
                );
                assert_eq!(args[12], 1, "597 must be trainable");
                assert_eq!(
                    i32::from_le_bytes([args[13], args[14], args[15], args[16]]),
                    646
                );
                assert_eq!(args[17], 1, "646 must be trainable");
                assert_eq!(
                    i32::from_le_bytes([args[18], args[19], args[20], args[21]]),
                    641
                );
                assert_eq!(args[22], 0, "641 must NOT be trainable at level 1");
                assert_eq!(
                    i32::from_le_bytes([args[23], args[24], args[25], args[26]]),
                    1000
                );
            }
            _ => panic!("expected EntityMethodCall"),
        }
    }

    /// Already-known ability filter regression (issue #55 deep dive).
    /// An ability that the player already has in `entity.abilities` must
    /// appear in the trainer list with `trainable=0`. Without this guard
    /// the client UI would show the ability as available — the DB UPDATE's
    /// `NOT (abilities @> ARRAY[...])` clause on the base side would
    /// reject the second grant, but the UI would still suggest it's
    /// available (bad UX).
    #[tokio::test]
    async fn already_known_ability_shows_untrainable() {
        let mut mgr = make_mgr_with_trainer_and_player();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(597);
        }
        let (tx, mut rx) = mpsc::channel(8);
        try_open_trainer(1, 200, &tx, &mgr).await;
        let msg = rx.try_recv().unwrap();
        if let CellToBaseMsg::EntityMethodCall { args, .. } = msg {
            assert_eq!(
                args[12], 0,
                "already-known ability must show as untrainable"
            );
        } else {
            panic!("expected EntityMethodCall");
        }
    }

    #[tokio::test]
    async fn player_without_archetype_skips() {
        let mut mgr = make_mgr_with_trainer_and_player();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = None;
        }
        let (tx, _rx) = mpsc::channel(8);
        let handled = try_open_trainer(1, 200, &tx, &mgr).await;
        assert!(!handled, "missing archetype must skip trainer flow");
    }
}
