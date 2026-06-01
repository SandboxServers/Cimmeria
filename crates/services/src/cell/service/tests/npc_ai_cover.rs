//! Cover-system integration with the `npc_ai_fight` tick.
//!
//! The cover branches in `npc_ai.rs` route the NPC's `nav_target_pos`
//! through the cover module on every fight tick when the NPC has
//! `use_cover = true`. The decision tree:
//!
//! - `StayInCover` → no movement change, NPC fires from cover.
//! - `MoveToCover` → `nav_target_pos` is the cover slot's position,
//!   not the threat's; the chase block paths to cover instead of
//!   running at the threat.
//! - `Released` (flanked) → reservation freed, `nav_path` cleared,
//!   `OnNpcFlanked` content trigger fired.
//! - `NoCover` → fall through to direct chase.
//!
//! These tests pin the observable side effects (reservation table
//! state, `nav_path` shape, cover release on combat-end transitions)
//! rather than the internal decision enum, so a refactor that swaps
//! the enum surface is still caught.

use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::cover::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSlotKey};
use crate::cell::space_manager::SpaceManager;

/// Castle fixture (non-instanced, same as `npc_ai.rs`'s `make_ai_fixture`)
/// with cover-system data pre-seeded into `space_mgr.cover`. NPC at id
/// 200 in AiState::Fighting with `use_cover = true`.
fn make_cover_fixture(
    npc_spawn: [f32; 3],
    npc_pos: [f32; 3],
    cover_nodes: Vec<CoverNode>,
) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", npc_pos, [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04;
        npc.ai_state = AiState::Fighting;
        npc.spawn_position = Some(Vector3::new(npc_spawn[0], npc_spawn[1], npc_spawn[2]));
        npc.use_cover = true;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr.cover = Cover::from_loaded(Vec::new(), cover_nodes);
    mgr
}

fn node(chunk_id: i32, node_id: i32, x: f32, z: f32, orient: f32) -> CoverNode {
    CoverNode {
        chunk_id,
        node_id,
        pos: Vector3::new(x, 0.0, z),
        orient,
        height: CoverHeight::Mid,
        quality: CoverQuality::Best,
        tail: [0; 4],
    }
}

/// Threat out of range + `use_cover = true` + an available cover slot
/// → the AI must reserve the slot. Pre-fix (cover branch wired
/// incorrectly), the NPC would run straight at the threat without
/// touching the cover module — the reservation table would stay
/// empty. Reverting the `use_cover && !is_stationary` block in
/// `npc_ai_fight` makes this assertion fail.
#[tokio::test]
async fn npc_ai_fight_reserves_cover_slot_when_out_of_range_and_use_cover_on() {
    let mut mgr = make_cover_fixture(
        [0.0; 3],
        [0.0; 3],
        // Cover slot 4m away from NPC, between NPC and target.
        vec![node(50, 0, 4.0, 0.0, 0.0)],
    );
    mgr.create_entity(100, "Castle", [40.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let r = mgr.cover.reservations.lock().unwrap();
    assert_eq!(
        r.slot_for_entity(EntityId(200)),
        Some(CoverSlotKey::new(50, 0)),
        "NPC with use_cover=true and target out of range must reserve the \
         single available cover slot. Empty reservation table means the \
         cover branch in npc_ai_fight didn't fire — either use_cover gate \
         broke or pick_best returned None when it shouldn't have."
    );
}

/// `use_cover = false` short-circuits the cover branch entirely — no
/// reservation. Pair with the previous test: same fixture geometry,
/// flip the flag, assert the OPPOSITE outcome. Without this paired
/// test, a refactor that always reserves would pass the first test
/// but break the carve-out for legacy NPCs.
#[tokio::test]
async fn npc_ai_fight_does_not_reserve_cover_when_use_cover_false() {
    let mut mgr = make_cover_fixture([0.0; 3], [0.0; 3], vec![node(50, 0, 4.0, 0.0, 0.0)]);
    mgr.create_entity(100, "Castle", [40.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        npc.use_cover = false; // the carve-out
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let r = mgr.cover.reservations.lock().unwrap();
    assert_eq!(
        r.reserved_count(),
        0,
        "use_cover=false must skip the cover branch entirely; a reservation \
         here means the gate was removed and every legacy NPC now contests \
         cover slots"
    );
}

/// Stationary NPCs skip cover too — the call-site predicate is
/// `use_cover && !is_stationary`. Turrets don't move to cover.
#[tokio::test]
async fn npc_ai_fight_stationary_does_not_reserve_cover() {
    let mut mgr = make_cover_fixture([0.0; 3], [0.0; 3], vec![node(50, 0, 4.0, 0.0, 0.0)]);
    mgr.create_entity(100, "Castle", [40.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        npc.is_stationary = true; // turret — fixed in place
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let r = mgr.cover.reservations.lock().unwrap();
    assert_eq!(
        r.reserved_count(),
        0,
        "stationary NPCs must skip cover (same carve-out the no-pathfind \
         branch uses); a reservation here means the && !is_stationary \
         predicate was dropped"
    );
}

/// NPC already holding a cover slot whose orient defends against the
/// threat → `StayInCover`, reservation preserved across the tick.
/// Pre-fix a `release_for_entity` always-fire branch would have
/// dropped this; the regression shape pins persistence.
#[tokio::test]
async fn npc_ai_fight_preserves_reservation_when_not_flanked() {
    let mut mgr = make_cover_fixture(
        [0.0; 3],
        [4.0, 0.0, 0.0],
        // Cover at NPC's position, orient=0 → faces +X = toward threat.
        vec![node(50, 0, 4.0, 0.0, 0.0)],
    );
    // Pre-seed: NPC already in cover.
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(200), CoverSlotKey::new(50, 0))
        .unwrap();
    // Threat in the defended half-plane (+X).
    mgr.create_entity(100, "Castle", [40.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let r = mgr.cover.reservations.lock().unwrap();
    assert_eq!(
        r.slot_for_entity(EntityId(200)),
        Some(CoverSlotKey::new(50, 0)),
        "non-flanked NPC must keep its reservation — releasing here would \
         force a re-pick on every tick and burn through the cover-set's \
         affinity counts"
    );
}

/// NPC holding a cover slot whose orient no longer defends against
/// the threat (threat moved behind) → `Released`, reservation freed,
/// `nav_path` cleared. Pin all three: release + nav_path empty + ai
/// still Fighting. Reverting `release_for_entity` to a no-op or
/// dropping the `nav_path.clear()` line in the Released arm fails
/// this test.
#[tokio::test]
async fn npc_ai_fight_flanked_npc_releases_cover_and_clears_nav_path() {
    let mut mgr = make_cover_fixture(
        [0.0; 3],
        [4.0, 0.0, 0.0],
        // Cover at NPC's position, orient=0 (faces +X). Threat behind cover (-X) = flanked.
        vec![node(50, 0, 4.0, 0.0, 0.0)],
    );
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(200), CoverSlotKey::new(50, 0))
        .unwrap();
    mgr.create_entity(100, "Castle", [-40.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        // Stage a stale nav_path that points at the now-abandoned slot.
        // The Released arm must clear it; otherwise the NPC walks one
        // more frame toward the dead slot.
        npc.nav_path.push_back(Vector3::new(4.0, 0.0, 0.0));
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let r = mgr.cover.reservations.lock().unwrap();
    assert!(
        r.slot_for_entity(EntityId(200)).is_none(),
        "flanked NPC must release its cover slot; lingering reservation \
         means is_flanked or release_for_entity didn't fire"
    );
    drop(r);
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "Released arm must clear nav_path so the NPC doesn't walk one more \
         tick toward the abandoned slot. Got: {:?}",
        npc.nav_path
    );
}

/// Fighting NPC with empty threat list — cover slot must be released
/// at the same time the state transitions to Idle. Pre-fix a leaked
/// reservation here would block a different NPC from taking the slot
/// later. Bug shape: the cover-release on empty-threat is a guard the
/// fight tick has to wire explicitly because the cover module
/// doesn't observe AI state.
#[tokio::test]
async fn npc_ai_fight_empty_threat_releases_cover_slot() {
    let mut mgr = make_cover_fixture([0.0; 3], [4.0, 0.0, 0.0], vec![node(50, 0, 4.0, 0.0, 0.0)]);
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(200), CoverSlotKey::new(50, 0))
        .unwrap();
    // No threat targets — fight branch resets to Idle and must release.

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let r = mgr.cover.reservations.lock().unwrap();
    assert!(
        r.slot_for_entity(EntityId(200)).is_none(),
        "empty-threat reset-to-Idle branch must release the cover slot — \
         otherwise dead NPCs hold cover indefinitely and the squad-affinity \
         calculator counts them as live"
    );
    drop(r);
    assert!(matches!(
        mgr.get_entity(200).unwrap().ai_state,
        AiState::Idle
    ));
}

/// Same as above but for the leash transition (target past
/// LEASH_DISTANCE). Two distinct combat-end seams — both must
/// release cover. Pin the leash one independently because the code
/// paths are separate.
#[tokio::test]
async fn npc_ai_fight_leash_transition_releases_cover_slot() {
    let mut mgr = make_cover_fixture([0.0; 3], [4.0, 0.0, 0.0], vec![node(50, 0, 4.0, 0.0, 0.0)]);
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(200), CoverSlotKey::new(50, 0))
        .unwrap();
    // Target past LEASH_DISTANCE (50) from spawn — fight branch
    // transitions to Leashing, which must also release cover.
    mgr.create_entity(100, "Castle", [100.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
    }

    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let r = mgr.cover.reservations.lock().unwrap();
    assert!(
        r.slot_for_entity(EntityId(200)).is_none(),
        "leash transition must release cover — leaving a reservation here \
         means a leashing NPC blocks the slot for any other NPC trying to \
         pick cover near the same chunk"
    );
    drop(r);
    assert!(matches!(
        mgr.get_entity(200).unwrap().ai_state,
        AiState::Leashing
    ));
}
