use super::*;

use std::sync::atomic::AtomicBool;

const LAUNCH: &str = "launch-0001";

fn sha1_of(ticket: &str) -> String {
    sha1_upper_hex(ticket)
}

#[test]
fn minted_ticket_shape() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let first = store.mint(1, "Player_1234", LAUNCH, now);
    let second = store.mint(1, "Player_1234", LAUNCH, now);
    assert_eq!(first.ticket.len(), TICKET_LEN);
    assert!(first.ticket.chars().all(|c| c.is_ascii_alphanumeric()));
    assert_ne!(first.ticket, second.ticket);
}

#[test]
fn mint_reports_the_unredeemed_life() {
    let store = LaunchTicketStore::default();
    let minted = store.mint(1, "Player_1234", LAUNCH, Instant::now());
    assert_eq!(minted.expires_in_secs, TICKET_UNREDEEMED.as_secs());
}

#[test]
fn verify_accepts_the_sha1_of_the_ticket_any_case() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let minted = store.mint(1, "PLAYER_1234", LAUNCH, now);
    let upper = sha1_of(&minted.ticket);
    assert!(matches!(
        store.verify("Player_1234", &upper, now),
        TicketCheck::Valid { account_id: 1, .. }
    ));
    let lower = upper.to_ascii_lowercase();
    assert!(matches!(
        store.verify("Player_1234", &lower, now),
        TicketCheck::Valid { account_id: 1, .. }
    ));
}

#[test]
fn verify_refuses_wrong_name_wrong_hash_and_plain_ticket() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let minted = store.mint(1, "Player_1234", LAUNCH, now);
    assert!(matches!(
        store.verify("Player_9999", &sha1_of(&minted.ticket), now),
        TicketCheck::NoMatch
    ));
    assert!(matches!(
        store.verify("Player_1234", &sha1_of("not-the-ticket"), now),
        TicketCheck::NoMatch
    ));
    assert!(matches!(
        store.verify("Player_1234", &minted.ticket, now),
        TicketCheck::NoMatch
    ));
}

#[test]
fn ticket_is_reusable_within_its_life() {
    let store = LaunchTicketStore::default();
    let t0 = Instant::now();
    let minted = store.mint(1, "Player_1234", LAUNCH, t0);
    let sha1 = sha1_of(&minted.ticket);
    for minutes in [0, 20, 40] {
        let now = t0 + Duration::from_secs(minutes * 60);
        assert!(matches!(
            store.verify("Player_1234", &sha1, now),
            TicketCheck::Valid { account_id: 1, .. }
        ));
    }
}

#[test]
fn idle_and_absolute_expiry() {
    let t0 = Instant::now();

    let idle = LaunchTicketStore::default();
    let minted = idle.mint(1, "Player_1234", LAUNCH, t0);
    let sha1 = sha1_of(&minted.ticket);
    assert!(matches!(
        idle.verify("Player_1234", &sha1, t0),
        TicketCheck::Valid { account_id: 1, .. }
    ));
    assert!(matches!(
        idle.verify("Player_1234", &sha1, t0 + Duration::from_secs(31 * 60)),
        TicketCheck::NoMatch
    ));

    let store = LaunchTicketStore::default();
    let minted = store.mint(1, "Player_1234", LAUNCH, t0);
    let sha1 = sha1_of(&minted.ticket);
    assert!(matches!(
        store.verify("Player_1234", &sha1, t0),
        TicketCheck::Valid { account_id: 1, .. }
    ));
    for step in 1..=35u64 {
        let now = t0 + Duration::from_secs(step * 20 * 60);
        assert!(store.heartbeat(1, LAUNCH, now), "heartbeat at step {step}");
    }
    let at_absolute = t0 + TICKET_ABSOLUTE;
    assert!(matches!(
        store.verify("Player_1234", &sha1, at_absolute),
        TicketCheck::NoMatch
    ));
}

#[test]
fn mint_replaces_the_previous_ticket() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let first = store.mint(1, "Player_1234", LAUNCH, now);
    let second = store.mint(1, "Player_1234", LAUNCH, now);
    assert!(matches!(
        store.verify("Player_1234", &sha1_of(&first.ticket), now),
        TicketCheck::NoMatch
    ));
    assert!(matches!(
        store.verify("Player_1234", &sha1_of(&second.ticket), now),
        TicketCheck::Valid { account_id: 1, .. }
    ));
}

#[test]
fn heartbeat_and_end_need_the_launch_id() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let minted = store.mint(1, "Player_1234", LAUNCH, now);
    assert!(!store.heartbeat(1, "launch-9999", now));
    assert!(!store.end(1, "launch-9999"));
    assert_eq!(store.live_count(), 1);
    assert!(matches!(
        store.verify("Player_1234", &sha1_of(&minted.ticket), now),
        TicketCheck::Valid { account_id: 1, .. }
    ));
    assert!(store.end(1, LAUNCH));
    assert_eq!(store.live_count(), 0);
}

#[test]
fn store_never_holds_the_ticket() {
    let store = LaunchTicketStore::default();
    let minted = store.mint(1, "Player_1234", LAUNCH, Instant::now());
    let shown = format!("{store:?}");
    assert!(!shown.contains(&minted.ticket));
    assert!(!shown.contains(&sha1_of(&minted.ticket)));
    assert!(shown.contains("live_count"));
}

#[test]
fn reap_counts_expired() {
    let store = LaunchTicketStore::default();
    let t0 = Instant::now();
    store.mint(1, "Player_0001", LAUNCH, t0);
    store.mint(2, "Player_0002", LAUNCH, t0);
    let later = t0 + TICKET_ABSOLUTE;
    store.mint(3, "Player_0003", LAUNCH, later);
    assert_eq!(store.reap(later), 2);
    assert_eq!(store.live_count(), 1);
}

#[test]
fn verify_finds_the_ticket_among_two_same_name_accounts() {
    let now = Instant::now();
    for order in [[1u32, 2], [2, 1]] {
        let store = LaunchTicketStore::default();
        let mut minted = Vec::new();
        for id in order {
            let name = if id == 1 { "Alice" } else { "ALICE" };
            minted.push((id, name, store.mint(id, name, LAUNCH, now).ticket));
        }
        for (id, name, ticket) in &minted {
            assert!(
                matches!(
                    store.verify(name, &sha1_of(ticket), now),
                    TicketCheck::Valid { account_id, .. } if account_id == *id
                ),
                "mint order {order:?}, account {id}"
            );
        }
    }
}

#[test]
fn revoke_account_flips_every_guard_of_the_account() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let first = store.guard(1);
    let second = store.guard(1);
    let other = store.guard(2);
    let minted = store.mint(1, "Player_1234", LAUNCH, now);
    let TicketCheck::Valid {
        guard: minted_guard,
        ..
    } = store.verify("Player_1234", &sha1_of(&minted.ticket), now)
    else {
        panic!("a fresh ticket should verify");
    };
    store.revoke_account(1);
    assert!(!first.is_current());
    assert!(!second.is_current());
    assert!(!minted_guard.is_current());
    assert!(other.is_current());
}

#[test]
fn end_and_expiry_leave_the_guard_current() {
    let store = LaunchTicketStore::default();
    let t0 = Instant::now();
    let minted = store.mint(1, "Player_1234", LAUNCH, t0);
    let TicketCheck::Valid { guard, .. } =
        store.verify("Player_1234", &sha1_of(&minted.ticket), t0)
    else {
        panic!("a fresh ticket should verify");
    };
    assert!(store.end(1, LAUNCH));
    assert!(guard.is_current());

    let second = store.mint(2, "Player_0002", LAUNCH, t0);
    let TicketCheck::Valid {
        guard: expiring, ..
    } = store.verify("Player_0002", &sha1_of(&second.ticket), t0)
    else {
        panic!("a fresh ticket should verify");
    };
    let later = t0 + TICKET_ABSOLUTE;
    assert!(matches!(
        store.verify("Player_0002", &sha1_of(&second.ticket), later),
        TicketCheck::NoMatch
    ));
    assert_eq!(store.live_count(), 0);
    assert!(expiring.is_current());
}

#[test]
fn mint_guarded_with_a_revoked_guard_mints_nothing() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let revoked = store.guard(1);
    store.revoke_account(1);
    assert!(store
        .mint_guarded(1, "Player_1234", LAUNCH, now, &revoked)
        .is_none());
    assert_eq!(store.live_count(), 0);

    let live = store.guard(1);
    assert!(store
        .mint_guarded(1, "Player_1234", LAUNCH, now, &live)
        .is_some());
    assert_eq!(store.live_count(), 1);
}

#[test]
fn verify_refuses_and_removes_a_ticket_whose_guard_was_flipped() {
    let store = LaunchTicketStore::default();
    let now = Instant::now();
    let ticket = "stale-ticket-value";
    {
        let mut inner = store.lock();
        inner.entries.insert(
            1,
            Entry {
                name_lower: "player_1234".to_string(),
                hash: sha256(sha1_of(ticket).as_bytes()),
                launch_id: LAUNCH.to_string(),
                minted: now,
                last_used: now,
                guard: TicketGuard(Arc::new(AtomicBool::new(false))),
                redeemed: false,
            },
        );
    }
    assert!(matches!(
        store.verify("Player_1234", &sha1_of(ticket), now),
        TicketCheck::NoMatch
    ));
    assert_eq!(store.live_count(), 0);
}

#[test]
fn reap_drops_dead_guards() {
    let store = LaunchTicketStore::default();
    let t0 = Instant::now();
    let kept = store.guard(1);
    {
        let _dropped = store.guard(1);
    }
    store.guard(2);
    assert_eq!(store.reap(t0), 0);
    assert!(kept.is_current());
    let inner = store.lock();
    assert_eq!(inner.guards.get(&1).map(Vec::len), Some(1));
    assert!(!inner.guards.contains_key(&2));
}

#[test]
fn an_unredeemed_ticket_expires_after_five_minutes() {
    let store = LaunchTicketStore::default();
    let t0 = Instant::now();
    let minted = store.mint(1, "Player_1234", LAUNCH, t0);
    let sha1 = sha1_of(&minted.ticket);
    let just_after = t0 + TICKET_UNREDEEMED + Duration::from_secs(1);
    assert!(matches!(
        store.verify("Player_1234", &sha1, just_after),
        TicketCheck::NoMatch
    ));
    assert_eq!(store.live_count(), 0);
}

#[test]
fn a_redeemed_ticket_keeps_the_idle_and_absolute_limits() {
    let store = LaunchTicketStore::default();
    let t0 = Instant::now();
    let minted = store.mint(1, "Player_1234", LAUNCH, t0);
    let sha1 = sha1_of(&minted.ticket);
    let redeemed_at = t0 + Duration::from_secs(60);
    assert!(matches!(
        store.verify("Player_1234", &sha1, redeemed_at),
        TicketCheck::Valid { account_id: 1, .. }
    ));
    let still_live = t0 + Duration::from_secs(20 * 60);
    assert!(matches!(
        store.verify("Player_1234", &sha1, still_live),
        TicketCheck::Valid { account_id: 1, .. }
    ));
    let idle_out = still_live + TICKET_IDLE + Duration::from_secs(1);
    assert!(matches!(
        store.verify("Player_1234", &sha1, idle_out),
        TicketCheck::NoMatch
    ));
}

#[test]
fn heartbeat_does_not_extend_an_unredeemed_ticket() {
    let store = LaunchTicketStore::default();
    let t0 = Instant::now();
    let minted = store.mint(1, "Player_1234", LAUNCH, t0);
    let sha1 = sha1_of(&minted.ticket);
    let four_minutes = t0 + Duration::from_secs(4 * 60);
    assert!(store.heartbeat(1, LAUNCH, four_minutes));
    let past_mint_limit = t0 + TICKET_UNREDEEMED + Duration::from_secs(1);
    assert!(!store.heartbeat(1, LAUNCH, past_mint_limit));
    assert!(matches!(
        store.verify("Player_1234", &sha1, past_mint_limit),
        TicketCheck::NoMatch
    ));
}
