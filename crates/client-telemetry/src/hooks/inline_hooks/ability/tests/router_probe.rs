//! The router probe against a hand-built `MethodDescription`.

use super::*;

/// An MSVC `std::string` holding `s` inline (`s.len() < 16`).
fn msvc_string(s: &str) -> [u8; 0x1c] {
    let mut o = [0u8; 0x1c];
    o[4..4 + s.len()].copy_from_slice(s.as_bytes());
    o[0x14..0x18].copy_from_slice(&(s.len() as u32).to_le_bytes());
    o[0x18..0x1c].copy_from_slice(&15u32.to_le_bytes());
    o
}

/// A send the router reached claims the waiting press and goes to the
/// join; with no bag (a null `args`) every argument is reported missing
/// rather than read. The bag readers are game code, so this test cannot
/// call them; their decode is covered against synthetic bags.
#[test]
fn a_reached_router_call_is_sent_with_its_press() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let names = [msvc_string("AbilityID"), msvc_string("TargetID")];
    let mut desc = vec![0u8; 0x50];
    desc[..0x1c].copy_from_slice(&msvc_string("useAbility"));
    let begin = names.as_ptr() as u32;
    desc[0x34..0x38].copy_from_slice(&begin.to_le_bytes());
    desc[0x38..0x3c].copy_from_slice(&(begin + 2 * 0x1c).to_le_bytes());
    desc[0x44..0x48].copy_from_slice(&0x44u32.to_le_bytes());
    desc[0x48..0x4c].copy_from_slice(&u32::MAX.to_le_bytes());

    with_pending(|t| {
        t.push(crate::hooks::ability_trace::press::PendingSend {
            press_id: 77,
            source: Source::Hotbar,
            method: "useAbility",
            ability_id: None,
            at_ms: now_ms(),
            ttl_ms: 5_000,
        })
    });
    let _ = capture::take();
    let probe = super::super::route::RouteProbe::begin(
        std::ptr::null_mut(),
        desc.as_mut_ptr() as *mut c_void,
        std::ptr::null_mut(),
    )
    .expect("useAbility is allowlisted");
    // What startEntityMessage's detour does inside the router.
    super::super::route::test_access::mark_reached();
    probe.finish();
    let outs = capture::take();
    assert_eq!(outs.len(), 1, "{outs:#?}");
    let f = &outs[0].fields;
    assert_eq!(outs[0].target, crate::hooks::ability_trace::TARGET_SENT);
    assert_eq!(field(f, "method"), Some(&json!("useAbility")));
    assert_eq!(field(f, "press_id"), Some(&json!(77)));
    assert_eq!(field(f, "msg_id"), Some(&json!(0x44)));
    assert_eq!(
        field(f, "args_missing"),
        Some(&json!(["AbilityID", "TargetID"]))
    );
    let send_id = field(f, "send_id").and_then(|v| v.as_u64()).unwrap();
    with_joiner(|j| {
        assert!(j.tag_bundle(0x77));
        let tags = j.take_bundle(0x77).unwrap();
        assert!(tags.iter().any(|t| u64::from(t.send_id) == send_id));
    });

    // A method outside the allowlist costs a name read and nothing else.
    desc[..0x1c].copy_from_slice(&msvc_string("createCharacter"));
    assert!(super::super::route::RouteProbe::begin(
        std::ptr::null_mut(),
        desc.as_mut_ptr() as *mut c_void,
        std::ptr::null_mut(),
    )
    .is_none());
}
