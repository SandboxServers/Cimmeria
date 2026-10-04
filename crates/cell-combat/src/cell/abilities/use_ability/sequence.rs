//! The three ability-phase `onSequence`s (client method 1): `Ability_Begin`,
//! `Ability_End` and `Ability_Interrupt`.
//!
//! All three carry the same 26-byte `onSequence` body; only the sequence id
//! differs. One builder keeps the phases byte-identical, which matters
//! because a launch and its fire now run in different ticks (AT-10).
//!
//! **Routing (NA43).** [`play_ability_sequence`] sends every phase to the
//! caster **and** its witnesses, as Python `AbilityManager.playSequence` did
//! (`deprecated/python/cell/AbilityManager.py:879-893`: `ent.client`, then
//! `ent.witnesses`). A player's own client gets an `EntityMethodCall` and
//! every player who has them in AoI gets a `WitnessEntityMethod`, so other
//! players see the charge, the shot and the cancel. Before NA43 each phase
//! went through `send_entity_method`, which routes a player to self only,
//! and nobody else saw a player fire. An NPC has no client, so it gets the
//! witness fan-out alone, as before.
//!
//! **Negative logs (NA43).** For an NPC attacker, each way the
//! `Ability_End` can go missing while the damage still lands is a throttled
//! WARN on `abilities.sequence`:
//!
//! | `outcome` | Meaning |
//! |---|---|
//! | `no_ability_def` | the ability has no `resources.abilities` row loaded |
//! | `no_event_set` | its `event_set_id` is NULL, so nothing is looked up |
//! | `no_end_sequence` | the event set has no Ability_End (1001) sequence |
//! | `no_witnesses` | the NPC shot with nobody in AoI to see it |
//! | `stance_not_announced` | the NPC shot before its `BSF_InCombat` stance reached its witnesses, so the client draws no muzzle flash, tracer or weapon sound |
//!
//! The throttle is keyed by ability id (`SpaceManager::ability_sequence_log`)
//! because the first three are facts about the seed row that every NPC
//! firing the ability repeats. Player abilities are not warned about: 1851 of
//! the 1886 seeded abilities have no event set, and most player abilities
//! legitimately animate through other paths.
//!
//! `Ability_Begin` and `Ability_Interrupt` do not WARN. A missing Begin
//! leaves the charge unanimated but the shot still plays at `Ability_End`,
//! whose WARN names the same seed row; an interrupted cast deals no damage,
//! so a missing Interrupt is not a hit from an invisible attacker. Most
//! event sets carry no Interrupt sequence at all. Both rows still report
//! `witness_count` at DEBUG.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::super::spawner::{
    EVENT_ABILITY_BEGIN, EVENT_ABILITY_END, EVENT_ABILITY_INTERRUPT,
};
use super::super::messaging::send_entity_method_to_self_and_witnesses;

/// Window for the per-ability `abilities.sequence` WARNs. The condition is a
/// seed defect that holds until the next deploy, so one row a minute per
/// ability (with `suppressed`) says everything the AI tick's repeats would.
pub(crate) const SEQUENCE_WARN_INTERVAL: Duration = Duration::from_secs(60);

/// Pack `onSequence(KismetEventSetSeqID, SourceID, TargetID, PrimaryTarget,
/// ImpactTime, NameValuePairs[], ViewType, InstanceId)` for an ability phase.
pub(super) fn ability_sequence_args(
    sequence_id: i32,
    source_id: u32,
    target_id: i32,
    instance_id: i32,
) -> Vec<u8> {
    let mut seq_args = Vec::with_capacity(28);
    seq_args.extend_from_slice(&sequence_id.to_le_bytes()); // KismetEventSetSeqID (sequence_id)
    seq_args.extend_from_slice(&(source_id as i32).to_le_bytes()); // SourceID
    seq_args.extend_from_slice(&target_id.to_le_bytes()); // TargetID
    seq_args.push(1); // PrimaryTarget = true
    seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
    seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs array count = 0
    seq_args.push(0); // ViewType = KISMET_VIEW_Witness
    seq_args.extend_from_slice(&instance_id.to_le_bytes()); // InstanceId
    seq_args
}

/// Which ability-phase sequence to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell::abilities) enum AbilityPhase {
    /// Kismet event 1000, the warmup animation, sent at launch.
    Begin,
    /// Kismet event 1001, the fire animation, sent when the cast fires.
    End,
    /// Kismet event 1002, sent when a warmup is cancelled.
    Interrupt,
}

impl AbilityPhase {
    fn kismet_event(self) -> i32 {
        match self {
            Self::Begin => EVENT_ABILITY_BEGIN,
            Self::End => EVENT_ABILITY_END,
            Self::Interrupt => EVENT_ABILITY_INTERRUPT,
        }
    }

    /// The `event` field of the `abilities.sequence` rows.
    fn event(self) -> &'static str {
        match self {
            Self::Begin => "ability_begin",
            Self::End => "ability_end",
            Self::Interrupt => "ability_interrupt",
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::Begin => "Ability_Begin (warmup animation)",
            Self::End => "Ability_End (main fire animation)",
            Self::Interrupt => "Ability_Interrupt (warmup cancelled)",
        }
    }
}

/// One ability-phase `onSequence` to send.
#[derive(Debug, Clone, Copy)]
pub(in crate::cell::abilities) struct PhaseSequence {
    pub phase: AbilityPhase,
    pub entity_id: u32,
    pub ability_id: i32,
    pub target_id: i32,
    /// The cast's `effect_seq`, minted at launch; the client pairs the
    /// phases of one cast by it.
    pub instance_id: i32,
    /// The ability's `event_set_id`; `None` when the row has none.
    pub event_set_id: Option<i32>,
}

fn is_npc(space_mgr: &SpaceManager, entity_id: u32) -> bool {
    space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| !e.is_player)
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

/// Throttled WARN for an NPC attack that lands with no fire animation.
/// A no-op for a player caster.
pub(super) fn warn_unanimated_npc_attack(
    space_mgr: &mut SpaceManager,
    entity_id: u32,
    target_id: i32,
    ability_id: i32,
    event_set_id: Option<i32>,
    outcome: &'static str,
) {
    if !is_npc(space_mgr, entity_id) {
        return;
    }
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

/// Play one ability-phase `onSequence` to the caster and its witnesses.
///
/// Returns the witness count, or `None` when nothing was sent because the
/// ability has no event set or the event set has no sequence for the phase.
/// For [`AbilityPhase::End`] and an NPC caster, each of those, and a send
/// that reached no witness, is a throttled WARN (see the module docs).
pub(in crate::cell::abilities) async fn play_ability_sequence(
    seq: PhaseSequence,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<usize> {
    let PhaseSequence {
        phase,
        entity_id,
        ability_id,
        target_id,
        instance_id,
        event_set_id,
    } = seq;
    let warns = phase == AbilityPhase::End;

    let Some(event_set_id) = event_set_id else {
        if warns {
            warn_unanimated_npc_attack(
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
    let Some(&sequence_id) = space_mgr
        .sequence_map
        .get(&(event_set_id, phase.kismet_event()))
    else {
        if warns && is_npc(space_mgr, entity_id) {
            warn_unanimated_npc_attack(
                space_mgr,
                entity_id,
                target_id,
                ability_id,
                Some(event_set_id),
                "no_end_sequence",
            );
        } else if warns {
            let who = space_mgr.player_identity(entity_id);
            tracing::debug!(
                target: "abilities.sequence",
                event = "ability_end",
                outcome = "no_end_sequence",
                account_id = who.account_id,
                player_id = who.player_id,
                source_id = entity_id,
                cast_id = instance_id,
                ability_id,
                event_set_id,
                "onSequence: no Ability_End sequence found for event_set"
            );
        }
        return None;
    };

    let target_id =
        super::summon::phase_sequence_target(space_mgr, entity_id, ability_id, target_id);
    let target_id =
        super::owner_pet::phase_sequence_target(space_mgr, entity_id, ability_id, target_id);
    let target_id = super::super::deployable::phase_sequence_target(
        space_mgr, entity_id, ability_id, target_id,
    );
    let args = ability_sequence_args(sequence_id, entity_id, target_id, instance_id);
    let witness_count = send_entity_method_to_self_and_witnesses(
        entity_id,
        crate::mercury::method_idx::ON_SEQUENCE,
        args,
        tx,
        space_mgr,
    )
    .await;
    let who = space_mgr.player_identity(entity_id);
    tracing::debug!(
        target: "abilities.sequence",
        event = phase.event(),
        account_id = who.account_id,
        player_id = who.player_id,
        source_id = entity_id,
        // The sequence's InstanceId is the cast's `cast_id` (AB-T1).
        cast_id = instance_id,
        target_id,
        ability_id,
        sequence_id,
        event_set_id,
        witness_count,
        "onSequence broadcast: {}",
        phase.describe()
    );

    // Witnesses can only draw the muzzle flash and tracer of a shot whose
    // Source pawn is in its combat stance (`npc_ai::combat_stance`). A Fighting
    // NPC is announced before it fires, so this row means an attack from
    // outside the Fighting pass (content, a GM ability) or a regression.
    if warns
        && witness_count > 0
        && space_mgr
            .get_entity(entity_id)
            .is_some_and(|e| !e.is_player && !crate::cell::service::npc_ai::stance_announced(e))
    {
        if let Some(suppressed) = admit_warn(space_mgr, ability_id, "stance_not_announced") {
            tracing::warn!(
                target: "abilities.sequence",
                event = "ability_end",
                outcome = "stance_not_announced",
                source_id = entity_id,
                target_id,
                ability_id,
                sequence_id,
                witness_count,
                suppressed,
                "PlaySequence: NPC fired an Ability_End before its BSF_InCombat stance was \
                 announced -- witnesses see no muzzle flash, tracer or weapon sound"
            );
        }
    }

    if warns && witness_count == 0 && is_npc(space_mgr, entity_id) {
        if let Some(suppressed) = admit_warn(space_mgr, ability_id, "no_witnesses") {
            tracing::warn!(
                target: "abilities.sequence",
                event = "ability_end",
                outcome = "no_witnesses",
                source_id = entity_id,
                target_id,
                ability_id,
                sequence_id,
                suppressed,
                "PlaySequence: NPC attack onSequence expected at least one witness but had \
                 none -- the target is not seeing the NPC that shoots it"
            );
        }
    }
    Some(witness_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte layout pinned so the Begin, End and Interrupt emitters cannot
    /// drift apart (Begin and End used to be two hand-rolled copies).
    #[test]
    fn ability_sequence_args_layout() {
        let b = ability_sequence_args(0x0102_0304, 7, -1, 42);
        assert_eq!(b.len(), 26);
        assert_eq!(&b[0..4], &0x0102_0304i32.to_le_bytes());
        assert_eq!(&b[4..8], &7i32.to_le_bytes());
        assert_eq!(&b[8..12], &(-1i32).to_le_bytes());
        assert_eq!(b[12], 1);
        assert_eq!(&b[13..17], &0.0f32.to_le_bytes());
        assert_eq!(&b[17..21], &0u32.to_le_bytes());
        assert_eq!(b[21], 0);
        assert_eq!(&b[22..26], &42i32.to_le_bytes());
    }
}
