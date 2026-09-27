//! The summon's target VFX, held until the pet has been introduced.
//!
//! A summon (PT-03) spawns the pet into the space; the next AoI tick sends
//! its CREATE_ENTITY. The Goa'uld summon's ground effect (event set 1122,
//! `Effect_Init` -> sequence 2293) is an `onSequence` whose target is the
//! pet, so it must not reach a client before that client knows the pet.
//! The summon therefore queues the finished `onSequence` bytes here, and
//! [`pet_arrival_tick`] sends them, once, when the owner witnesses the pet:
//! to every player who witnesses it at that moment (the AoI tick put each of
//! them in the witness set in the same pass that sent their `EnteredAoI`).
//!
//! Python played an effect sequence on the effect's target entity, to its
//! client and its witnesses (`AbilityManager.playSequence`); a pet has no
//! client, so the witnesses are the whole send.
//!
//! The queue lives on `PetRegistry` and `forget_pet` scrubs it, so every
//! teardown path drops a pending VFX with its pet. The drain also checks
//! that the pet still belongs to the queued owner, and that whoever holds
//! the owner's entity id now is the player who summoned it
//! (`PetRegistry::summoner_matches`): entity ids are reused, and the
//! owner-only intro is withheld from a new holder of the id, so the VFX
//! must be too.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::teardown::owner_identity;

/// How long a queued VFX waits for the owner to witness its pet. The intro
/// normally lands on the next 100 ms AoI tick; past this the effect would
/// play on a pet the player has long been looking at, so it is dropped.
pub const ARRIVAL_TIMEOUT: Duration = Duration::from_secs(2);

/// One queued summon VFX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetArrival {
    /// The summoner, whose view of the pet releases the VFX.
    pub owner_id: u32,
    /// The pet's template, captured at summon so every row about the VFX
    /// names it, even one written after the pet entity is gone.
    pub template_id: i32,
    /// The resolved sequence id, for the log.
    pub sequence_id: i32,
    /// The `onSequence` arguments, built by the summon.
    pub args: Vec<u8>,
    /// When the summon queued it.
    pub queued_at: Instant,
}

impl PetArrival {
    /// A VFX queued now.
    pub fn new(owner_id: u32, template_id: i32, sequence_id: i32, args: Vec<u8>) -> Self {
        Self {
            owner_id,
            template_id,
            sequence_id,
            args,
            queued_at: Instant::now(),
        }
    }
}

impl super::PetRegistry {
    /// Queue `arrival` for `pet`, replacing any earlier one. Ignored when
    /// `pet` is not a registered pet, so the queue never outlives the maps.
    pub fn queue_arrival(&mut self, pet: u32, arrival: PetArrival) {
        if self.is_pet(pet) {
            self.arrivals.insert(pet, arrival);
        }
    }

    /// The queued VFX for `pet`, if any.
    pub fn pending_arrival(&self, pet: u32) -> Option<&PetArrival> {
        self.arrivals.get(&pet)
    }
}

/// What the drain does with one queued VFX.
enum ArrivalStep {
    /// Keep waiting for the owner to witness the pet.
    Wait,
    /// Send to these witnesses.
    Send(Vec<u32>),
    /// Drop it, with this `reason`.
    Drop(&'static str),
}

fn arrival_step(
    space_mgr: &SpaceManager,
    pet: u32,
    arrival: &PetArrival,
    now: Instant,
) -> ArrivalStep {
    match space_mgr.pets.owner_of(pet) {
        None => return ArrivalStep::Drop("pet_gone"),
        // `forget_pet` scrubs the queue on every removal, so a different
        // owner means the id was reused past a missed scrub.
        Some(owner) if owner != arrival.owner_id => return ArrivalStep::Drop("owner_mismatch"),
        Some(_) => {}
    }
    // The owner's id now belongs to another player (destroyed and reused
    // before the sweep): that player is not the summoner, so the VFX is not
    // theirs to release.
    if !space_mgr
        .pets
        .summoner_matches(pet, space_mgr.player_identity(arrival.owner_id))
    {
        return ArrivalStep::Drop("owner_identity_mismatch");
    }
    if space_mgr.get_entity(pet).is_none() {
        return ArrivalStep::Drop("pet_gone");
    }
    let witnesses = space_mgr.get_witnesses_of(pet);
    if witnesses.contains(&arrival.owner_id) {
        return ArrivalStep::Send(witnesses);
    }
    if now.duration_since(arrival.queued_at) >= ARRIVAL_TIMEOUT {
        return ArrivalStep::Drop("owner_never_witnessed");
    }
    ArrivalStep::Wait
}

/// Per-tick drain, run after the AoI tick. Returns how many VFX reached at
/// least one witness.
/// Returns at once when nothing is queued.
pub async fn pet_arrival_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    drain_arrivals(Instant::now(), tx, space_mgr).await
}

/// [`pet_arrival_tick`] with the clock as a parameter, for tests.
pub async fn drain_arrivals(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    if space_mgr.pets.arrivals.is_empty() {
        return 0;
    }
    let mut pets: Vec<u32> = space_mgr.pets.arrivals.keys().copied().collect();
    pets.sort_unstable();

    let mut sent = 0;
    for pet in pets {
        let Some(arrival) = space_mgr.pets.arrivals.get(&pet) else {
            continue;
        };
        match arrival_step(space_mgr, pet, arrival, now) {
            ArrivalStep::Wait => {}
            ArrivalStep::Drop(reason) => {
                let Some(arrival) = space_mgr.pets.arrivals.remove(&pet) else {
                    continue;
                };
                let owner_id = arrival.owner_id;
                // The summoner's identity captured at summon (the live one
                // may belong to a new holder of a reused id), Rule 5.
                let id = owner_identity(space_mgr, pet, Some(owner_id));
                let registered_owner = space_mgr.pets.owner_of(pet);
                let waited_ms = now.duration_since(arrival.queued_at).as_millis() as u64;
                // A pet gone before its intro is ordinary (despawned at
                // once): DEBUG. An owner who never saw its live pet means the
                // intro went missing, and an owner or identity mismatch means
                // a reused id past a missed scrub: server faults no client
                // can cause, so WARN.
                if reason == "pet_gone" {
                    tracing::debug!(
                        target: "pets.lifecycle",
                        event = "arrival_vfx_dropped",
                        decision_outcome = "arrival_vfx_dropped",
                        entity_id = pet,
                        pet_id = pet,
                        owner_id,
                        account_id = id.account_id,
                        player_id = id.player_id,
                        template_id = arrival.template_id,
                        sequence_id = arrival.sequence_id,
                        waited_ms,
                        reason,
                        "summon VFX dropped with its pet"
                    );
                } else {
                    tracing::warn!(
                        target: "pets.lifecycle",
                        event = "arrival_vfx_dropped",
                        decision_outcome = "arrival_vfx_dropped",
                        entity_id = pet,
                        pet_id = pet,
                        owner_id,
                        registered_owner_id = registered_owner,
                        account_id = id.account_id,
                        player_id = id.player_id,
                        template_id = arrival.template_id,
                        sequence_id = arrival.sequence_id,
                        waited_ms,
                        reason,
                        "summon VFX dropped before reaching the owner"
                    );
                }
            }
            ArrivalStep::Send(witnesses) => {
                let Some(arrival) = space_mgr.pets.arrivals.remove(&pet) else {
                    continue;
                };
                // The summoner as captured at summon (Rule 5), resolved
                // before the sends so every row below names the same player.
                let id = owner_identity(space_mgr, pet, Some(arrival.owner_id));
                let mut delivered = 0usize;
                for &witness_id in &witnesses {
                    if tx
                        .send(CellToBaseMsg::WitnessEntityMethod {
                            witness_id,
                            entity_id: pet,
                            method_index: crate::mercury::method_idx::ON_SEQUENCE,
                            args: arrival.args.clone(),
                            entity_is_player: false,
                        })
                        .await
                        .is_ok()
                    {
                        delivered += 1;
                    } else {
                        tracing::warn!(
                            target: "pets.lifecycle",
                            event = "arrival_vfx_send_failed",
                            decision_outcome = "arrival_vfx_send_failed",
                            entity_id = pet,
                            pet_id = pet,
                            owner_id = arrival.owner_id,
                            account_id = id.account_id,
                            player_id = id.player_id,
                            template_id = arrival.template_id,
                            witness_id,
                            "summon VFX could not be queued (base channel closed)"
                        );
                    }
                }
                if delivered == 0 {
                    // Every witness send failed: the VFX reached nobody, so
                    // it is neither logged as sent nor counted.
                    tracing::warn!(
                        target: "pets.lifecycle",
                        event = "arrival_vfx_undelivered",
                        decision_outcome = "arrival_vfx_undelivered",
                        entity_id = pet,
                        pet_id = pet,
                        owner_id = arrival.owner_id,
                        account_id = id.account_id,
                        player_id = id.player_id,
                        template_id = arrival.template_id,
                        sequence_id = arrival.sequence_id,
                        witness_count = witnesses.len(),
                        reason = "cell_to_base_closed",
                        "summon VFX reached no witness (base channel closed)"
                    );
                    continue;
                }
                tracing::debug!(
                    target: "pets.lifecycle",
                    event = "arrival_vfx_sent",
                    decision_outcome = "arrival_vfx_sent",
                    entity_id = pet,
                    pet_id = pet,
                    owner_id = arrival.owner_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    template_id = arrival.template_id,
                    sequence_id = arrival.sequence_id,
                    witness_count = witnesses.len(),
                    delivered_count = delivered,
                    "summon VFX sent to the pet's witnesses"
                );
                sent += 1;
            }
        }
    }
    sent
}
