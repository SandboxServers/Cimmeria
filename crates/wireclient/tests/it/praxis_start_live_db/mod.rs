//! The Praxis start, mission 622 "Arm Yourself!", end to end on the wire
//! (WC-14). A wire-created Praxis Commando on a GM account plays the ten
//! client calls of the lab capture (`praxis_start_tap.json`; ADR § Praxis
//! start). After each call the test waits for the answers the WC-14 packet's
//! step table lists, plus the `InteractionType` clears, mission 1360's first
//! step and the weapon abilities the capture also shows, in the capture's
//! order; then it reads the server's own record (`sgw_mission`) as the lab
//! rows do. Not every capture answer is asserted yet: the worknote
//! (`docs/analysis/wireclient-ci/worknotes/WC-14.md`) lists the rest for
//! WC-17. Run with
//! `pwsh tools/build-lane/live-db-test.ps1 --wireclient praxis_start`.
//!
//! No entity id is a literal: Frost is found through the mirror by template
//! 14's static mesh and name id (D-WC1), and the Guard's body is whichever
//! entity the server flags as a mission object when Frost is searched,
//! cross-checked against template 21. The pistol is the item the Guard's
//! dialog grants, by design id from the seed.
//!
//! Every wait has a deadline and fails with the last 30 observed messages
//! and the last 30 decoded events (D-WC9); nothing sleeps. Waiting between
//! DB polls pumps the session, so keep-alives and ACKs keep flowing.
//!
//! Cleanup: the account and its rows are deleted at the start of the test
//! and at the end of a passing run. A panic skips the end-of-run cleanup;
//! the next run's start-of-test cleanup removes the leftovers, which is safe
//! because each running test has its own database clone (`wireclient-e2e`).

mod predicates;
mod waits;

use std::time::{Duration, Instant};

use cimmeria_wireclient::calls::CLIENT_CALL_ENTITY_ID;
use cimmeria_wireclient::script::{Observed, ScriptSession};
use cimmeria_wireclient::semantic::S2CEvent;
use cimmeria_wireclient::session::GameSession;
use cimmeria_wireclient::tap_fixture::TapCapture;

use crate::support::{
    self, cleanup_account, credentials_for, default_visual_choices,
    insert_sentinel_account_with_level, item_granted_by_dialog, start_server, template_query,
    SHARD,
};
use predicates::{
    dialog, goto_pos, interaction_on, is_interaction, is_teleport_line, knows_all, mission,
    region_id, region_pos, sequence, step, update_has,
};
use waits::{check, fail, only_item, send, wait_mission_status, wait_since};

/// Campaign sentinel block 0x7000_9C00..=0x7000_9CFF; WC-14 owns 0x..02.
const ACCOUNT: i32 = 0x7000_9C02;
const ACCOUNT_NAME: &str = "wc14_praxis";
/// `gmGotoXYZ` needs a GM account (D-WC2 (a)).
const ACCESS_GM: i32 = 2;
/// Praxis Commando, male (D-WC11).
const CHAR_DEF_PRAXIS: i32 = 3;
/// Corporal Frost's corpse (README F5).
const FROST_TEMPLATE: i32 = 14;
/// The Guard's corpse, `CA-Props.CA-GuardCorpse02`: the only template with
/// that mesh, spawned once in the Cellblock (spawnlist 15,
/// `ArmYourself_GuardBody`).
const GUARD_BODY_TEMPLATE: i32 = 21;
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
/// Mission 1360's first step, "Find a way to get Cpl. Frost's Letter to his
/// family" (fixture #29).
const STEP_FROSTS_LETTER: i32 = 4037;
const SEQ_STASIS_DOOR: i32 = 10000;
/// The Human archetype's `Item_Equip` animation (sequence 1872, event 4000,
/// `KIS-abilities_human.KIS-handling`, in event set 804 "Item handling
/// generic event set"): the equip itself plays it, not a mission effect.
const SEQ_HUMAN_ITEM_EQUIP: i32 = 1872;
/// The four CORE_TUTORIAL abilities the Guard's body teaches (chain 1005;
/// fixture #49 lists them as 597, 1218, 592, 594).
const TUTORIAL_ABILITIES: [i32; 4] = [592, 594, 597, 1218];
/// The pistol's own abilities once it is the active bandolier weapon:
/// `items_event_sets` for item 55, 579 ranged (event 7) and 708 melee
/// (event 6). Fixture #71's list is these plus the tutorial four.
const PISTOL_ABILITIES: [i32; 2] = [579, 708];
/// `INT_MissionWorldObject`: the highlight on a searchable corpse.
const INT_MISSION_WORLD_OBJECT: u64 = 0x4000_0000;
/// What a searched corpse's interaction type drops to once the content
/// chain clears its search bit (`set_interaction_type` op `~`, mask
/// 0x40_0000; fixture #21 on Frost, #44 on the body).
const INT_NONE: u64 = 0;
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

#[tokio::test]
async fn praxis_start_completes_mission_622_live_db() {
    let pool = cimmeria_test_support::require_db_or_skip!();
    support::init_test_logging();
    let started = Instant::now();
    cleanup_account(&pool, ACCOUNT).await;
    insert_sentinel_account_with_level(&pool, ACCOUNT, ACCOUNT_NAME, ACCESS_GM).await;
    let pistol_design = item_granted_by_dialog(&pool, DIALOG_GUARD).await;
    let frost_query = template_query(&pool, FROST_TEMPLATE).await;
    let body_query = template_query(&pool, GUARD_BODY_TEMPLATE).await;

    // Positions and the region id from the capture's bits, not decimals
    // (README F12).
    let cap = TapCapture::praxis_start();
    let p1 = goto_pos(cap.expect(4, "gmGotoXYZ"));
    let region_record = cap.expect(8, "triggerClientHintedGenericRegion");
    let p1_region_id = region_id(region_record);
    let p1_region = region_pos(region_record);
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
    let result = script.wait_entity(&frost_query, FROST_WAIT).await;
    let frost = check(&script, result);
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

    // ── 2: GM teleport next to Frost, then report leaving the region ─────
    send(
        &mut script,
        GameSession::gm_goto_xyz(CLIENT_CALL_ENTITY_ID, p1),
    )
    .await;
    let result = script
        .wait_event("step 2: the gmGotoXYZ line", STEP, is_teleport_line)
        .await;
    check(&script, result);
    // The client reports the region exit after it has moved (fixture #8
    // follows #5).
    send(
        &mut script,
        GameSession::trigger_client_hinted_generic_region(
            CLIENT_CALL_ENTITY_ID,
            p1_region_id,
            false,
            p1_region,
        ),
    )
    .await;

    // ── 3: search Frost ──────────────────────────────────────────────────
    // Frost is still the one entity the mirror matches: a second match that
    // arrived after `wait_entity` returned would make the id ambiguous.
    if let Err(e) = script.mirror.find_one(&frost_query) {
        fail(&script, format!("Frost after the flush: {e}"));
    }
    let frost_i32 = i32::try_from(frost).expect("entity id fits INT32");
    send(
        &mut script,
        GameSession::interact(CLIENT_CALL_ENTITY_ID, frost_i32),
    )
    .await;
    // Fixture #20-#29. Chains 1003 (letter, Guard unlock, step advance) and
    // 1121 (accept 1360) both trigger on dialog_open 3995 at priority 0, and
    // run in chain-id order: `engine_loader.rs` loads chains ORDER BY
    // chain_id and `content-engine` `chain/mod.rs` sorts each trigger bucket
    // by priority with a stable sort. So 1003's "80623 started" comes before
    // 1121's "1360 accepted"; a priority change on either chain flips it.
    let result = script
        .expect_in_order(
            "step 3: search Frost",
            STEP,
            &mut [
                ("onDialogDisplay 3995 on Frost", &mut |o: &Observed| {
                    dialog(o, DIALOG_FROST).is_some_and(|d| d.entity_id == frost_i32)
                }),
                (
                    "InteractionType 0 on Frost (search bit cleared)",
                    &mut |o: &Observed| interaction_on(o, frost, INT_NONE),
                ),
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
                ("step 4037 started", &mut |o: &Observed| {
                    step(o, STEP_FROSTS_LETTER, STARTED)
                }),
            ],
        )
        .await;
    let seen = check(&script, result);
    let Some(body) = seen[2].msg.entity_id else {
        fail(
            &script,
            "the mission-object InteractionType has no entity id",
        );
    };
    // The flagged entity is the Guard's corpse, not some other prop that
    // happens to carry the highlight.
    let result = script.wait_entity(&body_query, STEP).await;
    let guard_body = check(&script, result);
    if body != guard_body {
        fail(
            &script,
            format!("the highlighted entity {body} is not template 21's entity {guard_body}"),
        );
    }
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
    let result = script
        .wait_event("step 5: the gmGotoXYZ line", STEP, is_teleport_line)
        .await;
    check(&script, result);

    // ── 6: search the Guard's body ───────────────────────────────────────
    let grant_from = script.log().len();
    send(
        &mut script,
        GameSession::interact(CLIENT_CALL_ENTITY_ID, body_i32),
    )
    .await;
    // Fixture #43-#54.
    let result = script
        .expect_in_order(
            "step 6: search the Guard's body",
            STEP,
            &mut [
                ("onDialogDisplay 3996 on the body", &mut |o: &Observed| {
                    dialog(o, DIALOG_GUARD).is_some_and(|d| d.entity_id == body_i32)
                }),
                (
                    "InteractionType 0 on the body (search bit cleared)",
                    &mut |o: &Observed| interaction_on(o, body, INT_NONE),
                ),
                ("step 80623 done", &mut |o: &Observed| {
                    step(o, STEP_SEARCH_GUARD, DONE)
                }),
                ("step 80622 started", &mut |o: &Observed| {
                    step(o, STEP_EQUIP_PISTOL, STARTED)
                }),
                (
                    "onKnownAbilitiesUpdate with the tutorial four",
                    // Containment, not equality: the list is the player's
                    // whole known set (in hash-set order), and a future
                    // `char_creation_abilities` row for char def 3 would add
                    // to it.
                    &mut |o: &Observed| knows_all(o, &TUTORIAL_ABILITIES),
                ),
                ("onDialogDisplay 5882 on the player", &mut |o: &Observed| {
                    dialog(o, DIALOG_WEAPON_TUTORIAL)
                        .is_some_and(|d| i64::from(d.entity_id) == i64::from(own))
                }),
            ],
        )
        .await;
    check(&script, result);

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
    if pistol.container_id == BAG_BANDOLIER {
        fail(
            &script,
            format!("the pistol must start outside the bandolier: {pistol:?}"),
        );
    }

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
    // Pin the mission's completion to the move: nothing so far may have
    // opened the door or completed 622 (a regression completing it on a
    // dialog close would otherwise pass the ordered wait below). The
    // script's cursor cannot be moved to the send (ScriptSession has no
    // setter), so this check is what ties the answers to the move.
    if let Some(early) = script
        .log()
        .iter()
        .find(|o| sequence(o, SEQ_STASIS_DOOR) || mission(o, MISSION_ARM_YOURSELF, DONE))
    {
        fail(
            &script,
            format!("mission 622's completion arrived before the move: {early:?}"),
        );
    }
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
    // Fixture #66-#72.
    let result = script
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
                (
                    "onKnownAbilitiesUpdate with the pistol's two and the tutorial four",
                    &mut |o: &Observed| {
                        knows_all(o, &PISTOL_ABILITIES) && knows_all(o, &TUTORIAL_ABILITIES)
                    },
                ),
                ("onSequence 1872 (Item_Equip)", &mut |o: &Observed| {
                    sequence(o, SEQ_HUMAN_ITEM_EQUIP)
                }),
            ],
        )
        .await;
    check(&script, result);
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
    // `onUpdateItem` carries the slot 1-based, as `moveItem` does
    // (`send_update_item` adds one to the stored slot).
    if equipped.container_id != BAG_BANDOLIER || equipped.slot_id != BANDOLIER_SLOT_WIRE {
        fail(
            &script,
            format!("want the pistol in container 3, wire slot 1: {equipped:?}"),
        );
    }

    // ── 10: the server's own record ──────────────────────────────────────
    // Mission state is written as it changes (`CellToBaseMsg::MissionUpdate`
    // upserts `sgw_mission`), so no logout is needed first.
    wait_mission_status(
        &mut script,
        &pool,
        player_id,
        MISSION_ARM_YOURSELF,
        DB_COMPLETED,
        STEP,
    )
    .await;
    wait_mission_status(
        &mut script,
        &pool,
        player_id,
        MISSION_FROSTS_LETTER,
        DB_ACTIVE,
        STEP,
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
