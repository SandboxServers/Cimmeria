use super::*;

fn limiter() -> RateLimiter {
    RateLimiter::with_limits(&Limits::default())
}

#[test]
fn limiter_at_cap_drops_every_stale_window_first() {
    let rl = limiter();
    let t0 = Instant::now();
    for i in 0..MAX_KEYS {
        let key = format!("key-{i}");
        assert!(rl.check(Bucket::AnyRoute, &key, t0).is_ok());
    }
    let later = t0 + Duration::from_secs(61);
    assert!(rl.check(Bucket::AnyRoute, "new", later).is_ok());
    assert_eq!(rl.window_count(), 1);
}

#[test]
fn limiter_never_evicts_a_live_link_account_window_while_ip_windows_remain() {
    let rl = limiter();
    let t0 = Instant::now();
    for _ in 0..5 {
        assert!(rl.check(Bucket::LinkExistingAccount, "victim", t0).is_ok());
    }
    // The victim fills one slot, so the IP keys fill the rest of the cap.
    let t1 = t0 + Duration::from_secs(1);
    for i in 0..MAX_KEYS - 1 {
        let key = format!("203.0.113.{i}");
        assert!(rl.check(Bucket::LinkExistingIp, &key, t1).is_ok());
    }
    let t2 = t0 + Duration::from_secs(2);
    assert!(rl.check(Bucket::LinkExistingIp, "198.51.100.1", t2).is_ok());
    let t3 = t0 + Duration::from_secs(3);
    assert!(rl.check(Bucket::LinkExistingAccount, "victim", t3).is_err());
}

#[test]
fn limiter_full_of_live_account_windows_refuses_a_new_key() {
    let rl = limiter();
    let t0 = Instant::now();
    for i in 0..MAX_KEYS {
        let key = format!("Player_{i}");
        assert!(rl.check(Bucket::LinkExistingAccount, &key, t0).is_ok());
    }
    let t1 = t0 + Duration::from_secs(1);
    let refused = rl
        .check(Bucket::LinkExistingAccount, "Player_new", t1)
        .expect_err("a full map of live account windows refuses a new key");
    assert_eq!(refused, RetryAfter { secs: 60 });
    assert_eq!(rl.window_count(), MAX_KEYS);
}
