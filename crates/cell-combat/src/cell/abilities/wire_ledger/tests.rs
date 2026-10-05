//! `abilities.wire` guards (AB-T4): one per send family asserting the row
//! and its key fields at the real sending site, and the Pattern-A WARN for
//! each send that used to be a `let _`. The `onEffectResults` and hit
//! `onStatUpdate` guards live with the hit (`damage_apply/wire_rows_tests.rs`).

use std::time::Instant;

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_entity::abilities::{serialize_timer_update, AbilityDef, TIMER_ABILITY_COOLDOWN};
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::ACCURACY;

use super::super::use_ability::{play_ability_sequence, AbilityPhase, PhaseSequence};
use crate::cell::abilities::{
    request_appearance_refresh, send_auto_cycle_state, send_entity_method,
    send_entity_method_to_witnesses, send_timer_update,
};
use crate::cell::effects::stat_buff::StatBuffRemoval;
use crate::cell::effects::{flush_stat_buff_timers, stat_buffs::strip_timed_effects};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::EVENT_ABILITY_END;
use crate::mercury::method_idx;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const PLAYER: u32 = 1;
const OTHER_PLAYER: u32 = 2;
const NPC: u32 = 3;
const BSF_MOVEMENT_LOCK: u32 = 1 << 6;

/// Players 1 and 2 (player ids 101 / 102, accounts 11 / 12) and NPC 3 in
/// one space with AoI computed: each player witnesses the other and the NPC.
fn scene() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for id in [PLAYER, OTHER_PLAYER, NPC] {
        mgr.create_entity(id, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    }
    for id in [PLAYER, OTHER_PLAYER] {
        let p = mgr.get_entity_mut(id).unwrap();
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
        p.account_id = Some(10 + id);
        // A player with an account is introduced only once initialised.
        p.archetype_id = Some(1);
    }
    for id in [PLAYER, OTHER_PLAYER] {
        mgr.connect_entity(id);
    }
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_witnesses_of(PLAYER).contains(&OTHER_PLAYER),
        "fixture: the players must witness each other"
    );
    mgr
}

/// A sender whose receiver is gone: every send fails.
fn closed_channel() -> mpsc::Sender<CellToBaseMsg> {
    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    tx
}

fn wire_rows(logs: &LogCaptureGuard, method: &str) -> Vec<Captured> {
    logs.all()
        .into_iter()
        .filter(|c| {
            c.target == "abilities.wire"
                && c.has_field("event", "wire_sent")
                && c.has_field("method", method)
        })
        .collect()
}

fn send_failures(logs: &LogCaptureGuard) -> Vec<Captured> {
    logs.all()
        .into_iter()
        .filter(|c| {
            c.level == Level::WARN
                && c.target == "abilities.wire"
                && c.has_field("event", "wire_send_failed")
                && c.has_field("reason", "cell_to_base_closed")
        })
        .collect()
}

fn field<'a>(c: &'a Captured, key: &str) -> &'a str {
    c.fields
        .get(key)
        .map_or_else(|| panic!("row has no `{key}`: {c:?}"), String::as_str)
}

// ── onTimerUpdate ─────────────────────────────────────────────────────────

/// A cooldown start and its clear each write a row: the timer kind, its id
/// as the ability, `start` / `clear`, the recipient's ids. Fails if the row
/// in `send_timer_update_ctx` is removed (cooldowns logged nothing before).
#[tokio::test]
async fn a_cooldown_start_and_clear_each_write_a_timer_row() {
    let mgr = scene();
    let (tx, _rx) = mpsc::channel(8);
    let logs = LogCapture::install();

    let start = serialize_timer_update(559, TIMER_ABILITY_COOLDOWN, 1, 0, 1.5, 100.0);
    send_timer_update(PLAYER, start, &tx, &mgr).await;
    let clear = serialize_timer_update(559, TIMER_ABILITY_COOLDOWN, 1, 0, 0.0, 0.0);
    send_timer_update(PLAYER, clear, &tx, &mgr).await;

    let rows = wire_rows(&logs, "onTimerUpdate");
    assert_eq!(rows.len(), 2, "{:#?}", logs.all());
    for (row, action) in rows.iter().zip(["start", "clear"]) {
        assert_eq!(field(row, "timer_type"), "cooldown");
        assert_eq!(field(row, "action"), action);
        assert_eq!(field(row, "ability_id"), "559");
        assert_eq!(field(row, "timer_id"), "559");
        assert_eq!(field(row, "player_id"), "101");
        assert_eq!(field(row, "account_id"), "11");
        assert_eq!(field(row, "stage"), "wire");
        // The queue took it; the base logs delivery or the drop itself.
        assert_eq!(field(row, "delivery"), "queued_to_base");
    }
    assert_eq!(field(&rows[0], "complete_at"), "100.0");
}

/// A timed effect's icon: the start row names the entry's cast and effect,
/// the strip's clear row says `clear`, and the state flag the entry holds
/// rides the same flush with its bit and refcount. Fails if `stat_buffs`
/// stops passing its context (no `origin`, no `cast_id`) or the
/// `onStateFieldUpdate` row loses its transition fields.
#[tokio::test]
async fn a_timed_effect_logs_its_timer_start_clear_and_state_flag() {
    let mut mgr = scene();
    let (tx, _rx) = mpsc::channel(32);
    let now = Instant::now();
    // The fire's cast scope stamps the entry (AB-T1).
    let outer = mgr.enter_cast_scope(Some(77));
    mgr.apply_timed_effect(
        PLAYER,
        TimedEffectSpec {
            cast_id: None,
            effect_id: 700,
            ability_id: 637,
            invoker_id: PLAYER,
            effect_flags: 0,
            moniker_ids: vec![],
            stats: vec![(ACCURACY, 200)],
            absorb: Vec::new(),
            duration_secs: Some(15.0),
            stacking: TimedStacking::PerSource,
            state_flags: BSF_MOVEMENT_LOCK,
            invoker_identity: Default::default(),
            invoker_name: None,
        },
        now,
    );
    mgr.exit_cast_scope(outer);
    let logs = LogCapture::install();

    flush_stat_buff_timers(PLAYER, now, &tx, &mut mgr).await;
    strip_timed_effects(PLAYER, StatBuffRemoval::Death, |_| true, &tx, &mut mgr).await;

    let timers = wire_rows(&logs, "onTimerUpdate");
    assert_eq!(timers.len(), 2, "{:#?}", logs.all());
    let (start, clear) = (&timers[0], &timers[1]);
    assert_eq!(field(start, "origin"), "stat_buffs");
    assert_eq!(field(start, "timer_type"), "duration");
    assert_eq!(field(start, "action"), "start");
    assert_eq!(field(start, "cast_id"), "77");
    assert_eq!(field(start, "effect_id"), "700");
    assert_eq!(field(start, "ability_id"), "637");
    assert_eq!(field(clear, "action"), "clear");
    assert_eq!(field(clear, "effect_id"), "700");

    let states = wire_rows(&logs, "onStateFieldUpdate");
    assert_eq!(states.len(), 2, "lock on, lock off: {:#?}", logs.all());
    let (on, off) = (&states[0], &states[1]);
    assert_eq!(field(on, "bits_set"), "64");
    assert_eq!(field(on, "bits_cleared"), "0");
    assert_eq!(field(on, "refcounts"), "6:1");
    assert_eq!(field(on, "reason"), "timed_effect");
    assert_eq!(field(off, "bits_cleared"), "64");
    assert_eq!(field(off, "refcounts"), "");
}

// ── onStateFieldUpdate ────────────────────────────────────────────────────

/// The auto-cycle exit writes its row with the new field and reason.
#[tokio::test]
async fn an_auto_cycle_state_change_writes_a_state_field_row() {
    let mgr = scene();
    let (tx, _rx) = mpsc::channel(8);
    let logs = LogCapture::install();

    send_auto_cycle_state(PLAYER, 0b10, &tx, &mgr).await;

    let rows = wire_rows(&logs, "onStateFieldUpdate");
    assert_eq!(rows.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&rows[0], "state_field"), "2");
    assert_eq!(field(&rows[0], "reason"), "auto_cycle");
    assert_eq!(field(&rows[0], "self_sent"), "true");
    assert_eq!(field(&rows[0], "player_id"), "101");
}

// ── onErrorCode ───────────────────────────────────────────────────────────

/// A press of an ability the player does not know is answered with
/// `onErrorCode`, and the row names the code, the ability and the reason.
#[tokio::test]
async fn a_not_known_refusal_writes_an_error_code_row() {
    let mut mgr = scene();
    let ability = AbilityDef {
        ability_id: 4242,
        name: "unknown to the player".into(),
        cooldown: 0.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0.0,
        max_range: 30.0,
        target_type_id: 0,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    };
    mgr.ability_defs.insert(4242, ability);
    let (tx, _rx) = mpsc::channel(32);
    let logs = LogCapture::install();

    crate::cell::abilities::handle_use_ability(PLAYER, 4242, NPC as i32, &tx, &mut mgr).await;

    let rows = wire_rows(&logs, "onErrorCode");
    assert_eq!(rows.len(), 1, "{:#?}", logs.all());
    let row = &rows[0];
    assert_eq!(field(row, "ability_id"), "4242");
    assert_eq!(field(row, "system_id"), "0");
    // CONDITION_FEEDBACK_EntityDoesNotHaveAbility (`not_known.rs`).
    assert_eq!(field(row, "error_code"), "167");
    assert_eq!(field(row, "reason"), "ability_not_known");
    assert_eq!(field(row, "origin"), "not_known");
    assert_eq!(field(row, "player_id"), "101");
}

// ── onSequence ────────────────────────────────────────────────────────────

/// An `Ability_End` sequence writes one row for the whole fan-out: the
/// phase as `reason`, the sequence id, the cast (its `InstanceId`) and the
/// witness count, never one row per witness.
#[tokio::test]
async fn an_ability_sequence_writes_one_row_for_its_fan_out() {
    let mut mgr = scene();
    mgr.sequence_map.insert((1025, EVENT_ABILITY_END), 9001);
    let (tx, _rx) = mpsc::channel(32);
    let logs = LogCapture::install();

    play_ability_sequence(
        PhaseSequence {
            phase: AbilityPhase::End,
            entity_id: PLAYER,
            ability_id: 579,
            target_id: NPC as i32,
            instance_id: 41,
            event_set_id: Some(1025),
        },
        &tx,
        &mut mgr,
    )
    .await;

    let rows = wire_rows(&logs, "onSequence");
    assert_eq!(rows.len(), 1, "{:#?}", logs.all());
    let row = &rows[0];
    assert_eq!(field(row, "reason"), "ability_end");
    assert_eq!(field(row, "sequence_id"), "9001");
    assert_eq!(field(row, "instance_id"), "41");
    assert_eq!(field(row, "cast_id"), "41");
    assert_eq!(field(row, "ability_id"), "579");
    assert_eq!(field(row, "source_id"), "1");
    assert_eq!(field(row, "target_id"), "3");
    assert_eq!(field(row, "self_sent"), "true");
    assert_eq!(field(row, "witness_count"), "1");
    assert_eq!(field(row, "witness_player_ids"), "102");
}

// ── onStatUpdate fan-out and the empty send ───────────────────────────────

/// An NPC's stat update reaches both players: one row, `witness_count = 2`.
#[tokio::test]
async fn an_npc_stat_fan_out_is_one_row_with_its_witness_count() {
    let mgr = scene();
    let (tx, _rx) = mpsc::channel(8);
    let logs = LogCapture::install();

    let mut stats = 1u32.to_le_bytes().to_vec();
    for v in [1i32, 0, 90, 100] {
        stats.extend_from_slice(&v.to_le_bytes());
    }
    super::send(
        NPC,
        method_idx::ON_STAT_UPDATE,
        stats,
        super::WireRoute::EntityDefault,
        super::WireCtx::new("test"),
        &tx,
        &mgr,
    )
    .await;

    let rows = wire_rows(&logs, "onStatUpdate");
    assert_eq!(rows.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&rows[0], "witness_count"), "2");
    assert_eq!(field(&rows[0], "witness_player_ids"), "101,102");
    assert_eq!(field(&rows[0], "stats"), "1:90/100");
    assert!(
        !rows[0].fields.contains_key("player_id"),
        "an NPC has no ids"
    );
}

/// A send that reached nobody writes no row: the WARN is the record.
#[tokio::test]
async fn a_failed_send_writes_no_wire_row() {
    let mgr = scene();
    let logs = LogCapture::install();

    send_auto_cycle_state(PLAYER, 0, &closed_channel(), &mgr).await;

    assert!(wire_rows(&logs, "onStateFieldUpdate").is_empty());
    assert_eq!(send_failures(&logs).len(), 1);
}

// ── Death ─────────────────────────────────────────────────────────────────

/// The death burst's three non-ability-family sends write rows too (Copilot
/// on #1175): the killer's cleared target, the corpse's interaction flags
/// and the dead player's aid-wait window. Fails if any goes back to the
/// plain `send_entity_method`.
#[tokio::test]
async fn a_death_logs_target_clear_interaction_and_aid_wait_rows() {
    let mut mgr = scene();
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    super::super::death::resolve_death(NPC, PLAYER, None, true, false, &tx, &mut mgr).await;
    super::super::death::resolve_death(OTHER_PLAYER, NPC, None, false, false, &tx, &mut mgr).await;

    let target = wire_rows(&logs, "onTargetUpdate");
    assert_eq!(target.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&target[0], "entity_id"), "1");
    assert_eq!(field(&target[0], "reason"), "target_cleared");
    assert_eq!(field(&target[0], "origin"), "death");

    let interaction = wire_rows(&logs, "InteractionType");
    assert_eq!(interaction.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&interaction[0], "entity_id"), "3");
    assert_eq!(field(&interaction[0], "witness_count"), "2");

    let aid = wire_rows(&logs, "onBeginAidWait");
    assert_eq!(aid.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&aid[0], "entity_id"), "2");
    assert_eq!(field(&aid[0], "player_id"), "102");
}

// ── Pattern A: a closed channel is a WARN ─────────────────────────────────

/// `messaging.rs` used to `let _` the owner's send.
#[tokio::test]
async fn a_failed_owner_send_is_a_warn_naming_method_and_player() {
    let mgr = scene();
    let logs = LogCapture::install();

    send_entity_method(
        PLAYER,
        method_idx::ON_STAT_UPDATE,
        vec![0; 4],
        &closed_channel(),
        &mgr,
    )
    .await;

    let warns = send_failures(&logs);
    assert_eq!(warns.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&warns[0], "method"), "onStatUpdate");
    assert_eq!(field(&warns[0], "recipient_id"), "1");
    assert_eq!(field(&warns[0], "player_id"), "101");
}

/// The witness loop used to `let _` each send; now one WARN per fan-out
/// carries the failed count.
#[tokio::test]
async fn a_failed_witness_fan_out_is_one_warn_with_the_count() {
    let mgr = scene();
    let logs = LogCapture::install();

    let addressed = send_entity_method_to_witnesses(
        NPC,
        method_idx::ON_STATE_FIELD_UPDATE,
        vec![0; 4],
        &closed_channel(),
        &mgr,
    )
    .await;

    assert_eq!(addressed, 2, "the return still counts who was addressed");
    let warns = send_failures(&logs);
    assert_eq!(warns.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&warns[0], "failed_count"), "2");
    assert_eq!(field(&warns[0], "method"), "onStateFieldUpdate");
}

/// `request_appearance_refresh` used to `let _` its `RefreshAppearance`.
#[tokio::test]
async fn a_failed_appearance_refresh_is_a_warn() {
    let mgr = scene();
    let logs = LogCapture::install();

    request_appearance_refresh(PLAYER, &closed_channel(), &mgr).await;

    let warns = send_failures(&logs);
    assert_eq!(warns.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&warns[0], "method"), "RefreshAppearance");
    assert_eq!(field(&warns[0], "player_id"), "101");
}

/// `resolve_death` used to `let _` the contact-list death event.
#[tokio::test]
async fn a_failed_death_presence_event_is_a_warn() {
    let mut mgr = scene();
    mgr.get_entity_mut(OTHER_PLAYER).unwrap().character_name = Some("Victim".into());
    let logs = LogCapture::install();

    crate::cell::abilities::resolve_death_for_test(OTHER_PLAYER, NPC, &closed_channel(), &mut mgr)
        .await;

    let presence: Vec<_> = send_failures(&logs)
        .into_iter()
        .filter(|c| c.has_field("method", "ContactListPresenceEvent"))
        .collect();
    assert_eq!(presence.len(), 1, "{:#?}", logs.all());
    assert_eq!(field(&presence[0], "player_id"), "102");
}
