//! Handler for `BaseToCellMsg::RequestEntityUpdate` -- the client's
//! cache-stamp handshake (`requestEntityUpdate`, msg `0x07`).
//!
//! **This is not a recovery request.** The client's `EntityManager::onEntityEnter`
//! (`ghidra://SGW.exe@0x00dd24f0`) fires it once for *every* non-player entity
//! entering its AoI, carrying `[u32 entityId][N × u32 cacheStamp]` with `N`
//! always 0 on this client build -- BigWorld's cache-stamp versioning is
//! never populated. See
//! `docs/reverse-engineering/findings/request-entity-update-cache-stamp.md`
//! for the full RE evidence (issues #838, #1000).
//!
//! PR #390 originally treated this message as a recovery signal (an NPC's
//! `createEntity` dropped on the wire past the 20-retry cap) and re-emitted a
//! full `EnteredAoI` + cascade whenever the requested id was in the witness's
//! AoI. That assumption doesn't survive the RE: the message fires identically
//! on every routine entry, so re-creating on every request would double the
//! `CREATE_ENTITY` + property cascade for the overwhelming majority of calls,
//! which were never actually missing anything. The wire carries no field that
//! distinguishes "just entered, this is the routine handshake" from
//! "genuinely missing state" -- there is no signal here left to recover from.
//!
//! The cell's answer, therefore:
//!
//! - **id is in the witness's current AoI**: the base already sent this
//!   witness a full `CREATE_ENTITY` + cascade for it. The client already has
//!   it. Answer with **nothing**.
//! - **id is NOT in the witness's current AoI**: refused, same as before this
//!   fix -- a witness must not be able to pull another entity's state by
//!   asserting an arbitrary id (`docs/architecture/negative-logging-convention.md`
//!   Pattern C: logged, not silently dropped).
//!
//! A spammed request is truncated to [`MAX_REQUEST_ENTITIES`] to bound the
//! per-call cost (witness-set lookup per id).

use cimmeria_common::EntityId;

use super::super::super::space_manager::SpaceManager;

/// Cap on entity ids honoured per `RequestEntityUpdate` payload.
///
/// The corrected wire format (`[u32 entityId][N × u32 cacheStamp]`) yields
/// exactly one entity id per real client message today, but the cap stays as
/// a defensive bound against a future client build -- or a malicious one --
/// that packs more.
const MAX_REQUEST_ENTITIES: usize = 64;

/// Acknowledge a `requestEntityUpdate` cache-stamp handshake for each
/// `entity_id` the witness reports. Ids already in `witness_id`'s AoI get no
/// reply (the client already has full state); ids outside it are refused.
pub(super) async fn handle(witness_id: u32, mut entity_ids: Vec<u32>, space_mgr: &SpaceManager) {
    let requested = entity_ids.len();
    let truncated = requested > MAX_REQUEST_ENTITIES;
    if truncated {
        tracing::warn!(
            witness_id,
            witness_name = space_mgr.entity_label(witness_id),
            requested,
            cap = MAX_REQUEST_ENTITIES,
            reason = "request_too_large",
            "RequestEntityUpdate: payload exceeds cap -- truncating"
        );
        entity_ids.truncate(MAX_REQUEST_ENTITIES);
    }

    let identity = space_mgr.player_identity(witness_id);

    // The witness must exist in some space -- otherwise no AoI bookkeeping
    // exists to authorise against.
    let Some(witness) = space_mgr.get_entity(witness_id) else {
        tracing::warn!(
            witness_id,
            witness_name = identity.player_name,
            requested,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id = identity.player_id,
            player_name = identity.player_name,
            reason = "witness_not_in_space",
            "RequestEntityUpdate: witness entity not found -- dropping"
        );
        return;
    };

    let mut known = 0usize;
    let mut unknown = 0usize;
    // Kept for the acknowledgement log: which entity the client just created
    // is the evidence for "the client has it" in visibility investigations
    // (a real message carries exactly one id).
    let requested_ids = entity_ids.clone();
    for entity_id in entity_ids {
        let target_eid = EntityId(entity_id as i32);
        if witness.witnesses.contains(&target_eid) {
            // Normal case: the client already has this entity's full state
            // from its original CREATE_ENTITY. Nothing to send.
            known += 1;
        } else {
            // The id is outside this witness's AoI. Refuse rather than leak
            // another entity's state to a witness that hasn't earned
            // visibility of it -- same anti-probe posture as before this fix.
            unknown += 1;
            let names = space_mgr.entity_names(entity_id);
            tracing::warn!(
                witness_id,
                witness_name = identity.player_name,
                entity_id,
                entity_name = names.entity_name,
                template_id = names.template_id,
                template_name = names.template_name,
                account_id = identity.account_id,
                account_name = identity.account_name,
                player_id = identity.player_id,
                player_name = identity.player_name,
                reason = "not_in_witness_aoi",
                "RequestEntityUpdate: id outside witness's AoI -- refusing"
            );
        }
    }

    // This now fires once per non-player entity per AoI entry -- i.e.
    // routinely, at AoI-tick volume -- so the common (`known`) outcome stays
    // at debug. The `unknown` branch above already warns per occurrence.
    tracing::debug!(
        witness_id,
        witness_name = identity.player_name,
        requested,
        truncated,
        known,
        unknown,
        entity_ids = ?requested_ids,
        account_id = identity.account_id,
        account_name = identity.account_name,
        player_id = identity.player_id,
        player_name = identity.player_name,
        "RequestEntityUpdate acknowledged"
    );
}
