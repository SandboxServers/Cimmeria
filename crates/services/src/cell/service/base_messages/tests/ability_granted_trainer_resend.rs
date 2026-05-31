//! Regression for issue #55 Item B — `AbilityGranted` must re-send
//! `onTrainerOpen` when the player has a trainer pinned.
//!
//! Python parity (`AbilityTrainer.onTrainAbility:128`): after a successful
//! `trainAbility` round-trip, if the newly-learned ability is a prerequisite
//! for another offered ability in the trainer's list, the trainer UI must
//! refresh so the dependent ability's greyed-out state updates. Without
//! this resend, the player has to close+reopen the trainer to see the now-
//! unlocked ability.
//!
//! The implementation triggers on any `AbilityGranted` while
//! `last_interaction_target` is pinned to a trainer template. The decision
//! "is this newly-learned ability a prereq for something offered" is
//! deferred to `try_open_trainer` itself, which recomputes every
//! `trainable` flag from current state.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::ArchetypeAbilityTreeEntry;

/// Seed: trainer offers two abilities — 597 (lvl 1, no prereqs) and 598
/// (lvl 1, prereq = [597]). At t=0 the player knows nothing. 597 is
/// trainable; 598 is locked (prereq missing).
///
/// AbilityGranted for 597 arrives. After mirror + hotbar update, the
/// resend must fire because the player has the trainer pinned. The
/// resent `onTrainerOpen` must show 598 as trainable (prereq now met).
#[tokio::test]
async fn ability_granted_resends_trainer_open_with_unlocked_prereq() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#)
        .unwrap();

    // Player
    mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
        p.archetype_id = Some(2); // Commando
        p.level = 1;
        // Trainer pinned as the last interaction. Set by handle_interact
        // when the player clicks the trainer — preserved across the
        // train round-trip.
        p.last_interaction_target = Some(200);
    }

    // Trainer NPC
    mgr.spawn_npc(200, "W", [5.0; 3], [0.0; 3]).unwrap();
    if let Some(t) = mgr.get_entity_mut(200) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);

    // Trainer list: 597 (lvl 1, no prereqs) + 598 (lvl 1, prereq=[597]).
    mgr.trainer_abilities.insert((1, 2), vec![597, 598]);
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
                ability_id: 598,
                tree_index: 1,
                level: 1,
                prerequisite_abilities: vec![597],
            },
        ],
    );

    let (tx, mut rx) = mpsc::channel(32);
    let engine = ChainEngine::new();

    // Drive the AbilityGranted message — this is what base sends after
    // a successful train_ability UPDATE.
    handle_base_message(
        BaseToCellMsg::AbilityGranted {
            entity_id: 1,
            ability_id: 597,
            training_points_remaining: 4,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Verify the cell mirrored the grant — without this, the resent
    // trainer list would still show 597 as the unmet prereq for 598.
    assert!(
        mgr.get_entity(1)
            .map(|e| e.abilities.has_ability(597))
            .unwrap_or(false),
        "AbilityGranted must mirror the new ability onto the cell entity \
         BEFORE re-sending onTrainerOpen, or the prereq computation lags"
    );

    // Collect emitted method frames. We expect at least:
    // - onKnownAbilitiesUpdate (method 101) — the hotbar refresh
    // - onTrainerOpen (method 113) — the resend
    let mut saw_known_abilities_update = false;
    let mut on_trainer_open_payload: Option<Vec<u8>> = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id != 1 {
                continue;
            }
            if method_index == crate::mercury::method_idx::ON_KNOWN_ABILITIES_UPDATE {
                saw_known_abilities_update = true;
            } else if method_index == crate::mercury::method_idx::ON_TRAINER_OPEN {
                on_trainer_open_payload = Some(args);
            }
        }
    }

    assert!(
        saw_known_abilities_update,
        "AbilityGranted must always emit onKnownAbilitiesUpdate for the hotbar"
    );

    let args = on_trainer_open_payload.expect(
        "AbilityGranted while trainer is pinned must re-send onTrainerOpen \
         (issue #55 Item B regression — without this the dependent ability's \
         greyed-out state is stale until close+reopen)",
    );

    // Wire layout: [INT32 TrainerID, UINT32 count, N × (INT32 ability_id +
    // UINT8 trainable), INT32 CostToRespec]. Count = 2.
    assert_eq!(
        i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
        200,
        "TrainerID must be the pinned NPC"
    );
    assert_eq!(
        u32::from_le_bytes([args[4], args[5], args[6], args[7]]),
        2,
        "list still offers 2 abilities (597 + 598)"
    );

    // entry 0: ability 597 — now known, so trainable=0
    assert_eq!(
        i32::from_le_bytes([args[8], args[9], args[10], args[11]]),
        597
    );
    assert_eq!(
        args[12], 0,
        "597 is now known — must show as untrainable in the resend"
    );

    // entry 1: ability 598 — prereq (597) now met, so trainable=1.
    // THIS is the bit the resend exists to fix.
    assert_eq!(
        i32::from_le_bytes([args[13], args[14], args[15], args[16]]),
        598
    );
    assert_eq!(
        args[17], 1,
        "598 must be trainable now that prereq 597 is known — without the \
         resend the client would still show it greyed out (issue #55 Item B)"
    );
}

/// Negative case: when the player has NO trainer pinned (e.g. ability was
/// granted via a quest chain's `Action::GrantAbility`, not from a trainer),
/// AbilityGranted must NOT spuriously emit `onTrainerOpen`. Without this
/// guard a chain-granted ability would pop the trainer window on the
/// client even if the player wasn't near a trainer.
#[tokio::test]
async fn ability_granted_with_no_trainer_pinned_does_not_resend() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
        p.archetype_id = Some(2);
        p.level = 1;
        // last_interaction_target intentionally None.
    }

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    handle_base_message(
        BaseToCellMsg::AbilityGranted {
            entity_id: 1,
            ability_id: 597,
            training_points_remaining: 4,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            assert_ne!(
                method_index,
                crate::mercury::method_idx::ON_TRAINER_OPEN,
                "AbilityGranted with no pinned trainer must NOT emit onTrainerOpen \
                 (chain-granted abilities, GM grants, etc.)"
            );
        }
    }
}

/// Pins the documented "resend on ANY grant while pinned" contract — the
/// resend fires for every `AbilityGranted` while a trainer is pinned,
/// regardless of whether the granted ability is in the trainer's offered
/// list or is a prereq for anything offered.
///
/// Mirrors Python's `AbilityTrainer.onTrainAbility:128` shape: it
/// re-fires `onTrainerOpen` unconditionally after a successful train RPC.
/// The "is this newly-learned ability a prereq for B?" decision lives
/// inside `try_open_trainer`, not here.
///
/// Setup: trainer offers A (597) and B (646) — neither has prereqs on
/// the other. Grant a DIFFERENT ability that the trainer doesn't even
/// offer (700, a chain-granted quest reward, say). The resend must
/// still fire — list content won't change but the wire frame goes out.
///
/// Without this pin, a future "optimisation" that filters out resends
/// for non-prereq grants would silently diverge from Python parity.
#[tokio::test]
async fn ability_granted_resends_even_when_not_a_prereq() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#)
        .unwrap();

    // Player with trainer pinned.
    mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
        p.archetype_id = Some(2); // Commando
        p.level = 1;
        p.last_interaction_target = Some(200);
    }

    // Trainer NPC.
    mgr.spawn_npc(200, "W", [5.0; 3], [0.0; 3]).unwrap();
    if let Some(t) = mgr.get_entity_mut(200) {
        t.template_id = Some(25);
    }
    mgr.template_trainer_lists.insert(25, 1);

    // Trainer offers 597 + 646, both lvl 1, neither a prereq of the other.
    mgr.trainer_abilities.insert((1, 2), vec![597, 646]);
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
        ],
    );

    let (tx, mut rx) = mpsc::channel(32);
    let engine = ChainEngine::new();

    // Grant 700 — NOT in the trainer's offered list, NOT a prereq for
    // anything offered. The resend must still fire per the contract.
    handle_base_message(
        BaseToCellMsg::AbilityGranted {
            entity_id: 1,
            ability_id: 700,
            training_points_remaining: 4,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let mut on_trainer_open_seen = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } = msg
        {
            if entity_id == 1 && method_index == crate::mercury::method_idx::ON_TRAINER_OPEN {
                on_trainer_open_seen = true;
            }
        }
    }
    assert!(
        on_trainer_open_seen,
        "resend contract: AbilityGranted while a trainer is pinned must \
         re-emit onTrainerOpen even when the granted ability is unrelated \
         to the trainer's offered list (Python parity with \
         AbilityTrainer.onTrainAbility unconditional re-fire)"
    );
}
