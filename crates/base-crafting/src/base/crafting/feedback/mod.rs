//! The crafting rejection path: every refused request gets a visible text
//! line, and an `onErrorCode` where a condition code fits.
//!
//! The text is the legacy `feedback()` line: `onPlayerCommunication` from
//! speaker `SYSTEM` on `CHAN_FEEDBACK`, built by the one shared serializer in
//! `cimmeria_wire::cell::chat`, which is the path that reaches the player's
//! chat window. Whether the client shows anything for `onErrorCode` is not
//! known, so the code is only ever a secondary signal, sent after the text.
//!
//! [`reject`] also emits the `rejected` event (with the values the rule
//! compared) and counts the refusal on both crafting counters.

mod reason;

pub use reason::{Compared, CraftReject};

use crate::base::crafting::sync::CraftClient;
use crate::base::crafting::telemetry::{
    account_id_of, record_rejection, record_request, witness_send_failure, JobIds, Outcome,
};
use crate::base::helpers::send_to_witness_reliable;
use crate::mercury::{build_player_entity_method_packet, method_idx};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

/// `EErrorCodeSystem::ERRORCODE_SYSTEM_Ability`, the only system the enum
/// defines.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// The seven `onErrorCode` argument bytes: `UINT8 SystemID = 0,
/// INT32 InstanceID = 0, UINT16 ErrorCodeID`, little-endian. A crafting
/// rejection names no ability, so `InstanceID` is 0.
pub fn error_code_args(code: u16) -> Vec<u8> {
    let mut args = Vec::with_capacity(7);
    args.push(ERRORCODE_SYSTEM_ABILITY);
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&code.to_le_bytes());
    args
}

/// The `onPlayerCommunication` argument bytes of a crafting feedback line:
/// speaker `SYSTEM`, flags 0, channel `CHAN_FEEDBACK`, then `text`.
pub fn feedback_text_args(text: &str) -> Vec<u8> {
    serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text)
}

/// Refuse a crafting request for `verb` (the cell method name): log
/// `rejected`, count it, send the `CHAN_FEEDBACK` text line to the player's
/// own client and, where [`CraftReject::error_code`] maps one, `onErrorCode`
/// after it. A send that does not go out is a WARN (`event =
/// "feedback_send_failed"`).
pub async fn reject(
    verb: &'static str,
    entity_id: u32,
    player_id: i32,
    why: &CraftReject,
    client: CraftClient<'_>,
) {
    let account_id = account_id_of(entity_id, client.connected, client.entity_to_addr);
    refuse(verb, entity_id, player_id, account_id, why, client, true).await;
}

/// [`reject`] for an induction whose work is refused when it completes.
/// Its request was answered (and counted) when the job was queued, so this
/// counts the rejection but not a second request. The identity is the
/// queued job's, never re-read from the live session map: by now the
/// client may have left, or its entity id may belong to someone else.
pub async fn reject_at_completion(ids: &JobIds, why: &CraftReject, client: CraftClient<'_>) {
    refuse(
        ids.verb,
        ids.entity_id,
        ids.player_id,
        Some(ids.account_id),
        why,
        client,
        false,
    )
    .await;
}

async fn refuse(
    verb: &'static str,
    entity_id: u32,
    player_id: i32,
    account_id: Option<u32>,
    why: &CraftReject,
    client: CraftClient<'_>,
    answers_request: bool,
) {
    let reason = why.reason();
    let c = why.compared();
    tracing::info!(
        target: "crafting",
        event = "rejected",
        verb,
        account_id,
        player_id,
        entity_id,
        reason,
        discipline_id = c.discipline_id,
        asp = c.asp,
        paradigm_id = c.paradigm_id,
        paradigm_level = c.paradigm_level,
        required_level = c.required_level,
        prerequisite_id = c.prerequisite_id,
        prerequisite_expertise = c.prerequisite_expertise,
        required_expertise = c.required_expertise,
        station_mask = c.station_mask,
        item_id = c.item_id,
        design_id = c.design_id,
        type_id = c.type_id,
        container_id = c.container_id,
        needed = c.needed,
        available = c.available,
        queue_limit = c.queue_limit,
        applied_science_id = c.applied_science_id,
        tools = why.tools_considered(),
        blueprint_ids = why.blueprints_considered(),
        blueprint_id = c.blueprint_id,
        quantity = c.quantity,
        type_ids = why.types_submitted(),
        tier = c.tier,
        required_tier = c.required_tier,
        elementary_counts = why.elementary_counts(),
        tech_comp = c.tech_comp,
        item_disciplines = why.item_disciplines(),
        known_disciplines = why.known_disciplines(),
        "crafting request rejected"
    );
    record_rejection(verb, reason);
    if answers_request {
        record_request(verb, Outcome::Rejected);
    }

    let text_args = feedback_text_args(&why.text());
    send_line(
        verb,
        entity_id,
        player_id,
        account_id,
        method_idx::ON_PLAYER_COMMUNICATION,
        &text_args,
        client,
    )
    .await;
    if let Some(code) = why.error_code() {
        send_line(
            verb,
            entity_id,
            player_id,
            account_id,
            method_idx::ON_ERROR_CODE,
            &error_code_args(code),
            client,
        )
        .await;
    }
}

async fn send_line(
    verb: &'static str,
    entity_id: u32,
    player_id: i32,
    account_id: Option<u32>,
    method_index: u16,
    args: &[u8],
    client: CraftClient<'_>,
) {
    let outcome = send_to_witness_reliable(
        client.transport,
        client.connected,
        client.entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_index,
                args,
                version,
            )
        },
    )
    .await;
    if let Some(reason) = witness_send_failure(&outcome) {
        tracing::warn!(
            target: "crafting",
            event = "feedback_send_failed",
            verb,
            account_id,
            player_id,
            entity_id,
            method_index,
            reason,
            "crafting refusal line not sent -- the player sees nothing for this press"
        );
    }
}

#[cfg(test)]
mod tests;
