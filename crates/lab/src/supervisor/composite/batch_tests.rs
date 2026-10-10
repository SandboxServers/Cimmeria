use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};

use super::*;
use crate::supervisor::events::fake_bridge::{self, lua_ok};

fn vals(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn references_resolve_whole_with_offsets_and_paths() {
    let v = vals(&[
        ("p", json!(0x1000)),
        ("me", json!({ "position": { "x": 1.5 } })),
        ("hexp", json!("0x2000")),
    ]);
    assert_eq!(resolve(&json!("$p"), &v).unwrap(), json!(0x1000));
    assert_eq!(resolve(&json!("$p+0x270"), &v).unwrap(), json!(0x1270));
    assert_eq!(resolve(&json!("$p-4"), &v).unwrap(), json!(0xffc));
    assert_eq!(resolve(&json!("$hexp+16"), &v).unwrap(), json!(0x2010));
    assert_eq!(resolve(&json!("$me.position.x"), &v).unwrap(), json!(1.5));
    assert_eq!(
        resolve(&json!(["$p", { "deep": "$p+1" }]), &v).unwrap(),
        json!([0x1000, { "deep": 0x1001 }])
    );
}

#[test]
fn references_interpolate_inside_strings() {
    let v = vals(&[("p", json!(4096)), ("n", json!("Labone"))]);
    assert_eq!(
        resolve(&json!("return readu32(${p}) .. '${n}'"), &v).unwrap(),
        json!("return readu32(4096) .. 'Labone'")
    );
}

#[test]
fn bad_references_are_named() {
    let v = vals(&[("s", json!("text"))]);
    assert!(resolve(&json!("$nope"), &v)
        .unwrap_err()
        .contains("no earlier step"));
    assert!(resolve(&json!("$s+4"), &v)
        .unwrap_err()
        .contains("not a number"));
    assert!(resolve(&json!("$s*2"), &v)
        .unwrap_err()
        .contains("letters, digits"));
    assert!(resolve(&json!("${s"), &v).unwrap_err().contains("unclosed"));
}

#[test]
fn floats_are_passed_as_f32_bits_and_doubles_as_two_words() {
    let a = encode_args(&[
        json!(1.5),
        json!(2.0),
        json!(7),
        json!(-1),
        json!("0x10"),
        json!({ "f32": 1 }),
        json!({ "f64": 1.0 }),
    ])
    .unwrap();
    assert_eq!(a[0], json!("0x3fc00000"));
    assert_eq!(
        a[1],
        json!("0x40000000"),
        "2.0 written as a float is a float"
    );
    assert_eq!(a[2], json!(7));
    assert_eq!(a[3], json!(u32::MAX));
    assert_eq!(a[4], json!("0x10"));
    assert_eq!(a[5], json!("0x3f800000"));
    // 1.0f64 = 0x3FF00000_00000000: low word first.
    assert_eq!(&a[6..], &[json!("0x00000000"), json!("0x3ff00000")]);
    assert!(encode_args(&[json!(true)]).is_err());
}

#[test]
fn reads_decode_little_endian() {
    assert_eq!(
        decode_read("78563412", ReadAs::U32).unwrap(),
        json!(0x12345678)
    );
    assert_eq!(decode_read("ffffffff", ReadAs::I32).unwrap(), json!(-1));
    assert_eq!(decode_read("0000c03f", ReadAs::F32).unwrap(), json!(1.5));
    assert_eq!(decode_read("01000200", ReadAs::U16).unwrap(), json!([1, 2]));
    assert_eq!(decode_read("abcd", ReadAs::Hex).unwrap(), json!("abcd"));
    assert!(decode_read("ab", ReadAs::U32).is_err());
}

#[test]
fn steps_parse_and_bad_steps_name_themselves() {
    let s = parse_steps(&[
        json!({ "id": "p", "op": "mem_read", "addr": "0x1000", "as": "u32" }),
        json!({ "op": "wait", "frames": 2 }),
    ])
    .unwrap();
    assert_eq!(s[0].id, "p");
    assert_eq!(s[1].id, "2", "an unnamed step is named by position");
    // Regression guard (review of #1309): a 4-byte read with no `as` is a
    // u32, so a pointer chain adds to a number; other lengths stay hex.
    let d = parse_steps(&[
        json!({ "op": "mem_read", "addr": "0x1000" }),
        json!({ "op": "mem_read", "addr": "0x1000", "len": 16 }),
    ])
    .unwrap();
    assert!(matches!(
        d[0].op,
        Op::MemRead {
            read_as: ReadAs::U32,
            ..
        }
    ));
    assert!(matches!(
        d[1].op,
        Op::MemRead {
            read_as: ReadAs::Hex,
            ..
        }
    ));
    let e = parse_steps(&[json!({ "id": "x", "op": "mem_read", "as": "u32" })]).unwrap_err();
    assert!(e.starts_with("step x:") && e.contains("addr"), "{e}");
    assert!(parse_steps(&[json!({ "op": "wait" })])
        .unwrap_err()
        .contains("frames"));
    assert!(parse_steps(&[json!({ "op": "teleport" })])
        .unwrap_err()
        .contains("unknown op"));
    let dup = [
        json!({ "id": "a", "op": "wait", "ms": 1 }),
        json!({ "id": "a", "op": "wait", "ms": 1 }),
    ];
    assert!(parse_steps(&dup).unwrap_err().contains("two steps"));
    assert!(parse_steps(&[]).is_err());
}

/// Against the fake bridge: a pointer read feeds the next read's address,
/// a native call gets f32 bits, and a failing step stops the batch.
#[tokio::test]
async fn a_batch_chains_reads_and_stops_on_error() {
    let seen: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
    let log = seen.clone();
    let sup = fake_bridge::supervisor(Arc::new(move |method: &str, p: &Value| {
        log.lock().unwrap().push((method.to_string(), p.clone()));
        match (method, p["addr"].as_str()) {
            ("mem_read", Some("0x1000")) => Ok(json!({ "hex": "00200000" })),
            ("mem_read", Some("0x2270")) => Ok(json!({ "hex": "0000c03f" })),
            ("call_native", _) => Ok(json!({ "ret_u32": 9, "ret_f32": 0.5 })),
            ("lua_eval", _) => Ok(lua_ok(&["fine"])),
            _ => Err("bad address".into()),
        }
    }))
    .await;
    let steps = parse_steps(&[
        json!({ "id": "p", "op": "mem_read", "addr": "0x1000", "as": "u32" }),
        json!({ "id": "x", "op": "mem_read", "addr": "$p+0x270", "as": "f32" }),
        json!({ "id": "c", "op": "call_native", "addr": "0x400000", "args": ["$p", 1.5], "ret": "f32" }),
        json!({ "id": "l", "op": "lua", "chunk": "return '${x}'" }),
        json!({ "id": "bad", "op": "mem_read", "addr": "0x9" }),
        json!({ "id": "never", "op": "lua", "chunk": "return 1" }),
    ])
    .unwrap();
    let out = sup.batch(&steps, true).await;
    assert_eq!(out.values["p"], json!(0x2000));
    assert_eq!(out.values["x"], json!(1.5));
    assert_eq!(out.values["c"], json!(0.5));
    assert_eq!(out.values["l"], json!("fine"));
    assert!(out.values["bad"]["error"]
        .as_str()
        .unwrap()
        .contains("bad address"));
    assert!(!out.values.contains_key("never"));
    assert_eq!(out.stopped_at.as_deref(), Some("bad"));
    let calls = seen.lock().unwrap().clone();
    let native = calls.iter().find(|(m, _)| m == "call_native").unwrap();
    assert_eq!(native.1["args"], json!([0x2000, "0x3fc00000"]));
    assert_eq!(native.1["ret"], json!("f32"));
    let lua = calls.iter().find(|(m, _)| m == "lua_eval").unwrap();
    assert_eq!(lua.1["chunk"], json!("return '1.5'"));

    // Without stop_on_error the batch runs on past the failure.
    let on = sup.batch(&steps, false).await;
    assert!(on.stopped_at.is_none());
    assert_eq!(on.values["never"], json!("fine"));
}

/// Regression guard (review of #1309): a batch stops at the next step
/// once its lease is taken over, and nothing after it reaches the client.
#[tokio::test]
async fn a_batch_stops_when_its_lease_is_taken_over() {
    use crate::lease::permit::{scope, Permit};
    use crate::lease::{AcquireRequest, LeaseBook};
    let book = Arc::new(LeaseBook::default());
    let lease = book
        .acquire(AcquireRequest {
            owner: "a".into(),
            purpose: "batch".into(),
            ..Default::default()
        })
        .unwrap();
    let calls = Arc::new(Mutex::new(0u32));
    let (b2, c2) = (book.clone(), calls.clone());
    let sup = fake_bridge::supervisor(Arc::new(move |_m: &str, _p: &Value| {
        *c2.lock().unwrap() += 1;
        // Another session takes the lab over during the first step.
        let _ = b2.acquire(AcquireRequest {
            owner: "b".into(),
            purpose: "takeover".into(),
            force: true,
            reason: Some("test".into()),
            ..Default::default()
        });
        Ok(json!({ "hex": "01000000" }))
    }))
    .await;
    let steps = parse_steps(&[
        json!({ "id": "one", "op": "mem_read", "addr": "0x1000" }),
        json!({ "id": "two", "op": "mem_read", "addr": "0x2000" }),
        json!({ "id": "three", "op": "lua", "chunk": "return 1" }),
    ])
    .unwrap();
    let permit = Permit::Lease {
        book: book.clone(),
        id: lease.lease_id,
    };
    let out = scope(permit, sup.batch(&steps, true)).await;
    assert_eq!(out.values["one"], json!(1));
    let e = out.values["two"]["error"].as_str().unwrap();
    assert!(e.contains("lease revoked"), "{e}");
    assert_eq!(out.stopped_at.as_deref(), Some("two"));
    assert_eq!(
        *calls.lock().unwrap(),
        1,
        "nothing after the takeover reached the bridge"
    );
}
