use super::*;
use crate::base::helpers::{destroy_client_entities, destroy_owned_client_entities};
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_entity::manager::EntityManager;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;
use tracing::Level;

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

/// Sets net chaos with the default TTL from `now`.
fn set_net(chaos: &LabChaos, addr: SocketAddr, entity_id: u32, seed: u64, now: Instant) {
    chaos
        .set(
            addr,
            entity_id,
            format!("net seed={seed}"),
            Some(net(seed)),
            None,
            DEFAULT_TTL_S,
            now,
        )
        .expect("a valid set");
}

/// The process-wide registry: the runtime gate, then the teardown clear,
/// which reads `current()`. This is the only test that touches the static,
/// so the "nothing installed" check cannot race another test.
#[test]
fn process_registry_gate_and_teardown_clear() {
    assert_eq!(install(false).err(), Some(ChaosRefused::NotDeveloperMode));
    assert!(
        current().is_none(),
        "a refused install leaves nothing installed"
    );
    let installed = install(true).expect("DEVELOPER_MODE installs");
    assert!(current().is_some_and(|c| std::ptr::eq(c, installed)));
    assert_eq!(install(false).err(), Some(ChaosRefused::NotDeveloperMode));
    assert!(
        current().is_some(),
        "startup-only gate: a later refusal uninstalls nothing"
    );

    let a = addr(40_101);
    let now = Instant::now();
    installed
        .set(
            a,
            1007,
            "net+flush".into(),
            Some(net(3)),
            Some(FlushChaos::Pack { records: 24 }),
            DEFAULT_TTL_S,
            now,
        )
        .expect("a valid set");
    let connected = Arc::new(Mutex::new(HashMap::from([(
        a,
        test_default_connected_client_state(),
    )])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let transport: Arc<dyn cimmeria_mercury::transport::Transport> = Arc::new(TestTransport::new());

    // A late teardown from a session the address no longer belongs to (the
    // relaunch-takeover race) must not clear the current session's chaos.
    let stale_owner = Arc::new(AtomicBool::new(false));
    assert!(!destroy_owned_client_entities(
        &connected,
        &entity_manager,
        a,
        &None,
        &entity_to_addr,
        &transport,
        &None,
        "inactivity_timeout",
        &stale_owner,
    ));
    assert!(installed.net_for(a, now).is_some());

    // The session's own teardown clears it.
    destroy_client_entities(
        &connected,
        &entity_manager,
        a,
        &None,
        &entity_to_addr,
        &transport,
        &None,
        "client_disconnect",
    );
    assert_eq!(installed.net_for(a, now), None);
    assert_eq!(installed.flush_for(a, now), None);
}

#[test]
fn expired_entry_reads_as_none() {
    let chaos = LabChaos::new();
    let then = Instant::now();
    set_net(&chaos, addr(40_001), 1, 7, then);
    chaos
        .set(
            addr(40_002),
            2,
            "flush:pack".into(),
            None,
            Some(FlushChaos::Pack { records: 30 }),
            DEFAULT_TTL_S,
            then,
        )
        .expect("a valid set");
    let after = then + Duration::from_secs(u64::from(DEFAULT_TTL_S));
    assert_eq!(chaos.net_for(addr(40_001), after), None);
    assert_eq!(chaos.flush_for(addr(40_002), after), None);
    assert!(
        chaos.status(after).is_empty(),
        "expired entries are removed, not just hidden"
    );
}

#[test]
fn set_returns_the_expiry() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    let expires_at = chaos
        .set(addr(40_041), 9, "net".into(), Some(net(1)), None, 60, now)
        .expect("a valid set");
    assert_eq!(expires_at, now + Duration::from_secs(60));
    assert_eq!(chaos.status(now)[0].expires_at, expires_at);
}

/// Security condition C1: the cap lives in the registry, not only in the
/// tool. Fails if the check is removed.
#[test]
fn ttl_over_the_cap_is_refused() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    let refused = chaos.set(
        addr(40_051),
        9,
        "net".into(),
        Some(net(1)),
        None,
        MAX_TTL_S + 1,
        now,
    );
    assert_eq!(refused, Err(ChaosRefused::TtlTooLong(MAX_TTL_S + 1)));
    assert!(chaos.status(now).is_empty(), "a refused set stores nothing");
    assert!(chaos
        .set(
            addr(40_051),
            9,
            "net".into(),
            Some(net(1)),
            None,
            MAX_TTL_S,
            now
        )
        .is_ok());
}

/// An entry with a TTL of 0 would be expired on arrival. Fails if the check
/// is removed.
#[test]
fn ttl_of_zero_is_refused() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    let refused = chaos.set(addr(40_061), 9, "net".into(), Some(net(1)), None, 0, now);
    assert_eq!(refused, Err(ChaosRefused::BadTtl(0)));
    assert!(chaos.status(now).is_empty(), "a refused set stores nothing");
}

/// Every knob bound, each refused alone at one past its limit, with the
/// limit itself accepted. A dropped check shows as its own row.
#[test]
fn knob_bounds_are_refused() {
    let net_with = |f: fn(&mut NetChaos)| {
        let mut n = net(1);
        n.latency_ms = 0;
        n.loss_per_thousand = 0;
        f(&mut n);
        (Some(n), None)
    };
    let flush = |f: FlushChaos| (None, Some(f));
    let cases: Vec<(&str, (Option<NetChaos>, Option<FlushChaos>), bool)> = vec![
        ("loss 1000", net_with(|n| n.loss_per_thousand = 1000), true),
        ("loss 1001", net_with(|n| n.loss_per_thousand = 1001), false),
        ("dup 1000", net_with(|n| n.dup_per_thousand = 1000), true),
        ("dup 1001", net_with(|n| n.dup_per_thousand = 1001), false),
        ("latency 5000", net_with(|n| n.latency_ms = 5000), true),
        ("latency 5001", net_with(|n| n.latency_ms = 5001), false),
        ("jitter 5000", net_with(|n| n.jitter_ms = 5000), true),
        ("jitter 5001", net_with(|n| n.jitter_ms = 5001), false),
        ("reorder 1000", net_with(|n| n.reorder = 1000), true),
        ("reorder 1001", net_with(|n| n.reorder = 1001), false),
        ("delay 60000", flush(FlushChaos::Delay { ms: 60_000 }), true),
        (
            "delay 60001",
            flush(FlushChaos::Delay { ms: 60_001 }),
            false,
        ),
        ("pack 0", flush(FlushChaos::Pack { records: 0 }), false),
        ("pack 1", flush(FlushChaos::Pack { records: 1 }), true),
        ("pack 34", flush(FlushChaos::Pack { records: 34 }), true),
        ("pack 35", flush(FlushChaos::Pack { records: 35 }), false),
        ("shift 0", flush(FlushChaos::Shift { len: 0 }), true),
        ("shift 1", flush(FlushChaos::Shift { len: 1 }), false),
        ("shift 2", flush(FlushChaos::Shift { len: 2 }), true),
        ("shift 36", flush(FlushChaos::Shift { len: 36 }), true),
        ("shift 37", flush(FlushChaos::Shift { len: 37 }), true),
        ("shift 38", flush(FlushChaos::Shift { len: 38 }), true),
        ("shift 39", flush(FlushChaos::Shift { len: 39 }), false),
        ("no profile", (None, None), false),
    ];
    let chaos = LabChaos::new();
    let now = Instant::now();
    let mut wrong = Vec::new();
    for (name, (net, flush), accepted) in cases {
        let got = chaos.set(addr(40_071), 9, name.into(), net, flush, 60, now);
        let ok = match &got {
            Ok(_) => accepted,
            Err(ChaosRefused::BadKnob(_)) => !accepted,
            Err(_) => false,
        };
        if !ok {
            wrong.push(format!("{name}: {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "bounds not enforced: {wrong:#?}");
}

/// The pack cap is derived from the packet size, the record length and the
/// longest filler, and the refusal texts name the caps. Fails if any input
/// moves without the derivation, or the text the lab tools return, following.
#[test]
fn pack_cap_is_one_packet_with_the_longest_filler() {
    assert_eq!(SHIFT_LEN_MAX, 38);
    assert_eq!(PACK_RECORDS_MAX, 34);
    let fill = |records: u16| usize::from(SHIFT_LEN_MAX + records * PHASE1_RECORD_LEN);
    assert!(fill(PACK_RECORDS_MAX) <= cimmeria_mercury::packet::FRAGMENT_BODY_SIZE);
    assert!(fill(PACK_RECORDS_MAX + 1) > cimmeria_mercury::packet::FRAGMENT_BODY_SIZE);

    let chaos = LabChaos::new();
    let refusal = |flush: FlushChaos| match chaos.set(
        addr(40_091),
        9,
        "cap".into(),
        None,
        Some(flush),
        60,
        Instant::now(),
    ) {
        Err(ChaosRefused::BadKnob(text)) => text,
        other => panic!("not refused as a knob: {other:?}"),
    };
    let pack = refusal(FlushChaos::Pack {
        records: PACK_RECORDS_MAX + 1,
    });
    assert!(pack.contains(&PACK_RECORDS_MAX.to_string()), "{pack}");
    let shift = refusal(FlushChaos::Shift {
        len: SHIFT_LEN_MAX + 1,
    });
    assert!(shift.contains(&SHIFT_LEN_MAX.to_string()), "{shift}");
}

/// `set` sweeps every expired entry, not just the one it replaces, so the
/// map stays bounded without a reader touching each address.
#[test]
fn set_sweeps_expired_entries() {
    let chaos = LabChaos::new();
    let then = Instant::now();
    chaos
        .set(addr(40_081), 1, "net".into(), Some(net(1)), None, 1, then)
        .expect("a valid set");
    let later = then + Duration::from_secs(2);
    set_net(&chaos, addr(40_082), 2, 2, later);
    assert_eq!(chaos.sessions().len(), 1, "the expired entry was swept");
}

#[test]
fn clear_none_clears_every_session() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    set_net(&chaos, addr(40_011), 1, 1, now);
    set_net(&chaos, addr(40_012), 2, 2, now);
    set_net(&chaos, addr(40_013), 3, 3, now);
    assert_eq!(chaos.clear(Some(addr(40_011))), 1);
    assert_eq!(chaos.status(now).len(), 2);
    assert_eq!(chaos.clear(None), 2);
    assert!(chaos.status(now).is_empty());
    assert_eq!(chaos.net_for(addr(40_012), now), None);
    assert_eq!(chaos.net_for(addr(40_013), now), None);
}

#[test]
fn clear_if_not_drops_only_another_entitys_chaos() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    set_net(&chaos, addr(40_091), 1007, 5, now);
    assert!(
        !chaos.clear_if_not(addr(40_091), 1007),
        "the owner's own chaos stays"
    );
    assert!(chaos.net_for(addr(40_091), now).is_some());
    assert!(chaos.clear_if_not(addr(40_091), 2001));
    assert_eq!(chaos.net_for(addr(40_091), now), None);
    assert!(
        !chaos.clear_if_not(addr(40_091), 2001),
        "nothing left to clear"
    );
}

#[test]
fn status_reports_entity_ids_in_order() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    set_net(&chaos, addr(40_031), 30, 1, now);
    set_net(&chaos, addr(40_032), 10, 2, now);
    let ids: Vec<u32> = chaos.status(now).iter().map(|c| c.entity_id).collect();
    assert_eq!(ids, [10, 30]);
}

#[test]
fn scope_is_per_address() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    set_net(&chaos, addr(40_021), 1, 9, now);
    assert_eq!(chaos.net_for(addr(40_021), now).map(|n| n.seed), Some(9));
    // Same IP, other port: a second client on the same machine.
    assert_eq!(chaos.net_for(addr(40_022), now), None);
    assert_eq!(chaos.flush_for(addr(40_022), now), None);
}

/// The set WARN names the session by a hash only: the logs reach SigNoz
/// and Discord, and a lab address is still a player-style address.
#[test]
fn set_logs_a_hash_never_the_address() {
    let capture = LogCapture::install();
    let chaos = LabChaos::new();
    chaos
        .set(
            addr(40_031),
            9,
            "flush:pack seed=7".into(),
            None,
            Some(FlushChaos::Reorder { seed: 7 }),
            DEFAULT_TTL_S,
            Instant::now(),
        )
        .expect("a valid set");
    let row = capture
        .find_message(Level::WARN, "lab chaos set")
        .expect("the set WARN");
    assert_eq!(row.target, "lab.chaos");
    assert!(row.has_field("profile", "flush"));
    assert!(row.has_field("level", "flush:pack seed=7"));
    assert!(row.has_field("seed", "7"));
    assert!(row.has_field("expires_in_s", "300"));
    let hash = row.fields.get("addr_hash").expect("addr_hash");
    assert!(
        hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit()),
        "addr_hash is a 64-bit hex hash: {hash}"
    );
    for value in row.fields.values() {
        assert!(!value.contains("127.0.0.1"), "address leaked: {value}");
    }
}
