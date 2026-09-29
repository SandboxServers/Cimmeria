//! Turns what the detours read into events: the target, level, gate and
//! fields of `client.mercury.packet_in`, `client.mercury.fragment` and the
//! end of `client.mercury.bundle`, plus the `client.mercury.error` that
//! accompanies every non-happy one.
//!
//! Keeping this apart from the detours means the volume rule is a tested
//! function, not a property of hook plumbing: a non-happy outcome is always
//! [`Gate::Always`], so no throttle can hide it.

use serde_json::json;

use super::bundle::{BundleTrace, Exit};
use super::fragment::{self, GroupView, Outcome};
use super::tail::{self, Gate, Tail};
use super::window::{self, Disposition, WindowState};
use super::Fields;

/// What `queueAckForPacket` did, noted by its detour for the packet filter's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowNote {
    pub before: WindowState,
    pub after: WindowState,
    /// The channel's reorder window size.
    pub window: u32,
}

impl WindowNote {
    pub(crate) fn disposition(&self, seq: u32) -> Disposition {
        window::classify(seq, self.before, self.after, self.window)
    }
}

/// One event, ready to emit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Report {
    pub target: &'static str,
    pub level: &'static str,
    pub gate: Gate,
    pub fields: Fields,
    /// The reason for the paired `client.mercury.error`, if not happy.
    pub error: Option<String>,
}

/// `client.mercury.error` fields: the stage, the reason, and the event's own
/// fields so the error stands alone.
pub(crate) fn error_fields(stage: &'static str, reason: &str, of: &Fields) -> Fields {
    let mut f: Fields = vec![("stage", json!(stage)), ("reason", json!(reason))];
    f.extend(of.iter().filter(|(k, _)| *k != "flag_names").cloned());
    f
}

/// `client.mercury.packet_in` for the datagram `tail`, given the window note
/// (reliable packets) and the packet filter's `result`.
pub(crate) fn packet(
    tail: &Tail,
    note: Option<&WindowNote>,
    result: i32,
    bad_packets: Option<u32>,
    group_in_flight: bool,
) -> Report {
    let disposition = tail.seq.zip(note).map(|(seq, n)| (n.disposition(seq), n));
    let mut error: Option<String> = tail.fault.map(|f| f.reason().to_string());
    if error.is_none() {
        if let Some((d, _)) = &disposition {
            if !d.is_happy() {
                error = Some(d.name().to_string());
            }
        }
    }
    // A fault the packet filter itself named already explains the result;
    // otherwise a non-zero result on a packet that is neither ACK-only nor
    // already explained is reported as such (fragment drops surface here).
    if error.is_none() && result != 0 && !tail.ack_only {
        error = Some(format!("result_0x{:08x}", result as u32));
    }
    // The client bumps `Nub+0xf8` on almost every drop path, so a rise the
    // checks above did not explain still marks the packet as dropped.
    let bad = bad_packets.filter(|&n| n > 0);
    if error.is_none() && bad.is_some() && !tail.ack_only {
        error = Some("bad_packet_counter".to_string());
    }
    let happy = error.is_none();
    let buffered = disposition.is_some_and(|(d, _)| d.is_buffered());
    let mut fields = tail::fields(tail);
    if let Some((d, n)) = &disposition {
        fields.extend(window::fields(*d, n.before, n.after));
    }
    if tail.ack_only {
        fields.push(("ack_only", json!(true)));
    }
    fields.push(("result", json!(result)));
    if let Some(n) = bad {
        fields.push(("bad_packets_delta", json!(n)));
    }
    let level = if !happy {
        "warn"
    } else if buffered || tail.is_fragment() {
        "info"
    } else {
        "debug"
    };
    Report {
        target: "client.mercury.packet_in",
        level,
        gate: tail::gate(tail, happy, buffered, group_in_flight),
        fields,
        error,
    }
}

/// `client.mercury.fragment` for one fragment. Always [`Gate::Always`].
pub(crate) fn fragment(
    tail: &Tail,
    outcome: &Outcome,
    pre: Option<&GroupView>,
    post: Option<&GroupView>,
) -> Report {
    let happy = outcome.is_happy();
    Report {
        target: "client.mercury.fragment",
        level: if happy { "info" } else { "warn" },
        gate: Gate::Always,
        fields: fragment::fields(tail, outcome, pre, post),
        error: (!happy).then(|| outcome.name().to_string()),
    }
}

/// The start of `client.mercury.bundle`, only for a bundle assembled from
/// several packets (a single-packet bundle has no start worth a line).
pub(crate) fn bundle_start(trace: &BundleTrace) -> Option<Report> {
    (trace.packets() > 1).then(|| Report {
        target: "client.mercury.bundle",
        level: "info",
        gate: Gate::Always,
        fields: trace.start_fields(),
        error: None,
    })
}

/// The end of `client.mercury.bundle`. Non-happy and assembled bundles are
/// [`Gate::Always`]; a clean single-packet bundle is throttled.
pub(crate) fn bundle_end(
    trace: &BundleTrace,
    result: i32,
    dispatched: Option<u32>,
    aborted: Option<u32>,
    next_id: Option<u8>,
) -> Report {
    let happy = trace.is_happy(result);
    let error = (!happy).then(|| match &trace.fault {
        Some((_, f)) => f.reason().to_string(),
        None => match super::bundle::exit_of(result) {
            Exit::Clean => "bytes_left_unconsumed".to_string(),
            other => other.name().to_string(),
        },
    });
    Report {
        target: "client.mercury.bundle",
        level: if !happy {
            "warn"
        } else if trace.packets() > 1 {
            "info"
        } else {
            "debug"
        },
        gate: if !happy || trace.packets() > 1 {
            Gate::Always
        } else {
            Gate::Throttled
        },
        fields: trace.end_fields(result, dispatched, aborted, next_id),
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::mercury_recv::bundle::{
        IterState, PacketView, Unpacked, RESULT_UNKNOWN_MESSAGE_ID,
    };
    use crate::hooks::mercury_recv::tail::parse;

    fn dg(flags: u8, seq: u32, frag: Option<(u32, u32)>) -> Tail {
        let mut d = vec![flags, 1, 2, 3];
        if let Some((a, b)) = frag {
            d.extend(a.to_le_bytes());
            d.extend(b.to_le_bytes());
        }
        d.extend(seq.to_le_bytes());
        parse(&d)
    }

    fn st(in_seq_at: u32, buffered: u32) -> WindowState {
        WindowState {
            in_seq_at,
            buffered,
        }
    }

    fn get(r: &Report, k: &str) -> Option<serde_json::Value> {
        r.fields
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
    }

    #[test]
    fn ordinary_in_order_traffic_is_throttled_debug() {
        let t = dg(0x58, 70, None);
        let n = WindowNote {
            before: st(70, 0),
            after: st(71, 0),
            window: 512,
        };
        let r = packet(&t, Some(&n), 0, None, false);
        assert_eq!((r.gate, r.level), (Gate::Throttled, "debug"));
        assert_eq!(r.error, None);
        assert_eq!(get(&r, "disposition"), Some(json!("delivered")));
    }

    /// The volume rule: a fragment, an out-of-order packet and anything
    /// while a group is in flight are never throttled.
    #[test]
    fn fragments_buffered_and_in_flight_packets_bypass_the_throttle() {
        let frag = dg(0x78, 70, Some((70, 84)));
        let n = WindowNote {
            before: st(70, 0),
            after: st(71, 0),
            window: 512,
        };
        assert_eq!(packet(&frag, Some(&n), 0, None, false).gate, Gate::Always);
        let t = dg(0x58, 75, None);
        let buffered = WindowNote {
            before: st(74, 0),
            after: st(74, 1),
            window: 512,
        };
        let r = packet(&t, Some(&buffered), 0, None, false);
        assert_eq!((r.gate, r.level), (Gate::Always, "info"));
        let n2 = WindowNote {
            before: st(75, 0),
            after: st(76, 0),
            window: 512,
        };
        assert_eq!(packet(&t, Some(&n2), 0, None, true).gate, Gate::Always);
    }

    /// A packet the window dropped (already ACKed) or the filter rejected is
    /// a warn with a reason, and never throttled.
    #[test]
    fn a_dropped_packet_is_a_warn_with_a_reason_and_no_throttle() {
        let t = dg(0x58, 60, None);
        let stale = WindowNote {
            before: st(74, 0),
            after: st(74, 0),
            window: 512,
        };
        let r = packet(&t, Some(&stale), 0, None, false);
        assert_eq!((r.gate, r.level), (Gate::Always, "warn"));
        assert_eq!(r.error.as_deref(), Some("old_duplicate"));
        // A fragment the reassembler dropped surfaces as a non-zero result.
        let f = dg(0x78, 80, Some((70, 84)));
        let ok = WindowNote {
            before: st(80, 0),
            after: st(81, 0),
            window: 512,
        };
        let r = packet(&f, Some(&ok), -4, None, true);
        assert_eq!(r.error.as_deref(), Some("result_0xfffffffc"));
        assert_eq!(r.gate, Gate::Always);
        // A footer fault carries its own reason.
        let bad = parse(&[0x80, 1]);
        assert_eq!(
            packet(&bad, None, -4, None, false).error.as_deref(),
            Some("bad_flags")
        );
    }

    #[test]
    fn an_ack_only_packet_is_ordinary_traffic() {
        let mut d = vec![0x04u8];
        d.extend(9u32.to_le_bytes());
        d.push(1);
        let t = parse(&d);
        let r = packet(&t, None, -4, None, false);
        assert_eq!(r.error, None);
        assert_eq!(r.gate, Gate::Throttled);
    }

    #[test]
    fn a_non_happy_fragment_reports_an_error_reason() {
        let t = dg(0x78, 90, Some((90, 95)));
        let o = Outcome::MangledFooters { open_last: 84 };
        let r = fragment(&t, &o, None, None);
        assert_eq!((r.level, r.gate), ("warn", Gate::Always));
        assert_eq!(r.error.as_deref(), Some("mangled_footers"));
        let ok = fragment(&t, &Outcome::Added { remaining: 3 }, None, None);
        assert_eq!(ok.level, "info");
        assert_eq!(ok.error, None);
    }

    fn message_trace() -> BundleTrace {
        let c = vec![
            PacketView {
                ptr: 0x1000,
                len: 1401,
                seq: 70,
            },
            PacketView {
                ptr: 0x1100,
                len: 1401,
                seq: 71,
            },
        ];
        let mut t = BundleTrace::new(c);
        t.on_unpack(
            IterState {
                packet: 0x1000,
                packet_len: 1401,
                cursor: 1,
            },
            Unpacked {
                msg_id: 0x80,
                flag: 0,
                body_offset: 4,
                len: 1396,
                decoded_len: 1396,
            },
            Some(crate::hooks::mercury_recv::bundle::Element { style: 1, param: 2 }),
        );
        t
    }

    #[test]
    fn a_bundle_that_stopped_early_is_warn_and_paired_with_an_error() {
        let t = message_trace();
        let r = bundle_end(&t, RESULT_UNKNOWN_MESSAGE_ID, Some(1), Some(1), Some(0xEE));
        assert_eq!((r.level, r.gate), ("warn", Gate::Always));
        assert_eq!(r.error.as_deref(), Some("unknown_message_id"));
        let e = error_fields("bundle", "unknown_message_id", &r.fields);
        assert_eq!(e[0], ("stage", json!("bundle")));
        assert_eq!(e[1], ("reason", json!("unknown_message_id")));
    }

    #[test]
    fn a_clean_assembled_bundle_is_always_reported_at_info() {
        let mut t = message_trace();
        // Consume the second packet's bytes too.
        t.on_unpack(
            IterState {
                packet: 0x1100,
                packet_len: 1401,
                cursor: 1,
            },
            Unpacked {
                msg_id: 0x80,
                flag: 0,
                body_offset: 4,
                len: 1397,
                decoded_len: 1397,
            },
            Some(crate::hooks::mercury_recv::bundle::Element { style: 1, param: 2 }),
        );
        let r = bundle_end(&t, 0, Some(2), Some(0), None);
        assert_eq!((r.level, r.gate), ("info", Gate::Always));
        assert_eq!(r.error, None);
        assert_eq!(get(&r, "exit"), Some(json!("clean_end")));
        assert_eq!(get(&r, "messages"), Some(json!(2)));
    }
}
