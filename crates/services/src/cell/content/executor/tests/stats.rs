//! `Action::ChangeStat` executor coverage — heal, clamp, damage,
//! set-to-max, and the ammo-stat skip path.

use super::*;

/// Build a connected player at id=1 with HEALTH at `cur/max`, dirty
/// flags cleared so a single `serialize_dirty` reads only what
/// `Action::ChangeStat` writes. Used by the heal-action tests below.
fn make_player_with_health(mgr: &mut SpaceManager, cur: i32, max: i32) {
    use cimmeria_entity::stats::HEALTH;
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
        if let Some(h) = e.stats.get_mut(HEALTH) {
            h.update(0, cur, max);
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);
}

/// `change_stat { amount: +500 }` is the canonical heal-on-use shape
/// (Health Slappack TC1, chain 4001). Three things must hold: HP
/// advances by exactly the delta when room is available, the change
/// is broadcast as a single onStatUpdate carrying the new HEALTH
/// value, and the entity's dirty state is drained so a follow-up
/// serialize doesn't re-emit the same stat.
#[tokio::test]
async fn change_stat_amount_advances_health_and_emits_on_stat_update() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 200, 1000);

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: None,
                amount: Some(500),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // Entity state.
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.stats.get(HEALTH).unwrap().cur,
        700,
        "HEALTH.cur must advance by +500"
    );
    assert!(
        !entity.stats.has_dirty(),
        "executor must clear dirty after sending — otherwise the next \
         serialize would re-emit the same stat"
    );

    // Wire frame.
    let msg = rx.try_recv().expect("expected onStatUpdate");
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, crate::mercury::method_idx::ON_STAT_UPDATE);
            let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]) as usize;
            let mut found = false;
            for i in 0..count {
                let off = 4 + i * 16;
                let stat_id =
                    i32::from_le_bytes([args[off], args[off + 1], args[off + 2], args[off + 3]]);
                if stat_id == HEALTH {
                    let cur = i32::from_le_bytes([
                        args[off + 8],
                        args[off + 9],
                        args[off + 10],
                        args[off + 11],
                    ]);
                    assert_eq!(cur, 700, "wire payload carries the post-heal HEALTH.cur");
                    found = true;
                }
            }
            assert!(found, "onStatUpdate payload must include HEALTH");
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }

    assert!(rx.try_recv().is_err(), "no further messages expected");
}

/// Heal must clamp at `max` — a slappack on a near-full bar can't
/// push the wire payload to `cur > max`. The clamp is `Stat::change`'s
/// responsibility; this guard pins that the executor doesn't bypass
/// it (e.g., by writing `cur` directly).
#[tokio::test]
async fn change_stat_amount_clamps_to_max() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 950, 1000);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: None,
                amount: Some(500),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(
        h.cur, 1000,
        "heal clamps to max even on +500 over a 50-room bar"
    );
    assert!(h.cur <= h.max, "wire invariant cur <= max preserved");
}

/// Negative `amount` damages the stat. The same code path serves
/// debuff/poison-style chains; if the executor ever silently ignored
/// negative deltas (e.g., `cur += amount.max(0)`) this guard fails.
#[tokio::test]
async fn change_stat_negative_amount_damages_stat() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 800, 1000);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: None,
                amount: Some(-300),
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        500,
        "negative amount must subtract from cur"
    );
}

/// `set_to_max: true` snaps `cur` to `max` regardless of the prior
/// value. Pairs with the legacy reload-effect chain (effect_id-driven,
/// `set_to_max=true` on the ammo stat) so the bounds-modifying path
/// stays exercised alongside the new `amount` path.
#[tokio::test]
async fn change_stat_set_to_max_snaps_current_to_max() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 100, 1000);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            4001,
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: Some(true),
                amount: None,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(h.cur, 1000, "set_to_max snaps cur to max");
    assert_eq!(h.max, 1000, "max unchanged");
}

/// `use_ammo_stat=true` (and its legacy `stat_id=-1` sentinel form,
/// used by the seeded Reload effect chain 2011) must skip cleanly
/// rather than warn-and-no-op on a missing stat lookup. Pin the
/// no-side-effects shape: HP unchanged, no wire message sent. When
/// active-ammo-slot resolution lands, this test should be replaced
/// with one that asserts the resolved ammo stat actually mutates.
#[tokio::test]
async fn change_stat_with_use_ammo_stat_skips_cleanly() {
    use cimmeria_entity::stats::HEALTH;

    let mut mgr = make_space_mgr();
    make_player_with_health(&mut mgr, 500, 1000);

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![
            (
                2011,
                Action::ChangeStat {
                    stat_id: -1, // legacy sentinel for "ammo stat"
                    min: None,
                    max: None,
                    use_ammo_stat: Some(true),
                    set_to_max: Some(true),
                    amount: None,
                },
            ),
            (
                2011,
                Action::ChangeStat {
                    stat_id: -1, // negative-only path also skips
                    min: None,
                    max: None,
                    use_ammo_stat: None,
                    set_to_max: None,
                    amount: Some(50),
                },
            ),
        ],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let h = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(
        h.cur, 500,
        "HEALTH untouched — ammo-stat path is unimplemented"
    );
    assert!(
        rx.try_recv().is_err(),
        "no onStatUpdate must fire when the ammo-stat path skips"
    );
}
