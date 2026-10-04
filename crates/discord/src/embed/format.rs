//! Per-variant formatter: turn an [`Event`] into the embed's title,
//! description, fields, and timestamp strings.
//!
//! The gameplay and GM variants live in [`super::format_gameplay`]; this
//! file holds the dispatcher, the lifecycle / auth / world / chat /
//! error / ops variants, and the shared helpers.

use crate::event::{ChatKind, DisconnectReason, Event, Named, TracingEventKind};

use super::format_gameplay::format_gameplay;
use super::naming::name_with_id;
use super::tracing_fields::fold_fields;
use super::MAX_FIELDS;

/// One embed field: `(name, value, inline)`.
pub(super) type Field = (String, String, bool);

/// What a formatter returns: `(title, description, fields, timestamp_rfc3339)`.
pub(super) type Formatted = (String, String, Vec<Field>, String);

/// Per-variant formatter. Returns `(title, description, fields, timestamp_rfc3339)`.
///
/// **Privacy invariant.** `Chat { kind: Whisper, .. }` replaces `content`
/// with `[hidden]` regardless of caller intent — whisper text must never
/// leave the server. This formatter is the single enforcement point for
/// that invariant; the test `whisper_content_is_hidden_regardless_of_input`
/// pins it.
///
/// **Naming.** Every [`Named`] object renders through [`named`]:
/// `Name (#id)`, `#id`, the bare name, or `?` (Rule 6, "Discord").
pub(super) fn format_event(event: &Event) -> Formatted {
    match event {
        // ── Lifecycle ───────────────────────────────────────────────────
        Event::ServerStartup {
            version,
            bind_addrs,
            timestamp,
        } => (
            "Server up".to_string(),
            format!("`v{}`", version),
            vec![("Bind".into(), bind_addrs.join("\n"), false)],
            timestamp.to_rfc3339(),
        ),
        Event::ServerShutdown {
            reason,
            uptime_secs,
            timestamp,
        } => (
            "Server shutdown".to_string(),
            reason.clone(),
            vec![("Uptime".into(), format_duration(*uptime_secs), true)],
            timestamp.to_rfc3339(),
        ),
        Event::ServerPanic {
            location,
            message,
            timestamp,
        } => (
            "💥 Server panic".to_string(),
            format!("```\n{}\n```", message),
            vec![("At".into(), location.clone(), false)],
            timestamp.to_rfc3339(),
        ),

        // ── Auth ────────────────────────────────────────────────────────
        Event::PlayerLogin {
            account,
            character,
            addr: _,
            timestamp,
        } => (
            format!("🔓 Login: {}", character_or_select(character.as_ref())),
            String::new(),
            vec![("Account".into(), named(account), true)],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerLogout {
            account,
            character,
            session_secs,
            timestamp,
        } => (
            format!("🔒 Logout: {}", character_or_select(character.as_ref())),
            String::new(),
            vec![
                ("Account".into(), named(account), true),
                ("Session".into(), format_duration(*session_secs), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerDisconnect {
            account,
            character,
            addr: _,
            reason,
            session_secs,
            timestamp,
        } => (
            format!("⚠️ Disconnect: {}", reason_label(*reason)),
            character.as_ref().map(named).unwrap_or_default(),
            vec![
                ("Account".into(), named(account), true),
                ("Session".into(), format_duration(*session_secs), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerAuthFailed {
            account_name,
            addr: _,
            reason,
            timestamp,
        } => (
            "🚫 Auth failed".to_string(),
            reason.clone(),
            vec![("Account".into(), account_name.clone(), true)],
            timestamp.to_rfc3339(),
        ),

        // ── World ───────────────────────────────────────────────────────
        Event::PlayerWorldEntry {
            account,
            character,
            world,
            position,
            timestamp,
        } => (
            format!("🌍 Entered {}", named(world)),
            String::new(),
            vec![
                ("Character".into(), named(character), true),
                ("Account".into(), named(account), true),
                ("Position".into(), format_vec3(*position), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerWorldExit {
            account,
            character,
            from_world,
            to_world,
            timestamp,
        } => (
            format!(
                "🚪 Left {} → {}",
                named(from_world),
                to_world
                    .as_ref()
                    .map_or_else(|| "(unknown)".to_string(), named)
            ),
            String::new(),
            vec![
                ("Character".into(), named(character), true),
                ("Account".into(), named(account), true),
            ],
            timestamp.to_rfc3339(),
        ),

        // ── Chat ────────────────────────────────────────────────────────
        Event::Chat {
            kind,
            speaker,
            recipient,
            content,
            timestamp,
        } => {
            let (label, content) = format_chat(*kind, content);
            let mut fields = vec![("Speaker".into(), named(speaker), true)];
            if let Some(r) = recipient {
                fields.push(("To".into(), named(r), true));
            }
            (label.to_string(), content, fields, timestamp.to_rfc3339())
        }

        // ── Gameplay + GM ───────────────────────────────────────────────
        Event::PlayerLevelUp { .. }
        | Event::PlayerDeath { .. }
        | Event::PlayerRespawn { .. }
        | Event::MissionAccepted { .. }
        | Event::MissionCompleted { .. }
        | Event::MissionFailed { .. }
        | Event::MissionRewardGranted { .. }
        | Event::LootGenerated { .. }
        | Event::ItemUsed { .. }
        | Event::CharacterCreated { .. }
        | Event::NpcDeath { .. }
        | Event::MinigameResult { .. }
        | Event::Dialog { .. }
        | Event::GmCommand { .. }
        | Event::GmTeleport { .. }
        | Event::GmSpawn { .. }
        | Event::GmItemGrant { .. } => format_gameplay(event),

        // ── Errors ──────────────────────────────────────────────────────
        Event::TracingEvent {
            kind,
            target,
            message,
            fields,
            timestamp,
        } => {
            let title = match kind {
                TracingEventKind::Warn => "⚠️ warn",
                TracingEventKind::Error => "🛑 error",
            };
            let title = format!("{} — {}", title, target);
            // Fold Rule 6 ID/name pairs into one field each, leaving
            // one slot for the log target below.
            let mut field_pairs = fold_fields(fields, MAX_FIELDS - 1);
            // Always include the log target as a field for grepability
            // even when it's also in the title (titles get truncated;
            // fields get their own truncation cap). Not "Target": that
            // label belongs to the folded `target` entity pair.
            field_pairs.push(("Log target".into(), target.clone(), false));
            (title, message.clone(), field_pairs, timestamp.to_rfc3339())
        }
        Event::WireFormatError {
            kind,
            addr: _,
            details,
            timestamp,
        } => (
            format!("🧩 Wire format error: {}", kind),
            details.clone(),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::DbError {
            operation,
            details,
            timestamp,
        } => (
            format!("🗄️ DB error: {}", operation),
            details.clone(),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::AssertionFailure {
            location,
            message,
            timestamp,
        } => (
            "🚨 Assertion failure".to_string(),
            message.clone(),
            vec![("At".into(), location.clone(), false)],
            timestamp.to_rfc3339(),
        ),
        Event::MercuryTimeout {
            addr: _,
            account,
            character,
            silence_secs,
            timestamp,
        } => (
            "⏱️ Mercury timeout".to_string(),
            format!("No traffic for {} s", silence_secs),
            {
                let mut f = vec![("Account".into(), named(account), true)];
                if let Some(c) = character {
                    f.push(("Character".into(), named(c), true));
                }
                f
            },
            timestamp.to_rfc3339(),
        ),

        // ── Ops ─────────────────────────────────────────────────────────
        Event::HighLatency {
            addr: _,
            rtt_ms,
            threshold_ms,
            timestamp,
        } => (
            format!("📡 High latency: {} ms", rtt_ms),
            String::new(),
            vec![("Threshold".into(), format!("{} ms", threshold_ms), true)],
            timestamp.to_rfc3339(),
        ),
        Event::PacketLossSpike {
            loss_ratio,
            window_secs,
            timestamp,
        } => (
            format!("📉 Packet loss spike: {:.1}%", loss_ratio * 100.0),
            format!("Over {} s window", window_secs),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::MemoryWarning {
            rss_mb,
            threshold_mb,
            timestamp,
        } => (
            format!("💾 Memory warning: {} MB", rss_mb),
            format!("Threshold {} MB", threshold_mb),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::TickStall {
            tick_ms,
            budget_ms,
            subsystem,
            timestamp,
        } => (
            format!("⏳ Tick stall: {} ms", tick_ms),
            format!("Subsystem `{}` over {} ms budget", subsystem, budget_ms),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::AoiBurstWarning {
            witness,
            burst_size,
            threshold,
            timestamp,
        } => (
            format!("🌪️ AoI burst: {} entities", burst_size),
            format!("Witness {} (threshold {})", named(witness), threshold),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::OutboxLag {
            depth,
            threshold,
            timestamp,
        } => (
            format!("📤 Outbox lag: depth {}", depth),
            format!("Threshold {}", threshold),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// The one renderer for a typed event's [`Named`] object: `Name (#id)`,
/// `#id` when the name is missing, the bare name when the ID is, and `?`
/// when both are. It is [`name_with_id`], the renderer the tracing path
/// folds pairs with, so a pair reads the same in every embed. The account
/// renders its login name this way too (D-NT2).
pub(super) fn named(n: &Named) -> String {
    let id = n.id.map(|i| i.to_string());
    name_with_id(n.name.as_deref(), id.as_deref()).unwrap_or_else(|| "?".to_string())
}

/// A list of objects, eight at most, then a `+N` tail.
pub(super) fn named_list(items: &[Named]) -> String {
    const SHOWN: usize = 8;
    if items.is_empty() {
        return "(none)".to_string();
    }
    let head: Vec<String> = items.iter().take(SHOWN).map(named).collect();
    if items.len() <= SHOWN {
        head.join(", ")
    } else {
        format!("{}, … +{}", head.join(", "), items.len() - SHOWN)
    }
}

/// The login/logout title's character: the pair, or the character-select
/// placeholder before a character is picked.
fn character_or_select(character: Option<&Named>) -> String {
    character.map_or_else(|| "(character select)".to_string(), named)
}

fn format_chat(kind: ChatKind, content: &str) -> (&'static str, String) {
    let (label, body) = match kind {
        ChatKind::Global => ("💬 [Global]", content.to_string()),
        ChatKind::Say => ("💬 [Say]", content.to_string()),
        ChatKind::Guild => ("💬 [Guild]", content.to_string()),
        ChatKind::Team => ("💬 [Team]", content.to_string()),
        ChatKind::Command => ("💬 [Cmd]", content.to_string()),
        // Privacy invariant: whisper content is NEVER posted regardless
        // of how the channel is configured. The event itself still fires
        // (so moderators can see WHO whispered WHEN) but the body is
        // replaced with the sentinel. Whisper text never leaves the
        // server — see `format_event` doc comment and the regression
        // test in this file.
        ChatKind::Whisper => ("💬 [Whisper]", "`[hidden]`".to_string()),
    };
    (label, body)
}

fn reason_label(reason: DisconnectReason) -> &'static str {
    match reason {
        DisconnectReason::Clean => "client closed",
        DisconnectReason::Timeout => "timeout",
        DisconnectReason::PeerReset => "peer reset",
        DisconnectReason::ServerInitiated => "server-initiated",
    }
}

pub(super) fn format_vec3(v: [f32; 3]) -> String {
    format!("({:.1}, {:.1}, {:.1})", v[0], v[1], v[2])
}

fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, s)
    } else if mins > 0 {
        format!("{}m {}s", mins, s)
    } else {
        format!("{}s", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The typed renderer degrades the way Rule 6 says, down to `?`.
    #[test]
    fn named_degrades_to_id_then_name_then_question_mark() {
        assert_eq!(named(&Named::new(6, Some("steve".into()))), "steve (#6)");
        assert_eq!(named(&Named::new(6, None)), "#6");
        assert_eq!(named(&Named::new(6, Some(String::new()))), "#6");
        assert_eq!(named(&Named::name_only("steve")), "steve");
        assert_eq!(named(&Named::default()), "?");
    }

    #[test]
    fn named_list_caps_at_eight() {
        let items: Vec<Named> = (1..=10).map(|i| Named::new(i, None)).collect();
        assert_eq!(named_list(&items), "#1, #2, #3, #4, #5, #6, #7, #8, … +2");
        assert_eq!(named_list(&[]), "(none)");
    }
}
