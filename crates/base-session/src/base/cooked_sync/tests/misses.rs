//! Serving `elementDataRequest`: the entry goes out next, ahead of the
//! background stream; unknown categories and keys are refused; a session
//! is rate-limited.

use std::time::Instant;

use tracing::Level;

use super::super::{note_dialog_display, MissOutcome, MissRefusal, MISS_BURST};
use super::{decode, rig, server_entries, server_version, ClientModel, Sent};
use crate::test_support::{LogCapture, LogCaptureGuard};
use cimmeria_names::{NameBook, Table};

const ABILITIES: u32 = 2;
const DIALOGS: u32 = 5;
const TEXT_STRINGS: u32 = 10;

/// Transfers opened, in send order: `(category, key)`.
fn transfers(plaintexts: &[Vec<u8>]) -> Vec<(u32, u32)> {
    plaintexts
        .iter()
        .filter_map(|pt| match decode(pt) {
            Sent::Fragment {
                head: Some(head), ..
            } => Some(head),
            _ => None,
        })
        .collect()
}

fn a_dialog_key() -> u32 {
    *server_entries(DIALOGS).keys().nth(100).unwrap()
}

/// A miss asked for while TextStrings is streaming goes out as the very
/// next transfer, not behind the rest of the 29,000-entry stream.
#[tokio::test]
async fn a_miss_is_served_ahead_of_the_stream() {
    let rig = rig(47_601);
    rig.request(TEXT_STRINGS, server_version(TEXT_STRINGS).wrapping_add(1))
        .await;
    rig.idle_turns(500).await;
    rig.sent.clear();

    let key = a_dialog_key();
    assert_eq!(
        rig.miss(DIALOGS, key),
        MissOutcome::Queued { start_task: false }
    );
    rig.ack_oldest(4);
    rig.idle_turns(500).await;

    let sent = transfers(&rig.take_plaintexts());
    assert_eq!(
        sent.first(),
        Some(&(DIALOGS, key)),
        "the miss must be the next transfer; sent {sent:?}"
    );
}

/// With no resync running, a miss starts the task and is served.
#[tokio::test]
async fn a_miss_is_served_without_a_resync() {
    let rig = rig(47_602);
    let key = a_dialog_key();
    assert_eq!(
        rig.miss(DIALOGS, key),
        MissOutcome::Queued { start_task: true }
    );
    rig.pump_until_idle().await;
    let plaintexts = rig.take_plaintexts();
    assert_eq!(transfers(&plaintexts), vec![(DIALOGS, key)]);
    let mut client = ClientModel::default();
    client.apply_all(&plaintexts);
    assert_eq!(
        client.categories[&DIALOGS].entries.get(&key),
        server_entries(DIALOGS).get(&key),
        "the entry arrives verbatim"
    );
}

#[tokio::test]
async fn an_unknown_category_is_refused_with_a_warn() {
    let rig = rig(47_603);
    let capture = LogCapture::install();
    assert_eq!(
        rig.miss(99, 1),
        MissOutcome::Refused {
            why: MissRefusal::UnknownCategory,
            log: Some(0)
        }
    );
    rig.idle_turns(50).await;
    assert!(rig.sent.is_empty());
    let warn = capture
        .find_event(
            Level::WARN,
            "Refused a cooked-data cache miss",
            "unknown_category",
        )
        .unwrap_or_else(|| panic!("no refusal WARN; saw {:#?}", capture.all()));
    assert!(warn.has_field("category_id", "99"));
}

/// Unknown keys are refused; repeats are counted, not logged each time (the
/// client asks again on every lookup).
#[tokio::test]
async fn an_unknown_key_is_refused_and_repeats_are_throttled() {
    let rig = rig(47_604);
    let capture = LogCapture::install();
    let missing = u32::MAX - 3;
    assert!(!server_entries(DIALOGS).contains_key(&missing));
    for _ in 0..5 {
        assert!(matches!(
            rig.miss(DIALOGS, missing),
            MissOutcome::Refused {
                why: MissRefusal::UnknownKey,
                ..
            }
        ));
    }
    assert!(rig.sent.is_empty());
    let warns = capture
        .all()
        .into_iter()
        .filter(|e| e.has_field("event", "cooked_data.miss_refused"))
        .count();
    assert_eq!(warns, 1, "one WARN per reason per 5 s");
}

/// The name the test book gives the refused dialog id.
const NAMED_DIALOG: &str = "Refused_Dialog_Fixture";

/// Gives the process-wide name book a name for the refused dialog id, so the rows can carry
/// `element_name`. The id is far outside the shipped dialog range.
fn name_the_dialog(id: u32) {
    let mut book = NameBook::empty();
    book.insert(Table::Dialogs, i64::from(id), NAMED_DIALOG);
    cimmeria_names::global().store(book);
}

/// A refused dialog miss asked for right after the client was shown that
/// dialog carries the display age and raises the correlation WARN.
#[tokio::test]
async fn a_refused_dialog_miss_right_after_its_display_warns_with_the_age() {
    let rig = rig(47_606);
    let capture = LogCapture::install();
    let missing = u32::MAX - 3;
    name_the_dialog(missing);
    note_dialog_display(rig.addr, missing, Instant::now());
    assert!(matches!(
        rig.miss(DIALOGS, missing),
        MissOutcome::Refused {
            why: MissRefusal::UnknownKey,
            ..
        }
    ));
    let refused = capture
        .find_event(
            Level::WARN,
            "Refused a cooked-data cache miss",
            "unknown_key",
        )
        .unwrap_or_else(|| panic!("no refusal WARN; saw {:#?}", capture.all()));
    assert!(refused.fields.contains_key("display_age_ms"));
    assert!(
        refused.has_field("element_name", NAMED_DIALOG),
        "{refused:#?}"
    );
    let correlated: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|e| e.has_field("event", "dialog.cooked_miss_after_display"))
        .collect();
    assert_eq!(correlated.len(), 1, "saw {:#?}", capture.all());
    assert!(correlated[0].has_field("category_id", "5"));
    assert!(correlated[0].has_field("key", &missing.to_string()));
    assert!(
        correlated[0].has_field("element_name", NAMED_DIALOG),
        "{correlated:#?}"
    );
}

/// The correlation WARN is raised once per display: a client re-requesting the same refused key
/// in a loop does not write one per request, and a fresh display re-arms it.
#[tokio::test]
async fn the_correlation_warn_is_once_per_display() {
    let rig = rig(47_610);
    let capture = LogCapture::install();
    let missing = u32::MAX - 3;
    let correlated = |capture: &LogCaptureGuard| {
        capture
            .all()
            .into_iter()
            .filter(|e| e.has_field("event", "dialog.cooked_miss_after_display"))
            .count()
    };
    note_dialog_display(rig.addr, missing, Instant::now());
    for _ in 0..2 {
        assert!(matches!(
            rig.miss(DIALOGS, missing),
            MissOutcome::Refused { .. }
        ));
    }
    assert_eq!(correlated(&capture), 1, "saw {:#?}", capture.all());
    note_dialog_display(rig.addr, missing, Instant::now());
    rig.miss(DIALOGS, missing);
    assert_eq!(correlated(&capture), 2, "saw {:#?}", capture.all());
}

/// Without a display before it, a refused dialog miss has no age and no
/// correlation row.
#[tokio::test]
async fn a_refused_dialog_miss_without_a_display_has_no_correlation_row() {
    let rig = rig(47_607);
    let capture = LogCapture::install();
    let missing = u32::MAX - 3;
    name_the_dialog(missing);
    assert!(matches!(
        rig.miss(DIALOGS, missing),
        MissOutcome::Refused { .. }
    ));
    let refused = capture
        .find_event(
            Level::WARN,
            "Refused a cooked-data cache miss",
            "unknown_key",
        )
        .unwrap_or_else(|| panic!("no refusal WARN; saw {:#?}", capture.all()));
    assert!(!refused.fields.contains_key("display_age_ms"));
    assert!(
        refused.has_field("element_name", NAMED_DIALOG),
        "{refused:#?}"
    );
    assert!(capture
        .all()
        .iter()
        .all(|e| !e.has_field("event", "dialog.cooked_miss_after_display")));
}

/// A refused dialog with no name in the book carries no `element_name` field.
#[tokio::test]
async fn an_unnamed_refused_dialog_has_no_element_name() {
    let rig = rig(47_611);
    let capture = LogCapture::install();
    let missing = u32::MAX - 9;
    name_the_dialog(u32::MAX - 3);
    assert!(matches!(
        rig.miss(DIALOGS, missing),
        MissOutcome::Refused { .. }
    ));
    let refused = capture
        .find_event(
            Level::WARN,
            "Refused a cooked-data cache miss",
            "unknown_key",
        )
        .unwrap_or_else(|| panic!("no refusal WARN; saw {:#?}", capture.all()));
    assert!(!refused.fields.contains_key("element_name"), "{refused:#?}");
}

/// A burst of distinct valid keys past the bucket is refused as
/// rate-limited.
#[tokio::test]
async fn a_flood_of_misses_is_rate_limited() {
    let rig = rig(47_605);
    let keys: Vec<u32> = server_entries(TEXT_STRINGS)
        .keys()
        .copied()
        .take(200)
        .collect();
    let mut queued = 0;
    let mut limited = 0;
    for key in keys {
        match rig.miss(TEXT_STRINGS, key) {
            MissOutcome::Queued { .. } => queued += 1,
            MissOutcome::Refused {
                why: MissRefusal::RateLimited,
                ..
            } => limited += 1,
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(
        queued <= MISS_BURST as usize + 2,
        "{queued} queued, burst is {MISS_BURST}"
    );
    assert!(limited >= 90, "only {limited} refused");
}

/// The same entry asked for twice before it is sent is queued once.
#[tokio::test]
async fn a_repeated_miss_is_queued_once() {
    let rig = rig(47_606);
    rig.request(TEXT_STRINGS, server_version(TEXT_STRINGS).wrapping_add(1))
        .await;
    rig.idle_turns(500).await;
    let key = a_dialog_key();
    assert!(matches!(rig.miss(DIALOGS, key), MissOutcome::Queued { .. }));
    assert_eq!(rig.miss(DIALOGS, key), MissOutcome::Duplicate);
}

/// Misses interleaved with a resync do not break it: the client still ends
/// with exactly the server's category, and every missed entry arrives.
#[tokio::test]
async fn convergence_holds_with_misses_interleaved() {
    let rig = rig(47_607);
    let served = server_version(ABILITIES);
    let abilities = server_entries(ABILITIES);
    let dialogs = server_entries(DIALOGS);
    rig.request(ABILITIES, served.wrapping_add(1)).await;
    let ability_keys: Vec<u32> = abilities.keys().copied().step_by(97).collect();
    let dialog_keys: Vec<u32> = dialogs.keys().copied().step_by(501).collect();
    for (i, key) in ability_keys.iter().enumerate() {
        rig.idle_turns(20).await;
        rig.ack_all();
        rig.miss(ABILITIES, *key);
        if let Some(d) = dialog_keys.get(i) {
            rig.miss(DIALOGS, *d);
        }
    }
    rig.pump_until_idle().await;

    let mut client = ClientModel::holding(ABILITIES, served.wrapping_add(1), Default::default());
    client.apply_all(&rig.take_plaintexts());
    let held = &client.categories[&ABILITIES];
    assert!(held.entries == abilities, "abilities must converge exactly");
    assert_eq!(held.version, served);
    for d in &dialog_keys {
        assert_eq!(client.categories[&DIALOGS].entries.get(d), dialogs.get(d));
    }
}

/// A served miss logs the category, key and latency.
#[tokio::test]
async fn a_served_miss_logs_its_latency() {
    let rig = rig(47_608);
    let capture = LogCapture::install();
    let key = a_dialog_key();
    rig.miss(DIALOGS, key);
    rig.pump_until_idle().await;
    let served = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "cooked_data.miss_served"))
        .unwrap_or_else(|| panic!("no miss_served; saw {:#?}", capture.all()));
    assert_eq!(served.level, Level::INFO);
    assert!(served.has_field("category_id", "5"));
    assert!(served.has_field("key", &key.to_string()));
    assert!(served.fields.contains_key("latency_ms"));
}

/// The client keeps asking for misses throughout a stream. Pushes
/// decrement `RequiredUpdates` only while it is nonzero (`0x0043db73`:
/// `CMP EAX,ESI` / `JZ` over `ADD EAX,-1`), and the request functions send
/// only while it is exactly 0 (`0x00cfe07b`: `CMP [ECX+0x48],EBX` / `JNZ`).
/// The resync opens with 0, so after K background pushes the client still
/// asks, and the miss is served before the stream would have reached it.
#[tokio::test]
async fn a_miss_after_k_background_pushes_is_still_requested_and_served() {
    const K: usize = 100;
    let rig = rig(47_609);
    let served = server_version(TEXT_STRINGS);
    rig.request(TEXT_STRINGS, served.wrapping_add(1)).await;

    let mut client = ClientModel::holding(TEXT_STRINGS, served.wrapping_add(1), Default::default());
    while client.writes.get(&TEXT_STRINGS).copied().unwrap_or(0) < K as u32 {
        rig.idle_turns(20).await;
        rig.ack_all();
        client.apply_all(&rig.take_plaintexts());
    }
    assert!(
        client.requests_misses(TEXT_STRINGS),
        "after {K} pushes the client must still send elementDataRequest (RequiredUpdates = {})",
        client.categories[&TEXT_STRINGS].required_updates
    );

    let last_key = *server_entries(TEXT_STRINGS).keys().next_back().unwrap();
    assert!(!client.categories[&TEXT_STRINGS]
        .entries
        .contains_key(&last_key));
    assert!(matches!(
        rig.miss(TEXT_STRINGS, last_key),
        MissOutcome::Queued { .. }
    ));
    for _ in 0..10 {
        rig.idle_turns(20).await;
        rig.ack_all();
        client.apply_all(&rig.take_plaintexts());
    }
    assert_eq!(
        client.categories[&TEXT_STRINGS].entries.get(&last_key),
        server_entries(TEXT_STRINGS).get(&last_key),
        "the missed entry arrives long before the stream reaches it"
    );
    assert!(
        client.writes[&TEXT_STRINGS] < 1_000,
        "the stream is still far from the end"
    );
}
