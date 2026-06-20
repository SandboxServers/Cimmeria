//! Tests for the `SGWPlayer` world-interaction cell-method dispatch
//! surface. The dispatch smoke test, the region-trigger guard, and the
//! shared `make_mgr_with_player` fixture live here; per-feature suites
//! are split into siblings:
//!   - `auto_cycle.rs` — `SET_AUTO_CYCLE` toggle behaviour.
//!   - `reload.rs` — `handle_reload` core + wire format.
//!   - `reload_holster.rs` — `handle_reload` holster/phase choreography.

use super::*;

mod auto_cycle;
mod reload;
mod reload_holster;

pub(super) fn make_mgr_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    mgr
}

#[tokio::test]
async fn dispatch_returns_false_for_unknown_method() {
    let mut mgr = make_mgr_with_player();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    let handled = dispatch(1, 9999, &[], &tx, &mut mgr, &engine).await;
    assert!(!handled);
}

/// TRIGGER_REGION with a negative region_id must be rejected by
/// the explicit `u32::try_from` guard, NOT by accidentally
/// missing a sign-extended u32 lookup. Pre-seed a real region at
/// the sign-extended id (`-5i32 as u32 == 0xFFFFFFFB`); if the
/// regression resurfaces (the cast slips through), the lookup
/// would match the planted region and fire content events.
/// With the negative-id guard in place the planted region must
/// stay invisible.
#[tokio::test]
async fn trigger_region_with_negative_id_rejects_via_explicit_guard() {
    use crate::cell::space_manager::RegionData;
    let mut mgr = make_mgr_with_player();
    // Plant a region at the sign-extended id of -5. If a regression
    // reintroduces the `region_id as u32` cast, get_region(0xFFFFFFFB)
    // would match this row and fire ring_transport / fire_enter_region.
    let trap_id: u32 = (-5i32) as u32;
    mgr.regions.insert(
        trap_id,
        RegionData {
            runtime_id: trap_id,
            db_set_id: 9999,
            tag: "trap".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 0.0,
            radius: 0.0,
            flags: 0,
            points: vec![],
        },
    );

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    // Layout: i32 region_id + u8 b_entering + 3 × f32 position.
    let mut args = Vec::with_capacity(17);
    args.extend_from_slice(&(-5i32).to_le_bytes());
    args.push(1);
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());

    let handled = dispatch(1, TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
    assert!(
        handled,
        "TRIGGER_REGION must claim the method even when region_id is bogus"
    );
    // The planted trap region MUST NOT match. No fire_*_region
    // cascade, no ring_transport message.
    assert!(
        rx.try_recv().is_err(),
        "negative region_id must be rejected by u32::try_from before lookup, \
         so the trap region at 0xFFFFFFFB can't fire"
    );
}
