use std::sync::atomic::Ordering;

use serde_json::json;

use super::*;
use crate::bridge::dispatch::INVALID_PARAMS;
use crate::bridge::input::{
    dispatch_hold, dispatch_release, dispatch_status, hold_table, input_state,
};
use crate::bridge::tick::{self, HOLDS_ACTIVE};

fn hp(vk: u8, down: bool, lease_ms: Option<u32>) -> HoldParams {
    HoldParams {
        hwnd: 7,
        vk,
        lparam_down: 0x11,
        lparam_up: 0xC000_0022,
        down,
        lease_ms,
    }
}

/// The table, `InputState` and `HOLDS_ACTIVE` are process globals: take the
/// bridge-wide test lock and start from empty.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    let g = crate::bridge::meta::tests::SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    hold_table()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .release_all();
    input_state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .release_all();
    HOLDS_ACTIVE.store(false, Ordering::SeqCst);
    g
}

fn left(t: &HoldTable, now: u64, vk: u8) -> u32 {
    t.snapshot(now).iter().find(|h| h.0 == vk).unwrap().1
}

#[test]
fn press_posts_once_and_extends() {
    let mut t = HoldTable::default();
    let (posted, posts) = t.press(&hp(0x57, true, Some(1000)), 0).unwrap();
    assert!(posted);
    assert_eq!(
        posts,
        vec![Post::Down {
            hwnd: 7,
            vk: 0x57,
            lparam: 0x11
        }]
    );
    assert_eq!(left(&t, 0, 0x57), 1000);
    let (posted, posts) = t.press(&hp(0x57, true, Some(1000)), 600).unwrap();
    assert!(!posted);
    assert!(posts.is_empty());
    assert_eq!(left(&t, 600, 0x57), 1000);
}

#[test]
fn lease_is_clamped() {
    let mut t = HoldTable::default();
    t.press(&hp(1, true, Some(50)), 0).unwrap();
    t.press(&hp(2, true, Some(60_000)), 0).unwrap();
    t.press(&hp(3, true, None), 0).unwrap();
    assert_eq!(left(&t, 0, 1), 200);
    assert_eq!(left(&t, 0, 2), 10_000);
    assert_eq!(left(&t, 0, 3), 10_000);
}

#[test]
fn seventeenth_key_errors() {
    let mut t = HoldTable::default();
    for vk in 0..16u8 {
        t.press(&hp(vk, true, None), 0).unwrap();
    }
    let e = t.press(&hp(99, true, None), 0).unwrap_err();
    assert_eq!(e, "too many holds (max 16)");
    assert_eq!(t.snapshot(0).len(), 16);
    // Extending an existing key at the cap is not a 17th key.
    assert!(t.press(&hp(3, true, None), 0).is_ok());
}

#[test]
fn expire_posts_key_up_with_lparam_up() {
    let mut t = HoldTable::default();
    t.press(&hp(0x41, true, Some(300)), 1000).unwrap();
    assert!(t.expire(1299).is_empty());
    let posts = t.expire(1300);
    assert_eq!(
        posts,
        vec![Post::Up {
            hwnd: 7,
            vk: 0x41,
            lparam: 0xC000_0022
        }]
    );
    assert!(t.is_empty());
}

#[test]
fn release_all_posts_every_up() {
    let mut t = HoldTable::default();
    for vk in [1u8, 2, 3] {
        t.press(&hp(vk, true, None), 0).unwrap();
    }
    let posts = t.release_all();
    assert_eq!(posts.len(), 3);
    assert!(posts.iter().all(|p| matches!(p, Post::Up { .. })));
    assert!(t.is_empty());
}

#[test]
fn release_of_unheld_key_still_posts_up() {
    let mut t = HoldTable::default();
    assert_eq!(t.release(&hp(5, false, None)).len(), 1);
}

#[test]
fn dispatch_hold_bad_params_is_invalid_params() {
    let _g = serial();
    let r = dispatch_hold(json!(1), &json!({}));
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["error"]["code"], INVALID_PARAMS);
}

#[test]
fn status_reports_holds() {
    let _g = serial();
    let r = dispatch_hold(
        json!(1),
        &json!({"hwnd": 7, "vk": 87, "lparam_down": 1, "lparam_up": 2, "down": true}),
    );
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["result"]["posted"], true);
    let s = serde_json::to_value(dispatch_status(json!(2))).unwrap();
    assert_eq!(s["result"]["holds"][0]["vk"], 87);
    assert!(s["result"]["holds_expired"].is_u64());
    assert!(HOLDS_ACTIVE.load(Ordering::SeqCst));
}

#[test]
fn disconnect_releases_holds_and_input() {
    let _g = serial();
    dispatch_hold(
        json!(1),
        &json!({"hwnd": 7, "vk": 87, "lparam_down": 1, "lparam_up": 2, "down": true}),
    );
    input_state().lock().unwrap().key(0x11, true);
    let before = HOLDS_RELEASED_ON_DISCONNECT.load(Ordering::SeqCst);
    tick::on_disconnect();
    assert_eq!(
        HOLDS_RELEASED_ON_DISCONNECT.load(Ordering::SeqCst),
        before + 1
    );
    assert!(hold_table().lock().unwrap().is_empty());
    assert!(input_state().lock().unwrap().held_keys().is_empty());
    assert!(!HOLDS_ACTIVE.load(Ordering::SeqCst));
}

#[test]
fn input_release_releases_holds() {
    let _g = serial();
    dispatch_hold(
        json!(1),
        &json!({"hwnd": 7, "vk": 87, "lparam_down": 1, "lparam_up": 2, "down": true}),
    );
    dispatch_release(json!(2));
    let s = serde_json::to_value(dispatch_status(json!(3))).unwrap();
    assert_eq!(s["result"]["holds"].as_array().unwrap().len(), 0);
    assert!(!HOLDS_ACTIVE.load(Ordering::SeqCst));
}

/// The Tick path: an expired hold is dropped and counted, and the active
/// flag clears so the idle cost returns to two loads.
#[test]
fn tick_expires_holds_and_clears_active() {
    let _g = serial();
    crate::bridge::meta::mark_start();
    // Lease 200 ms from clock 0; the bridge clock passes that after a sleep.
    hold_table()
        .lock()
        .unwrap()
        .press(&hp(9, true, Some(200)), 0)
        .unwrap();
    HOLDS_ACTIVE.store(true, Ordering::SeqCst);
    let before = HOLDS_EXPIRED.load(Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(260));
    tick::on_tick();
    assert_eq!(HOLDS_EXPIRED.load(Ordering::SeqCst), before + 1);
    assert!(hold_table().lock().unwrap().is_empty());
    assert!(!HOLDS_ACTIVE.load(Ordering::SeqCst));
}

/// A panic while the table is locked poisons it. The Tick must still expire
/// holds instead of skipping every frame.
#[test]
fn tick_expires_holds_through_poisoned_table() {
    let _g = serial();
    crate::bridge::meta::mark_start();
    let poisoner = std::thread::spawn(|| {
        let mut t = hold_table().lock().unwrap();
        t.press(&hp(9, true, Some(200)), 0).unwrap();
        panic!("poison the hold table");
    });
    assert!(poisoner.join().is_err());
    assert!(hold_table().is_poisoned());
    HOLDS_ACTIVE.store(true, Ordering::SeqCst);
    let before = HOLDS_EXPIRED.load(Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(260));
    tick::on_tick();
    assert_eq!(HOLDS_EXPIRED.load(Ordering::SeqCst), before + 1);
    assert!(hold_table()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_empty());
    assert!(!HOLDS_ACTIVE.load(Ordering::SeqCst));
    hold_table().clear_poison();
}

/// Disconnect (IO thread) racing Tick (main thread) must not deadlock or
/// leave the active flag set over an empty table.
#[test]
fn disconnect_races_tick_without_stuck_flag() {
    let _g = serial();
    let t = std::thread::spawn(|| {
        for _ in 0..200 {
            tick::on_tick();
        }
    });
    // Checker: while the table lock is held, a non-empty table must have the
    // flag set. The lock discipline makes that hold at every point.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let s2 = stop.clone();
    let chk = std::thread::spawn(move || {
        while !s2.load(Ordering::SeqCst) {
            let t = hold_table().lock().unwrap_or_else(|e| e.into_inner());
            assert!(
                t.is_empty() || HOLDS_ACTIVE.load(Ordering::SeqCst),
                "held keys with HOLDS_ACTIVE false"
            );
            drop(t);
            std::thread::yield_now();
        }
    });
    for _ in 0..200 {
        dispatch_hold(
            json!(1),
            &json!({"hwnd": 7, "vk": 87, "lparam_down": 1, "lparam_up": 2, "down": true}),
        );
        tick::on_disconnect();
    }
    stop.store(true, Ordering::SeqCst);
    chk.join().unwrap();
    t.join().unwrap();
    assert!(hold_table().lock().unwrap().is_empty());
    assert!(!HOLDS_ACTIVE.load(Ordering::SeqCst));
}
