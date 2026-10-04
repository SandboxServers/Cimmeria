//! One inbound ability method, as `client.ability.recv`.
//!
//! The `onEntityMethod` detour (`0x00dd2b80`, network thread) calls
//! [`report`] before the original runs, with the message id, who it is
//! for, and a reader for the message's argument bytes. Those bytes are the
//! `[cursor, end)` of the `MemoryIStream` the game is about to read
//! (`docs/reverse-engineering/findings/ability-client-hook-anchors.md`
//! § AB-C3, Seam A); the hook reads them without moving the cursor.
//!
//! Fields: `method`, `method_index`, `entity_id`, `msg_id`, `len` (argument
//! bytes), `path` (`delivered`, `local_player`, `queued`: a queued message
//! is applied later, when its entity enters the world), then every argument
//! by its snake-case name (`recv_methods`). Array arguments carry
//! `<name>_count`. `cast_id` is the server's per-cast id where the wire
//! has it: `onEffectResults.EffectID`, and `onSequence.InstanceId` when
//! non-zero. A payload that does not decode as its `.def` says is a `warn`
//! with `decode_error`.

use serde_json::{json, Value};

use super::recv_methods::{self, Receiver, RecvMethod, CHAN_FEEDBACK};
use super::wire_decode;
use crate::hooks::entity_trace::Fields;

/// Argument bytes read from the stream at most. The largest payload here
/// is a full `onKnownAbilitiesUpdate` (a few hundred ids); anything longer
/// is decoded from its first bytes, and `len` says how long it was.
pub(crate) const MAX_ARG_BYTES: usize = 4096;

/// One message, as the detour saw it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Message {
    /// The `onEntityMethod` message id argument.
    pub msg_id: u32,
    /// The receiving entity.
    pub entity_id: i32,
    /// Who it is for.
    pub receiver: Receiver,
    /// `client.mercury.entity_method`'s delivery path.
    pub path: &'static str,
    /// Argument bytes in the stream (`end - cursor`), before any cap.
    pub len: u32,
}

/// The decoded event, before the throttle: `(level, throttle key, fields)`.
/// `None` when the message is not one of ours, or is an
/// `onPlayerCommunication` on a channel other than feedback. One that does
/// not decode as far as its channel is a `warn` with no decoded fields.
pub(crate) fn event(msg: &Message, bytes: &[u8]) -> Option<(&'static str, String, Fields)> {
    let (method, skip) = recv_methods::resolve(msg.msg_id, msg.receiver, bytes.first().copied())?;
    let args = bytes.get(skip..).unwrap_or(&[]);
    let (mut decoded, err) = wire_decode::decode(method.args, args);
    if method.index == recv_methods::ON_PLAYER_COMMUNICATION {
        match value_of(&decoded, "channel").and_then(Value::as_u64) {
            Some(c) if c == u64::from(CHAN_FEEDBACK) => {}
            // Another channel: players' chat, not ours to report.
            Some(_) => return None,
            // The decode stopped before the channel: a layout fault, which
            // is reported, but without the speaker or anything else read,
            // since the line may be chat.
            None if err.is_some() => decoded.clear(),
            None => return None,
        }
    }
    let mut f: Fields = vec![
        ("method", json!(method.name)),
        ("method_index", json!(method.index)),
        ("entity_id", json!(msg.entity_id)),
        ("msg_id", json!(msg.msg_id)),
        ("len", json!(msg.len)),
        ("path", json!(msg.path)),
    ];
    if let Some(cast) = cast_id(method, &decoded) {
        f.push(("cast_id", cast));
    }
    f.extend(decoded);
    let capped = bytes.len() < msg.len as usize;
    if capped {
        f.push(("bytes_capped", json!(bytes.len())));
    }
    // A payload cut at the cap is expected to stop early; anything else
    // that does not decode as its `.def` says is a finding.
    let level = match err {
        Some(e) => {
            f.push(("decode_error", json!(e.as_text())));
            if let Some(n) = e.trailing() {
                f.push(("trailing_bytes", json!(n)));
            }
            if capped {
                "info"
            } else {
                "warn"
            }
        }
        None => "info",
    };
    Some((level, format!("recv:{}", method.name), f))
}

fn value_of<'a>(f: &'a [(&'static str, Value)], key: &str) -> Option<&'a Value> {
    f.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}

/// The server's `cast_id` for this message, where the wire carries it.
fn cast_id(method: &RecvMethod, decoded: &[(&'static str, Value)]) -> Option<Value> {
    match method.index {
        recv_methods::ON_EFFECT_RESULTS => value_of(decoded, "effect_id").cloned(),
        recv_methods::ON_SEQUENCE => value_of(decoded, "instance_id")
            .filter(|v| v.as_i64().is_some_and(|n| n != 0))
            .cloned(),
        _ => None,
    }
}

/// Emit the event for one message through the per-name bucket.
/// `read(n)` returns up to `n` argument bytes; it runs only for a message
/// id that can be ours, and the full read only once the sub-index (for an
/// extended id) says it is.
pub(crate) fn report(
    msg: &Message,
    read: impl Fn(usize) -> Option<Vec<u8>>,
) -> Option<(&'static str, Fields)> {
    if !recv_methods::may_be_wanted(msg.msg_id) {
        return None;
    }
    let first = if msg.len > 0 {
        read(1)?.first().copied()
    } else {
        None
    };
    recv_methods::resolve(msg.msg_id, msg.receiver, first)?;
    let bytes = read((msg.len as usize).min(MAX_ARG_BYTES))?;
    let (level, key, fields) = event(msg, &bytes)?;
    // A decode failure is evidence and bypasses the bucket; the governor
    // keeps it as a warn.
    if level == "warn" {
        return Some((level, fields));
    }
    super::admit(&key, || fields).map(|f| (level, f))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(msg_id: u32, receiver: Receiver, len: usize) -> Message {
        Message {
            msg_id,
            entity_id: 4242,
            receiver,
            path: "delivered",
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
        assert_eq!((level, key.as_str()), ("info", "recv:onEffectResults"));
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
        assert_eq!(key, "recv:onStateFieldUpdate");
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
        assert!(report(&msg(2, Receiver::Player, 16), read).is_none());
        assert!(reads.borrow().is_empty());
        // Extended, sub-index 0: method 61, not ours.
        assert!(report(&msg(61, Receiver::Player, 16), read).is_none());
        assert_eq!(*reads.borrow(), vec![1]);
        reads.borrow_mut().clear();
        let _ = report(&msg(19, Receiver::Other, 16), read);
        assert_eq!(*reads.borrow(), vec![1, 16]);
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
}
