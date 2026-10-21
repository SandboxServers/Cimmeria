use std::net::IpAddr;

use super::*;

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

async fn register(registry: &SessionRegistry, entity_id: u32, from: &str) -> String {
    registry
        .register(
            entity_id,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(ip(from)),
            1,
        )
        .await
        .unwrap()
}

#[test]
fn admission_key_classifies_public_and_non_public() {
    let cases = [
        ("127.0.0.1", false),
        ("::1", false),
        ("10.1.2.3", false),
        ("172.16.0.1", false),
        ("172.31.255.255", false),
        ("172.32.0.1", true),
        ("192.168.1.1", false),
        ("100.64.0.1", false),
        ("100.128.0.1", true),
        ("169.254.1.1", false),
        ("fd00::1", false),
        ("fe80::1", false),
        ("8.8.8.8", true),
        ("2001:4860::8888", true),
        // Canonicalized to 10.0.0.1 before classifying.
        ("::ffff:10.0.0.1", false),
    ];
    for (addr, public) in cases {
        let key = admission_key(ip(addr));
        assert_eq!(
            matches!(key, AdmissionKey::Public(_)),
            public,
            "{addr} classified as {key:?}"
        );
    }
}

#[tokio::test]
async fn no_sessions_refuses_unexpected_peer() {
    let registry = SessionRegistry::new();
    let refusal = registry.try_admit(ip("203.0.113.5")).await.unwrap_err();
    assert_eq!(refusal.reason, "unexpected_peer");
    assert_eq!(refusal.pending_sessions, 0);
}

#[tokio::test]
async fn sessions_for_other_peers_refuse_with_sessions() {
    let registry = SessionRegistry::new();
    register(&registry, 1, "203.0.113.5").await;
    let refusal = registry.try_admit(ip("198.51.100.9")).await.unwrap_err();
    assert_eq!(refusal.reason, "unexpected_peer_with_sessions");
    assert_eq!(refusal.pending_sessions, 1);
}

#[tokio::test]
async fn one_ticket_admits_one_connection() {
    let registry = SessionRegistry::new();
    register(&registry, 1, "203.0.113.5").await;
    let first = registry.try_admit(ip("203.0.113.5")).await.unwrap();
    let second = registry.try_admit(ip("203.0.113.5")).await.unwrap_err();
    assert_eq!(second.reason, "no_free_ticket");
    drop(first);
    assert!(registry.try_admit(ip("203.0.113.5")).await.is_ok());
}

#[tokio::test]
async fn three_tickets_admit_three() {
    let registry = SessionRegistry::new();
    for id in 1..=3 {
        register(&registry, id, "203.0.113.5").await;
    }
    let mut guards = vec![
        registry.try_admit(ip("203.0.113.5")).await.unwrap(),
        registry.try_admit(ip("203.0.113.5")).await.unwrap(),
    ];
    // One connection claims its ticket: the session turns connected while
    // its guard is still held. It must keep counting as a slot.
    registry
        .authenticate_and_claim(
            1,
            &{ registry.peek_session(1).await.unwrap().ticket },
            "Livewire",
        )
        .await
        .unwrap();
    guards.push(registry.try_admit(ip("203.0.113.5")).await.unwrap());
    let refusal = registry.try_admit(ip("203.0.113.5")).await.unwrap_err();
    assert_eq!(refusal.reason, "no_free_ticket");
}

#[tokio::test]
async fn non_public_peers_share_the_ticket_pool() {
    let registry = SessionRegistry::new();
    register(&registry, 1, "192.168.1.10").await;
    let guard = registry.try_admit(ip("172.17.0.1")).await.unwrap();
    assert_eq!(
        registry.try_admit(ip("10.0.0.2")).await.unwrap_err().reason,
        "no_free_ticket"
    );
    drop(guard);
}

#[tokio::test(start_paused = true)]
async fn expired_pending_sessions_give_no_slot() {
    let registry = SessionRegistry::new();
    register(&registry, 1, "203.0.113.5").await;
    tokio::time::advance(PENDING_SESSION_TTL + std::time::Duration::from_secs(1)).await;
    // Not swept yet (no register/sweep ran), so the entry is still in the
    // map: the age check alone must withhold the slot.
    assert_eq!(registry.session_count().await, 1);
    let refusal = registry.try_admit(ip("203.0.113.5")).await.unwrap_err();
    assert_eq!(refusal.reason, "unexpected_peer_with_sessions");
}

/// Poison the counter map, as a panic while it is locked would.
fn poison(registry: &SessionRegistry) {
    let admitted = Arc::clone(&registry.admitted);
    let _ = std::thread::spawn(move || {
        let _held = admitted.lock().unwrap();
        panic!("poison the admission counters");
    })
    .join();
    assert!(registry.admitted.is_poisoned());
}

#[tokio::test]
async fn guard_drop_and_admission_survive_a_poisoned_counter() {
    let registry = SessionRegistry::new();
    register(&registry, 1, "203.0.113.5").await;
    let guard = registry.try_admit(ip("203.0.113.5")).await.unwrap();
    poison(&registry);
    // Drop must not panic, and the slot must come back.
    drop(guard);
    assert!(registry.try_admit(ip("203.0.113.5")).await.is_ok());
}

// Multi-threaded, so the 16 admissions really run in parallel: a count taken
// without holding the lock across the increment would admit more than one.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn guard_drop_frees_the_slot_under_concurrency() {
    let registry = SessionRegistry::new();
    register(&registry, 1, "203.0.113.5").await;
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let registry = registry.clone();
        tasks.push(tokio::spawn(async move {
            registry.try_admit(ip("203.0.113.5")).await
        }));
    }
    // Hold every result so no guard drops until all 16 have answered.
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap());
    }
    let admitted = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(admitted, 1);
    drop(results);
    assert!(registry.try_admit(ip("203.0.113.5")).await.is_ok());
}
