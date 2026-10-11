//! Chaos armed by account for its next world entry (D-CH10, CH-14).

use super::*;
use crate::test_support::LogCapture;
use std::time::Duration;
use tracing::Level;

const ACCOUNT: u32 = 4242;
const ENTITY: u32 = 1007;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn net(seed: u64) -> NetChaos {
    NetChaos {
        latency_ms: 120,
        jitter_ms: 0,
        loss_per_thousand: 10,
        dup_per_thousand: 0,
        reorder: 0,
        seed,
    }
}

fn arm_pack(chaos: &LabChaos, account_id: u32, records: u16, ttl_s: u32, now: Instant) {
    chaos
        .arm(
            account_id,
            "flush:pack".into(),
            None,
            Some(FlushChaos::Pack { records }),
            ttl_s,
            now,
        )
        .expect("a valid arm");
}

fn armed_accounts(chaos: &LabChaos, now: Instant) -> Vec<u32> {
    chaos
        .armed_status(now)
        .iter()
        .map(|a| a.account_id)
        .collect()
}

/// An arm is nothing on any session until its account enters the world;
/// then it is that session's chaos, tagged with the new entity, under the
/// TTL it was armed with, and survives the flush's stale-entity check for
/// that entity. Fails if promotion is removed or tags the wrong entity.
#[test]
fn promotion_moves_the_arm_onto_the_new_session() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    let expires_at = chaos
        .arm(
            ACCOUNT,
            "flush:pack".into(),
            None,
            Some(FlushChaos::Pack { records: 24 }),
            60,
            now,
        )
        .expect("a valid arm");
    assert_eq!(expires_at, now + Duration::from_secs(60));
    assert_eq!(chaos.flush_for(addr(41_001), now), None, "not set yet");
    assert!(
        chaos.status(now).is_empty(),
        "an arm is not a session entry"
    );

    let later = now + Duration::from_secs(10);
    assert!(chaos
        .promote_armed(ACCOUNT, addr(41_001), ENTITY, later)
        .is_some());
    assert!(
        !chaos.clear_if_not(addr(41_001), ENTITY),
        "tagged with the new entity, so the flush's check keeps it"
    );
    assert_eq!(
        chaos.flush_for(addr(41_001), later),
        Some(FlushChaos::Pack { records: 24 })
    );
    let status = chaos.status(later);
    assert_eq!(status.len(), 1);
    assert_eq!(status[0].entity_id, ENTITY);
    assert_eq!(
        status[0].expires_at, expires_at,
        "the armed TTL carries over"
    );
    assert!(chaos.armed_status(later).is_empty(), "taken, not copied");
}

/// One-shot: a second world entry of the same account finds nothing.
#[test]
fn an_arm_is_promoted_once() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    arm_pack(&chaos, ACCOUNT, 24, 60, now);
    assert!(chaos
        .promote_armed(ACCOUNT, addr(41_011), ENTITY, now)
        .is_some());
    assert_eq!(chaos.clear(Some(addr(41_011))), 1);
    assert!(chaos
        .promote_armed(ACCOUNT, addr(41_011), ENTITY + 1, now)
        .is_none());
    assert_eq!(chaos.flush_for(addr(41_011), now), None);
}

/// Another account's world entry leaves the arm waiting.
#[test]
fn promotion_is_by_account() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    arm_pack(&chaos, ACCOUNT, 24, 60, now);
    assert!(chaos
        .promote_armed(ACCOUNT + 1, addr(41_021), ENTITY, now)
        .is_none());
    assert_eq!(chaos.flush_for(addr(41_021), now), None);
    assert_eq!(armed_accounts(&chaos, now), [ACCOUNT]);
}

/// An arm past its TTL is dropped, by status and by a late world entry.
#[test]
fn an_arm_expires() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    arm_pack(&chaos, ACCOUNT, 24, 5, now);
    arm_pack(&chaos, ACCOUNT + 1, 24, 5, now);
    let after = now + Duration::from_secs(5);
    assert!(
        chaos
            .promote_armed(ACCOUNT, addr(41_031), ENTITY, after)
            .is_none(),
        "an expired arm is not promoted"
    );
    assert_eq!(chaos.flush_for(addr(41_031), after), None);
    assert!(armed_accounts(&chaos, after).is_empty(), "swept by status");
}

/// A second arm for the account replaces the first, knobs and TTL.
#[test]
fn re_arming_replaces() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    arm_pack(&chaos, ACCOUNT, 24, 60, now);
    arm_pack(&chaos, ACCOUNT, 10, 30, now);
    let armed = chaos.armed_status(now);
    assert_eq!(armed.len(), 1);
    assert_eq!(armed[0].flush, Some(FlushChaos::Pack { records: 10 }));
    assert_eq!(armed[0].expires_at, now + Duration::from_secs(30));
}

/// The arm goes through `set`'s checks: a bad TTL or knob arms nothing.
#[test]
fn arm_refuses_what_set_refuses() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    let pack = |records| Some(FlushChaos::Pack { records });
    for (got, want) in [
        (
            chaos.arm(ACCOUNT, "x".into(), None, pack(24), 0, now),
            ChaosRefused::BadTtl(0),
        ),
        (
            chaos.arm(ACCOUNT, "x".into(), None, pack(24), MAX_TTL_S + 1, now),
            ChaosRefused::TtlTooLong(MAX_TTL_S + 1),
        ),
        (
            chaos.arm(ACCOUNT, "x".into(), None, pack(35), 60, now),
            ChaosRefused::BadKnob("flush pack records is not 1 to 34 (one packet)"),
        ),
        (
            chaos.arm(ACCOUNT, "x".into(), None, None, 60, now),
            ChaosRefused::BadKnob("set needs a net or a flush profile"),
        ),
    ] {
        assert_eq!(got, Err(want));
    }
    assert!(
        chaos.armed_status(now).is_empty(),
        "a refused arm stores nothing"
    );
}

#[test]
fn clear_account_removes_only_that_arm() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    arm_pack(&chaos, ACCOUNT, 24, 60, now);
    arm_pack(&chaos, ACCOUNT + 1, 24, 60, now);
    assert_eq!(chaos.clear_account(ACCOUNT), 1);
    assert_eq!(chaos.clear_account(ACCOUNT), 0);
    assert_eq!(armed_accounts(&chaos, now), [ACCOUNT + 1]);
    assert!(chaos
        .promote_armed(ACCOUNT, addr(41_041), ENTITY, now)
        .is_none());
}

/// Clear-all counts and drops arms with sessions; a per-session clear (a
/// teardown) leaves the arms.
#[test]
fn clear_all_includes_arms_and_a_session_clear_does_not() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    arm_pack(&chaos, ACCOUNT, 24, 60, now);
    chaos
        .set(
            addr(41_051),
            ENTITY,
            "net".into(),
            Some(net(1)),
            None,
            60,
            now,
        )
        .expect("a valid set");
    assert_eq!(chaos.clear(Some(addr(41_051))), 1);
    assert_eq!(armed_accounts(&chaos, now), [ACCOUNT]);
    chaos
        .set(
            addr(41_051),
            ENTITY,
            "net".into(),
            Some(net(1)),
            None,
            60,
            now,
        )
        .expect("a valid set");
    assert_eq!(chaos.clear(None), 2);
    assert!(chaos.armed_status(now).is_empty());
}

/// An armed net profile is not net chaos until promoted; promotion goes
/// through the guard, so the shim's check counts it, under a generation no
/// earlier entry had. Fails if promotion writes the map past the guard.
#[test]
fn has_net_and_generation_after_promotion() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    chaos
        .set(
            addr(41_061),
            ENTITY,
            "net".into(),
            Some(net(1)),
            None,
            60,
            now,
        )
        .expect("a valid set");
    let (_, set_generation) = chaos.net_entry_for(addr(41_061), now).expect("set");
    assert_eq!(chaos.clear(Some(addr(41_061))), 1);

    chaos
        .arm(ACCOUNT, "net".into(), Some(net(2)), None, 60, now)
        .expect("a valid arm");
    assert!(!chaos.has_net(), "an arm is not net chaos");
    assert!(chaos
        .promote_armed(ACCOUNT, addr(41_061), ENTITY, now)
        .is_some());
    assert!(chaos.has_net(), "promotion recounts");
    let (promoted, generation) = chaos.net_entry_for(addr(41_061), now).expect("promoted");
    assert_eq!(promoted.seed, 2);
    assert!(
        generation > set_generation,
        "{generation} after {set_generation}"
    );
}

/// Status of arms carries account ids and no address; the promotion WARN
/// names the session by hash only.
#[test]
fn arm_and_promotion_logs_carry_no_address() {
    let capture = LogCapture::install();
    let chaos = LabChaos::new();
    let now = Instant::now();
    chaos
        .arm(
            ACCOUNT,
            "flush:reorder seed=7".into(),
            None,
            Some(FlushChaos::Reorder { seed: 7 }),
            DEFAULT_TTL_S,
            now,
        )
        .expect("a valid arm");
    let armed = capture
        .find_message(Level::WARN, "lab chaos armed for an account")
        .expect("the arm WARN");
    assert!(armed.has_field("event", "lab_chaos_armed"), "{armed:#?}");
    assert!(armed.has_field("account_id", "4242"), "{armed:#?}");
    assert!(armed.has_field("seed", "7"), "{armed:#?}");
    assert!(armed.has_field("expires_in_s", "300"), "{armed:#?}");

    assert!(chaos
        .promote_armed(ACCOUNT, addr(41_071), ENTITY, now)
        .is_some());
    let promoted = capture
        .find_message(Level::WARN, "lab chaos armed for the account set")
        .expect("the promotion WARN");
    assert!(promoted.has_field("event", "lab_chaos_promoted"));
    assert!(promoted.has_field("entity_id", "1007"), "{promoted:#?}");
    assert!(promoted.fields.contains_key("addr_hash"), "{promoted:#?}");
    for value in promoted.fields.values() {
        assert!(!value.contains("127.0.0.1"), "address leaked: {value}");
    }
}
