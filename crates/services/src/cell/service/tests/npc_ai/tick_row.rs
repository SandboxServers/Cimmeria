//! `npc_ai.tick`: one row per ticked NPC per AI tick, emitted after the handler,
//! carrying where the NPC is, where it is going, the facing clients are sent,
//! and what it is fighting or following -- plus the handler's terminal outcome,
//! including `fight.rs` outcomes that used to bypass the shared vocabulary.

use super::make_ai_fixture;
use crate::test_support::LogCapture;
use cimmeria_common::Vector3;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

#[tokio::test]
async fn ai_tick_row_reports_state_target_facing_and_a_fight_outcome() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.create_entity(101, "Castle", [-10.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(101) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(101, 10.0);
        npc.direction = Vector3::new(0.0, 0.0, 0.0);
    }

    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let row = logs
        .find_message(tracing::Level::DEBUG, "NPC AI tick")
        .expect("every ticked NPC must produce a tick row");
    assert_eq!(row.target, "npc_ai.tick");
    assert!(row.has_field("npc_id", "200"));
    assert!(row.has_field("target_id", "101"));
    assert!(row.has_field("state_before", "Fighting"));
    // Target is due west: the handler re-faced to -PI/2, which must go out as
    // byte 192 (not the saturated 0 of the old pack_angle).
    assert!(
        row.has_field("yaw_byte", "192"),
        "got {:?}",
        row.fields.get("yaw_byte")
    );
    let outcome = row
        .fields
        .get("decision_outcome")
        .cloned()
        .unwrap_or_default();
    assert!(
        !outcome.is_empty() && outcome != "\"\"",
        "a fight.rs decision must reach the tick row; got {outcome:?}"
    );
}

/// Castle space with `n` hostile Idle guards (ticked every AI tick since
/// NA13) spread 30 u apart.
fn idle_guards(n: u32) -> (crate::cell::space_manager::SpaceManager, Vec<u32>) {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.destroy_entity(200);
    let mut ids = Vec::new();
    for i in 0..n {
        let id = mgr.allocate_npc_id();
        mgr.spawn_npc(id, "Castle", [30.0 * i as f32, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc = mgr.get_entity_mut(id).unwrap();
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Idle);
        ids.push(id);
    }
    (mgr, ids)
}

/// Run `ticks` AI ticks; the `npc_id` of every `npc_ai.tick` row written.
async fn tick_rows(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    ticks: usize,
) -> Vec<String> {
    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(64);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    for _ in 0..ticks {
        crate::cell::service::npc_ai::npc_ai_tick(&tx, mgr, &engine).await;
    }
    logs.all()
        .into_iter()
        .filter(|c| c.target == "npc_ai.tick" && c.message_contains("NPC AI tick"))
        .map(|c| c.fields.get("npc_id").cloned().unwrap_or_default())
        .collect()
}

/// NA24 (UAT-1 E): an empty server wrote ~14 `npc_ai.tick` rows a second,
/// one per hostile Idle guard per 2 s tick. Measured here: 10 idle guards
/// nobody can see, 6 ticks -- 60 rows before, 10 after (the first row per
/// NPC, then one per 60 s). Revert proof: make `admit_ai_tick_row` always
/// return `Some(0)` and the count is 60.
#[tokio::test]
async fn idle_unwitnessed_npcs_sample_their_tick_row() {
    let (mut mgr, ids) = idle_guards(10);
    let rows = tick_rows(&mut mgr, 6).await;
    assert_eq!(
        rows.len(),
        ids.len(),
        "one row per idle unwitnessed NPC, got {rows:?}"
    );
}

/// Sampling never touches what a player can see: an Idle guard inside a
/// player's AoI (50 u away, outside its 18 u aggro radius) writes a row on
/// every tick.
#[tokio::test]
async fn a_witnessed_idle_npc_writes_every_tick_row() {
    let (mut mgr, ids) = idle_guards(1);
    mgr.create_entity(101, "Castle", [0.0, 0.0, 50.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(101) {
        p.is_player = true;
        p.player_id = Some(101);
    }
    mgr.connect_entity(101);
    let _ = mgr.compute_aoi_changes();
    assert_eq!(
        mgr.get_witnesses_of(ids[0]),
        vec![101],
        "fixture: the player sees the guard"
    );

    let rows = tick_rows(&mut mgr, 6).await;
    assert_eq!(rows.len(), 6, "{rows:?}");
}
