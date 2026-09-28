//! Flow control: a resync never has more than `SYNC_IN_FLIGHT_BUDGET`
//! reliable packets outstanding, counting everything else on the session,
//! and it never holds up the receive path.

use std::time::Duration;

use super::super::super::helpers::{drain_acks_and_seq, shadow_register_reliable_send};
use super::super::SYNC_IN_FLIGHT_BUDGET;
use super::{decode, rig, server_version, Sent};

/// Missions: 1,040 entries in 2,270 packets, several multi-fragment.
const MISSIONS: u32 = 3;
const STARGATES: u32 = 13;
const ABILITIES: u32 = 2;

/// A client that stops acking holds the push at exactly the budget, and
/// each ack releases exactly one more packet.
#[tokio::test]
async fn a_client_that_stops_acking_holds_the_push_at_the_budget() {
    let rig = rig(47_201);
    rig.request(MISSIONS, server_version(MISSIONS).wrapping_add(1))
        .await;
    rig.idle_turns(500).await;
    assert_eq!(rig.in_flight(), SYNC_IN_FLIGHT_BUDGET);
    assert_eq!(
        rig.sent.len(),
        SYNC_IN_FLIGHT_BUDGET,
        "nothing past the budget"
    );

    rig.ack_oldest(5);
    rig.idle_turns(500).await;
    assert_eq!(rig.in_flight(), SYNC_IN_FLIGHT_BUDGET);
    assert_eq!(rig.sent.len(), SYNC_IN_FLIGHT_BUDGET + 5);
}

/// Game traffic already in flight counts against the budget: the push only
/// fills what is left.
#[tokio::test]
async fn traffic_already_in_flight_shrinks_the_push() {
    let rig = rig(47_202);
    let game_packets = 10;
    for _ in 0..game_packets {
        let (_, seq) = drain_acks_and_seq(&rig.connected, rig.addr).unwrap();
        shadow_register_reliable_send(
            &rig.connected,
            rig.addr,
            seq,
            cimmeria_mercury::packet::Bytes::new(),
        );
    }
    rig.request(MISSIONS, server_version(MISSIONS).wrapping_add(1))
        .await;
    rig.idle_turns(500).await;
    assert_eq!(rig.sent.len(), SYNC_IN_FLIGHT_BUDGET - game_packets);
    assert_eq!(rig.in_flight(), SYNC_IN_FLIGHT_BUDGET);
}

/// Over a whole multi-fragment category the outstanding count peaks at the
/// budget and never above it, against a client that acks everything it has
/// each turn.
#[tokio::test]
async fn outstanding_packets_never_exceed_the_budget() {
    let rig = rig(47_203);
    rig.request(MISSIONS, server_version(MISSIONS).wrapping_add(1))
        .await;
    let peak = rig.pump_until_idle().await;
    assert_eq!(
        peak, SYNC_IN_FLIGHT_BUDGET,
        "the push uses its budget and no more"
    );
    let packets = rig.take_plaintexts().len();
    assert!(
        packets > 2_000,
        "the whole category went out ({packets} packets)"
    );
}

/// While a push is stalled on a full window, the receive path answers other
/// requests at once: an up-to-date category is answered straight away, and
/// a second mismatched category is queued without waiting.
#[tokio::test]
async fn the_receive_path_is_not_held_by_a_stalled_push() {
    let rig = rig(47_204);
    rig.request(MISSIONS, server_version(MISSIONS).wrapping_add(1))
        .await;
    rig.idle_turns(500).await;
    assert_eq!(rig.in_flight(), SYNC_IN_FLIGHT_BUDGET, "stalled");
    rig.sent.clear();

    tokio::time::timeout(Duration::from_millis(250), async {
        rig.request(STARGATES, server_version(STARGATES)).await;
        rig.request(ABILITIES, server_version(ABILITIES).wrapping_add(1))
            .await;
    })
    .await
    .expect("versionInfoRequest must not wait on the stalled push");

    let replies: Vec<Sent> = rig.take_plaintexts().iter().map(|pt| decode(pt)).collect();
    assert_eq!(replies.len(), 1, "only the up-to-date reply goes out now");
    let Sent::VersionInfo(v) = &replies[0] else {
        panic!("expected onVersionInfo, got {replies:?}");
    };
    assert_eq!(
        (v.category, v.version),
        (STARGATES, server_version(STARGATES))
    );
    assert!(!v.invalidate_all);
    assert!(rig.syncing());
}
