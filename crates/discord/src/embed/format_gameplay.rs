//! Per-variant formatter for the gameplay and GM events. Split from
//! [`super::format`], which dispatches here; same return shape, same
//! [`named`] renderer for every object.

use crate::event::{Event, Named};

use super::format::{format_vec3, named, named_list, Field, Formatted};

/// Format a gameplay or GM variant. [`super::format::format_event`] only
/// sends those here; anything else is a dispatcher bug, and renders as a
/// bare kind label rather than panicking in production.
pub(super) fn format_gameplay(event: &Event) -> Formatted {
    match event {
        Event::PlayerLevelUp {
            character,
            new_level,
            timestamp,
        } => (
            format!("⬆️ Level up: {}", named(character)),
            format!("Reached level {}", new_level),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::PlayerDeath {
            character,
            killer,
            cause,
            world,
            timestamp,
        } => (
            format!("💀 Death: {}", named(character)),
            cause.clone(),
            {
                let mut f = vec![(killer_label(cause), optional(killer.as_ref()), true)];
                push_world(&mut f, world.as_ref());
                f
            },
            timestamp.to_rfc3339(),
        ),
        Event::PlayerRespawn {
            character,
            world,
            timestamp,
        } => (
            format!("🔁 Respawn: {}", named(character)),
            format!("In {}", named(world)),
            Vec::new(),
            timestamp.to_rfc3339(),
        ),
        Event::MissionAccepted {
            character,
            mission,
            timestamp,
        } => (
            format!("📜 Mission accepted: {}", named(mission)),
            String::new(),
            vec![("Character".into(), named(character), true)],
            timestamp.to_rfc3339(),
        ),
        Event::MissionCompleted {
            character,
            mission,
            timestamp,
        } => (
            format!("✅ Mission completed: {}", named(mission)),
            String::new(),
            vec![("Character".into(), named(character), true)],
            timestamp.to_rfc3339(),
        ),
        Event::MissionFailed {
            character,
            mission,
            reason,
            timestamp,
        } => (
            format!("❌ Mission failed: {}", named(mission)),
            format!("_{}_", reason),
            vec![("Character".into(), named(character), true)],
            timestamp.to_rfc3339(),
        ),
        Event::MissionRewardGranted {
            character,
            mission,
            xp,
            cash,
            items,
            timestamp,
        } => (
            format!("🎁 Rewards: {}", named(mission)),
            String::new(),
            vec![
                ("Character".into(), named(character), true),
                ("XP".into(), xp.to_string(), true),
                ("Cash".into(), cash.to_string(), true),
                ("Items".into(), named_list(items), false),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::LootGenerated {
            character,
            source,
            items,
            timestamp,
        } => (
            format!("💰 Loot from {}", source),
            String::new(),
            vec![
                ("Character".into(), named(character), true),
                ("Items".into(), named_list(items), false),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::ItemUsed {
            character,
            item,
            target,
            timestamp,
        } => (
            format!("🧪 Item used: {}", named(item)),
            String::new(),
            {
                let mut f = vec![("Character".into(), named(character), true)];
                if let Some(t) = target {
                    f.push(("Target".into(), named(t), true));
                }
                f
            },
            timestamp.to_rfc3339(),
        ),
        Event::CharacterCreated {
            account,
            character,
            archetype,
            world,
            timestamp,
        } => (
            format!("✨ Character created: {}", named(character)),
            String::new(),
            vec![
                ("Account".into(), named(account), true),
                ("Archetype".into(), named(archetype), true),
                ("Start".into(), named(world), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::NpcDeath {
            npc,
            template,
            killer,
            cause,
            world,
            timestamp,
        } => (
            format!("☠️ NPC killed: {}", named(npc)),
            String::new(),
            {
                let mut f = Vec::new();
                if let Some(t) = template {
                    f.push(("Template".into(), named(t), true));
                }
                f.push((killer_label(cause), optional(killer.as_ref()), true));
                f.push(("Cause".into(), cause.clone(), true));
                push_world(&mut f, world.as_ref());
                f
            },
            timestamp.to_rfc3339(),
        ),
        Event::MinigameResult {
            game,
            character,
            success,
            victory_chains,
            timestamp,
        } => (
            format!(
                "🎮 Minigame {}: {}",
                if *success { "win" } else { "loss" },
                game
            ),
            String::new(),
            {
                let mut f = vec![("Character".into(), named(character), true)];
                if !victory_chains.is_empty() {
                    f.push(("For chains".into(), named_list(victory_chains), true));
                }
                f
            },
            timestamp.to_rfc3339(),
        ),
        Event::Dialog {
            character,
            dialog,
            choice,
            timestamp,
        } => (
            match choice {
                Some(b) => format!("💬 Dialog choice: {} → {}", named(dialog), named(b)),
                None => format!("💬 Dialog opened: {}", named(dialog)),
            },
            String::new(),
            vec![("Character".into(), named(character), true)],
            timestamp.to_rfc3339(),
        ),

        // ── GM ──────────────────────────────────────────────────────────
        Event::GmCommand {
            gm,
            command,
            args,
            target,
            timestamp,
        } => (
            format!("👮 GM: /{}", command),
            args.clone(),
            {
                let mut f = vec![("By".into(), named(gm), true)];
                if let Some(t) = target {
                    f.push(("Target".into(), named(t), true));
                }
                f
            },
            timestamp.to_rfc3339(),
        ),
        Event::GmTeleport {
            gm,
            target,
            world,
            position,
            timestamp,
        } => (
            format!("👮 GM teleport → {}", named(target)),
            format!("To {}", named(world)),
            vec![
                ("By".into(), named(gm), true),
                ("Position".into(), format_vec3(*position), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::GmSpawn {
            gm,
            template,
            position,
            timestamp,
        } => (
            format!("👮 GM spawn: {}", named(template)),
            String::new(),
            vec![
                ("By".into(), named(gm), true),
                ("Position".into(), format_vec3(*position), true),
            ],
            timestamp.to_rfc3339(),
        ),
        Event::GmItemGrant {
            gm,
            recipient,
            item,
            quantity,
            timestamp,
        } => (
            format!("👮 GM grant: {} × {}", quantity, named(item)),
            format!("To {}", named(recipient)),
            vec![("By".into(), named(gm), true)],
            timestamp.to_rfc3339(),
        ),

        other => (
            format!("{:?}", other.kind()),
            String::new(),
            Vec::new(),
            String::new(),
        ),
    }
}

/// An optional object, `(none)` when absent (no killer, for instance).
/// The killer field's label, from the death's `cause`. A player killer
/// pairs with its `player_id` and an NPC with its `entity_id`, so the label
/// says which ID space the `#id` is in. `pvp` / `pve` come from
/// `PlayerDeath`, `player` / `npc` from `NpcDeath`.
fn killer_label(cause: &str) -> String {
    match cause {
        "pvp" | "player" => "Killer (player)",
        "pve" | "npc" => "Killer (NPC)",
        _ => "Killer",
    }
    .to_string()
}

fn optional(n: Option<&Named>) -> String {
    n.map_or_else(|| "(none)".to_string(), named)
}

fn push_world(fields: &mut Vec<Field>, world: Option<&Named>) {
    if let Some(w) = world {
        fields.push(("World".into(), named(w), true));
    }
}
