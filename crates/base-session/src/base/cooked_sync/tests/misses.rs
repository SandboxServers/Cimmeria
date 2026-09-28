//! Serving `elementDataRequest`: the entry goes out next, ahead of the
//! background stream; unknown categories and keys are refused; a session
//! is rate-limited.

use tracing::Level;

use super::super::{MissOutcome, MissRefusal, MISS_BURST};
use super::{decode, rig, server_entries, server_version, ClientModel, Sent};
use crate::test_support::LogCapture;

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
