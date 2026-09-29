//! Many-fragment reliable bundles whose raw 1300-byte cut would land inside a
//! message header (#838), delivered over the loopback harness.
//!
//! The harness reassembler (like the client's) concatenates fragment bodies,
//! so an intact reassembly does not by itself prove the cuts are header safe.
//! Each test therefore also runs the fragment plan the sender used through
//! [`crate::client_model`], the model of the SGW client's bundle iterator.

use std::time::Duration;

use crate::client_model::unpack_packet_bodies;
use crate::packet::{fragment_count, plan_fragments, FRAGMENT_BODY_SIZE};
use crate::test_harness::LoopbackSession;

/// One entity-method message: `[0x85][len u16][payload]`.
fn word_msg(payload_len: usize, fill: u8) -> Vec<u8> {
    let mut m = vec![0x85];
    m.extend_from_slice(&(payload_len as u16).to_le_bytes());
    m.extend(std::iter::repeat_n(fill, payload_len));
    m
}

/// A ~15-fragment body of messages whose first header after the leading
/// filler lands one byte before the first cut, so a raw split straddles it.
/// The fragment count is forced even for the pair-reorder test.
fn straddling_body(fill: u8) -> Vec<u8> {
    straddling_body_of(fill, 15)
}

fn straddling_body_of(fill: u8, min_fragments: usize) -> Vec<u8> {
    let mut body = word_msg(FRAGMENT_BODY_SIZE - 1 - 3, fill);
    let mut n = 0usize;
    while body.len() < FRAGMENT_BODY_SIZE * min_fragments || fragment_count(&body) % 2 == 1 {
        body.extend(word_msg(30 + (n * 11) % 40, fill));
        n += 1;
    }
    body
}

/// Assert the plan the sender used is header safe under the client model and
/// reproduces every message.
fn assert_client_accepts_plan(body: &[u8]) {
    let plan = plan_fragments(body);
    assert!(
        plan.header_guarded_cuts >= 1,
        "the fixture must exercise the guard"
    );
    let bodies: Vec<&[u8]> = plan.ranges.iter().map(|r| &body[r.clone()]).collect();
    let got = unpack_packet_bodies(&bodies);
    assert_eq!(got.abort, None, "client aborts mid-bundle");
    assert_eq!(
        got.messages.iter().map(|(_, p)| p.len() + 3).sum::<usize>(),
        body.len(),
        "every message of the bundle reaches the client"
    );
}

#[tokio::test]
async fn header_straddling_bundle_reassembles_and_cuts_on_message_boundaries() {
    let session = LoopbackSession::connected(None).await.unwrap();
    let body = straddling_body(0x11);
    assert_client_accepts_plan(&body);

    session.a.send_bundle(&body, true).await.unwrap();
    let got = session.b.recv_n_bundles(1, Duration::from_secs(3)).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].as_ref(), body.as_slice());
}

/// Duplicated fragments (the client discards a duplicate `seq`) do not corrupt
/// or repeat the bundle.
#[tokio::test]
async fn duplicated_fragments_deliver_the_bundle_once() {
    let session = LoopbackSession::connected(None).await.unwrap();
    session.policy.lock().unwrap().duplicate_every.a_to_b = Some(2);
    let body = straddling_body(0x22);
    assert_client_accepts_plan(&body);

    session.a.send_bundle(&body, true).await.unwrap();
    let got = session.b.recv_n_bundles(1, Duration::from_secs(3)).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].as_ref(), body.as_slice());
    let extra = session
        .b
        .recv_n_bundles(1, Duration::from_millis(200))
        .await;
    assert!(
        extra.is_empty(),
        "a duplicate fragment must not yield a second bundle"
    );
}

/// Adjacent fragments swapped on the wire still reassemble in sequence order.
#[tokio::test]
async fn pair_reordered_fragments_reassemble_in_sequence_order() {
    let session = LoopbackSession::connected(None).await.unwrap();
    session.policy.lock().unwrap().reorder_pairs.a_to_b = true;
    let body = straddling_body(0x33);
    assert_client_accepts_plan(&body);

    session.a.send_bundle(&body, true).await.unwrap();
    let got = session.b.recv_n_bundles(1, Duration::from_secs(3)).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].as_ref(), body.as_slice());
}

/// A mid-group fragment is lost and retransmitted; a second many-fragment
/// bundle sent right behind it (a different `lastFrag`) still arrives whole
/// and after the first.
#[tokio::test]
async fn lost_fragment_then_second_group_delivers_both_in_order() {
    let session = LoopbackSession::connected(None).await.unwrap();
    {
        let mut policy = session.policy.lock().unwrap();
        policy.reset_counters();
        policy.drop_at_send_count.a_to_b = Some(7);
    }
    let first = straddling_body(0x44);
    let second = straddling_body_of(0x55, 4);
    assert_client_accepts_plan(&first);
    assert_client_accepts_plan(&second);

    session.a.send_bundle(&first, true).await.unwrap();
    session.a.send_bundle(&second, true).await.unwrap();

    for _ in 0..3 {
        session.a.clock.advance(Duration::from_secs(2));
        session.tick().await.unwrap();
    }

    let got = session.b.recv_n_bundles(2, Duration::from_secs(5)).await;
    assert_eq!(
        got.len(),
        2,
        "both groups must complete after the retransmit"
    );
    assert_eq!(got[0].as_ref(), first.as_slice());
    assert_eq!(got[1].as_ref(), second.as_slice());
}
