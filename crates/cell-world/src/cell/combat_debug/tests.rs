//! AB-N1 unit guards: the formatter, the split, the routing, the budget,
//! the row, and the wire bytes of one debug line.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::commands::{
    clear_ability_debug, set_ability_debug_target, toggle, toggle_ability, toggle_mob, Toggle,
};
use super::deliver::{flush, prepare, send_feedback_line, Outgoing, LINES_PER_WINDOW, WINDOW};
use super::format::{format_record, split_line, MAX_LINE_UNITS};
use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const GM: u32 = 1;
const OTHER: u32 = 2;
const MOB: u32 = 3;
const FAR: u32 = 4;
const ABILITY: i32 = 592;
const HEAL: i32 = 12;

fn mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for (id, pid, name) in [
        (GM, Some(100), "Gm"),
        (OTHER, Some(200), "Bob"),
        (MOB, None, "Jaffa"),
        (FAR, Some(400), "Far"),
    ] {
        mgr.create_entity(id, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        let e = mgr.get_entity_mut(id).unwrap();
        e.is_player = pid.is_some();
        e.player_id = pid;
        e.account_id = pid.map(|p| p as u32 + 9000);
        if pid.is_some() {
            e.character_name = Some(name.to_string());
        } else {
            e.npc_name = Some(name.to_string());
        }
    }
    crate::test_fixtures::seed_ability_defs(&mut mgr, &[ABILITY, HEAL]);
    mgr
}

fn hit(target_id: u32) -> Note {
    Note::Hit(HitNote {
        target_id,
        qr: 0.15,
        roll: 0.62,
        result_code: 1,
        result: "hit",
        dont_use_qr: false,
        before: Pools {
            health: 100,
            focus: 50,
        },
        after: Pools {
            health: 77,
            focus: 50,
        },
    })
}

fn plan(target_id: u32) -> Note {
    Note::Plan(PlanNote {
        target_id,
        effect_id: Some(4040),
        path: "nvp",
        reason: "hit_roll",
    })
}

/// A hostile cast by `caster` at `target`: fire, hit and one plan.
fn hostile(mgr: &mut SpaceManager, caster: u32, cast_id: i32, target: u32) {
    let fire = Note::Fire {
        target: Some(target),
        beneficial: false,
    };
    mgr.note_combat_debug(caster, Some(cast_id), ABILITY, fire);
    mgr.note_combat_debug(caster, Some(cast_id), ABILITY, hit(target));
    mgr.note_combat_debug(caster, Some(cast_id), ABILITY, plan(target));
}

fn texts(out: &[Outgoing], to: u32) -> Vec<String> {
    out.iter()
        .filter(|o| o.recipient == to)
        .map(|o| o.text.clone())
        .collect()
}

const HIT_LINE: &str = "[CD #7] TestAbility592 (592) Gm(1) -> Jaffa(3): hit, roll 0.620 qr 0.150; \
     HP 100->77 (-23), FP 50->50 (+0)";
const PLAN_LINE: &str = "[CD #7]  plan eff 4040 -> Jaffa(3): nvp (hit_roll)";

/// **Guard.** The simple line names the cast, the ability, caster and
/// target, the roll and result, and the pools; the verbose line the plan.
#[test]
fn the_formatter_builds_the_simple_and_verbose_lines() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    hostile(&mut m, GM, 7, MOB);
    let lines = format_record(&m, &m.combat_debug.open[0]);
    assert_eq!(lines.simple, vec![HIT_LINE.to_string()]);
    assert_eq!(lines.verbose, vec![PLAN_LINE.to_string()]);
}

/// A cast that reached no one still prints one line.
#[test]
fn a_cast_with_nothing_resolved_prints_one_line() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    let fire = Note::Fire {
        target: None,
        beneficial: false,
    };
    m.note_combat_debug(GM, Some(3), ABILITY, fire);
    let out = prepare(&mut m, Instant::now());
    assert_eq!(
        texts(&out, GM),
        vec!["[CD #3] TestAbility592 (592) Gm(1) -> no target: fired, nothing resolved"]
    );
}

/// No watcher: nothing is noted, so nothing is sent.
#[test]
fn nothing_is_noted_while_no_one_debugs() {
    let mut m = mgr();
    hostile(&mut m, GM, 7, MOB);
    assert!(m.combat_debug.open.is_empty());
    assert!(prepare(&mut m, Instant::now()).is_empty());
}

/// A long line splits under the chat cap, at spaces, losing nothing.
#[test]
fn a_long_line_splits_under_the_chat_cap() {
    let word = "abcdefghi ";
    let text = word.repeat(60);
    let parts = split_line(text.trim_end(), MAX_LINE_UNITS);
    assert!(parts.len() >= 3, "{parts:?}");
    for p in &parts {
        assert!(p.encode_utf16().count() <= MAX_LINE_UNITS, "{p}");
    }
    let joined: String = parts
        .iter()
        .map(|p| p.trim_start_matches("  ... "))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(joined, text.trim_end());
    assert_eq!(split_line("short", MAX_LINE_UNITS), vec!["short"]);
}

/// Combat debug delivers the simple line to the caster; verbose adds the
/// detail; an uninvolved watcher and a heal-only watcher get nothing.
#[test]
fn routing_follows_the_toggles() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    toggle(&mut m, FAR, Toggle::Combat).unwrap();
    toggle(&mut m, OTHER, Toggle::Heal).unwrap();
    hostile(&mut m, GM, 7, MOB);
    hostile(&mut m, OTHER, 8, MOB);
    let out = prepare(&mut m, Instant::now());
    assert_eq!(texts(&out, GM), vec![HIT_LINE.to_string()]);
    assert!(texts(&out, FAR).is_empty(), "FAR is in neither cast");
    assert!(
        texts(&out, OTHER).is_empty(),
        "heal debug ignores a hostile cast"
    );

    toggle(&mut m, GM, Toggle::Verbose).unwrap();
    hostile(&mut m, GM, 9, MOB);
    let out = prepare(&mut m, Instant::now() + WINDOW * 2);
    assert_eq!(texts(&out, GM).len(), 2, "simple + verbose: {out:?}");
}

/// The target of a hostile cast with combat debug sees the hit too.
#[test]
fn the_debugged_target_sees_the_hit_it_takes() {
    let mut m = mgr();
    toggle(&mut m, OTHER, Toggle::Combat).unwrap();
    hostile(&mut m, MOB, 4, OTHER);
    let out = prepare(&mut m, Instant::now());
    assert_eq!(texts(&out, OTHER).len(), 1);
}

/// Heal debug takes a beneficial cast; combat debug alone does not.
#[test]
fn heal_debug_takes_beneficial_casts() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    let fire = Note::Fire {
        target: Some(GM),
        beneficial: true,
    };
    m.note_combat_debug(GM, Some(5), HEAL, fire.clone());
    assert!(prepare(&mut m, Instant::now()).is_empty());
    toggle(&mut m, GM, Toggle::Heal).unwrap();
    m.note_combat_debug(GM, Some(6), HEAL, fire);
    let landing = Note::Landing(LandingNote {
        recipient: GM,
        before: Pools {
            health: 60,
            focus: 5,
        },
        after: Pools {
            health: 100,
            focus: 5,
        },
    });
    m.note_combat_debug(GM, Some(6), HEAL, landing);
    let out = prepare(&mut m, Instant::now());
    assert_eq!(
        texts(&out, GM),
        vec!["[CD #6] TestAbility12 (12) Gm(1) -> Gm(1): landed; HP 60->100 (+40), FP 5->5 (+0)"]
    );
}

/// The ability list prints only its abilities, with every toggle off.
#[test]
fn the_ability_list_prints_only_its_abilities() {
    let mut m = mgr();
    assert!(toggle_ability(&mut m, GM, ABILITY).unwrap().0);
    hostile(&mut m, GM, 7, MOB);
    let fire = Note::Fire {
        target: Some(MOB),
        beneficial: false,
    };
    m.note_combat_debug(GM, Some(8), HEAL, fire);
    let out = prepare(&mut m, Instant::now());
    assert_eq!(texts(&out, GM), vec![HIT_LINE.to_string()]);
    assert!(toggle_ability(&mut m, GM, 999_999).is_err(), "unknown id");
    let cleared = clear_ability_debug(&mut m, GM).unwrap();
    assert!(cleared.contains("cleared"));
    assert!(m.combat_debug.watchers.is_empty(), "nothing left on");
}

/// Mob debug prints the selected mob's casts to the GM, who is not in them.
#[test]
fn mob_debug_prints_the_mobs_casts() {
    let mut m = mgr();
    assert_eq!(
        toggle_mob(&mut m, GM, 0).unwrap_err().reason,
        "no_target",
        "a selection is needed"
    );
    m.get_entity_mut(GM).unwrap().current_target_id = Some(OTHER as i32);
    assert_eq!(
        toggle_mob(&mut m, GM, 0).unwrap_err().reason,
        "target_is_player"
    );
    m.get_entity_mut(GM).unwrap().current_target_id = Some(MOB as i32);
    let (on, text) = toggle_mob(&mut m, GM, ABILITY).unwrap();
    assert!(on && text.contains("Jaffa(3)"), "{text}");
    hostile(&mut m, MOB, 4, OTHER);
    let out = prepare(&mut m, Instant::now());
    assert_eq!(texts(&out, GM).len(), 1, "{out:?}");
}

/// `setAbilityDebugTarget` sends the lines to another player.
#[test]
fn the_debug_target_receives_the_lines() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    assert!(
        set_ability_debug_target(&mut m, GM, MOB).is_err(),
        "not a player"
    );
    set_ability_debug_target(&mut m, GM, OTHER).unwrap();
    hostile(&mut m, GM, 7, MOB);
    let out = prepare(&mut m, Instant::now());
    assert!(texts(&out, GM).is_empty());
    assert_eq!(texts(&out, OTHER), vec![HIT_LINE.to_string()]);
}

/// The scope still open keeps its record; it flushes once the scope ends.
#[test]
fn an_open_scopes_record_waits_for_its_scope() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    hostile(&mut m, GM, 7, MOB);
    let outer = m.enter_cast_scope(Some(7));
    assert!(prepare(&mut m, Instant::now()).is_empty());
    m.exit_cast_scope(outer);
    assert_eq!(prepare(&mut m, Instant::now()).len(), 1);
}

/// A watcher whose entity now plays someone else is dropped.
#[test]
fn a_stale_watcher_is_dropped() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    m.get_entity_mut(GM).unwrap().player_id = Some(555);
    m.get_entity_mut(GM).unwrap().account_id = Some(5555);
    let logs = LogCapture::install();
    hostile(&mut m, GM, 7, MOB);
    assert!(prepare(&mut m, Instant::now()).is_empty());
    assert!(m.combat_debug.watchers.is_empty());
    // The row names the watcher as it was when it turned debugging on.
    let row = one_row(&logs, "combat_debug_watcher_dropped");
    assert!(row.has_field("account_id", "9100"), "{row:?}");
    assert!(row.has_field("player_id", "100"), "{row:?}");
}

fn one_row(
    logs: &crate::test_support::LogCaptureGuard,
    event: &str,
) -> crate::test_support::Captured {
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", event))
        .collect();
    assert_eq!(rows.len(), 1, "one {event} row: {rows:#?}");
    rows[0].clone()
}

/// **Guard.** An evicted record's row names its caster by the ids
/// snapshotted when the record opened, even after the caster changed.
#[test]
fn an_evicted_record_names_its_caster() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    let logs = LogCapture::install();
    hostile(&mut m, GM, 1, MOB);
    m.get_entity_mut(GM).unwrap().account_id = None;
    for cast in 2..=(super::MAX_OPEN_RECORDS as i32 + 1) {
        hostile(&mut m, GM, cast, MOB);
    }
    let row = one_row(&logs, "combat_debug_record_evicted");
    for (k, v) in [
        ("entity_id", "1"),
        ("account_id", "9100"),
        ("player_id", "100"),
        ("cast_id", "1"),
    ] {
        assert!(row.has_field(k, v), "{k} = {v}: {row:?}");
    }
}

/// **Guard.** A relog on the same entity id starts with debug off:
/// `destroy_entity` forgets the watcher, so the new entity's casts print
/// nothing.
#[test]
fn a_relog_on_a_recycled_id_starts_with_debug_off() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    m.destroy_entity(GM);
    m.create_entity(GM, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let e = m.get_entity_mut(GM).unwrap();
    e.is_player = true;
    e.player_id = Some(100);
    assert!(m.combat_debug.settings(GM).is_none(), "debug starts off");
    hostile(&mut m, GM, 7, MOB);
    assert!(prepare(&mut m, Instant::now()).is_empty());
}

/// **Guard.** A destroyed mob takes its mob-debug entries with it, so a
/// mob spawned on the recycled id is not debugged.
#[test]
fn a_destroyed_mob_leaves_no_mob_debug_behind() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    m.get_entity_mut(GM).unwrap().current_target_id = Some(MOB as i32);
    toggle_mob(&mut m, GM, 0).unwrap();
    m.destroy_entity(MOB);
    assert!(m.combat_debug.settings(GM).unwrap().mobs.is_empty());
    m.create_entity(MOB, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    hostile(&mut m, MOB, 4, OTHER);
    assert!(texts(&prepare(&mut m, Instant::now()), GM).is_empty());
}

/// **Guard.** `destroy_space` forgets the debug state of every entity it
/// removes.
#[test]
fn destroy_space_forgets_its_watchers() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    toggle(&mut m, OTHER, Toggle::Heal).unwrap();
    let space = m.get_entity_space_id(GM).unwrap();
    m.destroy_space(space);
    assert!(!m.combat_debug.is_active(), "no watcher survives its space");
}

/// **Guard.** A storm past the budget sends `LINES_PER_WINDOW` lines,
/// writes a `suppressed` row for each of the rest, and the next flush after
/// the window says how many were held back. Never silent.
#[test]
fn a_storm_is_rate_limited_with_a_suppressed_notice() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    let logs = LogCapture::install();
    let t0 = Instant::now();
    let extra = 5;
    for cast in 0..(LINES_PER_WINDOW as i32 + extra) {
        hostile(&mut m, GM, cast + 1, MOB);
    }
    let out = prepare(&mut m, t0);
    assert_eq!(out.len(), LINES_PER_WINDOW as usize);
    let suppressed = logs
        .all()
        .into_iter()
        .filter(|c| c.has_field("delivery", "suppressed"))
        .count();
    assert_eq!(
        suppressed, extra as usize,
        "each held line still has its row"
    );

    let later = prepare(&mut m, t0 + WINDOW + Duration::from_millis(1));
    assert_eq!(
        texts(&later, GM),
        vec![format!(
            "[CD] +{extra} lines suppressed (over {LINES_PER_WINDOW} per second); \
             the abilities.debug rows have them all"
        )]
    );
}

/// **Guard.** The `abilities.debug` row's `text` is the exact text sent,
/// and it names the recipient and the cast.
#[tokio::test]
async fn the_row_text_equals_the_client_text() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    let logs = LogCapture::install();
    hostile(&mut m, GM, 7, MOB);
    let (tx, mut rx) = mpsc::channel(8);
    flush(&tx, &mut m).await;
    let Ok(CellToBaseMsg::EntityMethodCall { args, .. }) = rx.try_recv() else {
        panic!("one line sent");
    };
    let sent = decode_text(&args);
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "combat_debug_line"))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = &rows[0];
    assert_eq!(row.target, "abilities.debug");
    assert_eq!(
        row.fields.get("text").map(String::as_str),
        Some(sent.as_str())
    );
    for (k, v) in [
        ("player_id", "100"),
        ("cast_id", "7"),
        ("ability_id", "592"),
        ("line_kind", "simple"),
        ("delivery", "queued_to_base"),
        ("caster_account_id", "9100"),
    ] {
        assert!(row.has_field(k, v), "{k} = {v}: {row:?}");
    }
}

/// **Guard.** A line the closed cell-to-base queue refused logs
/// `send_failed`, never `queued_to_base`: the row is written after the send.
#[tokio::test]
async fn a_refused_send_logs_send_failed() {
    let mut m = mgr();
    toggle(&mut m, GM, Toggle::Combat).unwrap();
    let logs = LogCapture::install();
    hostile(&mut m, GM, 7, MOB);
    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    flush(&tx, &mut m).await;
    let row = one_row(&logs, "combat_debug_line");
    assert!(row.has_field("delivery", "send_failed"), "{row:?}");
}

/// The text WSTRING of an `onPlayerCommunication` payload.
fn decode_text(args: &[u8]) -> String {
    let spk = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let off = 4 + spk * 2 + 2;
    let n = u32::from_le_bytes(args[off..off + 4].try_into().unwrap()) as usize;
    let units: Vec<u16> = (0..n)
        .map(|i| u16::from_le_bytes([args[off + 4 + i * 2], args[off + 5 + i * 2]]))
        .collect();
    String::from_utf16(&units).unwrap()
}

/// **Byte-exact.** One debug line on the wire: `onPlayerCommunication`
/// (client method 28) to the recipient, speaker `SYSTEM`, flags 0, channel
/// 9 (`CHAN_FEEDBACK`), then the text as a WSTRING.
#[tokio::test]
async fn a_debug_line_is_one_feedback_channel_chat_line() {
    let m = mgr();
    let (tx, mut rx) = mpsc::channel(4);
    send_feedback_line(&tx, &m, GM, "[CD #7] hi").await;
    let Ok(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    }) = rx.try_recv()
    else {
        panic!("one EntityMethodCall");
    };
    assert_eq!((entity_id, method_index), (GM, 28));
    let mut want: Vec<u8> = vec![6, 0, 0, 0];
    for c in "SYSTEM".encode_utf16() {
        want.extend_from_slice(&c.to_le_bytes());
    }
    want.extend_from_slice(&[0, 9]);
    want.extend_from_slice(&10u32.to_le_bytes());
    for c in "[CD #7] hi".encode_utf16() {
        want.extend_from_slice(&c.to_le_bytes());
    }
    assert_eq!(args, want);
}
