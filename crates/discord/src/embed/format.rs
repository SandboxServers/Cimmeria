//! Per-variant formatter: turn an [`Event`] into the embed's title,
//! description, fields, and timestamp strings.

use crate::event::{ChatKind, DisconnectReason, Event, TracingEventKind};

use super::MAX_FIELDS;

/// Per-variant formatter. Returns `(title, description, fields, timestamp_rfc3339)`.
///
/// **Privacy invariant.** `Chat { kind: Whisper, .. }` replaces `content`
/// with `[hidden]` regardless of caller intent — whisper text must never
/// leave the server. This formatter is the single enforcement point for
/// that invariant; the test `whisper_content_is_hidden_regardless_of_input`
/// pins it.
pub(super) fn format_event(event: &Event) -> (String, String, Vec<(String, String, bool)>, String) {
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
            account_name,
            character_name,
            addr: _,
            timestamp,
        } => (
            format!(
                "🔓 Login: {}",
                character_name.as_deref().unwrap_or("(character select)")
            ),
            String::new(),
            vec![(
                "Account".into(),
                account_value(Some(*account_id), account_name.as_deref()),
                true,
            )],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerLogout {
            account_id,
            account_name,
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
                (
                    "Account".into(),
                    account_value(Some(*account_id), account_name.as_deref()),
                    true,
                ),
                ("Session".into(), format_duration(*session_secs), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerDisconnect {
            account_id,
            account_name,
            character_name,
            addr: _,
            reason,
            session_secs,
            timestamp,
        } => (
            format!("⚠️ Disconnect: {}", reason_label(*reason)),
            character_name.clone().unwrap_or_default(),
            vec![
                (
                    "Account".into(),
                    account_value(*account_id, account_name.as_deref()),
                    true,
                ),
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
            account_id,
            account_name,
            character_name,
            world_name,
            position,
            timestamp,
        } => (
            format!("🌍 Entered {}", world_name),
            String::new(),
            vec![
                ("Character".into(), character_name.clone(), true),
                (
                    "Account".into(),
                    account_value(Some(*account_id), account_name.as_deref()),
                    true,
                ),
                ("Position".into(), format_vec3(*position), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::PlayerWorldExit {
            account_id,
            account_name,
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
            String::new(),
            vec![
                ("Character".into(), character_name.clone(), true),
                (
                    "Account".into(),
                    account_value(Some(*account_id), account_name.as_deref()),
                    true,
                ),
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
            String::new(),
            vec![("Character".into(), character_name.clone(), true)],
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
            String::new(),
            vec![("Character".into(), character_name.clone(), true)],
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
            format!("_{}_", reason),
            vec![("Character".into(), character_name.clone(), true)],
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
            String::new(),
            vec![
                ("Character".into(), character_name.clone(), true),
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
            String::new(),
            vec![
                ("Character".into(), character_name.clone(), true),
                ("Items".into(), format_item_list(items), false),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::ItemUsed {
            character_name,
            item_type_id,
            target,
            timestamp,
        } => (
            format!("🧪 Item used: type {}", item_type_id),
            String::new(),
            {
                let mut f = vec![("Character".into(), character_name.clone(), true)];
                if let Some(t) = target {
                    f.push(("Target".into(), t.clone(), true));
                }
                f
            },
            timestamp.to_rfc3339(),
        ),
        Event::CharacterCreated {
            account_id,
            account_name,
            character_name,
            archetype,
            world_name,
            timestamp,
        } => (
            format!("✨ Character created: {}", character_name),
            String::new(),
            vec![
                (
                    "Account".into(),
                    account_value(Some(*account_id), account_name.as_deref()),
                    true,
                ),
                ("Archetype".into(), archetype.to_string(), true),
                ("Start".into(), world_name.clone(), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::NpcDeath {
            npc_name,
            killer,
            cause,
            world_name,
            timestamp,
        } => (
            format!("☠️ NPC killed: {}", npc_name),
            String::new(),
            {
                let mut f = vec![
                    (
                        "Killer".into(),
                        killer.clone().unwrap_or_else(|| "(none)".into()),
                        true,
                    ),
                    ("Cause".into(), cause.clone(), true),
                ];
                if let Some(w) = world_name {
                    f.push(("World".into(), w.clone(), true));
                }
                f
            },
            timestamp.to_rfc3339(),
        ),
        Event::MinigameResult {
            game,
            character_name,
            success,
            timestamp,
        } => (
            format!(
                "🎮 Minigame {}: {}",
                if *success { "win" } else { "loss" },
                game
            ),
            String::new(),
            vec![("Character".into(), character_name.clone(), true)],
            timestamp.to_rfc3339(),
        ),
        Event::Dialog {
            character_name,
            dialog_id,
            choice,
            timestamp,
        } => (
            match choice {
                Some(b) => format!("💬 Dialog choice: #{} → option {}", dialog_id, b),
                None => format!("💬 Dialog opened: #{}", dialog_id),
            },
            String::new(),
            vec![("Character".into(), character_name.clone(), true)],
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
            account_id,
            silence_secs,
            timestamp,
        } => (
            "⏱️ Mercury timeout".to_string(),
            format!("No traffic for {} s", silence_secs),
            vec![(
                "Account".into(),
                account_id.map_or("?".into(), |a| a.to_string()),
                true,
            )],
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

fn mission_label(id: i32, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("`{}` ({})", n, id),
        None => format!("#{}", id),
    }
}

/// Render the "Account" field value, preferring the human-readable name and
/// falling back to the numeric id. Player IPs are deliberately never shown.
fn account_value(id: Option<u32>, name: Option<&str>) -> String {
    match (name, id) {
        (Some(n), Some(i)) => format!("{n} (#{i})"),
        (Some(n), None) => n.to_string(),
        (None, Some(i)) => format!("#{i}"),
        (None, None) => "?".to_string(),
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
