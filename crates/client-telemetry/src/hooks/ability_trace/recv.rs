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
use super::recv_stream::{reason, Skip};
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
    /// Whether the receiver is the local player (throttled apart).
    pub local: bool,
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
    Some((
        level,
        format!("recv:{}:{}", method.name, super::whose(msg.local)),
        f,
    ))
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
///
/// `Ok(None)` is "not ours" (another method, another channel) or a row the
/// bucket suppressed. `Err` is a message that may be ours and was not
/// decoded: the caller reports it as `client.ability.recv_skipped`, so no
/// candidate is ever dropped without a trace.
pub(crate) fn report(
    msg: &Message,
    read: impl Fn(usize) -> Option<Vec<u8>>,
) -> Result<Option<(&'static str, Fields)>, Skip> {
    if !recv_methods::may_be_wanted(msg.msg_id) {
        return Ok(None);
    }
    let first = if msg.len > 0 {
        // A direct id names its method without the byte; an extended one
        // does not.
        let direct = || recv_methods::resolve(msg.msg_id, msg.receiver, None).map(|(m, _)| m);
        let b = read(1).ok_or_else(|| Skip::new(reason::READ_FAILED, msg.msg_id, direct()))?;
        b.first().copied()
    } else {
        None
    };
    // An extended id with no sub-index byte cannot be resolved; for a
    // receiver that can have extended ids it may be ours, so say so.
    if first.is_none()
        && recv_methods::is_extended(msg.msg_id)
        && matches!(msg.receiver, Receiver::Player | Receiver::Unknown)
    {
        return Err(Skip::new(reason::READ_FAILED, msg.msg_id, None));
    }
    let Some((method, _)) = recv_methods::resolve(msg.msg_id, msg.receiver, first) else {
        // A player-only method for an entity the hook could not place (a
        // message queued for an entity not created yet) may be ours.
        if msg.receiver == Receiver::Unknown {
            if let Some((m, _)) = recv_methods::resolve(msg.msg_id, Receiver::Player, first) {
                return Err(Skip::new(reason::RECEIVER_UNKNOWN, msg.msg_id, Some(m)));
            }
        }
        return Ok(None);
    };
    let bytes = read((msg.len as usize).min(MAX_ARG_BYTES))
        .ok_or_else(|| Skip::new(reason::READ_FAILED, msg.msg_id, Some(method)))?;
    let Some((level, key, mut fields)) = event(msg, &bytes) else {
        return Ok(None);
    };
    // AB-C6: join the send this answers and remember the receive for the
    // applied row, before the bucket, so a suppressed row still counts.
    super::timing::annotate_recv(&mut fields, method.name, msg.local, super::now_ms());
    // A decode failure is evidence and bypasses the bucket; the governor
    // keeps it as a warn.
    if level == "warn" {
        return Ok(Some((level, fields)));
    }
    Ok(super::admit(&key, || fields).map(|f| (level, f)))
}

#[cfg(test)]
#[path = "recv_tests.rs"]
mod tests;
