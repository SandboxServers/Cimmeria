//! The dial-side address-book gate (security finding CAT-O-01).
//!
//! `onDialGate` carries `targetAddressId` as a raw client `INT32`. The 2009
//! server refused any address the character did not hold
//! (`deprecated/python/cell/SGWPlayer.py:2060-2064`); Cimmeria loaded
//! `known_stargates` from the DB, serialised it to the client in
//! `setupStargateInfo`, and then read it nowhere on the server, so any
//! client could dial any of the 28 seeded gates and cross-world teleport
//! itself into content it never unlocked.
//!
//! One function, one call site — the top of
//! [`super::handle_dial_gate`]. Deliberately its own file so the
//! authorization surface is a thing you can point at, and so the gate does
//! not drift into the dial state machine next to it.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `EErrorCodeSystem` value carried in `onErrorCode`'s `SystemID`.
///
/// `ERRORCODE_SYSTEM_Ability = 0` is the *only* token the enum ever defines
/// (`deprecated/entities-editor/editor/enumerations.xml:1219`), so every
/// `onErrorCode` in the game — including the existing out-of-range feedback
/// in `abilities::use_ability` — ships a 0 here.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// `EConditionHandlerFeedback::CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress`
/// (`deprecated/entities-editor/editor/enumerations.xml:1404`). The client
/// renders the feedback string for this code; it is the only one in the enum
/// that names the address book, and 2009's own free-text
/// `onError("Failed to dial: not a known stargate address")` has no Cimmeria
/// equivalent (`SGWPlayer.def` exposes `onErrorCode` and nothing else).
const FEEDBACK_ENTITY_DOES_NOT_HAVE_STARGATE_ADDRESS: u16 = 180;

/// Does this player's address book contain `target_address_id`?
///
/// On refusal, emits the client-visible `onErrorCode` (121) and returns
/// `false`; the caller cancels any dial in flight and returns without
/// touching any other state.
///
/// **No GM exemption here, deliberately.** 2009 had none either, and an
/// `access_level` branch would put a second authorization surface on a check
/// whose whole value is having exactly one. `gmDHD` is the one caller that
/// needs an address it may not hold, and it solves that by topping up the
/// caller's in-memory book before dialling — see
/// [`crate::cell::cell_methods::gm::travel`], which is already authorised
/// against the session's access level by the cell-method GM gate.
pub(super) async fn player_knows_stargate(
    entity_id: u32,
    target_address_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(entity) = space_mgr.get_entity(entity_id) else {
        // Front-runs `handle_dial_gate`'s own entity-missing branch, which
        // keys on the space binding rather than the entity. Both refuse;
        // this one gets there first because the address book lives on the
        // entity.
        tracing::warn!(
            entity_id,
            target_address_id,
            reason = "dial_entity_missing",
            "onDialGate: no cell entity for the caller — refusing the dial; \
             the client gets no travel and no feedback"
        );
        return false;
    };
    if entity.known_stargates.contains(&target_address_id) {
        return true;
    }

    // `known_count = 0` has two causes worth telling apart when triaging a
    // "my gate is greyed out" report: a character that genuinely holds no
    // addresses (the column defaults to `'{}'` and `base::character_create`
    // does not seed it), or a dial sent inside the window between
    // `CreateEntity` and `InitPlayerState`, where the entity exists but its
    // address book has not arrived. Both refuse, which is the right
    // direction; the count is in the fields so the log can distinguish them
    // from "holds addresses, just not this one".
    tracing::warn!(
        entity_id,
        player_id = entity.player_id,
        target_address_id,
        known_count = entity.known_stargates.len(),
        reason = "unknown_stargate_address",
        "onDialGate: address is not in the player's known list — refusing the dial; \
         the traveller stays put and the client is told why"
    );

    // onErrorCode(UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID).
    // `InstanceID` is 0, not the stargate id: the client keys the field on
    // `SystemID`, and system 0 is the ability subsystem — handing it a
    // stargate id there invites a lookup against an unrelated ability row.
    let mut args = Vec::with_capacity(7);
    args.push(ERRORCODE_SYSTEM_ABILITY);
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&FEEDBACK_ENTITY_DOES_NOT_HAVE_STARGATE_ADDRESS.to_le_bytes());
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::cell::client_methods::player::ON_ERROR_CODE,
            args,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            target_address_id,
            reason = "error_code_send_failed",
            "onDialGate: refusal onErrorCode could not be enqueued ({e}) — the dial is \
             still refused, but the client gets no feedback and may look hung"
        );
    }
    false
}
