//! The resync is debuggable from SigNoz alone: every reply names its
//! branch, and every push logs its start and finish with counts.

use tracing::Level;

use super::{rig, server_entries, server_version};
use crate::test_support::LogCapture;

const WORLD_INFO: u32 = 12;
const STARGATES: u32 = 13;

#[tokio::test]
async fn a_resync_logs_its_reply_start_and_finish() {
    let rig = rig(47_401);
    let capture = LogCapture::install();
    rig.request(WORLD_INFO, 5959).await;
    rig.pump_until_idle().await;
    let served = server_version(WORLD_INFO);
    let entries = server_entries(WORLD_INFO);
    let bytes: usize = entries.values().map(Vec::len).sum();
    let events = capture.all();

    let reply = events
        .iter()
        .find(|e| e.has_field("event", "cooked_data.version_reply"))
        .unwrap_or_else(|| panic!("no reply event; saw {events:#?}"));
    assert_eq!(reply.level, Level::INFO);
    assert!(reply.has_field("outcome", "full_resync"));
    assert!(reply.has_field("reason", "version_mismatch"));
    assert!(reply.has_field("client_version", "5959"));
    assert!(reply.has_field("server_version", &format!("Some({served})")));
    assert!(reply.has_field("invalidate_all", "true"));

    let start = events
        .iter()
        .find(|e| e.has_field("event", "cooked_data.sync_start"))
        .expect("sync_start");
    assert_eq!(start.level, Level::INFO);
    assert!(start.has_field("category_id", "12"));
    assert!(start.has_field("entry_count", &entries.len().to_string()));
    assert!(start.has_field("bytes", &bytes.to_string()));

    let finish = events
        .iter()
        .find(|e| e.has_field("event", "cooked_data.sync_finish"))
        .expect("sync_finish");
    assert_eq!(finish.level, Level::INFO);
    assert!(finish.has_field("outcome", "complete"));
    assert!(finish.has_field("category_id", "12"));
    assert!(finish.has_field("client_version", "5959"));
    assert!(finish.has_field("server_version", &served.to_string()));
    assert!(finish.has_field("entry_count", &entries.len().to_string()));
    assert!(finish.has_field("bytes", &bytes.to_string()));
    assert!(finish.fields.contains_key("duration_ms"));
    assert!(finish.fields.contains_key("packets"));
}

#[tokio::test]
async fn an_up_to_date_reply_names_its_branch() {
    let rig = rig(47_402);
    let capture = LogCapture::install();
    rig.request(STARGATES, server_version(STARGATES)).await;
    let reply = capture
        .find_event(
            Level::INFO,
            "Responding to versionInfoRequest",
            "versions_match",
        )
        .unwrap_or_else(|| panic!("no reply event; saw {:#?}", capture.all()));
    assert!(reply.has_field("outcome", "up_to_date"));
    assert!(reply.has_field("invalidate_all", "false"));
    assert!(capture
        .all()
        .iter()
        .all(|e| !e.has_field("event", "cooked_data.sync_start")));
}
