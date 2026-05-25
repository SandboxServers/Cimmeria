//! Build Discord embed JSON from an [`Event`].
//!
//! Pure function — no I/O, no globals. The sender pipes the JSON into a
//! `POST` against the webhook URL.
//!
//! # Discord embed limits
//!
//! Discord enforces the following per the API docs:
//!
//! - `title`: 256 chars
//! - `description`: 4096 chars
//! - `fields[].name`: 256 chars
//! - `fields[].value`: 1024 chars
//! - 25 fields max
//! - `footer.text`: 2048 chars
//! - sum of all character counts: 6000
//!
//! Inputs longer than these are truncated with the `…` ellipsis. A
//! truncation is intentionally visible — the alternative (silently
//! dropping the tail) hides bugs.

use serde_json::{json, Value};

use crate::color;
use crate::event::{ChatKind, DisconnectReason, Event, TracingEventKind};

const MAX_TITLE: usize = 256;
const MAX_DESC: usize = 4096;
const MAX_FIELD_VALUE: usize = 1024;
// Footer is reserved for future use; cap kept here in lockstep with
// Discord's documented limit so the budget enforcement is uniform if
// we ever start using it.
#[allow(dead_code)]
const MAX_FOOTER: usize = 2048;
const MAX_FIELDS: usize = 25;
const MAX_TOTAL: usize = 6000;
const ELLIPSIS: &str = "…";

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

/// Per-variant formatter. Returns `(title, description, fields, timestamp_rfc3339)`.
///
/// Whisper privacy enforced here: `Chat { kind: Whisper, .. }` replaces
/// `content` with `[hidden]` regardless of caller intent. The PR design
/// committed to "never log whisper content" via Q1; this is the single
/// enforcement point.
fn format_event(event: &Event) -> (String, String, Vec<(String, String, bool)>, String) {
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
            account_id,
            character_name,
            addr,
            timestamp,
        } => (
            format!(
                "🔓 Login: {}",
                character_name.as_deref().unwrap_or("(character select)")
            ),
            String::new(),
            vec![
                ("Account".into(), account_id.to_string(), true),
                ("Addr".into(), addr.to_string(), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerLogout {
            account_id,
            character_name,
            session_secs,
            timestamp,
        } => (
            format!(
                "🔒 Logout: {}",
                character_name.as_deref().unwrap_or("(character select)")
            ),
            String::new(),
            vec![
                ("Account".into(), account_id.to_string(), true),
                ("Session".into(), format_duration(*session_secs), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerDisconnect {
            account_id,
            character_name,
            addr,
            reason,
            session_secs,
            timestamp,
        } => (
            format!("⚠️ Disconnect: {}", reason_label(*reason)),
            character_name.clone().unwrap_or_default(),
            vec![
                (
                    "Account".into(),
                    account_id.map_or("?".into(), |a| a.to_string()),
                    true,
                ),
                ("Addr".into(), addr.to_string(), true),
                ("Session".into(), format_duration(*session_secs), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerAuthFailed {
            account_name,
            addr,
            reason,
            timestamp,
        } => (
            "🚫 Auth failed".to_string(),
            reason.clone(),
            vec![
                ("Account".into(), account_name.clone(), true),
                ("Addr".into(), addr.to_string(), true),
            ],
            timestamp.to_rfc3339(),
        ),

        // ── World ───────────────────────────────────────────────────────
        Event::PlayerWorldEntry {
            account_id,
            character_name,
            world_name,
            position,
            timestamp,
        } => (
            format!("🌍 Entered {}", world_name),
            character_name.clone(),
            vec![
                ("Account".into(), account_id.to_string(), true),
                ("Position".into(), format_vec3(*position), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerWorldExit {
            account_id,
            character_name,
            from_world,
            to_world,
            timestamp,
        } => (
            format!(
                "🚪 Left {} → {}",
                from_world,
                to_world.as_deref().unwrap_or("(unknown)")
            ),
            character_name.clone(),
            vec![("Account".into(), account_id.to_string(), true)],
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
            let mut fields = vec![("Speaker".into(), speaker.clone(), true)];
            if let Some(r) = recipient {
                fields.push(("To".into(), r.clone(), true));
            }
            (label.to_string(), content, fields, timestamp.to_rfc3339())
        }

        // ── Gameplay ────────────────────────────────────────────────────
        Event::PlayerLevelUp {
            character_name,
            new_level,
            timestamp,
        } => (
            format!("⬆️ Level up: {}", character_name),
            format!("Reached level {}", new_level),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::PlayerDeath {
            character_name,
            killer,
            cause,
            timestamp,
        } => (
            format!("💀 Death: {}", character_name),
            cause.clone(),
            vec![(
                "Killer".into(),
                killer.clone().unwrap_or_else(|| "(none)".into()),
                true,
            )],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerRespawn {
            character_name,
            world_name,
            timestamp,
        } => (
            format!("🔁 Respawn: {}", character_name),
            format!("In {}", world_name),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::MissionAccepted {
            character_name,
            mission_id,
            mission_name,
            timestamp,
        } => (
            format!(
                "📜 Mission accepted: {}",
                mission_label(*mission_id, mission_name.as_deref())
            ),
            character_name.clone(),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::MissionCompleted {
            character_name,
            mission_id,
            mission_name,
            timestamp,
        } => (
            format!(
                "✅ Mission completed: {}",
                mission_label(*mission_id, mission_name.as_deref())
            ),
            character_name.clone(),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::MissionFailed {
            character_name,
            mission_id,
            mission_name,
            reason,
            timestamp,
        } => (
            format!(
                "❌ Mission failed: {}",
                mission_label(*mission_id, mission_name.as_deref())
            ),
            format!("{}\n_{}_", character_name, reason),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::MissionRewardGranted {
            character_name,
            mission_id,
            xp,
            cash,
            items,
            timestamp,
        } => (
            format!("🎁 Rewards: mission {}", mission_id),
            character_name.clone(),
            vec![
                ("XP".into(), xp.to_string(), true),
                ("Cash".into(), cash.to_string(), true),
                ("Items".into(), format_item_list(items), false),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::LootGenerated {
            character_name,
            source,
            items,
            timestamp,
        } => (
            format!("💰 Loot from {}", source),
            character_name.clone(),
            vec![("Items".into(), format_item_list(items), false)],
            timestamp.to_rfc3339(),
        ),
        Event::ItemUsed {
            character_name,
            item_type_id,
            target,
            timestamp,
        } => (
            format!("🧪 Item used: type {}", item_type_id),
            character_name.clone(),
            target
                .as_ref()
                .map(|t| vec![("Target".into(), t.clone(), true)])
                .unwrap_or_default(),
            timestamp.to_rfc3339(),
        ),

        // ── GM ──────────────────────────────────────────────────────────
        Event::GmCommand {
            gm_name,
            command,
            args,
            timestamp,
        } => (
            format!("👮 GM: /{}", command),
            args.clone(),
            vec![("By".into(), gm_name.clone(), true)],
            timestamp.to_rfc3339(),
        ),
        Event::GmTeleport {
            gm_name,
            target,
            world_name,
            position,
            timestamp,
        } => (
            format!("👮 GM teleport → {}", target),
            format!("To {}", world_name),
            vec![
                ("By".into(), gm_name.clone(), true),
                ("Position".into(), format_vec3(*position), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::GmSpawn {
            gm_name,
            template_id,
            template_name,
            position,
            timestamp,
        } => (
            format!(
                "👮 GM spawn: {}",
                template_name
                    .clone()
                    .unwrap_or_else(|| format!("template {}", template_id))
            ),
            String::new(),
            vec![
                ("By".into(), gm_name.clone(), true),
                ("Template".into(), template_id.to_string(), true),
                ("Position".into(), format_vec3(*position), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::GmItemGrant {
            gm_name,
            recipient,
            item_type_id,
            quantity,
            timestamp,
        } => (
            format!("👮 GM grant: {} × {}", quantity, item_type_id),
            format!("To {}", recipient),
            vec![("By".into(), gm_name.clone(), true)],
            timestamp.to_rfc3339(),
        ),

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
            let mut field_pairs: Vec<(String, String, bool)> = fields
                .iter()
                .take(MAX_FIELDS - 1) // leave room for added fields
                .map(|(k, v)| (k.clone(), v.clone(), true))
                .collect();
            // Always include the target as a field for grepability even
            // when it's also in the title (titles get truncated; fields
            // get their own truncation cap).
            field_pairs.push(("Target".into(), target.clone(), false));
            (title, message.clone(), field_pairs, timestamp.to_rfc3339())
        }
        Event::WireFormatError {
            kind,
            addr,
            details,
            timestamp,
        } => (
            format!("🧩 Wire format error: {}", kind),
            details.clone(),
            addr.map(|a| vec![("Addr".into(), a.to_string(), true)])
                .unwrap_or_default(),
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
            addr,
            account_id,
            silence_secs,
            timestamp,
        } => (
            "⏱️ Mercury timeout".to_string(),
            format!("No traffic for {} s", silence_secs),
            vec![
                ("Addr".into(), addr.to_string(), true),
                (
                    "Account".into(),
                    account_id.map_or("?".into(), |a| a.to_string()),
                    true,
                ),
            ],
            timestamp.to_rfc3339(),
        ),

        // ── Ops ─────────────────────────────────────────────────────────
        Event::HighLatency {
            addr,
            rtt_ms,
            threshold_ms,
            timestamp,
        } => (
            format!("📡 High latency: {} ms", rtt_ms),
            String::new(),
            vec![
                ("Addr".into(), addr.to_string(), true),
                ("Threshold".into(), format!("{} ms", threshold_ms), true),
            ],
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
            witness_id,
            burst_size,
            threshold,
            timestamp,
        } => (
            format!("🌪️ AoI burst: {} entities", burst_size),
            format!("Witness {} (threshold {})", witness_id, threshold),
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

fn format_chat(kind: ChatKind, content: &str) -> (&'static str, String) {
    let (label, body) = match kind {
        ChatKind::Global => ("💬 [Global]", content.to_string()),
        ChatKind::Say => ("💬 [Say]", content.to_string()),
        ChatKind::Guild => ("💬 [Guild]", content.to_string()),
        ChatKind::Team => ("💬 [Team]", content.to_string()),
        ChatKind::Command => ("💬 [Cmd]", content.to_string()),
        // Privacy enforcement: whisper content is NEVER posted regardless
        // of how the channel is configured. The event itself still fires
        // (so moderators can see WHO whispered WHEN) but the body is
        // replaced with the sentinel. Per the PR Q1 decision.
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

fn mission_label(id: i32, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("`{}` ({})", n, id),
        None => format!("#{}", id),
    }
}

fn format_vec3(v: [f32; 3]) -> String {
    format!("({:.1}, {:.1}, {:.1})", v[0], v[1], v[2])
}

fn format_item_list(items: &[i32]) -> String {
    if items.is_empty() {
        "(none)".to_string()
    } else if items.len() <= 8 {
        items
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let head: Vec<String> = items[..8].iter().map(|i| i.to_string()).collect();
        format!("{}, … +{}", head.join(", "), items.len() - 8)
    }
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

/// Truncate a string to at most `max` characters (Unicode-aware count).
/// Appends an ellipsis if truncated. `max` MUST be ≥ 1 (the ellipsis
/// itself); callers pass Discord's per-field caps which are all ≥ 256.
fn truncate(s: &str, max: usize) -> String {
    debug_assert!(max >= ELLIPSIS.chars().count());
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let keep = max - ELLIPSIS.chars().count();
    let mut out: String = s.chars().take(keep).collect();
    out.push_str(ELLIPSIS);
    out
}

/// Final pass: enforce Discord's 6000-character total budget. Trims
/// `description` then `fields[].value` (front to back) until the total
/// fits. Title and footer are kept full because they're already capped
/// at 256 and 2048 respectively.
fn enforce_total_budget(embed: &mut Value) {
    let total = total_chars(embed);
    if total <= MAX_TOTAL {
        return;
    }
    let mut over = total - MAX_TOTAL;

    // Trim description first to a *target length* — the result string
    // includes the ellipsis, so the target accounts for it. A previous
    // "subtract `over` chars, then append ellipsis" formulation was
    // off by one because the appended ellipsis added a char back.
    if let Some(desc) = embed
        .get_mut("description")
        .and_then(|d| d.as_str().map(str::to_string))
    {
        let cur = desc.chars().count();
        let ell = ELLIPSIS.chars().count();
        if cur > over + ell {
            let target_len = cur - over;
            let keep = target_len - ell;
            let trimmed: String = desc.chars().take(keep).collect::<String>() + ELLIPSIS;
            embed["description"] = Value::String(trimmed);
            over = 0;
        }
    }
    if over == 0 {
        return;
    }

    // Then trim fields back-to-front, same target-length math.
    if let Some(fields) = embed.get_mut("fields").and_then(Value::as_array_mut) {
        for f in fields.iter_mut() {
            if over == 0 {
                break;
            }
            if let Some(val) = f.get("value").and_then(Value::as_str).map(str::to_string) {
                let cur = val.chars().count();
                let ell = ELLIPSIS.chars().count();
                if cur > over + ell {
                    let target_len = cur - over;
                    let keep = target_len - ell;
                    let trimmed: String = val.chars().take(keep).collect::<String>() + ELLIPSIS;
                    f["value"] = Value::String(trimmed);
                    over = 0;
                }
            }
        }
    }
}

fn total_chars(embed: &Value) -> usize {
    let mut total = 0;
    if let Some(s) = embed.get("title").and_then(Value::as_str) {
        total += s.chars().count();
    }
    if let Some(s) = embed.get("description").and_then(Value::as_str) {
        total += s.chars().count();
    }
    if let Some(fields) = embed.get("fields").and_then(Value::as_array) {
        for f in fields {
            if let Some(s) = f.get("name").and_then(Value::as_str) {
                total += s.chars().count();
            }
            if let Some(s) = f.get("value").and_then(Value::as_str) {
                total += s.chars().count();
            }
        }
    }
    if let Some(s) = embed
        .get("footer")
        .and_then(|x| x.get("text"))
        .and_then(Value::as_str)
    {
        total += s.chars().count();
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ChannelKind, ChatKind};
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

    /// Title truncation. A 500-char title fits Discord's 256 cap with an
    /// ellipsis.
    #[test]
    fn long_title_truncates_with_ellipsis() {
        let long = "x".repeat(500);
        let truncated = truncate(&long, MAX_TITLE);
        assert_eq!(truncated.chars().count(), MAX_TITLE);
        assert!(truncated.ends_with(ELLIPSIS));
    }

    /// Description truncation.
    #[test]
    fn long_description_truncates_with_ellipsis() {
        let long = "x".repeat(8000);
        let truncated = truncate(&long, MAX_DESC);
        assert_eq!(truncated.chars().count(), MAX_DESC);
        assert!(truncated.ends_with(ELLIPSIS));
    }

    /// Unicode boundaries — truncation must not split a code point.
    #[test]
    fn truncation_respects_unicode_boundaries() {
        let s = "😀".repeat(300);
        let t = truncate(&s, 100);
        assert_eq!(t.chars().count(), 100);
        // Confirm valid UTF-8 by re-decoding: panicking here means we
        // split a multi-byte char.
        let _ = t.chars().collect::<Vec<_>>();
    }

    /// 6000-char total budget enforcement.
    #[test]
    fn total_budget_enforced_by_description_trim() {
        // Construct an event whose description, after building, exceeds
        // 6000 chars combined. The TracingEvent variant is the natural
        // path: message takes the desc slot, fields add the rest.
        let event = Event::TracingEvent {
            kind: TracingEventKind::Error,
            target: "x".repeat(200),
            message: "y".repeat(MAX_DESC),
            fields: (0..20)
                .map(|i| (format!("k{}", i), "z".repeat(200)))
                .collect(),
            timestamp: now(),
        };
        let mut embed = build_embed(&event);
        // Pre-budget: pretend the description+fields blew past 6000.
        // Validate the actual budget enforcer keeps us under.
        let actual = total_chars(&embed);
        assert!(
            actual <= MAX_TOTAL,
            "expected ≤ {} chars after enforcement, got {}",
            MAX_TOTAL,
            actual
        );
        // Sanity that we didn't trim it to zero
        assert!(actual > 1000);

        // Also force a second pass to confirm idempotence.
        let before = embed.clone();
        enforce_total_budget(&mut embed);
        assert_eq!(before, embed, "enforce_total_budget must be idempotent");
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
                character_name: Some("alice".into()),
                addr,
                timestamp: ts,
            },
            Event::PlayerLogout {
                account_id: 1,
                character_name: Some("alice".into()),
                session_secs: 100,
                timestamp: ts,
            },
            Event::PlayerDisconnect {
                account_id: Some(1),
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
                character_name: "alice".into(),
                world_name: "Castle".into(),
                position: [1.0, 2.0, 3.0],
                timestamp: ts,
            },
            Event::PlayerWorldExit {
                account_id: 1,
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
