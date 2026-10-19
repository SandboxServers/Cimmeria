//! UO-13: executed bind round trips (F3).
//!
//! UO-04..09 are resolve-only: they prove which actions a chain resolves,
//! not what the executor leaves behind. `add_dialog_set` pushes onto
//! `available_interactions[slot]` with no dedupe (F3), so only the executed
//! list shows a relog or a second ring beam stacking a second bind on one
//! NPC. These tests resolve with the **full** engine (`build_engine`, every
//! seeded chain), then run the resolved `AddDialogSet` / `RemoveDialogSet`
//! actions through `execute_actions` against a staged `Omega_Site_CmdCenter`
//! and read the player's `available_interactions`.
//!
//! Only the dialog-set actions run. `complete_mission`, `accept_mission`,
//! `grant_stargate_address` and `advance_step` need base and mission state
//! this harness lacks, so [`dialog_set_actions_only`] drops them (with their
//! `action_delays` entries). The state they would change is set by hand
//! between steps, exactly as the runtime would have left it.
//!
//! The harness is copied from `debug_hub.rs::staged_space`; the cache is
//! loaded with `spawner::load_dialog_set_maps`, as
//! `crates/cell/src/cell/service/startup.rs` does.

use std::collections::BTreeMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_player_loaded, fire_teleport_in,
    with_mission, with_objective, with_step, ARCH_HUMAN, ARCH_JAFFA, CMD_NAME, DLG_BRATAC,
    DLG_JACKSON, DLG_MODI, HUB_NAME, M_1423, M_1500, M_1655, OMEGA_CHAIN_RANGE, O_5645,
    REGION_CMD_PAD, S_4940, S_4942, WORLD_CMD, WORLD_HUB,
};
use crate::cell::content::build_engine;
use crate::cell::content::executor::execute_actions;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7413;
const PLAYER_ID: i32 = 43;

/// Interaction flags of the delivery ("!") and re-offer ("?") maps, spelled
/// out so a seed edit cannot make an assertion agree with itself.
const FLAG_DELIVERY: i64 = 16_777_216;
const FLAG_REOFFER: i64 = 8_388_608;

/// One `available_interactions` entry: `(dialog_set_map_id, dialog_id, flags)`.
type Entry = (i32, Option<i32>, i64);

/// Omega's four slots (templates 63800-63803).
const OMEGA_SLOTS: std::ops::RangeInclusive<i32> = 63800..=63803;

/// A Command Center space with the player at respawner 26 and the
/// dialog-set cache loaded from the real seed.
async fn staged_space(pool: &PgPool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Omega_Site_CmdCenter" Instanced="false" MinX="-1000" MaxX="1000" MinY="-1000" MaxY="1000" /></Spaces>"#;
    let cxml =
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Omega_Site_CmdCenter" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(
        PLAYER_EID,
        "Omega_Site_CmdCenter",
        [10.0, 35.23, 9.987],
        [0.0; 3],
    )
    .expect("the player must stage");
    let p = mgr.get_entity_mut(PLAYER_EID).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER_EID);
    mgr.dialog_set_maps = spawner::load_dialog_set_maps(pool)
        .await
        .expect("dialog set maps must load");
    mgr
}

/// Keep only the `AddDialogSet` / `RemoveDialogSet` actions of `resolved`,
/// with their `action_delays` entries (a missing entry means 0 ms).
fn dialog_set_actions_only(resolved: ResolvedActions) -> ResolvedActions {
    let ResolvedActions {
        actions,
        action_delays,
        params,
    } = resolved;
    let mut out = ResolvedActions {
        params,
        ..ResolvedActions::default()
    };
    for (i, (chain, action)) in actions.into_iter().enumerate() {
        if matches!(
            action,
            Action::AddDialogSet { .. } | Action::RemoveDialogSet { .. }
        ) {
            out.actions.push((chain, action));
            out.action_delays
                .push(action_delays.get(i).copied().unwrap_or(0));
        }
    }
    out
}

/// Execute the dialog-set actions of `resolved` for the staged player.
async fn apply(resolved: ResolvedActions, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(64);
    execute_actions(
        dialog_set_actions_only(resolved),
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
}

/// The player's entries on Omega's four slots. A slot the executor emptied
/// is absent, so "absent" and "never bound" read the same.
fn omega_slots(mgr: &mut SpaceManager) -> BTreeMap<i32, Vec<Entry>> {
    mgr.get_entity_mut(PLAYER_EID)
        .unwrap()
        .available_interactions
        .iter()
        .filter(|(slot, _)| OMEGA_SLOTS.contains(slot))
        .map(|(slot, v)| (*slot, v.clone()))
        .collect()
}

/// `{slot: [entry]}` for the one-entry expectation.
fn one(slot: i32, entry: Entry) -> BTreeMap<i32, Vec<Entry>> {
    BTreeMap::from([(slot, vec![entry])])
}

/// Replay the `mission_abandoned` dispatch. The abandoned mission already
/// reads `not_active` in `ctx`, as `fire_mission_abandoned` builds it (F22).
/// `UO-09`'s own copy in `reoffer.rs` is private, so this is a second one.
pub(super) fn fire_mission_abandoned(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    mission_id: i32,
) -> ResolvedActions {
    let mut out = ExecutionContext::new();
    out.world_id = ctx.world_id;
    out.params = ctx.params.clone();
    out.set_param("mission_id".to_string(), serde_json::json!(mission_id));
    let event = TriggerEvent {
        trigger_type: TriggerType::MissionAbandoned,
        source_entity: None,
        target_entity: None,
        params: out.params.clone(),
    };
    engine.resolve_event(&event, &out)
}

/// In the Command Center with 1655 active at `step` and `offer` not held.
fn delivering(archetype: i64, step: i32, offer: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(WORLD_CMD)), M_1655, "active");
    let ctx = with_step(ctx, M_1655, step, "active");
    with_mission(ctx, offer, "not_active")
}

/// 1655 completed (its S4942 key still `active`), `offer` not held: the
/// state after an abandon of 1423 or 1500. Built inline, not from UO-09.
fn reoffering(archetype: i64, world: i32, offer: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(world)), M_1655, "completed");
    let ctx = with_step(ctx, M_1655, S_4942, "active");
    let ctx = with_objective(ctx, M_1655, O_5645, "completed");
    with_mission(ctx, offer, "not_active")
}

/// One beam at S4940, then a relog and a second beam at S4942, asserting
/// `expected` on Omega's slots after every step. Returns the stage so the
/// caller can click the Accept.
async fn ring_round_trip(
    pool: &PgPool,
    archetype: i64,
    offer: i32,
    expected: &BTreeMap<i32, Vec<Entry>>,
) -> SpaceManager {
    let engine = build_engine(Some(pool)).await;
    let mut mgr = staged_space(pool).await;

    let beam = delivering(archetype, S_4940, offer);
    apply(fire_teleport_in(&engine, &beam, REGION_CMD_PAD), &mut mgr).await;
    assert_eq!(&omega_slots(&mut mgr), expected, "after the first beam");

    // 63801 has advanced the step by now; the load sees S4942.
    let relog = delivering(archetype, S_4942, offer);
    apply(fire_player_loaded(&engine, &relog, CMD_NAME), &mut mgr).await;
    assert_eq!(&omega_slots(&mut mgr), expected, "after a relog");
    apply(fire_player_loaded(&engine, &relog, CMD_NAME), &mut mgr).await;
    assert_eq!(&omega_slots(&mut mgr), expected, "after a second load");
    mgr
}

/// **A ring round trip leaves exactly one Jackson entry (F3).** The bind
/// chains run `remove_dialog_set` before `add_dialog_set`, so the beam, a
/// relog and a second beam never stack a second entry on slot 63800.
#[tokio::test]
async fn live_db_a_ring_round_trip_leaves_one_jackson_entry() {
    let pool = require_db_or_skip!();
    let expected = one(63800, (63800, Some(DLG_JACKSON), FLAG_DELIVERY));
    ring_round_trip(&pool, ARCH_HUMAN, M_1423, &expected).await;
}

/// **Jackson's Accept clears the slot.** After the round trip, the click on
/// 4641 removes the delivery map and leaves slot 63800 absent.
#[tokio::test]
async fn live_db_jacksons_accept_clears_the_slot() {
    let pool = require_db_or_skip!();
    let expected = one(63800, (63800, Some(DLG_JACKSON), FLAG_DELIVERY));
    let mut mgr = ring_round_trip(&pool, ARCH_HUMAN, M_1423, &expected).await;
    let engine = build_engine(Some(&pool)).await;
    let ctx = delivering(ARCH_HUMAN, S_4942, M_1423);
    apply(fire_dialog_choice(&engine, &ctx, DLG_JACKSON), &mut mgr).await;
    assert!(
        omega_slots(&mut mgr).is_empty(),
        "the Accept must leave slot 63800 absent"
    );
}

/// **A ring round trip leaves exactly one Bra'tac entry (F3), and the Accept
/// clears it.** The Free Jaffa twin of the two Jackson tests, on slot 63801.
#[tokio::test]
async fn live_db_a_ring_round_trip_leaves_one_bratac_entry() {
    let pool = require_db_or_skip!();
    let expected = one(63801, (63801, Some(DLG_BRATAC), FLAG_DELIVERY));
    let mut mgr = ring_round_trip(&pool, ARCH_JAFFA, M_1500, &expected).await;
    let engine = build_engine(Some(&pool)).await;
    let ctx = delivering(ARCH_JAFFA, S_4942, M_1500);
    apply(fire_dialog_choice(&engine, &ctx, DLG_BRATAC), &mut mgr).await;
    assert!(
        omega_slots(&mut mgr).is_empty(),
        "the Accept must leave slot 63801 absent"
    );
}

/// **The re-offer binds once, and never beside the delivery.** After the
/// departure, two Command Center loads and an abandon of 1423 each leave one
/// re-offer entry (map 63802, "?") on slot 63800, and map 63800 is never
/// there: the 1655-completed guard keeps the families apart.
#[tokio::test]
async fn live_db_the_reoffer_binds_once_and_never_beside_the_delivery() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // While 1655 is still active the delivery owns the slot: the re-offer
    // chains (guarded on 1655 completed) must add nothing beside it.
    let delivery = one(63800, (63800, Some(DLG_JACKSON), FLAG_DELIVERY));
    let mut held = staged_space(&pool).await;
    let ctx = delivering(ARCH_HUMAN, S_4942, M_1423);
    apply(fire_player_loaded(&engine, &ctx, CMD_NAME), &mut held).await;
    assert_eq!(
        omega_slots(&mut held),
        delivery,
        "the first load binds only the delivery"
    );
    apply(fire_player_loaded(&engine, &ctx, CMD_NAME), &mut held).await;
    apply(fire_mission_abandoned(&engine, &ctx, M_1423), &mut held).await;
    assert_eq!(
        omega_slots(&mut held),
        delivery,
        "a re-offer chain bound beside the delivery"
    );

    let mut mgr = staged_space(&pool).await;
    let expected = one(63800, (63802, Some(DLG_JACKSON), FLAG_REOFFER));
    let ctx = reoffering(ARCH_HUMAN, WORLD_CMD, M_1423);

    for step in ["first load", "second load"] {
        apply(fire_player_loaded(&engine, &ctx, CMD_NAME), &mut mgr).await;
        assert_eq!(omega_slots(&mut mgr), expected, "after the {step}");
    }
    apply(fire_mission_abandoned(&engine, &ctx, M_1423), &mut mgr).await;
    assert_eq!(omega_slots(&mut mgr), expected, "after the abandon");
    assert!(
        omega_slots(&mut mgr)
            .values()
            .flatten()
            .all(|(map, _, _)| *map != 63800),
        "the delivery map must never sit beside the re-offer"
    );
}

/// **No chain outside Omega binds Omega's slots, and no click opens two
/// dialogs.** Every Omega state and event, resolved with the full engine and
/// inspected **unfiltered**: an `AddDialogSet` on slots 63800-63803 must come
/// from a chain in 63800-63999, and 4641, 5025 and 6168 open no dialog from a
/// chain at all (the dialog opens through the bound map on click, not a
/// `display_dialog` action). A sibling that bound the same slot would show
/// on the next click as a second dialog.
#[tokio::test]
async fn live_db_no_chain_outside_omega_binds_omegas_slots() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let mut checked = 0_usize;
    let mut check = |state: &str, event: &str, resolved: &ResolvedActions| {
        for (chain, action) in &resolved.actions {
            if let Action::AddDialogSet { slot, .. } = action {
                assert!(
                    !OMEGA_SLOTS.contains(slot) || OMEGA_CHAIN_RANGE.contains(chain),
                    "{state} / {event}: chain {chain} binds Omega slot {slot}"
                );
            }
        }
        checked += 1;
    };
    for (archetype, offer) in [(ARCH_HUMAN, M_1423), (ARCH_JAFFA, M_1500)] {
        let at_beam = delivering(archetype, S_4940, offer);
        let at_4942 = delivering(archetype, S_4942, offer);
        let done_cmd = reoffering(archetype, WORLD_CMD, offer);
        let hub = with_step(
            with_mission(ctx_for(archetype, Some(WORLD_HUB)), M_1655, "active"),
            M_1655,
            S_4940,
            "active",
        );
        check(
            "S4940",
            "teleport_in 17",
            &fire_teleport_in(&engine, &at_beam, REGION_CMD_PAD),
        );
        check(
            "S4940",
            "player_loaded cmd",
            &fire_player_loaded(&engine, &at_beam, CMD_NAME),
        );
        check(
            "S4942",
            "player_loaded cmd",
            &fire_player_loaded(&engine, &at_4942, CMD_NAME),
        );
        check(
            "done",
            "player_loaded cmd",
            &fire_player_loaded(&engine, &done_cmd, CMD_NAME),
        );
        check(
            "done",
            "mission_abandoned",
            &fire_mission_abandoned(&engine, &done_cmd, offer),
        );
        check(
            "hub",
            "player_loaded hub",
            &fire_player_loaded(&engine, &hub, HUB_NAME),
        );
        for (state, ctx) in [
            ("S4940", &at_beam),
            ("S4942", &at_4942),
            ("done", &done_cmd),
            ("hub", &hub),
        ] {
            for d in [DLG_JACKSON, DLG_BRATAC, DLG_MODI] {
                let resolved = fire_dialog_choice(&engine, ctx, d);
                assert_eq!(
                    display_dialog_count(&resolved),
                    0,
                    "archetype {archetype} {state}: dialog {d} must open nothing from a chain"
                );
                check(state, "dialog_choice", &resolved);
            }
        }
    }
    assert!(checked > 0, "the sweep must inspect something");
}
