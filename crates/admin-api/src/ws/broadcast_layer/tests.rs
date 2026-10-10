//! Bounds on the admin log stream: per-entry truncation and the ring's byte
//! budget. Events go through a real subscriber so the visitor path is what
//! gets tested, not a hand-built `LogEntry`.

use tokio::sync::broadcast;
use tracing_subscriber::layer::SubscriberExt;

use super::*;

/// Run `f` with a subscriber whose only layer is a `BroadcastLayer` feeding
/// a fresh ring, and return that ring.
fn with_layer(f: impl FnOnce()) -> LogBuffer {
    let (tx, _) = broadcast::channel::<LogEntry>(16);
    with_layer_tx(tx, f)
}

fn with_layer_tx(tx: broadcast::Sender<LogEntry>, f: impl FnOnce()) -> LogBuffer {
    let buffer = LogBuffer::new();
    let subscriber = tracing_subscriber::registry().with(BroadcastLayer::new(tx, buffer.clone()));
    tracing::subscriber::with_default(subscriber, f);
    buffer
}

/// The ring's own entries, moved out so their allocations are the ones the
/// layer made (`snapshot` clones, and a clone's capacity is always tight).
fn stored_entries(ring: &LogBuffer) -> Vec<LogEntry> {
    let mut guard = ring.inner.lock().unwrap();
    guard.bytes = 0;
    guard.entries.drain(..).map(|(e, _)| e).collect()
}

/// The kept part of a truncated string, with the marker checked and removed.
fn strip_marker(s: &str) -> &str {
    s.strip_suffix(TRUNCATION_MARKER)
        .unwrap_or_else(|| panic!("no truncation marker on a {}-byte string", s.len()))
}

#[test]
fn oversized_message_is_cut_at_a_char_boundary_with_marker() {
    // 3-byte chars, so the byte cap falls mid-char unless the cut backs off.
    let big = "\u{20ac}".repeat(MAX_MESSAGE_BYTES); // 3x the cap
    assert_ne!(MAX_MESSAGE_BYTES % 3, 0, "cap must not land on a boundary");

    let ring = with_layer(|| tracing::info!("{big}"));
    let entries = ring.snapshot();
    assert_eq!(entries.len(), 1);

    let kept = strip_marker(&entries[0].message);
    assert!(kept.len() <= MAX_MESSAGE_BYTES);
    assert!(kept.len() > MAX_MESSAGE_BYTES - 3, "cut backed off too far");
    assert!(big.starts_with(kept));
}

#[test]
fn oversized_field_values_are_cut_with_marker() {
    // `%` goes through record_debug, a plain &str through record_str.
    let big = "\u{00e9}".repeat(MAX_FIELD_BYTES); // 2-byte chars, 2x the cap
    let big_odd = format!("x{big}"); // shifts the boundary to an odd offset

    let ring = with_layer(|| {
        tracing::info!(display = %big_odd, plain = big.as_str(), "short");
    });
    let entry = &ring.snapshot()[0];
    assert_eq!(entry.message, "short");

    for (key, source) in [("display", &big_odd), ("plain", &big)] {
        let value = entry.fields[key].as_str().expect("string field");
        let kept = strip_marker(value);
        assert!(kept.len() <= MAX_FIELD_BYTES, "{key}: {} bytes", kept.len());
        assert!(kept.len() >= MAX_FIELD_BYTES - 1, "{key}: cut too far");
        assert!(source.starts_with(kept), "{key}: not a prefix");
    }
}

#[test]
fn whole_entry_is_held_to_the_entry_budget() {
    // More fields than the entry budget can hold at their own cap.
    let v = "a".repeat(MAX_FIELD_BYTES * 2);
    let msg = "m".repeat(MAX_MESSAGE_BYTES * 2);
    let ring = with_layer(|| {
        tracing::info!(
            f01 = v.as_str(),
            f02 = v.as_str(),
            f03 = v.as_str(),
            f04 = v.as_str(),
            f05 = v.as_str(),
            f06 = v.as_str(),
            f07 = v.as_str(),
            f08 = v.as_str(),
            f09 = v.as_str(),
            f10 = v.as_str(),
            f11 = v.as_str(),
            f12 = v.as_str(),
            f13 = v.as_str(),
            f14 = v.as_str(),
            f15 = v.as_str(),
            f16 = v.as_str(),
            "{msg}"
        );
    });
    let entry = &ring.snapshot()[0];
    let text = entry.approx_bytes() - std::mem::size_of::<LogEntry>();
    // Every string may carry one marker past its share of the budget.
    let slack = 17 * TRUNCATION_MARKER.len() + entry.level.len() + entry.target.len();
    assert!(text <= MAX_ENTRY_BYTES + slack, "{text} bytes of text");
}

#[test]
fn large_entries_keep_the_ring_under_its_byte_budget_oldest_evicted_first() {
    const N: usize = BUFFER_CAPACITY;
    let filler = "z".repeat(MAX_MESSAGE_BYTES * 4);
    let ring = with_layer(|| {
        for i in 0..N {
            tracing::info!("{i:05} {filler}");
        }
    });

    let running = ring.total_bytes();
    let entries = stored_entries(&ring);
    let recomputed: usize = entries.iter().map(LogEntry::approx_bytes).sum();
    assert!(recomputed <= RING_BYTE_BUDGET, "{recomputed} bytes held");
    assert_eq!(running, recomputed, "running total drifted");
    assert!(entries.len() < N, "nothing was evicted");

    // What survives is the newest run, in order, ending with the last event.
    let first = N - entries.len();
    for (offset, entry) in entries.iter().enumerate() {
        let want = format!("{:05} ", first + offset);
        assert!(
            entry.message.starts_with(&want),
            "entry {offset} out of order"
        );
    }
}

#[test]
fn small_entries_behave_as_before() {
    let ring = with_layer(|| {
        for i in 0..BUFFER_CAPACITY + 1 {
            tracing::warn!(n = i as u64, ok = true, who = "gate", "event {i}");
        }
    });
    let entries = ring.snapshot();
    // The entry-count cap still applies: the oldest one went.
    assert_eq!(entries.len(), BUFFER_CAPACITY);
    assert_eq!(entries[0].message, "event 1");

    let last = entries.last().unwrap();
    assert_eq!(last.message, format!("event {BUFFER_CAPACITY}"));
    assert_eq!(last.level, "WARN");
    assert!(last.target.ends_with("tests"));
    assert_eq!(last.fields["n"], BUFFER_CAPACITY as u64);
    assert_eq!(last.fields["ok"], true);
    assert_eq!(last.fields["who"], "gate");
    assert!(!last.message.contains(TRUNCATION_MARKER));
}

#[test]
fn live_subscribers_get_the_same_bounded_entry() {
    let (tx, mut rx) = broadcast::channel::<LogEntry>(16);
    let big = "q".repeat(MAX_MESSAGE_BYTES * 2);
    let ring = with_layer_tx(tx, || tracing::info!("{big}"));

    let sent = rx.try_recv().expect("entry was broadcast");
    let kept = strip_marker(&sent.message);
    assert_eq!(kept.len(), MAX_MESSAGE_BYTES);
    assert_eq!(sent.message, ring.snapshot()[0].message);
}

#[test]
fn truncated_text_holds_no_spare_capacity() {
    // The ring budgets by capacity, so a cut string must not keep the
    // doubled allocation it grew into while being formatted.
    let big = "w".repeat(MAX_MESSAGE_BYTES * 4);
    let ring = with_layer(|| tracing::info!(display = %big, "{big}"));
    // Inspect the stored entry: a snapshot clone would be tight regardless.
    let stored = stored_entries(&ring);
    let entry = &stored[0];

    let display = entry.fields["display"].as_str().expect("string field");
    strip_marker(display);
    strip_marker(&entry.message);
    let slack = 64;
    assert!(
        entry.message.capacity() <= entry.message.len() + slack,
        "message: {} capacity for {} bytes",
        entry.message.capacity(),
        entry.message.len()
    );
    if let serde_json::Value::Object(map) = &entry.fields {
        for (key, value) in map {
            if let serde_json::Value::String(s) = value {
                assert!(s.capacity() <= s.len() + slack, "{key}: spare capacity");
            }
        }
    }
}

#[test]
fn full_ring_capacity_stays_within_budget() {
    let filler = "c".repeat(MAX_MESSAGE_BYTES * 4);
    let ring = with_layer(|| {
        for i in 0..BUFFER_CAPACITY {
            tracing::info!("{i:05} {filler}");
        }
    });
    let resident: usize = stored_entries(&ring)
        .iter()
        .map(|e| std::mem::size_of::<LogEntry>() + e.message.capacity() + e.target.capacity())
        .sum();
    assert!(resident <= RING_BYTE_BUDGET, "{resident} bytes resident");
    // The budget is spent on text, not slack: the ring is within one entry
    // of full.
    let one = std::mem::size_of::<LogEntry>() + MAX_MESSAGE_BYTES + TRUNCATION_MARKER.len() + 256;
    assert!(
        resident + one >= RING_BYTE_BUDGET,
        "only {resident} bytes used"
    );
}
