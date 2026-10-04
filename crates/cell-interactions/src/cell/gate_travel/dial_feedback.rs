//! What the dialling player is told when a dial is refused (#727).
//!
//! Every refusal in [`super::handle_dial_gate`] (and the arrival refusal in
//! `perform_gate_travel`) ends here, so a DHD press always gets a visible
//! answer on the first press (project rule) instead of silence.
//!
//! **Surface.** One `onPlayerCommunication("SYSTEM", 0, CHAN_FEEDBACK, text)`
//! (client method 28) per refusal: the sky-blue Info-tab line every other
//! one-player refusal in the cell uses (pets, org registrar, bank, duel).
//!
//! Not `onDHDReply` (SGWPlayer client method 100, `WSTRING aMessage`), even
//! though the `.def` declares it for exactly this. #1024 traced its
//! recorded subscriber, VCommunicator, to the same `Communicator` chat/
//! system-communication component that renders `onPlayerCommunication`
//! itself — `onDHDReply`'s RTTI accessor sits inside a dense cluster of
//! `Communicator` chat-event accessors (`onSystemCommunication`,
//! `onTellSent`, `onChatJoined`/`onChatLeft`, `onNickChanged`, …), and the
//! DHD window's own CEGUI Lua (`Content/UI/Core/DHD/DHD.lua`) has no text
//! widget at all — it only shows/hides a frame around an external Scaleform
//! movie. So `onDHDReply` is not a DHD-window message, and switching to it
//! would trade this verified, byte-exact line for an unverified one that
//! appears to land in the same chat-adjacent place, with a less specific
//! payload (one bare `WSTRING`, no channel or speaker)
//! (`docs/reverse-engineering/findings/stargate-dhd-state-machine.md`
//! §"onDHDReply render-target resolution (#1024, 2026-09-28)"). Kept unused
//! on purpose.
//!
//! Not the 2009 shape verbatim either. `SGWPlayer.onError`
//! (`deprecated/python/cell/SGWPlayer.py:879-884`) sent
//! `onPlayerCommunication('', 0, CHAN_server, msg)`. Channel 8 opens a modal
//! "Server Message" prompt in the client's chat Lua, and an empty speaker
//! garbles the line; `CHAN_FEEDBACK` with speaker `SYSTEM` is the text the
//! 2009 server meant, on the channel the client shows without a popup. The
//! `Failed to dial: …` wording is kept from 2009.
//!
//! **Existence oracle.** An address the player does not hold and an address
//! that does not exist produce the *same* bytes: [`DialRefusal::UnknownAddress`]
//! covers both, and it also carries the address-book `onErrorCode(180)` the
//! CAT-O-01 gate has always sent, so neither branch can be told apart from the
//! other by message count or content.

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `EErrorCodeSystem` value carried in `onErrorCode`'s `SystemID`.
///
/// `ERRORCODE_SYSTEM_Ability = 0` is the *only* token the enum ever defines
/// (`deprecated/entities-editor/editor/enumerations.xml:1219`), so every
/// `onErrorCode` in the game ships a 0 here.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// `EConditionHandlerFeedback::CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress`
/// (`deprecated/entities-editor/editor/enumerations.xml:1404`), the only code
/// in the enum that names the address book. Kept as a secondary signal beside
/// the text line: its on-screen rendering is unverified.
const FEEDBACK_ENTITY_DOES_NOT_HAVE_STARGATE_ADDRESS: u16 = 180;

/// Why a dial was refused, as far as the player is allowed to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialRefusal {
    /// The address is not in the player's book, or no such gate exists.
    /// One variant on purpose: see the module doc's existence-oracle note.
    UnknownAddress,
    /// No cell entity or space binding for the caller (the load window, or
    /// a dial racing a transfer).
    NotInWorld,
    /// The dialled gate is on the world the player is already on.
    AlreadyOnDestination,
    /// The dial was accepted but the destination has no standable arrival,
    /// so the transfer was refused (`perform_gate_travel`).
    NoSafeArrival,
}

impl DialRefusal {
    /// The line the player reads.
    pub(crate) fn text(self) -> &'static str {
        match self {
            // 2009: "Failed to dial: not a known stargate address"
            // (`SGWPlayer.py:2063`), now also used for the nonexistent
            // address that 2009 called "invalid" (`:2052`).
            DialRefusal::UnknownAddress => "Failed to dial: not a known stargate address",
            DialRefusal::NotInWorld => "Failed to dial: you are not in a world yet, try again",
            DialRefusal::AlreadyOnDestination => "Failed to dial: you are already on that world",
            DialRefusal::NoSafeArrival => {
                "Failed to dial: the destination gate cannot be reached right now"
            }
        }
    }

    /// Stable `reason=` value for the negative log.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            DialRefusal::UnknownAddress => "unknown_stargate_address",
            DialRefusal::NotInWorld => "dial_entity_missing",
            DialRefusal::AlreadyOnDestination => "already_on_destination",
            DialRefusal::NoSafeArrival => "arrival_unrecoverable_off_mesh",
        }
    }
}

/// The client-method calls a refusal sends, in order, as
/// `(method_index, args)`. Pure so the byte-exact test pins exactly what
/// [`send_dial_refusal`] enqueues.
pub(crate) fn refusal_messages(refusal: DialRefusal) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::with_capacity(2);
    out.push((
        ON_PLAYER_COMMUNICATION,
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, refusal.text()),
    ));
    if refusal == DialRefusal::UnknownAddress {
        // onErrorCode(UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID).
        // `InstanceID` is 0, not the stargate id: under SystemID 0 the client
        // reads it as an ability id.
        let mut args = Vec::with_capacity(7);
        args.push(ERRORCODE_SYSTEM_ABILITY);
        args.extend_from_slice(&0i32.to_le_bytes());
        args.extend_from_slice(&FEEDBACK_ENTITY_DOES_NOT_HAVE_STARGATE_ADDRESS.to_le_bytes());
        out.push((ON_ERROR_CODE, args));
    }
    out
}

/// Tell the dialling player why their dial was refused.
pub(crate) async fn send_dial_refusal(
    entity_id: u32,
    target_address_id: i32,
    refusal: DialRefusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    for (method_index, args) in refusal_messages(refusal) {
        if let Err(e) = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await
        {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                target_address_id,
                target_address_name = cimmeria_names::book().stargate(target_address_id),
                refusal = refusal.reason(),
                method_index,
                method_name = cimmeria_wire::names::player_client_method(method_index),
                reason = "dial_feedback_send_failed",
                "onDialGate: refusal feedback could not be enqueued ({e}) — the dial is \
                 still refused, but the client gets no feedback and may look hung"
            );
            return;
        }
    }
}
