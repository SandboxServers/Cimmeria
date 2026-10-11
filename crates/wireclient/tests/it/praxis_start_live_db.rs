//! The Praxis start, mission 622 "Arm Yourself!", end to end on the wire
//! (WC-14). A wire-created Praxis Commando on a GM account plays the ten
//! client calls of the lab capture (`praxis_start_tap.json`; ADR § Praxis
//! start) and the test checks every server answer the capture shows, in the
//! capture's order, then reads the server's own record (`sgw_mission`) as
//! the lab rows do. Run with
//! `pwsh tools/build-lane/live-db-test.ps1 --wireclient praxis_start`.
//!
//! No entity id is a literal: Frost is found through the mirror by template
//! 14's static mesh and name id (D-WC1), and the Guard's body is whichever
//! entity the server flags as a mission object when Frost is searched. The
//! pistol is the item the Guard's dialog grants, by design id from the seed.
//!
//! Every wait has a deadline and fails with the last 30 observed messages
//! and the last 30 decoded events (D-WC9); nothing sleeps. Waiting between DB polls pumps the session, so
//! keep-alives and ACKs keep flowing.
//!
//! Cleanup: the account and its rows are deleted at the start of the test
//! and at the end of a passing run. A panic skips the end-of-run cleanup;
//! the next run's start-of-test cleanup removes the leftovers, which is safe
//! because each running test has its own database clone (`wireclient-e2e`).

use std::time::{Duration, Instant};

use sqlx::PgPool;

use cimmeria_wireclient::calls::CLIENT_CALL_ENTITY_ID;
use cimmeria_wireclient::script::{Observed, ScriptSession};
use cimmeria_wireclient::semantic::{DialogDisplay, InvItem, S2CEvent};
use cimmeria_wireclient::session::GameSession;
use cimmeria_wireclient::tap_fixture::{TapCapture, TapRecord};

use crate::support::{
    self, cleanup_account, credentials_for, default_visual_choices,
    insert_sentinel_account_with_level, item_granted_by_dialog, start_server, template_query,
    SHARD,
};

/// Campaign sentinel block 0x7000_9C00..=0x7000_9CFF; WC-14 owns 0x..02.
const ACCOUNT: i32 = 0x7000_9C02;
const ACCOUNT_NAME: &str = "wc14_praxis";
/// `gmGotoXYZ` needs a GM account (D-WC2 (a)).
const ACCESS_GM: i32 = 2;
/// Praxis Commando, male (D-WC11).
const CHAR_DEF_PRAXIS: i32 = 3;
/// Corporal Frost's corpse (README F5).
const FROST_TEMPLATE: i32 = 14;
const INTRO_MOVIE: &str = "Cine-SGWLogo.SGWLogo";

const DIALOG_ARRIVAL: i32 = 2982;
const DIALOG_FROST: i32 = 3995;
const DIALOG_GUARD: i32 = 3996;
const DIALOG_WEAPON_TUTORIAL: i32 = 5882;
const MISSION_ARM_YOURSELF: i32 = 622;
const MISSION_FROSTS_LETTER: i32 = 1360;
const STEP_SEARCH_FROST: i32 = 2113;
const STEP_SEARCH_GUARD: i32 = 80623;
const STEP_EQUIP_PISTOL: i32 = 80622;
const SEQ_STASIS_DOOR: i32 = 10000;
const SEQ_AFTER_EQUIP: i32 = 1872;
/// The four CORE_TUTORIAL abilities the Guard's body teaches (fixture #49
/// lists them as 597, 1218, 592, 594). The list is the player's whole known
/// set, which the cell keeps in a hash set, so its order changes from run to
/// run; the test compares it as a set.
const TUTORIAL_ABILITIES: [i32; 4] = [592, 594, 597, 1218];
/// `INT_MissionWorldObject`: the highlight on a searchable corpse.
const INT_MISSION_WORLD_OBJECT: u64 = 0x4000_0000;
/// Mission and step status on the wire: 0 started, 1 done.
const STARTED: i8 = 0;
const DONE: i8 = 1;
/// `sgw_mission.status`: 1 active, 2 completed (what the lab rows grade).
const DB_ACTIVE: i32 = 1;
const DB_COMPLETED: i32 = 2;
/// The bandolier, as the client names it in `moveItem` (README F7).
const BAG_BANDOLIER: i32 = 3;
/// `moveItem`'s slot is 1-based on the wire; the cell subtracts one.
const BANDOLIER_SLOT_WIRE: i32 = 1;

const RECV: Duration = Duration::from_secs(5);
/// Each step's answers, unless the packet's table says otherwise.
const STEP: Duration = Duration::from_secs(10);
/// The AoI flush after `cancelMovie` introduces the whole Cellblock.
const FROST_WAIT: Duration = Duration::from_secs(20);
/// How long one pump runs while a helper below waits on the log or the DB.
const PUMP_SLICE: Duration = Duration::from_millis(50);
const DB_POLL: Duration = Duration::from_millis(100);
/// How many entries a failure prints (D-WC9).
const TRAIL: usize = 30;

/// Unwrap a script result, or [`fail`] with its error and both trails. The
/// result is bound first, so the script call's `&mut` borrow has ended
/// before `fail` reads the log.
macro_rules! checked {
    ($script:ident, $result:expr) => {{
        let result = $result;
        match result {
            Ok(value) => value,
            Err(err) => fail(&$script, err),
        }
    }};
}

#[tokio::test]
async fn praxis_start_completes_mission_622_live_db() {
    let pool = cimmeria_test_support::require_db_or_skip!();
    support::init_test_logging();
    let started = Instant::now();
    cleanup_account(&pool, ACCOUNT).await;
    insert_sentinel_account_with_level(&pool, ACCOUNT, ACCOUNT_NAME, ACCESS_GM).await;
    let pistol_design = item_granted_by_dialog(&pool, DIALOG_GUARD).await;
    let frost_query = template_query(&pool, FROST_TEMPLATE).await;

    // Positions from the capture's bits, not decimals (README F12).
    let cap = TapCapture::praxis_start();
    let p1 = goto_pos(cap.expect(4, "gmGotoXYZ"));
    let p1_region = region_pos(cap.expect(8, "triggerClientHintedGenericRegion"));
    let p2 = goto_pos(cap.expect(35, "gmGotoXYZ"));

    let server = start_server(&support::db_url()).await;

    // ── Step 0: create the character on the wire and enter the world ─────
    let mut session =
        GameSession::connect(&server.auth_url, &credentials_for(ACCOUNT_NAME), SHARD, 1)
            .await
            .expect("connect");
    session
        .open_character_list(RECV)
        .await
        .expect("character list");
    let choices = default_visual_choices(&pool, CHAR_DEF_PRAXIS).await;
    session
        .create_character_on_wire("Wirescript", CHAR_DEF_PRAXIS, &choices, RECV)
        .await
        .expect("createCharacter accepted");
    let (player_id,): (i32,) =
        sqlx::query_as("SELECT player_id FROM sgw_player WHERE account_id = $1")
            .bind(ACCOUNT)
            .fetch_one(&pool)
            .await
            .expect("the new character");
    session.play(player_id, RECV).await.expect("play");
    let own = session.player_entity_id.expect("own entity id");
    let mut script = ScriptSession::new(session).unwrap_or_else(|e| panic!("{e}"));

    // ── 0a: the first-login movie; cancel it to release the AoI hold ─────
    // The movie (base, on onClientReady) and the arrival dialog (cell, on
    // player_loaded) race, so 0a and 0b scan the whole log rather than move
    // the cursor past one of them. The cursor stays at 0 until step 2.
    let movie = wait_since(&mut script, 0, "onPlayMovie", STEP, |o| {
        matches!(&o.event, Some(S2CEvent::PlayMovie { movie_name, .. }) if movie_name == INTRO_MOVIE)
    })
    .await;
    assert_eq!(
        movie.msg.entity_id,
        Some(own),
        "the movie plays for the player"
    );
    send(
        &mut script,
        GameSession::cancel_movie(CLIENT_CALL_ENTITY_ID, INTRO_MOVIE),
    )
    .await;

    // ── 0b: Frost arrives in the flush; the arrival dialog is up ─────────
    let frost = checked!(script, script.wait_entity(&frost_query, FROST_WAIT).await);
    let arrival = wait_since(&mut script, 0, "the arrival dialog 2982", STEP, |o| {
        dialog(o, DIALOG_ARRIVAL).is_some()
    })
    .await;
    assert_eq!(
        arrival.msg.entity_id,
        Some(own),
        "arrival dialog targets the player"
    );

    // ── 1: close the arrival dialog ──────────────────────────────────────
    send(
        &mut script,
        GameSession::dialog_button_choice(CLIENT_CALL_ENTITY_ID, DIALOG_ARRIVAL, -1),
    )
    .await;

    // ── 2: GM teleport next to Frost, then report leaving region 14 ──────
    send(
        &mut script,
        GameSession::gm_goto_xyz(CLIENT_CALL_ENTITY_ID, p1),
    )
    .await;
    checked!(
        script,
        script
            .wait_event("step 2: the gmGotoXYZ line", STEP, is_teleport_line)
            .await
    );
    // The client reports the region exit after it has moved (fixture #8
    // follows #5).
    send(
        &mut script,
        GameSession::trigger_client_hinted_generic_region(
            CLIENT_CALL_ENTITY_ID,
            14,
            false,
            p1_region,
        ),
    )
    .await;

    // ── 3: search Frost ──────────────────────────────────────────────────
    // Frost is still the one entity the mirror matches: a second match that
    // arrived after `wait_entity` returned would make the id ambiguous.
    script
        .mirror
        .find_one(&frost_query)
        .unwrap_or_else(|e| panic!("Frost after the flush: {e}"));
    let frost_i32 = i32::try_from(frost).expect("entity id fits INT32");
    send(
        &mut script,
        GameSession::interact(CLIENT_CALL_ENTITY_ID, frost_i32),
    )
    .await;
    let step3 = script
        .expect_in_order(
            "step 3: search Frost",
            STEP,
            &mut [
                ("onDialogDisplay 3995 on Frost", &mut |o: &Observed| {
                    dialog(o, DIALOG_FROST).is_some_and(|d| d.entity_id == frost_i32)
                }),
                (
                    "InteractionType mission object on the body",
                    &mut |o: &Observed| {
                        is_interaction(o, INT_MISSION_WORLD_OBJECT)
                            && o.msg.entity_id.is_some_and(|id| id != frost && id != own)
                    },
                ),
                ("step 2113 done", &mut |o: &Observed| {
                    step(o, STEP_SEARCH_FROST, DONE)
                }),
                ("step 80623 started", &mut |o: &Observed| {
                    step(o, STEP_SEARCH_GUARD, STARTED)
                }),
                ("mission 1360 accepted", &mut |o: &Observed| {
                    mission(o, MISSION_FROSTS_LETTER, STARTED)
                }),
            ],
        )
        .await;
    let seen = checked!(script, step3);
    let body = seen[1].msg.entity_id.expect("the body's entity id");
    let body_i32 = i32::try_from(body).expect("entity id fits INT32");

    // ── 4, 5: close Frost's dialog; teleport next to the body ────────────
    send(
        &mut script,
        GameSession::dialog_button_choice(CLIENT_CALL_ENTITY_ID, DIALOG_FROST, -1),
    )
    .await;
    send(
        &mut script,
        GameSession::gm_goto_xyz(CLIENT_CALL_ENTITY_ID, p2),
    )
    .await;
    checked!(
        script,
        script
            .wait_event("step 5: the gmGotoXYZ line", STEP, is_teleport_line)
            .await
    );

    // ── 6: search the Guard's body ───────────────────────────────────────
    let grant_from = script.log().len();
    send(
        &mut script,
        GameSession::interact(CLIENT_CALL_ENTITY_ID, body_i32),
    )
    .await;
    let step6 = script
        .expect_in_order(
            "step 6: search the Guard's body",
            STEP,
            &mut [
                ("onDialogDisplay 3996 on the body", &mut |o: &Observed| {
                    dialog(o, DIALOG_GUARD).is_some_and(|d| d.entity_id == body_i32)
                }),
                ("step 80623 done", &mut |o: &Observed| {
                    step(o, STEP_SEARCH_GUARD, DONE)
                }),
                ("step 80622 started", &mut |o: &Observed| {
                    step(o, STEP_EQUIP_PISTOL, STARTED)
                }),
                (
                    "onKnownAbilitiesUpdate, the tutorial four",
                    &mut |o: &Observed| {
                        matches!(&o.event, Some(S2CEvent::KnownAbilitiesUpdate { ability_ids })
                        if sorted(ability_ids) == TUTORIAL_ABILITIES)
                    },
                ),
                ("onDialogDisplay 5882 on the player", &mut |o: &Observed| {
                    dialog(o, DIALOG_WEAPON_TUTORIAL)
                        .is_some_and(|d| i64::from(d.entity_id) == i64::from(own))
                }),
            ],
        )
        .await;
    checked!(script, step6);

    // The pistol arrives in an `onUpdateItem` from the base (chain 1005's
    // `add_item` to the backpack). It is not in the world-entry inventory:
    // the Praxis Commando profile starts without a gun (OD-CS01).
    wait_since(
        &mut script,
        grant_from,
        "onUpdateItem with the Guard's pistol",
        STEP,
        |o| update_has(o, |i| i.dbid == pistol_design),
    )
    .await;
    let pistol = only_item(&script, |i| i.dbid == pistol_design);
    assert_ne!(
        pistol.container_id, BAG_BANDOLIER,
        "the pistol must start outside the bandolier: {pistol:?}"
    );

    // ── 7, 8: close the tutorial, then the Guard's dialog ────────────────
    send(
        &mut script,
        GameSession::dialog_button_choice(CLIENT_CALL_ENTITY_ID, DIALOG_WEAPON_TUTORIAL, -1),
    )
    .await;
    send(
        &mut script,
        GameSession::dialog_button_choice(CLIENT_CALL_ENTITY_ID, DIALOG_GUARD, -1),
    )
    .await;

    // ── 9: drag the pistol to the bandolier ──────────────────────────────
    let move_from = script.log().len();
    send(
        &mut script,
        GameSession::move_item(
            CLIENT_CALL_ENTITY_ID,
            pistol.id,
            BAG_BANDOLIER,
            BANDOLIER_SLOT_WIRE,
            1,
        ),
    )
    .await;
    let step9 = script
        .expect_in_order(
            "step 9: equip the pistol",
            STEP,
            &mut [
                ("onSequence 10000", &mut |o: &Observed| {
                    sequence(o, SEQ_STASIS_DOOR)
                }),
                ("step 80622 done", &mut |o: &Observed| {
                    step(o, STEP_EQUIP_PISTOL, DONE)
                }),
                ("mission 622 complete", &mut |o: &Observed| {
                    mission(o, MISSION_ARM_YOURSELF, DONE)
                }),
                ("onSequence 1872", &mut |o: &Observed| {
                    sequence(o, SEQ_AFTER_EQUIP)
                }),
            ],
        )
        .await;
    checked!(script, step9);
    // The base's answer to the move: the full-inventory resync
    // (`after_commit`), which the cell-side tap never saw (README F11).
    wait_since(
        &mut script,
        move_from,
        "onUpdateItem with the pistol in the bandolier",
        STEP,
        |o| update_has(o, |i| i.id == pistol.id && i.container_id == BAG_BANDOLIER),
    )
    .await;
    let equipped = only_item(&script, |i| i.id == pistol.id);
    assert_eq!(equipped.container_id, BAG_BANDOLIER, "{equipped:?}");
    // `onUpdateItem` carries the slot 1-based, as `moveItem` does
    // (`send_update_item` adds one to the stored slot).
    assert_eq!(equipped.slot_id, BANDOLIER_SLOT_WIRE, "{equipped:?}");

    // ── 10: the server's own record ──────────────────────────────────────
    // Mission state is written as it changes (`CellToBaseMsg::MissionUpdate`
    // upserts `sgw_mission`), so no logout is needed first.
    wait_mission_status(
        &mut script,
        &pool,
        player_id,
        MISSION_ARM_YOURSELF,
        DB_COMPLETED,
    )
    .await;
    wait_mission_status(
        &mut script,
        &pool,
        player_id,
        MISSION_FROSTS_LETTER,
        DB_ACTIVE,
    )
    .await;

    tracing::info!(
        elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        logged = script.log().len(),
        "praxis start: mission 622 complete on the wire"
    );
    server.orchestrator.stop_all().await;
    cleanup_account(&pool, ACCOUNT).await;
}

/// Send one client call as its own reliable bundle.
async fn send(script: &mut ScriptSession, msg: Vec<u8>) {
    script
        .send(&[msg])
        .await
        .unwrap_or_else(|e| panic!("send: {e}"));
}

/// Pump until a log entry at or after `from` matches `pred`, without moving
/// the script's cursor. For answers whose order against the cursor's last
/// match is not fixed (two senders, the base and the cell).
async fn wait_since(
    script: &mut ScriptSession,
    from: usize,
    what: &str,
    timeout: Duration,
    mut pred: impl FnMut(&Observed) -> bool,
) -> Observed {
    let deadline = Instant::now() + timeout;
    let mut scanned = from;
    loop {
        if let Some(o) = script.log()[scanned..].iter().find(|o| pred(o)) {
            return o.clone();
        }
        scanned = script.log().len();
        let now = Instant::now();
        if now >= deadline {
            fail(script, format!("waited {timeout:?} for {what}"));
        }
        checked!(script, script.pump((deadline - now).min(PUMP_SLICE)).await);
    }
}

/// Poll `sgw_mission` every [`DB_POLL`] for up to [`STEP`] until the
/// mission's status is `want`. The script pumps between polls.
async fn wait_mission_status(
    script: &mut ScriptSession,
    pool: &PgPool,
    player_id: i32,
    mission_id: i32,
    want: i32,
) {
    let deadline = Instant::now() + STEP;
    loop {
        let status: Option<i32> = sqlx::query_scalar(
            "SELECT status FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(mission_id)
        .fetch_optional(pool)
        .await
        .unwrap_or_else(|e| panic!("read sgw_mission {mission_id}: {e}"));
        if status == Some(want) {
            return;
        }
        if Instant::now() >= deadline {
            fail(
                script,
                format!(
                    "sgw_mission {mission_id} status is {status:?} after {STEP:?}, want {want}"
                ),
            );
        }
        checked!(script, script.pump(DB_POLL).await);
    }
}

/// Fail with `why`, the raw trail (the last [`TRAIL`] messages, which
/// `ScriptTimeout` also carries) and the last [`TRAIL`] decoded events. NPC
/// avatar updates (`0x10`) can fill the whole raw trail while the script
/// waits; the decoded list still shows what the server answered.
fn fail(script: &ScriptSession, why: impl std::fmt::Display) -> ! {
    panic!(
        "{why}\nlast messages:\n{}\nlast decoded events:\n{}",
        script.trail(TRAIL),
        events_trail(script, TRAIL)
    )
}

/// The last `n` log entries that decoded to an event, one line each.
fn events_trail(script: &ScriptSession, n: usize) -> String {
    let mut lines: Vec<String> = script
        .log()
        .iter()
        .rev()
        .filter_map(|o| {
            let event = o.event.as_ref()?;
            let mut text = format!("{event:?}");
            if text.len() > 200 {
                text.truncate(text.floor_char_boundary(200));
                text.push_str("...");
            }
            Some(format!(
                "+{:.3}s entity {} {text}",
                o.at.as_secs_f64(),
                o.msg
                    .entity_id
                    .map_or_else(|| "-".to_owned(), |id| id.to_string())
            ))
        })
        .take(n)
        .collect();
    if lines.is_empty() {
        return "(none)".to_owned();
    }
    lines.reverse();
    lines.join("\n")
}

/// The one inventory item matching `pred` in the mirror.
fn only_item(script: &ScriptSession, pred: impl Fn(&InvItem) -> bool) -> InvItem {
    let found: Vec<&InvItem> = script
        .mirror
        .inventory()
        .values()
        .filter(|i| pred(i))
        .collect();
    match found.as_slice() {
        [only] => (*only).clone(),
        other => panic!("want exactly one matching inventory item, found {other:?}"),
    }
}

fn dialog(o: &Observed, dialog_id: i32) -> Option<&DialogDisplay> {
    match &o.event {
        Some(S2CEvent::DialogDisplay(d)) if d.dialog_id == dialog_id => Some(d),
        _ => None,
    }
}

fn step(o: &Observed, step_id: i32, status: i8) -> bool {
    matches!(o.event, Some(S2CEvent::StepUpdate { step_id: s, status: st }) if s == step_id && st == status)
}

fn mission(o: &Observed, mission_id: i32, status: i8) -> bool {
    matches!(o.event, Some(S2CEvent::MissionUpdate { mission_id: m, status: st, .. })
        if m == mission_id && st == status)
}

fn sequence(o: &Observed, seq_id: i32) -> bool {
    matches!(&o.event, Some(S2CEvent::Sequence(s)) if s.kismet_event_set_seq_id == seq_id)
}

fn sorted(ids: &[i32]) -> Vec<i32> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids
}

fn is_interaction(o: &Observed, type_id: u64) -> bool {
    matches!(o.event, Some(S2CEvent::InteractionType { type_id: t }) if t == type_id)
}

/// The SYSTEM line `gmGotoXYZ` answers with (`cell-console` `gm/travel.rs`).
fn is_teleport_line(o: &Observed) -> bool {
    matches!(&o.event, Some(S2CEvent::PlayerCommunication { text, .. })
        if text.starts_with("gmGotoXYZ: teleported to"))
}

fn update_has(o: &Observed, pred: impl Fn(&InvItem) -> bool) -> bool {
    matches!(&o.event, Some(S2CEvent::UpdateItem { items }) if items.iter().any(pred))
}

/// `gmGotoXYZ`'s position: three `f32` after the 4-byte entity id and the
/// `0xBD` sub-slot byte.
fn goto_pos(rec: &TapRecord) -> [f32; 3] {
    let a = rec.args();
    [f32_at(&a, 5), f32_at(&a, 9), f32_at(&a, 13)]
}

/// The region trigger's position: after the prefix, the sub-slot byte, the
/// INT32 region id and the UINT8 entering flag.
fn region_pos(rec: &TapRecord) -> [f32; 3] {
    let a = rec.args();
    [f32_at(&a, 10), f32_at(&a, 14), f32_at(&a, 18)]
}

fn f32_at(args: &[u8], at: usize) -> f32 {
    let bytes: [u8; 4] = args[at..at + 4]
        .try_into()
        .expect("the fixture record holds a full f32 here");
    f32::from_le_bytes(bytes)
}
