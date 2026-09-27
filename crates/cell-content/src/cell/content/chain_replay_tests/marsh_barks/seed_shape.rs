//! Guards that are about the seed as a whole rather than one chain.
//!
//! Two concerns live here:
//!
//! - **Relog.** [`player_loaded_into_castle_cellblock_resolves_no_bark`]
//!   fires the whole seeded engine, not one chain, so it also catches a
//!   bark row added to somebody else's `player_loaded` restore chain.
//! - **Seed shape.** The remaining two scan every `npc_bark` row in the
//!   database: that the screens DU-07 deliberately withheld stay
//!   withheld, and that every row that does ship names a real
//!   `dialog_screens` line and a named speaker.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::super::engine_loader::build_engine;
use super::{SCREEN_HALLWAY05, SCREEN_MESS_HALL, SCREEN_MOVE_OUT};
use crate::test_support::require_db_or_skip;

/// Authored by no chain: "Crouch down when you're in cover!" has no
/// reachable trigger, because no cover set is placed in world space
/// outside C05's hand-seeded tutorial med station.
const SCREEN_COVER: i32 = 96353;
/// The excluded "Future Self" screens. No chain may ever name one.
const SCREEN_FUTURE_SELF: [i32; 3] = [96355, 96356, 96357];

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
