//! The mark that ties a BigWorld message to the `unpack` fault before it
//! (TG-CLI-02): `client.bw.message` rows raised on the game thread after a
//! `client.mercury.unpack_fault`, and before the next bundle, carry
//! `after_unpack_fault = true` and the faulting packet's `seq`.
//!
//! # Why a thread-local is enough here
//!
//! `Bundle::iterator::unpack` runs inside `Nub::processOrderedPacket`'s
//! message loop on the game thread (see
//! `docs/reverse-engineering/findings/client-mercury-receive-path.md`, the
//! function map). The messages a fault causes are raised on that same
//! thread and inside the same loop: `processOrderedPacket`'s own
//! "Discarding bundle due to corrupted header" `ERROR_MSG`, and anything a
//! handler dispatched from the loop logs. `DebugMsgHelper::message` runs on
//! the caller's thread, so a per-thread mark reaches exactly those, and a
//! message from the network thread (resend scan, `processFilteredPacket`)
//! is never tagged. The `(packet, seq)` table that TG-MER-12 needed exists
//! because the *network* thread's stash cannot reach `unpack`; this mark
//! never crosses a thread.
//!
//! # Lifetime
//!
//! `processOrderedPacket` clears the mark on entry, so a fault tags only
//! its own bundle's later messages. The deadline bounds the case where the
//! faulting bundle was the frame's last: the game thread's other logging
//! for the rest of that frame is not about the fault.
//!
//! A message raised *inside* the faulting `unpack` call (its own
//! "Error unpacking header length" `ERROR_MSG`) is not tagged: the fault
//! is only known once the original returns.

use std::cell::Cell;
use std::time::{Duration, Instant};

use serde_json::json;

use super::{Fields, SEQ_MASK, SEQ_UNSET};

/// How long after a fault a message on the same thread still counts as
/// following it, when no later bundle clears the mark first.
pub(crate) const AFTER_FAULT_WINDOW: Duration = Duration::from_millis(250);

#[derive(Clone, Copy)]
struct Mark {
    /// The faulting packet's sequence number, if it had a valid one.
    seq: Option<u32>,
    deadline: Instant,
}

thread_local! {
    /// The last `unpack` fault on this thread, until the next bundle or
    /// the deadline.
    static MARK: Cell<Option<Mark>> = const { Cell::new(None) };
}

/// Record an `unpack` fault on this thread for the packet whose stored
/// sequence number (`Packet+0x44`) is `seq`.
pub(crate) fn mark_fault(seq: Option<u32>) {
    mark_fault_at(seq, Instant::now());
}

fn mark_fault_at(seq: Option<u32>, now: Instant) {
    let seq = seq.filter(|&s| s != SEQ_UNSET && s & SEQ_MASK == s);
    let _ = MARK.try_with(|m| {
        m.set(Some(Mark {
            seq,
            deadline: now + AFTER_FAULT_WINDOW,
        }))
    });
}

/// A new bundle starts on this thread: an earlier fault no longer applies.
pub(crate) fn clear() {
    let _ = MARK.try_with(|m| m.set(None));
}

/// The tag fields for a message raised now on this thread: empty unless an
/// `unpack` fault came first. The unmarked case is one thread-local read,
/// with no clock read, so the sink's hot path stays cheap.
pub(crate) fn tag_fields() -> Fields {
    match MARK.try_with(Cell::get).ok().flatten() {
        None => Vec::new(),
        Some(mark) => tag_fields_at(mark, Instant::now()),
    }
}

fn tag_fields_at(mark: Mark, now: Instant) -> Fields {
    if now > mark.deadline {
        clear();
        return Vec::new();
    }
    let mut f: Fields = vec![("after_unpack_fault", json!(true))];
    if let Some(seq) = mark.seq {
        f.push(("seq", json!(seq)));
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(f: &Fields, key: &str) -> Option<serde_json::Value> {
        f.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }

    /// A message after a fault on the same thread is tagged with the
    /// faulting seq; one on another thread is not.
    #[test]
    fn a_fault_tags_its_own_threads_later_messages_only() {
        clear();
        assert!(tag_fields().is_empty(), "nothing before a fault");
        mark_fault(Some(4242));
        let f = tag_fields();
        assert_eq!(get(&f, "after_unpack_fault"), Some(json!(true)));
        assert_eq!(get(&f, "seq"), Some(json!(4242)));

        let other = std::thread::spawn(tag_fields).join().unwrap();
        assert!(other.is_empty(), "another thread is never tagged");

        // Still tagged on this thread: the mark lasts until the next bundle.
        assert_eq!(get(&tag_fields(), "seq"), Some(json!(4242)));
        clear();
    }

    /// The next bundle clears the mark.
    #[test]
    fn the_next_bundle_clears_the_mark() {
        mark_fault(Some(7));
        clear();
        assert!(tag_fields().is_empty());
    }

    /// Past the deadline the mark no longer applies, and is dropped.
    #[test]
    fn the_mark_expires() {
        let t0 = Instant::now();
        mark_fault_at(Some(7), t0);
        let mark = MARK.with(Cell::get).expect("marked");
        assert!(!tag_fields_at(mark, t0 + AFTER_FAULT_WINDOW).is_empty());
        let late = t0 + AFTER_FAULT_WINDOW + Duration::from_millis(1);
        assert!(tag_fields_at(mark, late).is_empty());
        assert!(MARK.with(Cell::get).is_none(), "an expired mark is cleared");
    }

    /// A packet without a valid stored seq still tags its messages, without
    /// a `seq`.
    #[test]
    fn an_unset_seq_is_left_out() {
        mark_fault(Some(SEQ_UNSET));
        let f = tag_fields();
        assert_eq!(get(&f, "after_unpack_fault"), Some(json!(true)));
        assert_eq!(get(&f, "seq"), None);
        mark_fault(None);
        assert_eq!(get(&tag_fields(), "seq"), None);
        clear();
    }
}
