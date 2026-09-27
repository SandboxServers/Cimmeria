//! GM broadcast (SS-C2, D-SS16): `sendGMShout(UINT8 isGlobal, WSTRING
//! Text)`, cell method 222, and the `.announce` console command, which both
//! end in [`broadcast`].
//!
//! The native `/gmshout` is wired straight to CM 222 with no Lua in between
//! (SS-E1 `chat-wire-formats.md` C-Q2). Authorization is the dispatch
//! layer's: CM 222 is in the SGWGmPlayer tail, so `gm_gate` has already
//! refused anyone below GameMaster before this runs, and `.announce` only
//! reaches the console for a GameMaster caller. Both read the server's
//! `CellEntity::access_level`, never the payload.
//!
//! Delivery is `onPlayerCommunication(<GM name>, SPEAKER_GM, CHAN_SERVER,
//! text)` ([`serialize_gm_broadcast`]):
//!
//! - **space** (`isGlobal = 0`, `.announce space ...`): every player entity
//!   in the GM's space instance, sent from the cell, which knows them;
//! - **global** (`isGlobal != 0`, `.announce ...`): every online player,
//!   through the base (`ChatCellToBase::GmBroadcast`), which knows every
//!   session whatever space it is in.
//!
//! The GM is always one of the recipients, so the GM sees the line as the
//! players do; that is the command's visible feedback.
//!
//! Text follows the D-SS12 chat rules (`TextField::ChatText`: at most 255
//! UTF-16 units, no control or format characters), rejected with feedback,
//! never truncated. Blank text is refused too: a shout with nothing in it
//! is a mistyped command.
//!
//! The SGWGmPlayer.def base-method `sendGMShout(ChannelID, BroadcastScope,
//! SpaceID, Text)` and `Communicator.hearGMShout` were the original
//! server-internal hops; this port goes straight to `onPlayerCommunication`
//! and uses neither.

use cimmeria_entity::organization::org_text::{validate, TextField, TextReject};
use cimmeria_wire::cell::chat::serialize_gm_broadcast;
use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::forward_to_base;
use crate::cell::messages::{CellToBaseMsg, ChatCellToBase};
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;
use crate::mercury::read_wstring;

/// Who a GM broadcast reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShoutScope {
    /// Every player in the GM's space instance.
    Space,
    /// Every online player.
    Global,
}

impl ShoutScope {
    /// The def's `isGlobal` is a BigWorld boolean byte: zero is the GM's
    /// space, anything else is everyone.
    fn from_wire(is_global: u8) -> Self {
        if is_global == 0 {
            ShoutScope::Space
        } else {
            ShoutScope::Global
        }
    }

    /// Stable label for the `scope` log field.
    pub(crate) fn name(self) -> &'static str {
        match self {
            ShoutScope::Space => "space",
            ShoutScope::Global => "global",
        }
    }
}

/// Speaker name when the GM's entity has no cached character name (it is
/// threaded in at `InitPlayerState`, so this is a fallback, not a path).
const FALLBACK_SPEAKER: &str = "GM";

/// `sendGMShout(UINT8 isGlobal, WSTRING Text)` (CM 222).
pub(super) async fn handle_send_gm_shout(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let decoded = args
        .first()
        .ok_or_else(|| "missing UINT8 isGlobal".to_string())
        .and_then(|&b| read_wstring(args, 1).map(|(text, _)| (b, text)));
    let (is_global, text) = match decoded {
        Ok(v) => v,
        Err(error) => {
            let id = space_mgr.player_identity(entity_id);
            tracing::warn!(
                target: "chat",
                event = "chat.gm_broadcast_rejected",
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                source = "native",
                args_len = args.len(),
                reason = "malformed_args",
                error = %error,
                "sendGMShout rejected: arguments did not decode",
            );
            send_gm_feedback(entity_id, "GM shout: the message could not be read.", tx).await;
            return true;
        }
    };
    broadcast(
        entity_id,
        ShoutScope::from_wire(is_global),
        &text,
        "native",
        tx,
        space_mgr,
    )
    .await;
    true
}

/// Validate and send one GM broadcast from `caller`. `source` is the fixed
/// label of the entry point (`"native"`, `"console"`). Returns the number
/// of cell-side recipients for a space broadcast, `None` for a refusal or a
/// global broadcast (the base counts those).
pub(crate) async fn broadcast(
    caller: u32,
    scope: ShoutScope,
    text: &str,
    source: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<usize> {
    let id = space_mgr.player_identity(caller);
    let text_units = text.encode_utf16().count();

    let refusal = if text.trim().is_empty() {
        Some(("empty_text", "GM shout: there is nothing to announce."))
    } else {
        validate(TextField::ChatText, text)
            .err()
            .map(|reject| (reject.reason(), reject_text(&reject)))
    };
    if let Some((reason, feedback)) = refusal {
        tracing::warn!(
            target: "chat",
            event = "chat.gm_broadcast_rejected",
            entity_id = caller,
            account_id = id.account_id,
            player_id = id.player_id,
            source,
            scope = scope.name(),
            text_units,
            reason,
            "GM broadcast rejected: text breaks the chat text rules",
        );
        send_gm_feedback(caller, feedback, tx).await;
        return None;
    }

    let (Some(caller_entity), Some(space_id)) = (
        space_mgr.get_entity(caller),
        space_mgr.get_entity_space_id(caller),
    ) else {
        tracing::warn!(
            target: "chat",
            event = "chat.gm_broadcast_rejected",
            entity_id = caller,
            source,
            scope = scope.name(),
            reason = "caller_not_found",
            "GM broadcast rejected: the caller's entity is gone",
        );
        return None;
    };
    let speaker = caller_entity
        .character_name
        .clone()
        .unwrap_or_else(|| FALLBACK_SPEAKER.to_string());
    let args = serialize_gm_broadcast(&speaker, text);

    // The audit row. A GM broadcast is a public announcement, not a private
    // line, so the text is logged (it is already capped at 255 units).
    tracing::info!(
        target: "chat",
        event = "chat.gm_broadcast",
        entity_id = caller,
        account_id = id.account_id,
        player_id = id.player_id,
        speaker = %speaker,
        source,
        scope = scope.name(),
        space_id,
        text_units,
        text = %text,
        "GM broadcast accepted",
    );

    match scope {
        ShoutScope::Global => {
            let forwarded = forward_to_base(
                tx,
                CellToBaseMsg::Chat(ChatCellToBase::GmBroadcast {
                    entity_id: caller,
                    player_id: id.player_id,
                    account_id: id.account_id,
                    source,
                    args,
                }),
                "sendGMShout",
            )
            .await;
            if !forwarded {
                // The GM saw nothing and nobody else did either: the audit
                // row above must not be the last word on this shout.
                tracing::warn!(
                    target: "chat",
                    event = "chat.gm_broadcast_send_failed",
                    entity_id = caller,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    source,
                    scope = "global",
                    reason = "base_channel_closed",
                    "GM broadcast not handed to the base: nobody received it",
                );
            }
            None
        }
        ShoutScope::Space => {
            let mut recipients: Vec<u32> = space_mgr
                .all_player_entity_ids()
                .into_iter()
                .filter(|&eid| space_mgr.get_entity_space_id(eid) == Some(space_id))
                .collect();
            recipients.sort_unstable();
            let mut delivered = 0usize;
            for &eid in &recipients {
                let sent = tx
                    .send(CellToBaseMsg::EntityMethodCall {
                        entity_id: eid,
                        method_index: ON_PLAYER_COMMUNICATION,
                        args: args.clone(),
                    })
                    .await;
                if sent.is_ok() {
                    delivered += 1;
                } else {
                    tracing::warn!(
                        target: "chat",
                        event = "chat.gm_broadcast_send_failed",
                        entity_id = caller,
                        account_id = id.account_id,
                        player_id = id.player_id,
                        target_player_id = space_mgr.get_entity(eid).and_then(|e| e.player_id),
                        target_entity_id = eid,
                        source,
                        scope = "space",
                        reason = "base_channel_closed",
                        "GM broadcast line not handed to the base",
                    );
                }
            }
            tracing::info!(
                target: "chat",
                event = "chat.gm_broadcast_delivered",
                entity_id = caller,
                account_id = id.account_id,
                player_id = id.player_id,
                source,
                scope = "space",
                space_id,
                delivered,
                failed = recipients.len() - delivered,
                "GM broadcast sent to every player in the space",
            );
            Some(delivered)
        }
    }
}

/// The GM's feedback line for a refused text.
fn reject_text(reject: &TextReject) -> &'static str {
    match reject {
        TextReject::TooLong { .. } => "GM shout: the message is too long (255 characters at most).",
        _ => "GM shout: the message contains a character that cannot be sent.",
    }
}
