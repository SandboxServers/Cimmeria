//! The attack animation: the Ability_Begin / Ability_End `onSequence`
//! (client method 1) a committed `useAbility` plays.
//!
//! **Routing.** Owner + witnesses, as Python `AbilityManager.playSequence`
//! did (`deprecated/python/cell/AbilityManager.py:879-893`: `ent.client` and
//! then `ent.witnesses`). A player's own client gets an `EntityMethodCall`
//! and every player who has them in AoI gets a `WitnessEntityMethod`, so
//! other players see the shot. Before NA43 the send went through
//! `send_entity_method`, which routes a player to self only, and nobody else
//! saw a player fire. An NPC has no client, so it gets the witness fan-out
//! alone, as before.
//!
//! **Negative logs (NA43).** For an NPC attacker, each way the animation can
//! go missing while damage still lands is a throttled WARN on
//! `abilities.sequence`:
//!
//! | `outcome` | Meaning |
//! |---|---|
//! | `no_ability_def` | the ability has no `resources.abilities` row loaded |
//! | `no_event_set` | its `event_set_id` is NULL, so nothing is looked up |
//! | `no_end_sequence` | the event set has no Ability_End (1001) sequence |
//! | `no_witnesses` | the NPC shot with nobody in AoI to see it |
//!
//! The throttle is keyed by ability id (`SpaceManager::ability_sequence_log`)
//! because the first three are facts about the seed row that every NPC
//! firing the ability repeats. Player abilities are not warned about: 1851 of
//! the 1886 seeded abilities have no event set, and most player abilities
//! legitimately animate through other paths.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_entity::abilities::AbilityDef;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::super::spawner::{EVENT_ABILITY_BEGIN, EVENT_ABILITY_END};
use super::super::messaging::send_entity_method_to_self_and_witnesses;

/// Window for the per-ability `abilities.sequence` WARNs. The condition is a
/// seed defect that holds until the next deploy, so one row a minute per
/// ability (with `suppressed`) says everything the AI tick's repeats would.
pub(crate) const SEQUENCE_WARN_INTERVAL: Duration = Duration::from_secs(60);

/// `onSequence` args, 26 bytes: `INT32 KismetEventSetSeqID, INT32 SourceID,
/// INT32 TargetID, INT8 PrimaryTarget, FLOAT ImpactTime,
/// ARRAY<NameValuePair> NameValuePairs, INT8 ViewType, INT32 InstanceId`
/// (`docs/protocol/client-method-dispatch-table.md`, SGWSpawnableEntity 1).
fn sequence_args(sequence_id: i32, source_id: u32, target_id: i32, effect_seq: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(26);
    args.extend_from_slice(&sequence_id.to_le_bytes()); // KismetEventSetSeqID (sequence_id)
    args.extend_from_slice(&(source_id as i32).to_le_bytes()); // SourceID
    args.extend_from_slice(&target_id.to_le_bytes()); // TargetID
    args.push(1); // PrimaryTarget = true
    args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
    args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs array count = 0
    args.push(0); // ViewType = KISMET_VIEW_Witness
    args.extend_from_slice(&effect_seq.to_le_bytes()); // InstanceId
    args
}

/// `Some(suppressed)` when this occurrence of `outcome` for `ability_id`
/// should be written.
fn admit_warn(space_mgr: &mut SpaceManager, ability_id: i32, outcome: &'static str) -> Option<u32> {
    space_mgr.ability_sequence_log.admit(
        ability_id as u32,
        outcome,
        Instant::now(),
        SEQUENCE_WARN_INTERVAL,
    )
}

/// Throttled WARN for an NPC attack that lands with no animation.
fn warn_missing(
    space_mgr: &mut SpaceManager,
    entity_id: u32,
    target_id: i32,
    ability_id: i32,
    event_set_id: Option<i32>,
    outcome: &'static str,
) {
    let Some(suppressed) = admit_warn(space_mgr, ability_id, outcome) else {
        return;
    };
    tracing::warn!(
        target: "abilities.sequence",
        event = "ability_end",
        outcome,
        source_id = entity_id,
        target_id,
        ability_id,
        event_set_id = event_set_id.unwrap_or(0),
        suppressed,
        "PlaySequence: NPC attack expected an Ability_End onSequence but none can be resolved \
         -- damage lands with no attack animation on any client"
    );
}

/// Send the Ability_Begin (warmup only) and Ability_End `onSequence` for a
/// committed cast. Returns the Ability_End witness count, or `None` when no
/// Ability_End went out.
pub(super) async fn send_ability_sequences(
    entity_id: u32,
    target_id: i32,
    ability_id: i32,
    ability_def: Option<&AbilityDef>,
    effect_seq: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<usize> {
    let is_npc = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| !e.is_player);

    let Some(def) = ability_def else {
        if is_npc {
            warn_missing(
                space_mgr,
                entity_id,
                target_id,
                ability_id,
                None,
                "no_ability_def",
            );
        }
        return None;
    };
    let Some(event_set_id) = def.event_set_id else {
        if is_npc {
            warn_missing(
                space_mgr,
                entity_id,
                target_id,
                ability_id,
                None,
                "no_event_set",
            );
        }
        return None;
    };

    // Ability_Begin (event 1000) only when the ability has a warmup phase.
    if def.warmup > 0.0 {
        if let Some(&begin_seq_id) = space_mgr
            .sequence_map
            .get(&(event_set_id, EVENT_ABILITY_BEGIN))
        {
            let args = sequence_args(begin_seq_id, entity_id, target_id, effect_seq);
            let witness_count = send_entity_method_to_self_and_witnesses(
                entity_id,
                crate::mercury::method_idx::ON_SEQUENCE,
                args,
                tx,
                space_mgr,
            )
            .await;
            tracing::debug!(
                target: "abilities.sequence",
                event = "ability_begin",
                source_id = entity_id,
                target_id,
                ability_id,
                sequence_id = begin_seq_id,
                event_set_id,
                witness_count,
                "onSequence broadcast: Ability_Begin (warmup animation)"
            );
        }
    }

    // Ability_End (event 1001): the main fire animation.
    let Some(&end_seq_id) = space_mgr
        .sequence_map
        .get(&(event_set_id, EVENT_ABILITY_END))
    else {
        if is_npc {
            warn_missing(
                space_mgr,
                entity_id,
                target_id,
                ability_id,
                Some(event_set_id),
                "no_end_sequence",
            );
        } else {
            tracing::debug!(
                target: "abilities.sequence",
                event = "ability_end",
                outcome = "no_end_sequence",
                source_id = entity_id,
                ability_id,
                event_set_id,
                "onSequence: no Ability_End sequence found for event_set"
            );
        }
        return None;
    };
    let args = sequence_args(end_seq_id, entity_id, target_id, effect_seq);
    let witness_count = send_entity_method_to_self_and_witnesses(
        entity_id,
        crate::mercury::method_idx::ON_SEQUENCE,
        args,
        tx,
        space_mgr,
    )
    .await;
    tracing::debug!(
        target: "abilities.sequence",
        event = "ability_end",
        source_id = entity_id,
        target_id,
        ability_id,
        sequence_id = end_seq_id,
        event_set_id,
        witness_count,
        "onSequence broadcast: Ability_End (main fire animation)"
    );
    if is_npc && witness_count == 0 {
        if let Some(suppressed) = admit_warn(space_mgr, ability_id, "no_witnesses") {
            tracing::warn!(
                target: "abilities.sequence",
                event = "ability_end",
                outcome = "no_witnesses",
                source_id = entity_id,
                target_id,
                ability_id,
                sequence_id = end_seq_id,
                suppressed,
                "PlaySequence: NPC attack onSequence expected at least one witness but had \
                 none -- the target is not seeing the NPC that shoots it"
            );
        }
    }
    Some(witness_count)
}
