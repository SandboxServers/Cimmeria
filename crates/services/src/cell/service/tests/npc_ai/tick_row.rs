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
