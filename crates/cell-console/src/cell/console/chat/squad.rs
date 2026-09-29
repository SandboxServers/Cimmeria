//! Squad chat (`CHAN_SQUAD`, ORG-04): `onPlayerCommunication` to every
//! member of the speaker's squad, in whatever space they are.
//!
//! The base has already applied the chat flood limit (`RateCategory::Chat`)
//! and the text rules (`org_text::validate(TextField::ChatText)`) before
//! forwarding the line, so neither is repeated here; the base logs those
//! refusals as `squad.chat` rows itself.
//!
//! Recipients are resolved to their live entity per line
//! (`SpaceManager::player_entity_by_player_id`), never cached: entity ids
//! are recycled and gate travel re-creates the entity. A member in gate
//! transit has no entity and misses the line. The speaker is sent the line
//! too, as the legacy `ChatChannel.sendMessage` did for every member.
//!
//! Telemetry is on the `squad` target: one INFO outcome row `squad.chat`
//! per line with `recipients` (members reached, the speaker's own copy not
//! counted) and `text_units` (never the text), counted on
//! `squad_actions_total{action = "chat"}`; WARN `squad.send_failed` for a
//! line that could not be queued.

use cimmeria_cell_world::cell::squad::SquadResources;
use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::count_action;

use super::feedback::send_channel_feedback;
use super::{serialize_on_player_communication, CHAN_SQUAD, ON_PLAYER_COMMUNICATION};

/// The line a speaker in no squad reads.
pub(super) const NOT_IN_SQUAD_TEXT: &str = "You are not in a squad.";

/// The one `squad.chat` outcome row.
struct ChatRow {
    entity_id: u32,
    who: PlayerIdentity,
    squad_id: Option<i32>,
    recipients: usize,
    text_units: usize,
}

impl ChatRow {
    fn emit(self, reason: Option<&'static str>) {
        let outcome = if reason.is_some() { "rejected" } else { "ok" };
        tracing::info!(
            target: "squad",
            event = "squad.chat",
            outcome,
            reason,
            account_id = self.who.account_id,
            player_id = self.who.player_id,
            entity_id = self.entity_id,
            squad_id = self.squad_id,
            recipients = self.recipients,
            text_units = self.text_units,
            "squad chat {}",
            outcome
        );
        count_action("chat", outcome, reason.unwrap_or("none"));
    }
}

/// Relay `text` from the player at `entity_id` to their squad.
#[tracing::instrument(
    name = "squad.chat",
    level = "info",
    target = "squad",
    skip_all,
    fields(entity_id = entity_id)
)]
pub(super) async fn relay_to_squad(
    entity_id: u32,
    speaker_name: &str,
    speaker_flags: u8,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let mut row = ChatRow {
        entity_id,
        who,
        squad_id: None,
        recipients: 0,
        text_units: text.encode_utf16().count(),
    };
    let squad = who
        .player_id
        .and_then(|pid| space_mgr.resources.squads().squad_for(pid));
    let Some(squad) = squad else {
        row.emit(Some("not_in_squad"));
        send_channel_feedback(entity_id, NOT_IN_SQUAD_TEXT, tx).await;
        return;
    };
    row.squad_id = Some(squad.id());

    let args = serialize_on_player_communication(speaker_name, speaker_flags, CHAN_SQUAD, text);
    for member in squad.members() {
        let Some(eid) = space_mgr.player_entity_by_player_id(member.player_id) else {
            continue;
        };
        let msg = CellToBaseMsg::EntityMethodCall {
            entity_id: eid,
            method_index: ON_PLAYER_COMMUNICATION,
            args: args.clone(),
        };
        if tx.send(msg).await.is_err() {
            tracing::warn!(
                target: "squad",
                event = "squad.send_failed",
                entity_id = eid,
                method_index = ON_PLAYER_COMMUNICATION,
                squad_id = squad.id(),
                reason = "cell_to_base_closed",
                "squad chat line could not be queued"
            );
            continue;
        }
        if Some(member.player_id) != who.player_id {
            row.recipients += 1;
        }
    }
    row.emit(None);
}
