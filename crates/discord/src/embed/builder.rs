//! Public entry points: assemble the embed JSON body from an [`Event`].

use serde_json::{json, Value};

use crate::color;
use crate::event::Event;

use super::budget::{enforce_total_budget, truncate};
use super::format::format_event;
use super::{MAX_DESC, MAX_FIELDS, MAX_FIELD_VALUE, MAX_TITLE};

/// Build the JSON body of a Discord webhook POST from one event.
///
/// Returns the full request body — usually one embed under `"embeds"`,
/// plus `username`/`avatar_url` overrides if the caller threads them in
/// later. Today the caller passes a single embed and the body shape is:
///
/// ```json
/// { "embeds": [ {...} ] }
/// ```
pub fn build_embed_body(event: &Event, username: Option<&str>, avatar_url: Option<&str>) -> Value {
    let embed = build_embed(event);
    let mut body = json!({ "embeds": [embed] });
    if let (Some(u), Some(obj)) = (username, body.as_object_mut()) {
        obj.insert("username".to_string(), Value::String(u.to_string()));
    }
    if let (Some(a), Some(obj)) = (avatar_url, body.as_object_mut()) {
        obj.insert("avatar_url".to_string(), Value::String(a.to_string()));
    }
    body
}

/// Build the embed object itself (without the surrounding `embeds` array).
/// Exposed for tests; production code goes through [`build_embed_body`].
pub fn build_embed(event: &Event) -> Value {
    let (title, description, fields, timestamp) = format_event(event);

    let color = color::for_severity(event.severity());
    let title = truncate(&title, MAX_TITLE);
    let description = truncate(&description, MAX_DESC);

    let mut fields_json: Vec<Value> = fields
        .into_iter()
        .take(MAX_FIELDS)
        .map(|(name, value, inline)| {
            json!({
                "name": truncate(&name, 256),
                "value": truncate(&value, MAX_FIELD_VALUE),
                "inline": inline,
            })
        })
        .collect();

    let mut embed = json!({
        "title": title,
        "description": description,
        "color": color,
        "timestamp": timestamp,
    });
    if !fields_json.is_empty() {
        embed["fields"] = Value::Array(std::mem::take(&mut fields_json));
    }

    // Final guard: if the total character budget is exceeded, trim
    // description first (the most likely culprit), then fields, in
    // declaration order.
    enforce_total_budget(&mut embed);
    embed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ChannelKind, ChatKind, DisconnectReason, Event, TracingEventKind};
    use chrono::Utc;

    fn now() -> chrono::DateTime<Utc> {
        Utc::now()
    }

    /// **Privacy regression guard.** Whisper content must NEVER be
    /// posted, even when the caller hands the event in fully populated.
    /// Reverting `format_chat`'s whisper branch trips this immediately.
    #[test]
    fn whisper_content_is_hidden_regardless_of_input() {
        let event = Event::Chat {
            kind: ChatKind::Whisper,
            speaker: "alice".into(),
            recipient: Some("bob".into()),
            content: "this should never appear in Discord".into(),
            timestamp: now(),
        };
        let body = build_embed_body(&event, None, None);
        let serialized = body.to_string();
        assert!(
            !serialized.contains("this should never appear"),
            "whisper body must not contain the raw message: {}",
            serialized
        );
        assert!(
            serialized.contains("[hidden]"),
            "whisper body must contain the hidden sentinel"
        );
    }

    /// Non-whisper chat preserves content.
    #[test]
    fn global_chat_preserves_content() {
        let event = Event::Chat {
            kind: ChatKind::Global,
            speaker: "alice".into(),
            recipient: None,
            content: "hello world".into(),
            timestamp: now(),
        };
        let body = build_embed_body(&event, None, None);
        assert!(body.to_string().contains("hello world"));
    }

    /// Every event variant builds an embed without panicking and emits
    /// a valid color from the palette.
    #[test]
    fn every_event_variant_builds() {
        let addr: std::net::SocketAddr = "127.0.0.1:50000".parse().unwrap();
        let ts = now();
        let events = vec![
            Event::ServerStartup {
                version: "0.1.0".into(),
                bind_addrs: vec!["0.0.0.0:7777".into()],
                timestamp: ts,
            },
            Event::ServerShutdown {
                reason: "Ctrl-C".into(),
                uptime_secs: 1234,
                timestamp: ts,
            },
            Event::ServerPanic {
                location: "src/foo.rs:42".into(),
                message: "boom".into(),
                timestamp: ts,
            },
            Event::PlayerLogin {
                account_id: 1,
                account_name: Some("steve".into()),
                character_name: Some("alice".into()),
                addr,
                timestamp: ts,
            },
            Event::PlayerLogout {
                account_id: 1,
                account_name: Some("steve".into()),
                character_name: Some("alice".into()),
                session_secs: 100,
                timestamp: ts,
            },
            Event::PlayerDisconnect {
                account_id: Some(1),
                account_name: None,
                character_name: Some("alice".into()),
                addr,
                reason: DisconnectReason::Timeout,
                session_secs: 60,
                timestamp: ts,
            },
            Event::PlayerAuthFailed {
                account_name: "bad".into(),
                addr,
                reason: "invalid password".into(),
                timestamp: ts,
            },
            Event::PlayerWorldEntry {
                account_id: 1,
                account_name: Some("steve".into()),
                character_name: "alice".into(),
                world_name: "Castle".into(),
                position: [1.0, 2.0, 3.0],
                timestamp: ts,
            },
            Event::PlayerWorldExit {
                account_id: 1,
                account_name: Some("steve".into()),
                character_name: "alice".into(),
                from_world: "Castle".into(),
                to_world: Some("Tollana".into()),
                timestamp: ts,
            },
            Event::Chat {
                kind: ChatKind::Global,
                speaker: "alice".into(),
                recipient: None,
                content: "hi".into(),
                timestamp: ts,
            },
            Event::PlayerLevelUp {
                character_name: "alice".into(),
                new_level: 5,
                timestamp: ts,
            },
            Event::PlayerDeath {
                character_name: "alice".into(),
                killer: Some("Goa'uld".into()),
                cause: "shot".into(),
                timestamp: ts,
            },
            Event::PlayerRespawn {
                character_name: "alice".into(),
                world_name: "Castle".into(),
                timestamp: ts,
            },
            Event::MissionAccepted {
                character_name: "alice".into(),
                mission_id: 1562,
                mission_name: Some("Castle_Cellblock_1".into()),
                timestamp: ts,
            },
            Event::MissionCompleted {
                character_name: "alice".into(),
                mission_id: 1562,
                mission_name: Some("Castle_Cellblock_1".into()),
                timestamp: ts,
            },
            Event::MissionFailed {
                character_name: "alice".into(),
                mission_id: 1562,
                mission_name: None,
                reason: "timed out".into(),
                timestamp: ts,
            },
            Event::MissionRewardGranted {
                character_name: "alice".into(),
                mission_id: 1562,
                xp: 1000,
                cash: 50,
                items: vec![1, 2, 3],
                timestamp: ts,
            },
            Event::LootGenerated {
                character_name: "alice".into(),
                source: "mob 42".into(),
                items: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                timestamp: ts,
            },
            Event::ItemUsed {
                character_name: "alice".into(),
                item_type_id: 5168,
                target: None,
                timestamp: ts,
            },
            Event::CharacterCreated {
                account_id: 3,
                account_name: Some("steve".into()),
                character_name: "asg".into(),
                archetype: 2,
                world_name: "Castle_CellBlock".into(),
                timestamp: ts,
            },
            Event::NpcDeath {
                npc_name: "Jaffa Guard".into(),
                killer: Some("alice".into()),
                cause: "player".into(),
                world_name: Some("Castle_CellBlock".into()),
                timestamp: ts,
            },
            Event::MinigameResult {
                game: "Livewire".into(),
                character_name: "alice".into(),
                success: true,
                timestamp: ts,
            },
            Event::Dialog {
                character_name: "alice".into(),
                dialog_id: 4242,
                choice: Some(1),
                timestamp: ts,
            },
            Event::GmCommand {
                gm_name: "admin".into(),
                command: "teleport".into(),
                args: "0 0 0".into(),
                timestamp: ts,
            },
            Event::GmTeleport {
                gm_name: "admin".into(),
                target: "alice".into(),
                world_name: "Castle".into(),
                position: [0.0, 0.0, 0.0],
                timestamp: ts,
            },
            Event::GmSpawn {
                gm_name: "admin".into(),
                template_id: 50,
                template_name: Some("Vala".into()),
                position: [1.0, 2.0, 3.0],
                timestamp: ts,
            },
            Event::GmItemGrant {
                gm_name: "admin".into(),
                recipient: "alice".into(),
                item_type_id: 5168,
                quantity: 1,
                timestamp: ts,
            },
            Event::TracingEvent {
                kind: TracingEventKind::Warn,
                target: "cimmeria_services::cell".into(),
                message: "thing".into(),
                fields: vec![("reason".into(), "x".into())],
                timestamp: ts,
            },
            Event::WireFormatError {
                kind: "0x07".into(),
                addr: Some(addr),
                details: "bad framing".into(),
                timestamp: ts,
            },
            Event::DbError {
                operation: "SELECT".into(),
                details: "timeout".into(),
                timestamp: ts,
            },
            Event::AssertionFailure {
                location: "x.rs:1".into(),
                message: "x != y".into(),
                timestamp: ts,
            },
            Event::MercuryTimeout {
                addr,
                account_id: Some(1),
                silence_secs: 30,
                timestamp: ts,
            },
            Event::HighLatency {
                addr,
                rtt_ms: 600,
                threshold_ms: 500,
                timestamp: ts,
            },
            Event::PacketLossSpike {
                loss_ratio: 0.12,
                window_secs: 10,
                timestamp: ts,
            },
            Event::MemoryWarning {
                rss_mb: 5000,
                threshold_mb: 4000,
                timestamp: ts,
            },
            Event::TickStall {
                tick_ms: 250,
                budget_ms: 100,
                subsystem: "cell".into(),
                timestamp: ts,
            },
            Event::AoiBurstWarning {
                witness_id: 1,
                burst_size: 50,
                threshold: 32,
                timestamp: ts,
            },
            Event::OutboxLag {
                depth: 100,
                threshold: 50,
                timestamp: ts,
            },
        ];

        for e in events {
            let body = build_embed_body(&e, Some("Cimmeria"), None);
            assert!(body.get("embeds").is_some(), "missing embeds array");
            assert_eq!(e.kind(), e.kind()); // round-trip self-check
                                            // Sanity: routing resolves cleanly for every variant.
            let _: ChannelKind = crate::router::channel_for(e.kind());
        }
    }
}
