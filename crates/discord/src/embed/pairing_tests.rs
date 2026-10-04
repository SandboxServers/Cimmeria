//! NT-10: every typed [`Event`] renders each object it names as a pair.
//!
//! [`every_variant`] builds one event of every variant, each object a
//! [`Named`] with a sentinel name and a sentinel ID, alongside the
//! rendered `Name (#id)` strings the embed must contain. Asserting the
//! whole `Name (#id)` string, not the name and the ID apart, is what
//! makes the guard bite: a variant that drops the ID, or renders the
//! name alone, no longer contains it.

use std::net::SocketAddr;

use chrono::Utc;

use crate::event::{ChatKind, DisconnectReason, Event, Named, TracingEventKind};

use super::build_embed_body;

/// A documentation-range address (RFC 5737). The privacy rule says it
/// never reaches an embed.
const SENTINEL_IP: &str = "198.51.100.77";

/// A sentinel object, and the string the embed must render it as.
fn obj(id: i64, name: &str) -> (Named, String) {
    (
        Named::new(id, Some(name.to_string())),
        format!("{name} (#{id})"),
    )
}

/// One event of every variant, each with the rendered pairs it must show.
pub(super) fn every_variant() -> Vec<(Event, Vec<String>)> {
    let addr: SocketAddr = format!("{SENTINEL_IP}:50000").parse().unwrap();
    let ts = Utc::now();
    let (account, account_s) = obj(910_001, "SentinelLogin");
    let (character, character_s) = obj(910_002, "Sentinel Hero");
    let (world, world_s) = obj(910_003, "Sentinel_World");
    let (world2, world2_s) = obj(910_004, "Sentinel_World_Two");
    let (mission, mission_s) = obj(910_005, "Sentinel Mission");
    let (item, item_s) = obj(910_006, "Sentinel Medkit");
    let (item2, item2_s) = obj(910_007, "Sentinel Ammo");
    let (npc, npc_s) = obj(910_008, "Sentinel Jaffa");
    let (template, template_s) = obj(910_009, "Sentinel Template");
    let (killer, killer_s) = obj(910_010, "Sentinel Killer");
    let (archetype, archetype_s) = obj(910_011, "Sentinel Archetype");
    let (dialog, dialog_s) = obj(910_012, "Sentinel Dialog");
    let (button, button_s) = obj(910_013, "Sentinel Accept");
    let (target, target_s) = obj(910_014, "Sentinel Target");
    let (chain, chain_s) = obj(910_015, "Sentinel Chain");
    let (witness, witness_s) = obj(910_016, "Sentinel Witness");
    let (gm, gm_s) = obj(910_017, "Sentinel GM");

    let mut cases = vec![
        // ── Lifecycle: no object fields ──────────────────────────────
        (
            Event::ServerStartup {
                version: "0.1.0".into(),
                bind_addrs: vec!["0.0.0.0:7777".into()],
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::ServerShutdown {
                reason: "Ctrl-C".into(),
                uptime_secs: 1234,
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::ServerPanic {
                location: "src/foo.rs:42".into(),
                message: "boom".into(),
                timestamp: ts,
            },
            vec![],
        ),
        // ── Auth ─────────────────────────────────────────────────────
        (
            Event::PlayerLogin {
                account: account.clone(),
                character: Some(character.clone()),
                addr,
                timestamp: ts,
            },
            vec![account_s.clone(), character_s.clone()],
        ),
        (
            Event::PlayerLogout {
                account: account.clone(),
                character: Some(character.clone()),
                session_secs: 100,
                timestamp: ts,
            },
            vec![account_s.clone(), character_s.clone()],
        ),
        (
            Event::PlayerDisconnect {
                account: account.clone(),
                character: Some(character.clone()),
                addr,
                reason: DisconnectReason::Timeout,
                session_secs: 60,
                timestamp: ts,
            },
            vec![account_s.clone(), character_s.clone()],
        ),
        // A rejected login has no account ID to pair (see the variant).
        (
            Event::PlayerAuthFailed {
                account_name: "SentinelLogin".into(),
                addr,
                reason: "invalid password".into(),
                timestamp: ts,
            },
            vec!["SentinelLogin".into()],
        ),
        // ── World ────────────────────────────────────────────────────
        (
            Event::PlayerWorldEntry {
                account: account.clone(),
                character: character.clone(),
                world: world.clone(),
                position: [1.0, 2.0, 3.0],
                timestamp: ts,
            },
            vec![account_s.clone(), character_s.clone(), world_s.clone()],
        ),
        (
            Event::PlayerWorldExit {
                account: account.clone(),
                character: character.clone(),
                from_world: world.clone(),
                to_world: Some(world2.clone()),
                timestamp: ts,
            },
            vec![
                account_s.clone(),
                character_s.clone(),
                world_s.clone(),
                world2_s.clone(),
            ],
        ),
        // ── Gameplay ─────────────────────────────────────────────────
        (
            Event::PlayerLevelUp {
                character: character.clone(),
                new_level: 5,
                timestamp: ts,
            },
            vec![character_s.clone()],
        ),
        (
            Event::PlayerDeath {
                character: character.clone(),
                killer: Some(killer.clone()),
                cause: "pve".into(),
                world: Some(world.clone()),
                timestamp: ts,
            },
            vec![character_s.clone(), killer_s.clone(), world_s.clone()],
        ),
        (
            Event::PlayerRespawn {
                character: character.clone(),
                world: world.clone(),
                timestamp: ts,
            },
            vec![character_s.clone(), world_s.clone()],
        ),
        (
            Event::MissionAccepted {
                character: character.clone(),
                mission: mission.clone(),
                timestamp: ts,
            },
            vec![character_s.clone(), mission_s.clone()],
        ),
        (
            Event::MissionCompleted {
                character: character.clone(),
                mission: mission.clone(),
                timestamp: ts,
            },
            vec![character_s.clone(), mission_s.clone()],
        ),
        (
            Event::MissionFailed {
                character: character.clone(),
                mission: mission.clone(),
                reason: "timed out".into(),
                timestamp: ts,
            },
            vec![character_s.clone(), mission_s.clone()],
        ),
        (
            Event::MissionRewardGranted {
                character: character.clone(),
                mission: mission.clone(),
                xp: 1000,
                cash: 50,
                items: vec![item.clone(), item2.clone()],
                timestamp: ts,
            },
            vec![
                character_s.clone(),
                mission_s.clone(),
                item_s.clone(),
                item2_s.clone(),
            ],
        ),
        (
            Event::LootGenerated {
                character: character.clone(),
                source: "corpse".into(),
                items: vec![item.clone(), item2.clone()],
                timestamp: ts,
            },
            vec![character_s.clone(), item_s.clone(), item2_s.clone()],
        ),
        (
            Event::ItemUsed {
                character: character.clone(),
                item: item.clone(),
                target: Some(target.clone()),
                timestamp: ts,
            },
            vec![character_s.clone(), item_s.clone(), target_s.clone()],
        ),
        (
            Event::CharacterCreated {
                account: account.clone(),
                character: character.clone(),
                archetype: archetype.clone(),
                world: world.clone(),
                timestamp: ts,
            },
            vec![
                account_s.clone(),
                character_s.clone(),
                archetype_s.clone(),
                world_s.clone(),
            ],
        ),
        (
            Event::NpcDeath {
                npc: npc.clone(),
                template: Some(template.clone()),
                killer: Some(killer.clone()),
                cause: "player".into(),
                world: Some(world.clone()),
                timestamp: ts,
            },
            vec![
                npc_s.clone(),
                template_s.clone(),
                killer_s.clone(),
                world_s.clone(),
            ],
        ),
        (
            Event::MinigameResult {
                game: "Livewire".into(),
                character: character.clone(),
                success: true,
                victory_chains: vec![chain.clone()],
                timestamp: ts,
            },
            vec![character_s.clone(), chain_s.clone(), "Livewire".into()],
        ),
        (
            Event::Dialog {
                character: character.clone(),
                dialog: dialog.clone(),
                choice: None,
                timestamp: ts,
            },
            vec![character_s.clone(), dialog_s.clone()],
        ),
        (
            Event::Dialog {
                character: character.clone(),
                dialog: dialog.clone(),
                choice: Some(button.clone()),
                timestamp: ts,
            },
            vec![character_s.clone(), dialog_s.clone(), button_s.clone()],
        ),
        // ── GM ───────────────────────────────────────────────────────
        (
            Event::GmCommand {
                gm: gm.clone(),
                command: ".missionfail".into(),
                args: "1562".into(),
                target: Some(target.clone()),
                timestamp: ts,
            },
            vec![gm_s.clone(), target_s.clone()],
        ),
        (
            Event::GmTeleport {
                gm: gm.clone(),
                target: target.clone(),
                world: world.clone(),
                position: [0.0; 3],
                timestamp: ts,
            },
            vec![gm_s.clone(), target_s.clone(), world_s.clone()],
        ),
        (
            Event::GmSpawn {
                gm: gm.clone(),
                template: template.clone(),
                position: [1.0, 2.0, 3.0],
                timestamp: ts,
            },
            vec![gm_s.clone(), template_s.clone()],
        ),
        (
            Event::GmItemGrant {
                gm: gm.clone(),
                recipient: character.clone(),
                item: item.clone(),
                quantity: 1,
                timestamp: ts,
            },
            vec![gm_s.clone(), character_s.clone(), item_s.clone()],
        ),
        // ── Errors ───────────────────────────────────────────────────
        // The harvested path pairs `<p>_id` / `<p>_name` fields; NT-11
        // owns its fold, this row pins that it still renders the pair.
        (
            Event::TracingEvent {
                kind: TracingEventKind::Warn,
                target: "cimmeria_cell::spawner".into(),
                message: "thing".into(),
                fields: vec![
                    ("mission_id".into(), "910005".into()),
                    ("mission_name".into(), "Sentinel Mission".into()),
                ],
                timestamp: ts,
            },
            vec![mission_s.clone()],
        ),
        (
            Event::TracingEvent {
                kind: TracingEventKind::Error,
                target: "cimmeria_cell::spawner".into(),
                message: "thing".into(),
                fields: vec![
                    ("item_type_id".into(), "910006".into()),
                    ("item_name".into(), "Sentinel Medkit".into()),
                ],
                timestamp: ts,
            },
            vec![item_s.clone()],
        ),
        (
            Event::WireFormatError {
                kind: "0x07".into(),
                addr: Some(addr),
                details: "bad framing".into(),
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::DbError {
                operation: "SELECT".into(),
                details: "timeout".into(),
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::AssertionFailure {
                location: "x.rs:1".into(),
                message: "x != y".into(),
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::MercuryTimeout {
                addr,
                account: account.clone(),
                character: Some(character.clone()),
                silence_secs: 30,
                timestamp: ts,
            },
            vec![account_s.clone(), character_s.clone()],
        ),
        // ── Ops ──────────────────────────────────────────────────────
        (
            Event::HighLatency {
                addr,
                rtt_ms: 600,
                threshold_ms: 500,
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::PacketLossSpike {
                loss_ratio: 0.12,
                window_secs: 10,
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::MemoryWarning {
                rss_mb: 5000,
                threshold_mb: 4000,
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::TickStall {
                tick_ms: 250,
                budget_ms: 100,
                subsystem: "cell".into(),
                timestamp: ts,
            },
            vec![],
        ),
        (
            Event::AoiBurstWarning {
                witness,
                burst_size: 50,
                threshold: 32,
                timestamp: ts,
            },
            vec![witness_s],
        ),
        (
            Event::OutboxLag {
                depth: 100,
                threshold: 50,
                timestamp: ts,
            },
            vec![],
        ),
    ];

    // One chat event per channel kind; every kind pairs the speaker and
    // the recipient the same way.
    for kind in [
        ChatKind::Global,
        ChatKind::Say,
        ChatKind::Whisper,
        ChatKind::Guild,
        ChatKind::Team,
        ChatKind::Command,
    ] {
        cases.push((
            Event::Chat {
                kind,
                speaker: character.clone(),
                recipient: Some(target.clone()),
                content: "hi".into(),
                timestamp: ts,
            },
            vec![character_s.clone(), target_s.clone()],
        ));
    }
    cases
}

/// Compile-time completeness: a new `Event` variant fails to compile
/// here until it is added to [`every_variant`] (and to this match).
fn variant_is_covered(event: &Event) {
    match event {
        Event::ServerStartup { .. }
        | Event::ServerShutdown { .. }
        | Event::ServerPanic { .. }
        | Event::PlayerLogin { .. }
        | Event::PlayerLogout { .. }
        | Event::PlayerDisconnect { .. }
        | Event::PlayerAuthFailed { .. }
        | Event::PlayerWorldEntry { .. }
        | Event::PlayerWorldExit { .. }
        | Event::Chat { .. }
        | Event::PlayerLevelUp { .. }
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
        | Event::GmItemGrant { .. }
        | Event::TracingEvent { .. }
        | Event::WireFormatError { .. }
        | Event::DbError { .. }
        | Event::AssertionFailure { .. }
        | Event::MercuryTimeout { .. }
        | Event::HighLatency { .. }
        | Event::PacketLossSpike { .. }
        | Event::MemoryWarning { .. }
        | Event::TickStall { .. }
        | Event::AoiBurstWarning { .. }
        | Event::OutboxLag { .. } => {}
    }
}

/// **NT-10 regression guard.** Every object every variant names renders
/// as `Name (#id)`. Reverting any variant's pairing (rendering the name
/// alone, dropping the ID, or leaving the object out) fails it for that
/// variant.
#[test]
fn every_typed_event_renders_each_object_as_name_and_id() {
    let cases = every_variant();
    let mut kinds = std::collections::HashSet::new();
    let mut failures = Vec::new();
    for (event, expected) in &cases {
        variant_is_covered(event);
        kinds.insert(event.kind());
        let body = build_embed_body(event, None, None).to_string();
        for want in expected {
            if !body.contains(want.as_str()) {
                failures.push(format!("{:?}: missing `{want}` in {body}", event.kind()));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "unpaired objects:\n{}",
        failures.join("\n")
    );
    assert_eq!(
        kinds.len(),
        crate::EventKind::ALL.len(),
        "every EventKind needs a row in every_variant()"
    );
}

/// **Privacy regression guard.** No variant renders the connection
/// address, whichever field carries it.
#[test]
fn no_event_renders_the_player_ip() {
    for (event, _) in every_variant() {
        let body = build_embed_body(&event, None, None).to_string();
        assert!(
            !body.contains(SENTINEL_IP),
            "{:?} leaked the player IP: {body}",
            event.kind()
        );
    }
}

/// Each object degrades by the one renderer's rules: `#id` without a
/// name, `?` with neither.
#[test]
fn missing_names_render_as_id_then_question_mark() {
    let event = Event::MissionAccepted {
        character: Named::default(),
        mission: Named::new(1562, None),
        timestamp: Utc::now(),
    };
    let body = build_embed_body(&event, None, None).to_string();
    assert!(body.contains("Mission accepted: #1562"), "{body}");
    assert!(body.contains("\"value\":\"?\""), "{body}");
}
