use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::bounded;
use serde_json::{json, Value};

use super::Meta;
use crate::bridge::tick::TICK_WORK;
use crate::bridge::{
    dispatch, enqueue, start, transport, PendingRequest, RpcRequest, QUEUE_CAPACITY,
};
use crate::session::LabConfig;

/// Tests that read `TICK_WORK` serialise on this and assert on differences.
pub(crate) static SERIAL: Mutex<()> = Mutex::new(());

fn cfg(token: &str) -> LabConfig {
    LabConfig {
        bind: "127.0.0.1".into(),
        port: 0,
        token: token.to_string(),
    }
}

#[test]
fn drained_responses_carry_meta() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let token = "1".repeat(64);
    let h = start(cfg(&token)).unwrap();
    let addr = h.local_addr();
    let stop = Arc::new(AtomicBool::new(false));
    let drain_stop = stop.clone();
    let drainer = std::thread::spawn(move || {
        while !drain_stop.load(Ordering::SeqCst) {
            h.drain();
            std::thread::sleep(Duration::from_millis(2));
        }
    });

    let mut s = TcpStream::connect(addr).unwrap();
    transport::write_frame(&mut s, &transport::auth_frame_body(&token)).unwrap();
    transport::read_frame(&mut s).unwrap();
    transport::write_frame(&mut s, br#"{"jsonrpc":"2.0","id":5,"method":"heartbeat"}"#).unwrap();
    let v: Value = serde_json::from_slice(&transport::read_frame(&mut s).unwrap()).unwrap();

    assert!(v["meta"]["frame"].is_u64(), "{v}");
    assert!(v["meta"]["mono_ms"].is_u64(), "{v}");
    assert_eq!(v["meta"]["frame"], v["result"]["tick_count"]);

    stop.store(true, Ordering::SeqCst);
    drainer.join().unwrap();
}

#[test]
fn queue_errors_carry_no_meta() {
    let (tx, _rx) = bounded::<PendingRequest>(QUEUE_CAPACITY);
    let req = |i: i64| RpcRequest {
        id: Value::from(i),
        method: "heartbeat".into(),
        params: Value::Null,
    };
    let mut keep = Vec::new();
    for i in 0..QUEUE_CAPACITY {
        keep.push(enqueue(&tx, req(i as i64)).expect("fits"));
    }
    let overflow = enqueue(&tx, req(999)).expect_err("must overflow");
    assert_eq!(overflow.error.as_ref().unwrap().code, dispatch::QUEUE_FULL);
    let v: Value = serde_json::from_slice(&overflow.to_frame_body()).unwrap();
    assert!(v.get("meta").is_none(), "{v}");
}

#[test]
fn meta_omits_absent_game_t() {
    let v = serde_json::to_value(Meta {
        frame: 1,
        mono_ms: 2,
        game_t: None,
    })
    .unwrap();
    assert!(v.get("game_t").is_none());
    assert_eq!(v["frame"], json!(1));
    let with = serde_json::to_value(Meta {
        frame: 1,
        mono_ms: 2,
        game_t: Some(1.5),
    })
    .unwrap();
    assert_eq!(with["game_t"], json!(1.5));
}

#[test]
fn disconnect_runs_once_per_session() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let token = "2".repeat(64);
    let h = start(cfg(&token)).unwrap();
    let addr = h.local_addr();
    let before = TICK_WORK.load(Ordering::SeqCst);

    // Bad token: the connection ends before authentication, so no hub work.
    let mut bad = TcpStream::connect(addr).unwrap();
    transport::write_frame(&mut bad, &transport::auth_frame_body(&"3".repeat(64))).unwrap();
    assert!(transport::read_frame(&mut bad).is_err());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(TICK_WORK.load(Ordering::SeqCst), before);

    // Authenticated connect then close: both disconnect stubs, once.
    let mut ok = TcpStream::connect(addr).unwrap();
    transport::write_frame(&mut ok, &transport::auth_frame_body(&token)).unwrap();
    transport::read_frame(&mut ok).unwrap();
    drop(ok);
    let deadline = Instant::now() + Duration::from_secs(5);
    while TICK_WORK.load(Ordering::SeqCst) == before && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(TICK_WORK.load(Ordering::SeqCst) - before, 2);
}
