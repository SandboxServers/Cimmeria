//! CH-14 review fixes: a promotion is one step to a clear of everything, and
//! a world entry that fails after promoting puts the arm back (`rearm`).

use super::*;
use std::time::Duration;

const ACCOUNT: u32 = 4343;
const ENTITY: u32 = 2007;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn net() -> NetChaos {
    NetChaos {
        latency_ms: 80,
        jitter_ms: 5,
        loss_per_thousand: 0,
        dup_per_thousand: 0,
        reorder: 0,
        seed: 9,
    }
}

/// A leaked registry (threads borrow it) with a net + pack arm for
/// [`ACCOUNT`]; returns the arm's expiry too.
fn armed(now: Instant) -> (&'static LabChaos, Instant) {
    let chaos: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
    let expires_at = chaos
        .arm(
            ACCOUNT,
            "net+flush:pack".into(),
            Some(net()),
            Some(FlushChaos::Pack { records: 12 }),
            120,
            now,
        )
        .expect("a valid arm");
    (chaos, expires_at)
}

/// While the session map is locked, a promotion has not taken the arm: it
/// takes the sessions lock first, so the arm leaves the arms map and the
/// entry enters the session map in one step. Fails if `promote_armed` takes
/// the arm under the arms lock alone and then waits for the session map: the
/// arm is then gone with no entry to show for it, which a clear of
/// everything running in that gap would miss.
#[test]
fn a_promotion_takes_its_arm_under_the_sessions_lock() {
    let now = Instant::now();
    let (chaos, _) = armed(now);
    let held = chaos.sessions();
    let promoter =
        std::thread::spawn(move || chaos.promote_armed(ACCOUNT, addr(42_001), ENTITY, now));
    // Long enough for the promoter to get as far as it can.
    std::thread::sleep(Duration::from_millis(150));
    let arm_visible = chaos.armed().contains_key(&ACCOUNT);
    drop(held);
    let promoted = promoter.join().expect("the promoter");
    assert!(
        arm_visible,
        "the arm left before the entry could be inserted"
    );
    assert!(promoted.is_some());
    assert_eq!(chaos.status(now).len(), 1);
    assert!(chaos.armed_status(now).is_empty());
}

/// A clear of everything holds the session map while it clears the arms
/// (sessions first, as a promotion takes them), so no promotion fits
/// between the two halves and moves an arm out of reach of both. Fails if
/// the clear releases the session map before it locks the arms.
#[test]
fn a_clear_of_everything_holds_the_sessions_lock_over_the_arms() {
    let now = Instant::now();
    let (chaos, _) = armed(now);
    let held = chaos.armed();
    let clearer = std::thread::spawn(move || chaos.clear(None));
    std::thread::sleep(Duration::from_millis(150));
    let sessions_busy = chaos.sessions.try_lock().is_err();
    drop(held);
    assert_eq!(clearer.join().expect("the clearer"), 1);
    assert!(
        sessions_busy,
        "the clear let go of the session map before it took the arms"
    );
    assert!(chaos.armed_status(now).is_empty());
}

/// A failed world entry puts the arm back as it was armed: same knobs,
/// label and expiry, and the promoted entry gone (no net chaos left on the
/// session). Fails if `rearm` drops the arm or leaves the entry.
#[test]
fn rearm_restores_the_arm_and_removes_the_entry() {
    let now = Instant::now();
    let (chaos, expires_at) = armed(now);
    let promotion = chaos
        .promote_armed(ACCOUNT, addr(42_011), ENTITY, now)
        .expect("promoted");
    assert_eq!(promotion.account_id(), ACCOUNT);
    assert!(chaos.has_net());

    let later = now + Duration::from_secs(5);
    assert!(chaos.rearm(promotion, later));
    assert!(chaos.status(later).is_empty(), "the promoted entry is gone");
    assert!(!chaos.has_net());
    let back = chaos.armed_status(later);
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].account_id, ACCOUNT);
    assert_eq!(back[0].label, "net+flush:pack");
    assert_eq!(back[0].net, Some(net()));
    assert_eq!(back[0].flush, Some(FlushChaos::Pack { records: 12 }));
    assert_eq!(back[0].expires_at, expires_at, "the armed TTL is kept");
    // And the account's next world entry gets it.
    assert!(chaos
        .promote_armed(ACCOUNT, addr(42_012), ENTITY + 1, later)
        .is_some());
}

/// Teardown of the failed session clears its entry first; the arm still
/// comes back, since teardown is not an operator's clear.
#[test]
fn rearm_after_the_session_was_torn_down() {
    let now = Instant::now();
    let (chaos, _) = armed(now);
    let promotion = chaos
        .promote_armed(ACCOUNT, addr(42_021), ENTITY, now)
        .expect("promoted");
    assert_eq!(chaos.clear(Some(addr(42_021))), 1);
    assert!(chaos.rearm(promotion, now));
    assert_eq!(chaos.armed_status(now).len(), 1);
}

/// An operator clear since the promotion (everything, the entity, or the
/// account) wins: nothing comes back. Fails if `rearm` ignores the clears.
#[test]
fn rearm_after_an_operator_clear_puts_nothing_back() {
    let clears: [(&str, fn(&LabChaos)); 3] = [
        ("all", |c| {
            c.clear(None);
        }),
        ("entity", |c| {
            c.clear_entity(ENTITY);
        }),
        ("account", |c| {
            c.clear_account(ACCOUNT);
        }),
    ];
    for (name, clear) in clears {
        let now = Instant::now();
        let (chaos, _) = armed(now);
        let promotion = chaos
            .promote_armed(ACCOUNT, addr(42_031), ENTITY, now)
            .expect("promoted");
        clear(chaos);
        assert!(!chaos.rearm(promotion, now), "{name}");
        assert!(chaos.armed_status(now).is_empty(), "{name}");
        assert!(chaos.status(now).is_empty(), "{name}");
    }
}

/// A newer arm for the account stays; a newer set on the session stays;
/// an expired arm is not put back.
#[test]
fn rearm_leaves_newer_chaos_and_drops_an_expired_arm() {
    let now = Instant::now();
    let (chaos, _) = armed(now);
    let promotion = chaos
        .promote_armed(ACCOUNT, addr(42_041), ENTITY, now)
        .expect("promoted");
    chaos
        .arm(
            ACCOUNT,
            "flush:delay".into(),
            None,
            Some(FlushChaos::Delay { ms: 10 }),
            60,
            now,
        )
        .expect("a newer arm");
    chaos
        .set(
            addr(42_041),
            ENTITY,
            "flush:reorder".into(),
            None,
            Some(FlushChaos::Reorder { seed: 1 }),
            60,
            now,
        )
        .expect("a newer set");
    assert!(!chaos.rearm(promotion, now));
    assert_eq!(
        chaos.armed_status(now)[0].flush,
        Some(FlushChaos::Delay { ms: 10 })
    );
    assert_eq!(
        chaos.flush_for(addr(42_041), now),
        Some(FlushChaos::Reorder { seed: 1 }),
        "only the promotion's own entry is removed"
    );

    let (chaos, expires_at) = armed(now);
    let promotion = chaos
        .promote_armed(ACCOUNT, addr(42_042), ENTITY, now)
        .expect("promoted");
    assert!(!chaos.rearm(promotion, expires_at));
    assert!(chaos.armed_status(now).is_empty());
}
