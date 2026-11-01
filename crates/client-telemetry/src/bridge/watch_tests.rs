use std::sync::atomic::Ordering;

use serde_json::{json, Value};

use super::*;
use crate::bridge::meta::tests::SERIAL;
use crate::bridge::tick;
use crate::hooks::camera::layout::{
    ACTOR_LOCATION_OFF, ENTITY_MANAGER_VA, ENTITY_PAWN_OFF, MANAGER_LOCAL_ENTITY_OFF,
};
use crate::hooks::entity_trace::map::fake::FakeMem;
use crate::hooks::nav::controller::{
    PC_BUTTONS_OFF, PC_DRAG_X_OFF, PC_DRAG_Y_OFF, PC_HOVER_OFF, PC_MODE_OFF,
};

const WORLD: u32 = 0x4000_0000;
const LEVEL: u32 = 0x4000_1000;
const ACTORS: u32 = 0x4000_2000;
const PC: u32 = 0x4000_3000;

/// A world whose Actors array holds `count` slots and, when `with_pc`, the
/// local controller in slot 0 with mode 3, buttons 1 and hover 77.
fn world(with_pc: bool, count: u32) -> FakeMem {
    let mut m = FakeMem::default();
    m.set(GLEVEL_PTR_VA, WORLD);
    m.set(WORLD + WORLD_LEVEL_OFF, LEVEL);
    m.set(LEVEL + LEVEL_ACTORS_OFF, ACTORS);
    m.set(LEVEL + LEVEL_ACTORS_OFF + 4, count);
    if with_pc {
        m.set(ACTORS, PC);
        m.set(PC, PLAYER_CONTROLLER_VTABLE_VA);
        m.set(PC + PC_MODE_OFF, 3);
        m.set(PC + PC_BUTTONS_OFF, 1);
        m.set(PC + PC_DRAG_X_OFF, 0);
        m.set(PC + PC_DRAG_Y_OFF, 0);
        m.set(PC + PC_HOVER_OFF, 77);
    }
    m
}

fn rpc(r: RpcResponse) -> Value {
    r.result.expect("ok result")
}

fn reset() {
    *lock() = None;
    WATCH_ACTIVE.store(false, Ordering::SeqCst);
    PC_CACHE.store(0, Ordering::Relaxed);
}

#[test]
fn sample_omits_failed_reads() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let m = world(false, 5);
    let s = sample(7, 99, &m);
    assert_eq!((s.f, s.ms), (7, 99));
    assert_eq!((s.hover, s.mode, s.buttons), (None, None, None));
    assert_eq!((s.pos, s.rot, s.t), (None, None, None));
    // The actor count does not depend on the controller.
    assert_eq!(s.actors, Some(5));
    let v = serde_json::to_value(&s).unwrap();
    for k in ["hover", "mode", "buttons", "pos", "rot", "t"] {
        assert!(v.get(k).is_none(), "{k} must be absent: {v}");
    }
    // An unreadable world leaves the bare stamp.
    let bare = serde_json::to_value(sample(1, 2, &FakeMem::default())).unwrap();
    assert_eq!(bare, json!({ "i": 0, "f": 1, "ms": 2 }));
}

#[test]
fn sample_reads_controller_pose_and_actors() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut m = world(true, 50);
    let (mgr, ent, pawn) = (0x4100_0000, 0x4100_1000, 0x4100_2000);
    m.set(ENTITY_MANAGER_VA, mgr);
    m.set(mgr + MANAGER_LOCAL_ENTITY_OFF, ent);
    m.set(ent + ENTITY_PAWN_OFF, pawn);
    let at = pawn + ACTOR_LOCATION_OFF;
    m.set(at, 1.5f32.to_bits());
    m.set(at + 4, (-2.0f32).to_bits());
    m.set(at + 8, 3.0f32.to_bits());
    m.set(at + 0xc, 100);
    m.set(at + 0x10, (-200i32) as u32);
    m.set(at + 0x14, 0);
    let s = sample(3, 4, &m);
    assert_eq!(s.pos, Some([1.5, -2.0, 3.0]));
    assert_eq!(s.rot, Some([100, -200, 0]));
    assert_eq!((s.hover, s.mode, s.buttons), (Some(77), Some(3), Some(1)));
    assert_eq!(s.actors, Some(50));
}

#[test]
fn params_are_clamped() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    dispatch_start(json!(1), &json!({ "period_frames": 0, "cap": 1 }));
    {
        let g = lock();
        let r = g.as_ref().unwrap();
        assert_eq!((r.period(), r.cap()), (1, 16));
    }
    dispatch_start(json!(1), &json!({ "period_frames": 99, "cap": 99999 }));
    {
        let g = lock();
        let r = g.as_ref().unwrap();
        assert_eq!((r.period(), r.cap()), (60, 4096));
    }
    // No params at all means the defaults.
    dispatch_start(json!(1), &Value::Null);
    {
        let g = lock();
        let r = g.as_ref().unwrap();
        assert_eq!((r.period(), r.cap()), (1, 1024));
    }
    let bad = dispatch_start(json!(1), &json!({ "cap": "many" }));
    assert_eq!(bad.error.unwrap().code, INVALID_PARAMS);
    reset();
}

#[test]
fn start_replaces_and_stop_reports() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let first = rpc(dispatch_start(json!(1), &json!({})));
    assert_eq!(first["replaced"], json!(false));
    assert!(WATCH_ACTIVE.load(Ordering::SeqCst));

    let second = rpc(dispatch_start(json!(2), &json!({ "cap": 16 })));
    assert_eq!(second["replaced"], json!(true));

    // Overfill the 16-slot ring through the same path the Tick uses.
    {
        let mut g = lock();
        let r = g.as_mut().unwrap();
        for f in 0..20 {
            r.push(Sample::bare(f, 0));
        }
    }
    let stop = rpc(dispatch_stop(json!(3)));
    assert_eq!(stop, json!({ "samples_taken": 20, "dropped": 4 }));
    assert!(!WATCH_ACTIVE.load(Ordering::SeqCst));

    // Stopped, but what was taken is still readable and says so.
    let read = rpc(dispatch_read(json!(4), &json!({ "since": 0, "max": 1024 })));
    assert_eq!(read["running"], json!(false));
    assert_eq!(read["samples"].as_array().unwrap().len(), 16);
    assert_eq!(read["next"], json!(20));
    assert_eq!(read["dropped"], json!(4));

    // A start after a stop is a fresh watch, not a replacement.
    let third = rpc(dispatch_start(json!(5), &json!({})));
    assert_eq!(third["replaced"], json!(false));
    reset();
}

#[test]
fn tick_samples_only_while_running() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    tick::on_tick(); // inactive: nothing
    dispatch_start(json!(1), &json!({ "period_frames": 1 }));
    tick::on_tick();
    let taken = |d: &Value| d["next"].as_u64().unwrap();
    let after_one = rpc(dispatch_read(json!(2), &json!({})));
    assert_eq!(taken(&after_one), 1, "{after_one}");
    dispatch_stop(json!(3));
    tick::on_tick();
    let after_stop = rpc(dispatch_read(json!(4), &json!({})));
    assert_eq!(taken(&after_stop), 1, "a stopped watch takes no samples");
    reset();
}

#[test]
fn disconnect_stops_the_watch() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    dispatch_start(json!(1), &json!({}));
    assert!(WATCH_ACTIVE.load(Ordering::SeqCst));
    tick::on_disconnect();
    assert!(!WATCH_ACTIVE.load(Ordering::SeqCst));
    let read = rpc(dispatch_read(json!(2), &json!({})));
    assert_eq!(read["running"], json!(false));
    assert_eq!(read["samples"], json!([]));
}
