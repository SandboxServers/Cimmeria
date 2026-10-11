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

    // A return to character select keeps the session but clears its chaos,
    // and only at its own address.
    let b = addr(40_102);
    set_net(installed, a, 1007, 3, now);
    set_net(installed, b, 1008, 4, now);
    crate::base::helpers::clear_lab_chaos_on_character_select(a);
    assert_eq!(installed.net_for(a, now), None);
    assert!(installed.net_for(b, now).is_some());
    installed.clear(None);
}

#[test]
fn clear_entity_removes_every_entry_tagged_with_it() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    // The same entity on two addresses: a relaunch while the old session
    // lingered. Both go; another entity's entry stays.
    set_net(&chaos, addr(40_061), 77, 1, now);
    set_net(&chaos, addr(40_062), 77, 2, now);
    set_net(&chaos, addr(40_063), 78, 3, now);
    assert_eq!(chaos.clear_entity(77), 2);
    assert_eq!(chaos.clear_entity(77), 0);
    let left: Vec<u32> = chaos.status(now).iter().map(|s| s.entity_id).collect();
    assert_eq!(left, [78]);
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
            Some(FlushChaos::Pack { records: 40 }),
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
        ("pack 255", flush(FlushChaos::Pack { records: 255 }), true),
        ("pack 256", flush(FlushChaos::Pack { records: 256 }), false),
        ("shift 0", flush(FlushChaos::Shift { len: 0 }), true),
        ("shift 1", flush(FlushChaos::Shift { len: 1 }), false),
        ("shift 2", flush(FlushChaos::Shift { len: 2 }), true),
        ("shift 36", flush(FlushChaos::Shift { len: 36 }), true),
        ("shift 37", flush(FlushChaos::Shift { len: 37 }), false),
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

/// The net shim's lock-free check (CH-06) follows every path that adds or
/// removes net chaos. Fails if any writer skips the recount: the shim would
/// then skip a live entry (no chaos) or lock on every send forever.
#[test]
fn has_net_follows_every_net_entry_change() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    assert!(!chaos.has_net(), "a new registry has no net chaos");
    chaos
        .set(
            addr(40_111),
            1,
            "flush:pack".into(),
            None,
            Some(FlushChaos::Pack { records: 24 }),
            DEFAULT_TTL_S,
            now,
        )
        .expect("a valid set");
    assert!(!chaos.has_net(), "flush-only chaos is not net chaos");
    set_net(&chaos, addr(40_112), 2, 1, now);
    assert!(chaos.has_net(), "set");
    assert_eq!(chaos.clear(Some(addr(40_112))), 1);
    assert!(!chaos.has_net(), "clear one");
    set_net(&chaos, addr(40_112), 2, 1, now);
    assert!(chaos.clear_if_not(addr(40_112), 3));
    assert!(!chaos.has_net(), "clear_if_not");
    set_net(&chaos, addr(40_112), 2, 1, now);
    assert_eq!(chaos.clear(None), 2);
    assert!(!chaos.has_net(), "clear all");
    set_net(&chaos, addr(40_112), 2, 1, now);
    let after = now + Duration::from_secs(u64::from(DEFAULT_TTL_S));
    assert!(
        chaos.has_net(),
        "expired but not yet removed still reads true"
    );
    assert_eq!(chaos.net_for(addr(40_112), after), None);
    assert!(!chaos.has_net(), "an expired entry removed by a read");
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

/// A re-set with identical knobs reads back the same knobs under a new
/// generation, which is what restarts the net shim's seeded streams.
#[test]
fn every_set_gets_a_new_generation() {
    let chaos = LabChaos::new();
    let now = Instant::now();
    set_net(&chaos, addr(40_041), 1, 7, now);
    let (first_net, first) = chaos.net_entry_for(addr(40_041), now).expect("set");
    set_net(&chaos, addr(40_041), 1, 7, now);
    let (again_net, again) = chaos.net_entry_for(addr(40_041), now).expect("set");
    assert_eq!(again_net, first_net);
    assert!(again > first, "generation {again} after {first}");
    assert_eq!(chaos.net_for(addr(40_041), now), Some(first_net));
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
