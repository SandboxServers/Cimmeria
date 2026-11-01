use std::cell::Cell;

use serde_json::{json, Value};

use super::*;
use crate::bridge::batch::ALLOWED;
use crate::bridge::dispatch::RpcResponse;
use crate::bridge::meta::tests::SERIAL;
use crate::hooks::entity_trace::map::fake::FakeMem;

const WORLD: u32 = 0x4000_0000;
const PAWN: u32 = 0x1234_0000;

fn world_mem() -> FakeMem {
    let mut m = FakeMem::default();
    m.set(GLEVEL_PTR_VA, WORLD);
    m.set(WORLD, UWORLD_VTABLE_VA);
    m
}

fn with_pawn(m: &mut FakeMem) {
    let (mgr, ent) = (0x4100_0000, 0x4100_1000);
    m.set(ENTITY_MANAGER_VA, mgr);
    m.set(mgr + MANAGER_LOCAL_ENTITY_OFF, ent);
    m.set(ent + ENTITY_PAWN_OFF, PAWN);
}

fn params() -> Value {
    json!({ "start": [0.0, 0.0, 100.0], "end": [0.0, 0.0, -100.0] })
}

fn run(m: &FakeMem, p: &Value, native: &Native<'_>) -> RpcResponse {
    dispatch_with(json!(1), p, m, native)
}

fn err(r: &RpcResponse) -> (i32, String) {
    let e = r.error.as_ref().expect("error response");
    (e.code, e.message.clone())
}

fn fault() -> SehFault {
    SehFault {
        raw_code: 0xC000_0005,
        address: 0x0091_4FD3,
        description: "access violation".into(),
    }
}

fn put(buf: *mut u8, off: usize, bytes: &[u8]) {
    // SAFETY: tests pass the 0x48-byte buffer `dispatch_with` owns.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.add(off), bytes.len()) };
}

#[test]
fn flags_default_and_allowlist() {
    assert_eq!(parse(&params()).unwrap().flags, 0x2086);
    let mut p = params();
    p["flags"] = json!(0x20bf);
    assert_eq!(parse(&p).unwrap().flags, 0x20bf);
    p["flags"] = json!(0x3fff);
    assert_eq!(
        parse(&p).unwrap_err(),
        "trace_line: flags must be 0x2086 or 0x20bf"
    );
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let r = run(&world_mem(), &p, &|_, _, _, _, _, _| Ok(1));
    assert_eq!(err(&r).0, INVALID_PARAMS);
    // Wrong shapes are -32602 too.
    let bad = json!({ "start": [0.0, 0.0], "end": [0.0, 0.0, 0.0] });
    assert_eq!(
        parse(&bad).unwrap_err(),
        "trace_line: start and end must be [x, y, z]"
    );
}

#[test]
fn check_result_is_initialised() {
    let b = init_check_result();
    for (i, v) in b.iter().enumerate() {
        match i {
            0x20..=0x23 => assert_eq!(*v, 1.0f32.to_le_bytes()[i - 0x20]),
            0x24..=0x27 | 0x40..=0x43 => assert_eq!(*v, 0xff),
            _ => assert_eq!(*v, 0, "byte {i:#x}"),
        }
    }
}

#[test]
fn return_one_is_no_hit() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let r = run(&world_mem(), &params(), &|_, _, _, _, _, _| Ok(1));
    let v = r.result.expect("ok");
    assert_eq!(v, json!({ "hit": false, "time": 1.0 }));
    assert!(v.get("location").is_none());
}

#[test]
fn bsp_hit_reports_item() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let native = |_w, buf: *mut u8, _s, _e: &[f32; 3], _st: &[f32; 3], _f| {
        put(buf, OFF_ACTOR, &0x5000_0000u32.to_le_bytes());
        for (i, x) in [1.0f32, 2.0, 3.0].iter().enumerate() {
            put(buf, OFF_LOCATION + 4 * i, &x.to_le_bytes());
        }
        for (i, x) in [0.0f32, 0.0, 1.0].iter().enumerate() {
            put(buf, OFF_NORMAL + 4 * i, &x.to_le_bytes());
        }
        put(buf, OFF_TIME, &0.5f32.to_le_bytes());
        put(buf, OFF_ITEM, &17i32.to_le_bytes());
        Ok(0)
    };
    let v = run(&world_mem(), &params(), &native).result.expect("ok");
    assert_eq!(v["hit"], json!(true));
    assert_eq!(v["item"], json!(17));
    assert_eq!(v["actor"], json!(0x5000_0000u32));
    assert_eq!(v["time"], json!(0.5));
    assert_eq!(v["location"], json!([1.0, 2.0, 3.0]));
    assert_eq!(v["normal"], json!([0.0, 0.0, 1.0]));
}

#[test]
fn own_pawn_is_the_source() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let seen = Cell::new(u32::MAX);
    let native = |_w, _b, s, _e: &[f32; 3], _st: &[f32; 3], _f| {
        seen.set(s);
        Ok(1)
    };
    let mut m = world_mem();
    with_pawn(&mut m);
    run(&m, &params(), &native);
    assert_eq!(seen.get(), PAWN);
    // A broken chain gives 0.
    let mut broken = world_mem();
    with_pawn(&mut broken);
    broken.set(ENTITY_MANAGER_VA, 0);
    run(&broken, &params(), &native);
    assert_eq!(seen.get(), 0);
}

#[test]
fn refuses_without_world() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let called = Cell::new(false);
    let native = |_w, _b, _s, _e: &[f32; 3], _st: &[f32; 3], _f| {
        called.set(true);
        Ok(1)
    };
    let mut m = FakeMem::default();
    m.set(GLEVEL_PTR_VA, 0);
    let r = run(&m, &params(), &native);
    assert_eq!(
        err(&r),
        (-32000, "trace_line unavailable: world not ready".into())
    );
    assert!(!called.get());
}

#[test]
fn wrong_world_vtable_refuses() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let called = Cell::new(false);
    let native = |_w, _b, _s, _e: &[f32; 3], _st: &[f32; 3], _f| {
        called.set(true);
        Ok(1)
    };
    let mut m = world_mem();
    m.set(WORLD, 0x0183_8E80);
    let r = run(&m, &params(), &native);
    assert_eq!(
        err(&r),
        (-32000, "trace_line unavailable: world not ready".into())
    );
    assert!(!called.get());
}

#[test]
fn fault_is_reported_not_raised() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let r = run(&world_mem(), &params(), &|_, _, _, _, _, _| Err(fault()));
    let e = r.error.expect("error");
    assert_eq!(e.code, -32000);
    assert_eq!(e.data.expect("data")["Exception"]["code"], "0xc0000005");
    reset_latch();
}

#[test]
fn fault_latches_off() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    run(&world_mem(), &params(), &|_, _, _, _, _, _| Err(fault()));
    let calls = Cell::new(0u32);
    let ok = |_w, _b, _s, _e: &[f32; 3], _st: &[f32; 3], _f| {
        calls.set(calls.get() + 1);
        Ok(1)
    };
    let r = run(&world_mem(), &params(), &ok);
    let e = r.error.expect("error");
    assert_eq!(e.code, -32000);
    assert_eq!(
        e.message,
        "trace_line unavailable: faulted earlier; restart the client"
    );
    assert_eq!(e.data.expect("data"), json!({ "trace_latched": true }));
    assert_eq!(calls.get(), 0);
    reset_latch();
}

#[test]
fn flags_other_than_proven_refused() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_latch();
    let called = Cell::new(false);
    let native = |_w, _b, _s, _e: &[f32; 3], _st: &[f32; 3], _f| {
        called.set(true);
        Ok(1)
    };
    for f in [0x20beu32, 0x3fff, 0] {
        let mut p = params();
        p["flags"] = json!(f);
        let r = run(&world_mem(), &p, &native);
        assert_eq!(
            err(&r),
            (
                INVALID_PARAMS,
                "trace_line: flags must be 0x2086 or 0x20bf".into()
            ),
            "flags {f:#x}"
        );
    }
    assert!(!called.get());
}

#[test]
fn batch_does_not_allow_trace() {
    assert!(!ALLOWED.contains(&"trace_line"));
}

#[test]
fn trace_line_is_routed() {
    use crate::bridge::dispatch::{dispatch, RpcRequest, METHOD_NOT_FOUND};
    let r = dispatch(&RpcRequest {
        id: json!(1),
        method: "trace_line".to_string(),
        params: json!({}),
    });
    let e = r.error.expect("error");
    assert_ne!(e.code, METHOD_NOT_FOUND);
    assert_eq!(e.code, INVALID_PARAMS);
    assert_eq!(e.message, "trace_line: start and end must be [x, y, z]");
}
