//! `recv` against synthetic wire bytes, as `cimmeria-wire` writes them.

use super::*;

fn msg(msg_id: u32, receiver: Receiver, len: usize) -> Message {
    Message {
        msg_id,
        entity_id: 4242,
        receiver,
        path: "delivered",
        local: false,
        len: len as u32,
    }
}

fn get(f: &Fields, k: &str) -> Value {
    value_of(f, k).cloned().unwrap_or(Value::Null)
}

fn i32s(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|n| n.to_le_bytes()).collect()
}

fn wstr(s: &str) -> Vec<u8> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let mut b = (units.len() as u32).to_le_bytes().to_vec();
    for u in units {
        b.extend_from_slice(&u.to_le_bytes());
    }
    b
}

/// `onEffectResults` as `cimmeria-cell-combat` writes it: the effect
/// id is the cast id, and the result list decodes to rows.
#[test]
fn effect_results_carry_the_cast_id_and_the_rows() {
    let mut b = i32s(&[100, 597, 31337, 200]);
    b.push(2); // ResultCode
    b.extend_from_slice(&2u32.to_le_bytes());
    b.extend_from_slice(&[6]); // StatID
    b.extend_from_slice(&(-250i32).to_le_bytes());
    b.extend_from_slice(&[1, 0]); // DamageCode, StatResultCode
    b.extend_from_slice(&[7]);
    b.extend_from_slice(&40i32.to_le_bytes());
    b.extend_from_slice(&[0, 3]);
    let (level, key, f) = event(&msg(14, Receiver::Other, b.len()), &b).unwrap();
    assert_eq!(
        (level, key.as_str()),
        ("info", "recv:onEffectResults:other")
    );
    assert_eq!(get(&f, "method"), json!("onEffectResults"));
    assert_eq!(get(&f, "entity_id"), json!(4242));
    assert_eq!(get(&f, "cast_id"), json!(31337));
    assert_eq!(get(&f, "source_id"), json!(100));
    assert_eq!(get(&f, "ability_id"), json!(597));
    assert_eq!(get(&f, "target_id"), json!(200));
    assert_eq!(get(&f, "result_code"), json!(2));
    assert_eq!(get(&f, "results_count"), json!(2));
    assert_eq!(get(&f, "results"), json!([[6, -250, 1, 0], [7, 40, 0, 3]]));
}

#[test]
fn timer_updates_decode_every_field() {
    let mut b = i32s(&[597]);
    b.push(5);
    b.extend(i32s(&[100, 31337]));
    b.extend_from_slice(&12.5f32.to_le_bytes());
    b.extend_from_slice(&1234.25f32.to_le_bytes());
    let (_, _, f) = event(&msg(12, Receiver::Player, b.len()), &b).unwrap();
    assert_eq!(get(&f, "timer_id"), json!(597));
    assert_eq!(get(&f, "timer_type"), json!(5));
    assert_eq!(get(&f, "source_id"), json!(100));
    assert_eq!(get(&f, "secondary_id"), json!(31337));
    assert_eq!(get(&f, "total_time"), json!(12.5));
    assert_eq!(get(&f, "complete_time"), json!(1234.25));
    assert_eq!(get(&f, "cast_id"), Value::Null);
}

/// The 26-byte payload `cimmeria_wire::cell::kismet` pins, with an
/// instance id: that is the cast id.
#[test]
fn sequences_take_the_instance_id_as_the_cast_id() {
    let mut b = i32s(&[0x1122_3344, 0x00aa_bbcc, 0x00aa_bbcc]);
    b.push(1);
    b.extend_from_slice(&0.0f32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.push(3);
    b.extend_from_slice(&77i32.to_le_bytes());
    assert_eq!(b.len(), 26);
    let (_, _, f) = event(&msg(1, Receiver::Other, 26), &b).unwrap();
    assert_eq!(get(&f, "sequence_id"), json!(0x1122_3344));
    assert_eq!(get(&f, "nvps_count"), json!(0));
    assert_eq!(get(&f, "view_type"), json!(3));
    assert_eq!(get(&f, "cast_id"), json!(77));
    // InstanceId 0 is "no instance", not a cast.
    b[22..26].copy_from_slice(&0i32.to_le_bytes());
    let (_, _, f) = event(&msg(1, Receiver::Other, 26), &b).unwrap();
    assert_eq!(get(&f, "cast_id"), Value::Null);
}

#[test]
fn sequence_name_value_pairs_decode() {
    let mut b = i32s(&[5, 6, 7]);
    b.push(0);
    b.extend_from_slice(&0.5f32.to_le_bytes());
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend(wstr("Damage"));
    b.extend(wstr("12"));
    b.push(0);
    b.extend(i32s(&[9]));
    let (_, _, f) = event(&msg(1, Receiver::Other, b.len()), &b).unwrap();
    assert_eq!(get(&f, "nvps"), json!([["Damage", "12"]]));
    assert_eq!(get(&f, "instance_id"), json!(9));
}

#[test]
fn stat_updates_decode_as_id_min_current_max_rows() {
    let mut b = 2u32.to_le_bytes().to_vec();
    b.extend(i32s(&[6, 0, 840, 1000, 7, 0, 55, 100]));
    for idx in [20, 21] {
        let (_, _, f) = event(&msg(idx, Receiver::Other, b.len()), &b).unwrap();
        assert_eq!(get(&f, "stats_count"), json!(2));
        assert_eq!(
            get(&f, "stats"),
            json!([[6, 0, 840, 1000], [7, 0, 55, 100]])
        );
    }
}

#[test]
fn state_field_updates_decode() {
    let b = i32s(&[0x22]);
    let (_, key, f) = event(&msg(19, Receiver::Other, 4), &b).unwrap();
    assert_eq!(key, "recv:onStateFieldUpdate:other");
    let mut own = msg(19, Receiver::Player, 4);
    own.local = true;
    assert_eq!(event(&own, &b).unwrap().1, "recv:onStateFieldUpdate:self");
    assert_eq!(get(&f, "state_field"), json!(0x22));
}

/// `onErrorCode` is player method 121: `0xBD`, sub-byte 60.
#[test]
fn error_codes_decode_after_the_sub_index() {
    let mut b = vec![60u8, 1];
    b.extend(i32s(&[597]));
    b.extend_from_slice(&306u16.to_le_bytes());
    let (_, _, f) = event(&msg(61, Receiver::Player, b.len()), &b).unwrap();
    assert_eq!(get(&f, "method"), json!("onErrorCode"));
    assert_eq!(get(&f, "method_index"), json!(121));
    assert_eq!(get(&f, "system_id"), json!(1));
    assert_eq!(get(&f, "instance_id"), json!(597));
    assert_eq!(get(&f, "error_code"), json!(306));
}

#[test]
fn known_abilities_and_trees_decode() {
    let mut b = vec![40u8];
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend(i32s(&[597, 598, 599]));
    let (_, _, f) = event(&msg(61, Receiver::Player, b.len()), &b).unwrap();
    assert_eq!(get(&f, "ability_ids"), json!([597, 598, 599]));
    assert_eq!(get(&f, "ability_ids_count"), json!(3));

    let mut b = vec![80u8];
    b.extend_from_slice(&2u32.to_le_bytes());
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend(i32s(&[10]));
    b.extend_from_slice(&0u32.to_le_bytes());
    let (_, _, f) = event(&msg(61, Receiver::Player, b.len()), &b).unwrap();
    assert_eq!(get(&f, "ability_lists"), json!([[10], []]));
}

/// Only the feedback channel is reported: other channels are players'
/// chat, which is not ability evidence and not ours to upload.
#[test]
fn player_communication_is_feedback_only() {
    let line = |channel: u8| {
        let mut b = wstr("");
        b.push(0);
        b.push(channel);
        b.extend(wstr("[combat] Heal hit for 120"));
        b
    };
    let b = line(CHAN_FEEDBACK);
    let (_, _, f) = event(&msg(28, Receiver::Player, b.len()), &b).unwrap();
    assert_eq!(get(&f, "text"), json!("[combat] Heal hit for 120"));
    assert_eq!(get(&f, "channel"), json!(9));
    for other in [0u8, 3, 8, 12] {
        let b = line(other);
        assert!(event(&msg(28, Receiver::Player, b.len()), &b).is_none());
    }
}

/// A short payload is a `warn` that names the argument it stopped in.
#[test]
fn a_payload_that_does_not_match_the_def_is_a_warn() {
    let b = i32s(&[1, 2]);
    let (level, _, f) = event(&msg(14, Receiver::Other, 8), &b).unwrap();
    assert_eq!(level, "warn");
    assert_eq!(get(&f, "decode_error"), json!("truncated in EffectID"));
    assert_eq!(get(&f, "source_id"), json!(1));
}

/// `report` reads nothing for a message that cannot be ours, reads one
/// byte (the sub-index) for an extended id that is not, and the whole
/// payload only for one that is.
#[test]
fn report_reads_only_what_it_needs() {
    use std::cell::RefCell;
    let reads = RefCell::new(Vec::new());
    let payload = [0u8; 16];
    let read = |n: usize| {
        reads.borrow_mut().push(n);
        Some(payload[..n.min(payload.len())].to_vec())
    };
    assert!(report(&msg(2, Receiver::Player, 16), read)
        .unwrap()
        .is_none());
    assert!(reads.borrow().is_empty());
    // Extended, sub-index 0: method 61, not ours.
    assert!(report(&msg(61, Receiver::Player, 16), read)
        .unwrap()
        .is_none());
    assert_eq!(*reads.borrow(), vec![1]);
    reads.borrow_mut().clear();
    let _ = report(&msg(19, Receiver::Other, 16), read);
    assert_eq!(*reads.borrow(), vec![1, 16]);
}

/// An extended id with no bytes cannot name its method: a `read_failed`
/// skip for a receiver that has extended ids, nothing for one that has
/// none, and no read is attempted.
#[test]
fn an_empty_extended_message_is_a_skip_not_a_silent_drop() {
    let read = |_: usize| -> Option<Vec<u8>> { panic!("nothing to read") };
    for r in [Receiver::Player, Receiver::Unknown] {
        let skip = report(&msg(61, r, 0), read).expect_err("a skip");
        assert_eq!(skip.reason, "read_failed");
        assert!(skip.method.is_none());
    }
    assert!(report(&msg(61, Receiver::Other, 0), read)
        .unwrap()
        .is_none());
}

/// AB-C6: a synthetic cast. The client sent ability 31999; a witnessed
/// `onEffectResults` for it joins nothing, the first local one carries
/// the send's ids and the interval on the client clock, and the next one
/// of the same cast follows the send with no interval.
#[test]
fn the_first_answer_to_a_send_carries_its_interval() {
    use crate::hooks::ability_trace::{now_ms, timing};
    let sent_at = now_ms();
    timing::with_timing(|t| t.note_sent(77_001, Some(77_002), "useAbility", Some(31_999), sent_at));
    let mut b = i32s(&[100, 31_999, 555, 200]);
    b.push(0);
    b.extend_from_slice(&0u32.to_le_bytes());
    let read = |n: usize| Some(b[..n.min(b.len())].to_vec());
    let (_, theirs) = report(&msg(14, Receiver::Other, b.len()), read)
        .unwrap()
        .unwrap();
    assert_eq!(get(&theirs, "send_id"), Value::Null);
    let mine = Message {
        local: true,
        receiver: Receiver::Player,
        ..msg(14, Receiver::Player, b.len())
    };
    let (_, f) = report(&mine, read).unwrap().unwrap();
    assert_eq!(get(&f, "send_id"), json!(77_001));
    assert_eq!(get(&f, "press_id"), json!(77_002));
    assert_eq!(get(&f, "sent_method"), json!("useAbility"));
    let ms = get(&f, "sent_to_recv_ms").as_u64().unwrap();
    assert!(ms <= now_ms() - sent_at, "{ms}");
    assert_eq!(get(&f, "send_reply"), json!("first"));
    let (_, again) = report(&mine, read).unwrap().unwrap();
    assert_eq!(get(&again, "send_id"), json!(77_001));
    assert_eq!(get(&again, "send_reply"), json!("follow_up"));
    assert_eq!(get(&again, "sent_to_recv_ms"), Value::Null);
}

/// An `onPlayerCommunication` cut before its channel is a layout fault:
/// a `warn`, carrying nothing that was decoded (it may be chat).
#[test]
fn a_truncated_player_communication_is_a_sanitized_warn() {
    let mut b = 10u32.to_le_bytes().to_vec(); // Speaker claims 10 chars
    for u in "Bob".encode_utf16() {
        b.extend_from_slice(&u.to_le_bytes());
    }
    let (level, _, f) = event(&msg(28, Receiver::Player, b.len()), &b).unwrap();
    assert_eq!(level, "warn");
    assert_eq!(get(&f, "decode_error"), json!("truncated in Speaker"));
    for k in ["speaker", "speaker_flags", "channel", "text"] {
        assert_eq!(get(&f, k), Value::Null, "{k}");
    }
    assert_eq!(get(&f, "method"), json!("onPlayerCommunication"));
}

/// A known-abilities list longer than the read cap keeps its full count
/// and its first ids, and is not a warn: the cap is expected.
#[test]
fn a_capped_array_keeps_its_count_and_first_ids() {
    let mut full = vec![40u8];
    full.extend_from_slice(&2000u32.to_le_bytes());
    for i in 0..2000i32 {
        full.extend_from_slice(&(1000 + i).to_le_bytes());
    }
    let capped = &full[..MAX_ARG_BYTES];
    let (level, _, f) = event(&msg(61, Receiver::Player, full.len()), capped).unwrap();
    assert_eq!(level, "info");
    assert_eq!(get(&f, "ability_ids_count"), json!(2000));
    let ids = get(&f, "ability_ids");
    assert_eq!(ids.as_array().unwrap().len(), wire_decode::MAX_ELEMENTS);
    assert_eq!(ids[0], json!(1000));
    assert_eq!(get(&f, "bytes_capped"), json!(MAX_ARG_BYTES));
    assert_eq!(get(&f, "decode_error"), json!("truncated in AbilityData"));
}

/// A payload longer than its `.def` is a `warn` with `trailing_bytes`,
/// for every method: no method is allowed extra bytes.
#[test]
fn trailing_bytes_are_a_warn_for_every_method() {
    let mut b = i32s(&[0x22]);
    b.push(7);
    let (level, _, f) = event(&msg(19, Receiver::Other, b.len()), &b).unwrap();
    assert_eq!(level, "warn");
    assert_eq!(get(&f, "decode_error"), json!("trailing_bytes"));
    assert_eq!(get(&f, "trailing_bytes"), json!(1));
    assert_eq!(get(&f, "state_field"), json!(0x22));
    // The 26-byte `onSequence` plus one stray byte.
    let mut s = i32s(&[1, 2, 2]);
    s.push(1);
    s.extend_from_slice(&0.0f32.to_le_bytes());
    s.extend_from_slice(&0u32.to_le_bytes());
    s.push(3);
    s.extend(i32s(&[0]));
    s.push(0);
    let (level, _, f) = event(&msg(1, Receiver::Other, s.len()), &s).unwrap();
    assert_eq!((level, get(&f, "trailing_bytes")), ("warn", json!(1)));
}

/// A payload longer than the cap is decoded from its head and says so.
#[test]
fn a_capped_payload_says_so() {
    let mut b = vec![40u8];
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend(i32s(&[1, 2, 3]));
    let mut m = msg(61, Receiver::Player, b.len());
    m.len = 10_000;
    let (_, _, f) = event(&m, &b).unwrap();
    assert_eq!(get(&f, "bytes_capped"), json!(b.len()));
    assert_eq!(get(&f, "len"), json!(10_000));
}
