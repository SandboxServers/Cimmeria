//! SS-U2: `.duel_status [name]` and `.duel_end <name>` through the console
//! parser, the refusals (type 12), and the GM gate.

use cimmeria_cell_world::cell::duel::DuelResources;
use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_wire::cell::client_methods::duel::TEXT_DUEL_ABORTED;
use tokio::sync::mpsc;
use tracing::Level;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::super::duel::{parse_duel_end, parse_duel_status, DUEL_END_USAGE};
use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{LogCapture, LogCaptureGuard};

/// The GM: entity 1 from `setup`, account 7, player 70, named "Gm".
const GM_PID: i32 = 70;
/// Two duelists beside the GM.
const ANA: (u32, i32) = (2, 4401);
const BO: (u32, i32) = (3, 4402);

fn fixture() -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    {
        let e = mgr.get_entity_mut(gm).unwrap();
        e.account_id = Some(7);
        e.player_id = Some(GM_PID);
        e.character_name = Some("Gm".into());
        e.current_target_id = None;
    }
    for ((eid, pid), name, x) in [(ANA, "Ana", 11.0), (BO, "Bo", 13.0)] {
        mgr.create_entity(eid, "Agnos", [x, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(eid);
        let e = mgr.get_entity_mut(eid).unwrap();
        e.is_player = true;
        e.player_id = Some(pid);
        e.account_id = Some(eid + 100);
        e.character_name = Some(name.into());
    }
    (mgr, gm)
}

/// Ana challenged Bo and Bo accepted: a duel in the countdown.
fn start_duel(mgr: &mut SpaceManager) {
    let now = Instant::now();
    let p = mgr
        .resources
        .duels_mut()
        .open_challenge(ANA.1, BO.1, now)
        .unwrap();
    mgr.resources
        .duels_mut()
        .take_pending_for(BO.1, now)
        .unwrap();
    mgr.resources
        .duels_mut()
        .start_duel(&p, 1, Vector3::new(12.0, 0.0, 10.0), now);
}

async fn run(mgr: &mut SpaceManager, gm: u32, line: &str) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, line, &tx, mgr, &ChainEngine::new()).await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn lines_to(msgs: &[CellToBaseMsg], entity: u32) -> Vec<String> {
    msgs.iter()
        .filter(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id, .. } if *entity_id == entity))
        .filter_map(decode_feedback)
        .collect()
}

fn rejected(capture: &LogCaptureGuard, reason: &str) -> bool {
    capture.all().iter().any(|c| {
        c.level == Level::DEBUG
            && c.target == "duel"
            && c.has_field("event", "duel.gm_rejected")
            && c.has_field("reason", reason)
            && c.has_field("account_id", "7")
            && c.has_field("player_id", &GM_PID.to_string())
            && c.has_field("entity_id", "1")
    })
}

#[test]
fn duel_status_parse_takes_an_optional_name() {
    assert_eq!(parse_duel_status(&[]), None);
    assert_eq!(parse_duel_status(&["Bo"]), Some("Bo"));
}

#[test]
fn duel_end_parse_requires_a_name() {
    assert_eq!(parse_duel_end(&[]), Err(DUEL_END_USAGE));
    assert_eq!(parse_duel_end(&["Bo"]), Ok("Bo"));
}

/// `.duel_end Bo` ends the countdown duel: 878 to Ana and to Bo, both
/// entries gone, the GM told who and what, and the audit row names the
/// GM and the subject.
#[tokio::test]
async fn duel_end_clears_both_entries_and_tells_both() {
    let capture = LogCapture::install();
    let (mut mgr, gm) = fixture();
    start_duel(&mut mgr);

    let msgs = run(&mut mgr, gm, ".duel_end Bo").await;

    assert_eq!(lines_to(&msgs, ANA.0), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&msgs, BO.0), vec![TEXT_DUEL_ABORTED.to_string()]);
    let gm_lines = lines_to(&msgs, gm);
    assert_eq!(gm_lines.len(), 1, "{gm_lines:?}");
    assert!(
        gm_lines[0].starts_with(".duel_end: ended duel #1 (countdown) between Ana and Bo"),
        "{gm_lines:?}"
    );
    assert!(
        mgr.resources.duels().duel_of(ANA.1).is_none()
            && mgr.resources.duels().duel_of(BO.1).is_none()
    );
    assert!(!mgr.resources.duels().is_busy(ANA.1) && !mgr.resources.duels().is_busy(BO.1));

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.gm_ended"))
        .expect("duel.gm_ended row");
    assert!(row.has_field("account_id", "7"));
    assert!(row.has_field("player_id", &GM_PID.to_string()));
    assert!(row.has_field("subject_player_id", &BO.1.to_string()));
    assert!(row.has_field("opponent_player_id", &ANA.1.to_string()));
}

/// `.duel_end Ana` on a pending challenge withdraws it; both hear 878.
#[tokio::test]
async fn duel_end_withdraws_a_pending_challenge() {
    let (mut mgr, gm) = fixture();
    mgr.resources
        .duels_mut()
        .open_challenge(ANA.1, BO.1, Instant::now())
        .unwrap();

    let msgs = run(&mut mgr, gm, ".duel_end Ana").await;

    assert_eq!(lines_to(&msgs, ANA.0), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert_eq!(lines_to(&msgs, BO.0), vec![TEXT_DUEL_ABORTED.to_string()]);
    assert!(
        mgr.resources.duels().is_idle(),
        "no challenge and no cooldown left"
    );
}

/// Type 12: a bare `.duel_end` reaches the command's own usage line and
/// `reason=no_name`, not the generic argc check.
#[tokio::test]
async fn bare_duel_end_logs_no_name() {
    let capture = LogCapture::install();
    let (mut mgr, gm) = fixture();
    let msgs = run(&mut mgr, gm, ".duel_end").await;
    assert!(rejected(&capture, "no_name"));
    assert_eq!(lines_to(&msgs, gm), vec![DUEL_END_USAGE.to_string()]);
}

/// Type 12: an unknown name is refused with `reason=target_not_found`, for
/// both commands, and nothing else is sent.
#[tokio::test]
async fn unknown_name_logs_target_not_found() {
    for cmd in ["duel_end", "duel_status"] {
        let capture = LogCapture::install();
        let (mut mgr, gm) = fixture();
        start_duel(&mut mgr);
        let msgs = run(&mut mgr, gm, &format!(".{cmd} Nobody")).await;
        assert!(rejected(&capture, "target_not_found"), ".{cmd}");
        assert_eq!(msgs.len(), 1, ".{cmd}: only the GM's line: {msgs:?}");
        assert!(lines_to(&msgs, gm)[0].contains("no online player is named Nobody"));
        assert!(
            mgr.resources.duels().duel_of(ANA.1).is_some(),
            "the duel is untouched"
        );
    }
}

/// Type 12: ending a player who is in nothing is refused with
/// `reason=nothing_to_end` and names the subject.
#[tokio::test]
async fn duel_end_on_an_idle_player_logs_nothing_to_end() {
    let capture = LogCapture::install();
    let (mut mgr, gm) = fixture();
    let msgs = run(&mut mgr, gm, ".duel_end Bo").await;
    assert!(rejected(&capture, "nothing_to_end"));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.gm_rejected")
            && c.has_field("subject_player_id", &BO.1.to_string())));
    assert_eq!(
        lines_to(&msgs, gm),
        vec![".duel_end: Bo is not in a duel and has no duel challenge.".to_string()]
    );
    assert!(lines_to(&msgs, BO.0).is_empty());
}

/// `.duel_status` with no name reads the caller's own entry; with a name,
/// that player's, naming the opponent.
#[tokio::test]
async fn duel_status_reports_self_and_a_named_duelist() {
    let (mut mgr, gm) = fixture();
    start_duel(&mut mgr);

    let own = lines_to(&run(&mut mgr, gm, ".duel_status").await, gm);
    assert_eq!(
        own,
        vec![format!(
            ".duel_status: Gm (player {GM_PID}) is not in a duel and has no duel challenge."
        )]
    );

    let bo = lines_to(&run(&mut mgr, gm, ".duel_status Bo").await, gm);
    assert_eq!(bo.len(), 1);
    assert!(
        bo[0].starts_with(&format!(
            ".duel_status: Bo (player {}) is in duel #1 with Ana (player {}), in the countdown, starting in ",
            BO.1, ANA.1
        )),
        "{bo:?}"
    );
    assert!(
        mgr.resources.duels().duel_of(BO.1).is_some(),
        "a status read changes nothing"
    );
}

/// The GM gate: a player typing `.duel_end` is chatting, and the duel it
/// names is untouched.
#[tokio::test]
async fn non_gm_duel_end_is_chat() {
    let (mut mgr, gm) = fixture();
    mgr.get_entity_mut(gm).unwrap().access_level = 0;
    start_duel(&mut mgr);
    let (tx, _rx) = mpsc::channel(64);
    handle_chat_message(
        gm,
        "Gm",
        0,
        CHAN_SAY,
        ".duel_end Bo",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    assert!(mgr.resources.duels().duel_of(BO.1).is_some());
}

/// `.help duel_end` shows the summary and the argument detail line.
#[tokio::test]
async fn help_duel_end_shows_argument_detail() {
    let (mut mgr, gm) = fixture();
    let lines = lines_to(&run(&mut mgr, gm, ".help duel_end").await, gm);
    assert!(
        lines.iter().any(|l| l.starts_with(".duel_end: ")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("    [name] (str): Required.")),
        "{lines:?}"
    );
}

/// An engaged duel (SS-D2): `.duel_end` goes through `end_engaged` with
/// `reason = gm_aborted`, so each duelist gets its PvP flag cleared (7),
/// `onDuelEntitiesClear` (153) and "Duel aborted" exactly once, and the
/// registry entry is gone. Fails if `gm_end` removes an engaged duel itself
/// (no 7, no 153) or sends its own 878 as well (two lines).
#[tokio::test]
async fn duel_end_on_an_engaged_duel_clears_through_end_engaged() {
    use crate::cell::duel::limits::COUNTDOWN;
    use crate::cell::duel::DuelState;
    const PVP_FLAG: u16 = 7;
    const DUEL_CLEAR: u16 = 153;

    let capture = LogCapture::install();
    let (mut mgr, gm) = fixture();
    for eid in [ANA.0, BO.0] {
        mgr.get_entity_mut(eid).unwrap().archetype_id = Some(1);
    }
    let _ = mgr.compute_aoi_changes();
    let (tx, mut rx) = mpsc::channel(256);
    let t0 = Instant::now();
    let p = mgr
        .resources
        .duels_mut()
        .open_challenge(ANA.1, BO.1, t0)
        .unwrap();
    mgr.resources
        .duels_mut()
        .take_pending_for(BO.1, t0)
        .unwrap();
    let space_id = *mgr.entity_space.get(&ANA.0).unwrap();
    mgr.resources
        .duels_mut()
        .start_duel(&p, space_id, Vector3::new(12.0, 0.0, 10.0), t0);
    cimmeria_cell_duel::cell::duel::tick::run_at(&tx, &mut mgr, t0 + COUNTDOWN).await;
    while rx.try_recv().is_ok() {}
    assert!(matches!(
        mgr.resources.duels().duel_of(ANA.1).unwrap().state,
        DuelState::Engaged { .. }
    ));

    let msgs = run(&mut mgr, gm, ".duel_end Bo").await;

    let own = |entity: u32, method: u16| {
        msgs.iter()
            .filter(|m| {
                matches!(m, CellToBaseMsg::EntityMethodCall { entity_id, method_index, .. }
                    if *entity_id == entity && *method_index == method)
            })
            .count()
    };
    for eid in [ANA.0, BO.0] {
        assert_eq!(own(eid, PVP_FLAG), 1, "PvP flag cleared once for {eid}");
        assert_eq!(own(eid, DUEL_CLEAR), 1, "153 once to {eid}");
        assert_eq!(
            lines_to(&msgs, eid),
            vec![TEXT_DUEL_ABORTED.to_string()],
            "878 once to {eid}"
        );
    }
    assert!(
        mgr.resources.duels().duel_of(ANA.1).is_none()
            && mgr.resources.duels().duel_of(BO.1).is_none()
    );
    assert!(!mgr.resources.duels().can_harm(ANA.1, BO.1));
    let rows = capture.all();
    assert!(rows
        .iter()
        .any(|c| c.has_field("event", "duel.ended") && c.has_field("reason", "gm_aborted")));
    assert!(rows
        .iter()
        .any(|c| c.has_field("event", "duel.gm_ended") && c.has_field("stage", "engaged")));
    assert!(lines_to(&msgs, gm)[0].contains("(engaged)"));
}
