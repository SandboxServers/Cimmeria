//! DU-07 — Col. Marsh companion barks, chains 1176-1178 in
//! `castle_cellblock_chains.sql`.
//!
//! Three `npc_bark` rows speak dialog 5019's screens 96351, 96352 and
//! 96354 into the escorting player's chat window at three points on the
//! Castle_CellBlock escape route. Dialog 5019 itself must never be
//! displayed (its last three screens are excluded "Future Self"
//! content), so these lines have no other delivery route.
//!
//! What each half of this file is actually guarding:
//!
//! - The **positive** cases pin that each seed row survives the loader
//!   and resolves to an `Action::NpcBark` carrying the right `screen_id`.
//!   A row that names the wrong screen still resolves, so the screen id
//!   is asserted rather than the action kind.
//! - The **negative** cases are the load-bearing half. The engine has no
//!   fire-once primitive — `content_triggers.once` is read out of the DB
//!   and never consulted again (`loader/mod.rs`, `engine_loader.rs`) —
//!   so "at most once per mission run" rests entirely on each chain's
//!   mission/step gate. Each chain therefore gets the adjacent
//!   wrong-state case its own gate is supposed to refuse: the phase not
//!   yet reached, and the phase already passed.
//! - [`player_loaded_into_castle_cellblock_resolves_no_bark`] fires the
//!   whole seeded engine, not one chain, so it would also catch a bark
//!   row added to somebody else's `player_loaded` restore chain later.
//! - [`chain_1176_executes_the_seeded_line_onto_the_wire`] is the only
//!   test here that runs the executor. Resolve-only cannot tell a wired
//!   `Action::NpcBark` arm from the `other =>` catch-all, and it cannot
//!   see the text at all, because the line is resolved server-side from
//!   the `dialog_screens` cache and never appears in the seed row.
//!
//! Chain-level guards live here rather than in [`super::npc_bark`],
//! which owns the verb itself against a sentinel chain.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::{build_engine, load_single_chain_for_test};
use super::super::executor::execute_actions;
use super::assert_no_deferred_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

/// Escort start, on the ring ride to the topside route.
const CHAIN_MOVE_OUT: i32 = 1176;
/// Mess Hall threshold (`Castle_Cellblock.Region3`).
const CHAIN_MESS_HALL: i32 = 1177;
/// Hallway05 threshold (`Castle_Cellblock.Region5`).
const CHAIN_HALLWAY05: i32 = 1178;

/// Dialog 5019's screens, in `db/resources/Dialogs/Seed/dialog_screens.sql`.
const SCREEN_MOVE_OUT: i32 = 96351;
const SCREEN_MESS_HALL: i32 = 96352;
/// Authored by no chain: "Crouch down when you're in cover!" has no
/// reachable trigger (no placed cover set exists outside the tutorial
/// med station). Asserted absent by
/// [`no_chain_speaks_the_unauthored_cover_line`].
const SCREEN_COVER: i32 = 96353;
const SCREEN_HALLWAY05: i32 = 96354;
/// The excluded "Future Self" screens. No chain may ever name one.
const SCREEN_FUTURE_SELF: [i32; 3] = [96355, 96356, 96357];

/// The seeded text of screen 96351, asserted rather than assumed: the
/// seed row carries only the id, so an edit to that `dialog_screens` row
/// silently changes what a player hears.
const MOVE_OUT_TEXT: &str = "Let's move out!";
/// `speakers.speaker_id 261`, the spelling dialog 2309 ships on its Marsh
/// lines in this same phase.
const MARSH_SPEAKER: &str = "Col. Marsh";
/// `CHAN_say`. The loader accepts no other channel.
const CHAN_SAY: u8 = 0;

/// `onPlayerCommunication`. Spelled out rather than imported from
/// `method_idx` so a change to the constant cannot make the assertion
/// agree with itself.
const ON_PLAYER_COMMUNICATION: u16 = 28;

const PLAYER_EID: u32 = 7601;
const PLAYER_ID: i32 = 42;

/// Region keys, byte-identical to `point_sets.sql`. Note the lowercase
/// `b` — this file also contains `Castle_CellBlock.Region8` with a
/// capital one, and trigger matching is an exact string compare.
const REGION_MESS_HALL: &str = "Castle_Cellblock.Region3";
const REGION_HALLWAY05: &str = "Castle_Cellblock.Region5";

async fn load(pool: &PgPool, chain_id: i32) -> cimmeria_content_engine::chain::Chain {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains and assemble \
                 successfully — a None here means the trigger or action row was \
                 rejected at load (check the npc_bark params)"
            )
        })
}

fn engine_with(chain: cimmeria_content_engine::chain::Chain) -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Resolve a `teleport_in` arrival with the given mission-step context.
fn resolve_teleport_in(
    engine: &ChainEngine,
    region_id: i32,
    params: &[(&str, &str)],
) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(region_id));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), serde_json::json!(*v));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Resolve a region crossing with the given mission-status context.
fn resolve_region_enter(
    engine: &ChainEngine,
    region_key: &str,
    params: &[(&str, &str)],
) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), serde_json::json!(*v));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Every `NpcBark` screen id the given chain resolved.
fn bark_screens(resolved: &ResolvedActions, chain_id: i64) -> Vec<i32> {
    resolved
        .actions
        .iter()
        .filter_map(|(id, action)| match action {
            Action::NpcBark { screen_id, .. } if *id == chain_id => Some(*screen_id),
            _ => None,
        })
        .collect()
}

/// Assert the chain resolved exactly one bark, with the right screen,
/// speaker, channel and no delay.
fn assert_single_bark(resolved: &ResolvedActions, chain_id: i64, screen_id: i32) {
    let barks: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(id, action)| *id == chain_id && matches!(action, Action::NpcBark { .. }))
        .map(|(_, action)| action)
        .collect();
    assert_eq!(
        barks.len(),
        1,
        "chain {chain_id} must resolve exactly one NpcBark; got {}. Zero \
         means either the gate refused the happy path or the loader dropped \
         the row (a bad `speaker`/`channel` param is rejected, not defaulted \
         through). Resolved: {:?}",
        barks.len(),
        resolved.actions,
    );
    match barks[0] {
        Action::NpcBark {
            screen_id: got,
            speaker,
            channel,
        } => {
            assert_eq!(
                *got, screen_id,
                "chain {chain_id} must speak dialog 5019 screen {screen_id}, not {got}"
            );
            assert_eq!(
                speaker, MARSH_SPEAKER,
                "chain {chain_id}'s chat prefix must be the speaker-261 spelling \
                 dialog 2309 ships in this same phase"
            );
            assert_eq!(
                *channel, CHAN_SAY,
                "chain {chain_id} must ride CHAN_say; the loader accepts no other \
                 channel, so anything else means the row was rewritten"
            );
        }
        other => panic!("filtered for NpcBark but got {other:?}"),
    }
    assert_no_deferred_actions(resolved, chain_id);
}

/// Assert the chain resolved nothing at all.
fn assert_refused(resolved: &ResolvedActions, chain_id: i64, why: &str) {
    let actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, a)| a)
        .collect();
    assert!(
        actions.is_empty(),
        "chain {chain_id} must NOT resolve when {why} — the engine has no \
         fire-once primitive, so this gate is the only thing stopping the \
         line repeating. Got {actions:?}",
    );
}

// ── Chain 1176: "Let's move out!" on the ring ride ─────────────────────

/// Happy path: ringing up to the topside route while mission 680 is still
/// on step 2344 speaks the departure line.
#[tokio::test]
async fn chain_1176_ring_ride_while_step_2344_active_barks_the_departure_line() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(&engine, 3, &[("mission_680_step_2344_status", "active")]);
    assert_single_bark(&resolved, CHAIN_MOVE_OUT as i64, SCREEN_MOVE_OUT);
}

/// Adjacent wrong state — phase already passed. Step 2344 retires into
/// `completed_steps` the moment chain 1072 advances 680 to 2345 on this
/// same edge, so a second ring ride (die topside, respawn at a
/// Preparation-room respawner, walk back to ring switch 2) must not
/// re-speak the line.
///
/// This is the case that forced the gate off `mission_status 680 eq
/// active`: mission 680 stays active all the way to Region9, so a mission
/// gate would still be open on that second ride.
#[tokio::test]
async fn chain_1176_does_not_re_bark_on_a_second_ring_ride() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(
        &engine,
        3,
        &[
            ("mission_680_status", "active"),
            ("mission_680_step_2344_status", "completed"),
            ("mission_680_step_2345_status", "active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_MOVE_OUT as i64,
        "step 2344 has already been advanced past (mission 680 is still \
         active, which is exactly why a mission-status gate would not have \
         closed here)",
    );
}

/// Adjacent wrong state — wrong ring. Ring 2 is the downstairs hop chain
/// 1044 owns; only ring 3 is the topside arrival.
#[tokio::test]
async fn chain_1176_does_not_fire_on_the_other_ring_transport() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(&engine, 2, &[("mission_680_step_2344_status", "active")]);
    assert_refused(
        &resolved,
        CHAIN_MOVE_OUT as i64,
        "the player teleported into ring region 2, not the topside ring 3",
    );
}

// ── Chain 1177: the Mess Hall long-table cue ───────────────────────────

/// Happy path: walking into the Mess Hall with 681 accepted and the room
/// not yet cleared speaks the long-table flank cue.
#[tokio::test]
async fn chain_1177_mess_hall_entry_barks_the_long_table_flank_cue() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MESS_HALL).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_MESS_HALL,
        &[
            ("mission_681_status", "active"),
            ("mission_682_status", "not_active"),
        ],
    );
    assert_single_bark(&resolved, CHAIN_MESS_HALL as i64, SCREEN_MESS_HALL);
}

/// Adjacent wrong state — phase not yet reached. Mission 681 is accepted
/// by chain 1073 on the Region9 crossing, which is on the way in; a
/// player who somehow reaches Region3 before that must not hear a cue
/// about an objective they do not have.
#[tokio::test]
async fn chain_1177_does_not_fire_before_mission_681_is_accepted() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MESS_HALL).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_MESS_HALL,
        &[
            ("mission_680_status", "active"),
            ("mission_681_status", "not_active"),
            ("mission_682_status", "not_active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_MESS_HALL as i64,
        "mission 681 has not been accepted yet",
    );
}

/// Adjacent wrong state — already fired, phase passed. Chain 1087
/// completes 681 and accepts 682 on the mess-hall kill counter, closing
/// both arms of this gate. Re-entering the cleared room must be silent.
///
/// This is also the state the H52 step-activation replay re-evaluates the
/// chain against: `accept_mission 682` re-fires `enter_region` for every
/// region the player is standing in, which includes Region3. If this gate
/// did not close, the player would hear the line twice on one crossing.
#[tokio::test]
async fn chain_1177_does_not_re_bark_once_the_mess_hall_is_cleared() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MESS_HALL).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_MESS_HALL,
        &[
            ("mission_681_status", "completed"),
            ("mission_682_status", "active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_MESS_HALL as i64,
        "the Mess Hall guards are dead — 681 is completed and 682 accepted \
         (this is also the post-mutation state the H52 region replay \
         re-evaluates the chain against on the very same crossing)",
    );
}

// ── Chain 1178: the Hallway05 cue ──────────────────────────────────────

/// Happy path: crossing into Hallway05 with 685 cleared and 686 not yet
/// accepted speaks the second flank cue.
#[tokio::test]
async fn chain_1178_hallway05_entry_barks_the_second_flank_cue() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_685_status", "completed"),
            ("mission_686_status", "not_active"),
        ],
    );
    assert_single_bark(&resolved, CHAIN_HALLWAY05 as i64, SCREEN_HALLWAY05);
}

/// Adjacent wrong state — phase not yet reached. Hallway04 (mission 685)
/// is still being fought.
#[tokio::test]
async fn chain_1178_does_not_fire_before_hallway04_is_cleared() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_685_status", "active"),
            ("mission_686_status", "not_active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_HALLWAY05 as i64,
        "mission 685 (Hallway04) is not yet complete",
    );
}

/// Adjacent wrong state — already fired. Chain 1083 accepts 686 on this
/// same crossing, so every later crossing, and the H52 replay that
/// `accept_mission 686` kicks off, must find the gate shut.
#[tokio::test]
async fn chain_1178_does_not_re_bark_once_686_is_accepted() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_685_status", "completed"),
            ("mission_686_status", "active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_HALLWAY05 as i64,
        "mission 686 was accepted by chain 1083 on this same crossing",
    );
}

/// The two bark chains that share a region key with a mission chain must
/// share its gate exactly. If chain 1083 is ever loosened, 1178 starts
/// repeating — the seed says so in a comment; this says so in a test.
#[tokio::test]
async fn chain_1178_is_gated_identically_to_the_mission_accept_it_rides() {
    let pool = require_db_or_skip!();
    let engine = {
        let mut e = ChainEngine::new();
        e.register_chain(load(&pool, 1083).await);
        e.register_chain(load(&pool, CHAIN_HALLWAY05).await);
        e
    };

    // Every context in which 1083 accepts 686 must also bark, and vice
    // versa. Walk the three states that matter.
    for (params, both_fire, what) in [
        (
            vec![
                ("mission_685_status", "completed"),
                ("mission_686_status", "not_active"),
            ],
            true,
            "Hallway04 cleared, 686 not yet accepted",
        ),
        (
            vec![
                ("mission_685_status", "active"),
                ("mission_686_status", "not_active"),
            ],
            false,
            "Hallway04 still contested",
        ),
        (
            vec![
                ("mission_685_status", "completed"),
                ("mission_686_status", "active"),
            ],
            false,
            "686 already accepted",
        ),
    ] {
        let resolved = resolve_region_enter(&engine, REGION_HALLWAY05, &params);
        let accepts = resolved
            .actions
            .iter()
            .filter(|(id, action)| {
                *id == 1083 && matches!(action, Action::AcceptMission { mission_id: 686 })
            })
            .count();
        let barks = bark_screens(&resolved, CHAIN_HALLWAY05 as i64).len();
        assert_eq!(
            (accepts > 0, barks > 0),
            (both_fire, both_fire),
            "with {what}: chain 1083 accept and chain 1178 bark must agree \
             (accepts={accepts}, barks={barks}). They are co-gated by \
             duplicated conditions, so a divergence here means somebody \
             edited one gate and not the other"
        );
    }
}

// ── Relog: no bark is ever replayed ────────────────────────────────────

/// A `player_loaded` into Castle_CellBlock resolves no bark, in any
/// escort state.
///
/// Loaded through [`build_engine`] rather than one chain, so this also
/// fails if a bark row is ever added to one of the zone's `player_loaded`
/// restore chains (1045/1046/1062-1065/1104/1110/1111/1162). Barks are
/// neither persisted nor replayed by design: the escort lines are moment
/// cues, and a player who relogs mid-route should not be greeted by three
/// of them at once.
///
/// This holds structurally as well as by gate. Nothing on the login path
/// re-fires a region or teleport edge: `fire_player_loaded` raises only
/// `PlayerLoaded`, `fire_enter_region` has a single non-test caller (the
/// client's `triggerClientHintedGenericRegion`), and the H52 replay is
/// reachable only from the executor's accept/advance arms and the two GM
/// mission commands — never from mission restore. The assertion below is
/// the seed-side half of that.
#[tokio::test]
async fn player_loaded_into_castle_cellblock_resolves_no_bark() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // Three points on the escort route, each the state a player could
    // plausibly log out in.
    for (what, params) in [
        (
            "mid-ring-ride (680 on step 2344)",
            vec![
                ("mission_680_status", "active"),
                ("mission_680_step_2344_status", "active"),
            ],
        ),
        (
            "in the Mess Hall (681 active)",
            vec![
                ("mission_681_status", "active"),
                ("mission_682_status", "not_active"),
            ],
        ),
        (
            "at Hallway05 (685 cleared, 686 not accepted)",
            vec![
                ("mission_685_status", "completed"),
                ("mission_686_status", "not_active"),
            ],
        ),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        for (k, v) in &params {
            ctx.set_param((*k).to_string(), serde_json::json!(*v));
        }
        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);
        let barks: Vec<&Action> = resolved
            .actions
            .iter()
            .filter(|(_, a)| matches!(a, Action::NpcBark { .. }))
            .map(|(_, a)| a)
            .collect();
        assert!(
            barks.is_empty(),
            "a player_loaded into Castle_CellBlock {what} must resolve no \
             npc_bark at all — barks are moment cues and are deliberately \
             not replayed on relog. Got {barks:?}",
        );
    }
}

// ── Seed-shape guards for the screens that must NOT ship ───────────────

/// No chain anywhere speaks 5019's unauthored or excluded screens.
///
/// 96353 ("Crouch down when you're in cover!") has no reachable trigger:
/// the only "player is in cover" event needs a placed cover set, and the
/// only world-space-correct set in the game is the tutorial med station.
/// 96355-96357 are the excluded "Future Self" content. If a later packet
/// hangs either on an unrelated event, this fails.
#[tokio::test]
async fn no_chain_speaks_the_unauthored_cover_line() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT chain_id, (params->>'screen_id')::int \
         FROM resources.content_actions \
         WHERE action_type = 'npc_bark' AND params->>'screen_id' IS NOT NULL",
    )
    .fetch_all(&pool)
    .await
    .expect("npc_bark row scan must succeed");

    let mut banned: Vec<i32> = vec![SCREEN_COVER];
    banned.extend_from_slice(&SCREEN_FUTURE_SELF);

    for (chain_id, screen_id) in &rows {
        assert!(
            !banned.contains(screen_id),
            "chain {chain_id} speaks dialog 5019 screen {screen_id}, which is \
             deliberately unauthored. 96353 needs a placed Mess Hall cover set \
             before it has an honest trigger; 96355-96357 are the excluded \
             Future Self content the whole dialog was withheld for"
        );
    }

    // And the three that DO ship are all present, so a silent deletion of
    // a seed row is not mistaken for a clean run.
    let shipped: Vec<i32> = rows.iter().map(|(_, s)| *s).collect();
    for want in [SCREEN_MOVE_OUT, SCREEN_MESS_HALL, SCREEN_HALLWAY05] {
        assert!(
            shipped.contains(&want),
            "dialog 5019 screen {want} has no npc_bark row; DU-07's three \
             chains 1176-1178 are the only ones that should ship it. Found \
             {shipped:?}"
        );
    }
}

/// Every seeded `npc_bark` names a `dialog_screens` row that exists and
/// carries a non-blank speaker.
///
/// A bark that names a missing screen is not a load error — the loader
/// accepts any integer, and the executor refuses at runtime with
/// `reason = screen_not_cached` and speaks nothing. That failure is
/// invisible in play: the chain fires, the line does not. This is the
/// only place a typo'd `screen_id` is caught before UAT.
#[tokio::test]
async fn every_seeded_bark_names_a_real_screen_and_a_named_speaker() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, serde_json::Value)> = sqlx::query_as(
        "SELECT chain_id, params FROM resources.content_actions \
         WHERE action_type = 'npc_bark'",
    )
    .fetch_all(&pool)
    .await
    .expect("npc_bark row scan must succeed");

    assert!(
        !rows.is_empty(),
        "no npc_bark rows found at all — DU-07 seeds three, so an empty \
         result means the seed did not load and every assertion below would \
         pass vacuously"
    );

    for (chain_id, params) in rows {
        let screen_id = params
            .get("screen_id")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_else(|| {
                panic!("chain {chain_id}: npc_bark params have no integer screen_id: {params}")
            }) as i32;

        let text: Option<(String,)> =
            sqlx::query_as("SELECT text FROM resources.dialog_screens WHERE screen_id = $1")
                .bind(screen_id)
                .fetch_optional(&pool)
                .await
                .expect("dialog_screens lookup must succeed");

        let (text,) = text.unwrap_or_else(|| {
            panic!(
                "chain {chain_id}: npc_bark names screen_id {screen_id}, which has \
                 no resources.dialog_screens row. At runtime the executor would \
                 warn `screen_not_cached` and speak nothing — the chain fires and \
                 the player hears silence"
            )
        });
        assert!(
            !text.trim().is_empty(),
            "chain {chain_id}: screen {screen_id} exists but its text is blank; \
             the executor refuses with `empty_text` rather than drawing a speaker \
             prefix with no line"
        );

        let speaker = params
            .get("speaker")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| {
                panic!("chain {chain_id}: npc_bark params have no string speaker: {params}")
            });
        assert!(
            !speaker.trim().is_empty(),
            "chain {chain_id}: npc_bark speaker is blank; the chat window would \
             render the client's empty-name prefix"
        );
    }
}

// ── Executor: the seeded line actually reaches the wire ────────────────

/// Chain 1176 end to end: resolve the real seed row, run it through
/// [`execute_actions`], and assert the emitted method-28 call carries
/// Marsh's actual 2009 line, resolved from `resources.dialog_screens`.
///
/// Three separate things fail this and nothing else catches them:
/// the executor losing its `Action::NpcBark` arm (zero calls, the action
/// falls into the `other =>` catch-all), the `dialog_screen_text` startup
/// cache going away (zero calls, `screen_not_cached`), and the seed row
/// naming a screen whose text has changed (wrong bytes on the wire).
#[tokio::test]
async fn chain_1176_executes_the_seeded_line_onto_the_wire() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(&engine, 3, &[("mission_680_step_2344_status", "active")]);
    assert_single_bark(&resolved, CHAIN_MOVE_OUT as i64, SCREEN_MOVE_OUT);

    let mut mgr = make_space_mgr();
    stage_player(&mut mgr, PLAYER_EID);
    // The real startup loader against the real seed — what a running cell
    // has in the cache.
    mgr.dialog_screen_text = spawner::load_dialog_screen_text(&pool)
        .await
        .expect("dialog screen text must load");

    let (tx, mut rx) = mpsc::channel(32);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let mut calls = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            calls.push((entity_id, method_index, args));
        }
    }
    assert_eq!(
        calls.len(),
        1,
        "chain 1176 must emit exactly one EntityMethodCall; got {:?}. Zero \
         means the executor has no Action::NpcBark arm, or the screen-text \
         cache did not load",
        calls.iter().map(|(e, m, _)| (*e, *m)).collect::<Vec<_>>(),
    );
    let (target, method_index, args) = &calls[0];
    assert_eq!(
        *target, PLAYER_EID,
        "the bark is addressed to the ringing player"
    );
    assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);

    let (speaker, speaker_units) = read_wstring(args, 0);
    assert_eq!(speaker, MARSH_SPEAKER, "speaker comes from the seed param");
    let flags_off = 4 + speaker_units * 2;
    assert_eq!(args[flags_off], 0, "SpeakerFlags must be SPEAKER_None");
    assert_eq!(args[flags_off + 1], CHAN_SAY, "Channel must be CHAN_say");

    let (text, _) = read_wstring(args, flags_off + 2);
    assert_eq!(
        text, MOVE_OUT_TEXT,
        "the line on the wire must be the seeded dialog 5019 screen \
         {SCREEN_MOVE_OUT} text, resolved server-side — the seed row carries \
         only the id, so this is the only assertion that pins what the player \
         actually hears"
    );
}

fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

fn stage_player(mgr: &mut SpaceManager, eid: u32) {
    mgr.create_entity(
        eid,
        "Castle_CellBlock",
        [-91.689, 45.188, -161.533],
        [0.0; 3],
    )
    .expect("Castle_CellBlock startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(eid)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(eid);
}

/// Read a WSTRING at `offset`: a `u32` UTF-16 code-unit count followed by
/// that many little-endian `u16`s. Returns the decoded string and the
/// **code-unit count** (not the byte length), so the caller keeps doing
/// its offset arithmetic in the wire's own terms.
fn read_wstring(args: &[u8], offset: usize) -> (String, usize) {
    let units = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    let (pairs, _) = args[offset + 4..offset + 4 + units * 2].as_chunks::<2>();
    let s = char::decode_utf16(pairs.iter().copied().map(u16::from_le_bytes))
        .map(|r| r.expect("wire text must be valid UTF-16"))
        .collect();
    (s, units)
}
