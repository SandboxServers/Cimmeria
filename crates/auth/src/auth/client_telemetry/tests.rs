use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::*;

// The process-wide registry is shared by tests running in one process:
// every test that touches it uses its own machine ids, account names and
// addresses. The `unknown`/`absent` split depends on whether anything was
// ever recorded, so those tests use a local `Registry`.

fn ip(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(198, 51, 100, last))
}

fn rec<'a>(sid: &'a str, exp: i64, mid: Option<&'a str>, at: Option<IpAddr>) -> StreamRecord<'a> {
    StreamRecord {
        session_id: sid,
        expires_at_unix: exp,
        machine_id: mid,
        account_name: None,
        peer_ip: at,
    }
}

fn present(sid: &str, mid: Option<&str>) -> ClientTelemetry {
    ClientTelemetry::Present {
        session_id: sid.into(),
        machine_id: mid.map(str::to_string),
    }
}

#[test]
fn nothing_recorded_since_boot_is_unknown_not_absent() {
    let mut reg = Registry::default();
    assert_eq!(
        reg.lookup(Some("anyone"), Some(ip(1)), 0),
        ClientTelemetry::Unknown
    );
    // A record with neither a machine nor an account does not flip it.
    reg.record(&rec("sid", 10, Some("  "), Some(ip(1))));
    assert_eq!(reg.lookup(None, Some(ip(1)), 0), ClientTelemetry::Unknown);
    assert_eq!(ClientTelemetry::Unknown.as_str(), "unknown");
    assert_eq!(ClientTelemetry::Unknown.session_id(), None);
}

#[test]
fn one_machine_on_the_address_is_present() {
    let mut reg = Registry::default();
    reg.record(&rec("sid-a", 1_000, Some("aaaa"), Some(ip(1))));
    let got = reg.lookup(None, Some(ip(1)), 0);
    assert_eq!(got, present("sid-a", Some("aaaa")));
    assert_eq!(got.as_str(), "present");
    assert_eq!(got.machine_id(), Some("aaaa"));
    assert_eq!(got.candidates(), None);
}

#[test]
fn several_machines_on_the_address_are_ambiguous() {
    let mut reg = Registry::default();
    reg.record(&rec("sid-a", 1_000, Some("aaaa"), Some(ip(1))));
    reg.record(&rec("sid-b", 1_000, Some("bbbb"), Some(ip(1))));
    reg.record(&rec("sid-c", 1_000, Some("cccc"), Some(ip(2))));
    let got = reg.lookup(None, Some(ip(1)), 0);
    assert_eq!(got, ClientTelemetry::Ambiguous { candidates: 2 });
    assert_eq!(got.as_str(), "ambiguous");
    assert_eq!(got.candidates(), Some(2));
    assert_eq!(got.session_id(), None);
    // Once one expires, the other is the only candidate.
    reg.record(&rec("sid-b2", 2_000, Some("bbbb"), Some(ip(1))));
    assert_eq!(
        reg.lookup(None, Some(ip(1)), 1_500),
        present("sid-b2", Some("bbbb"))
    );
}

#[test]
fn once_any_stream_is_recorded_an_unmatched_session_is_absent() {
    let mut reg = Registry::default();
    reg.record(&rec("sid-other", 1_000, Some("aaaa"), Some(ip(1))));
    assert_eq!(reg.lookup(None, Some(ip(9)), 0), ClientTelemetry::Absent);
    assert_eq!(reg.lookup(Some("x"), None, 0), ClientTelemetry::Absent);
    // An expired stream leaves `absent`, never back to `unknown`.
    assert_eq!(
        reg.lookup(None, Some(ip(1)), 1_000),
        ClientTelemetry::Absent
    );
    assert_eq!(ClientTelemetry::Absent.as_str(), "absent");
}

#[test]
fn an_account_match_wins_over_an_ambiguous_address() {
    let mut reg = Registry::default();
    reg.record(&rec("sid-a", 1_000, Some("aaaa"), Some(ip(1))));
    reg.record(&StreamRecord {
        account_name: Some(" SGC_Login "),
        ..rec("sid-b", 1_000, Some("bbbb"), Some(ip(1)))
    });
    assert_eq!(
        reg.lookup(Some("sgc_login"), Some(ip(1)), 0),
        present("sid-b", Some("bbbb"))
    );
    assert_eq!(
        reg.lookup(Some("someone"), Some(ip(1)), 0),
        ClientTelemetry::Ambiguous { candidates: 2 }
    );
}

/// The mint may see an IPv4 peer as `::ffff:a.b.c.d` on a dual-stack
/// socket while the game's UDP socket sees plain IPv4.
#[test]
fn a_v4_mapped_peer_matches_the_plain_v4_session() {
    let mut reg = Registry::default();
    let mapped = IpAddr::V6(Ipv4Addr::new(198, 51, 100, 7).to_ipv6_mapped());
    reg.record(&rec("sid-m", 1_000, Some("mmmm"), Some(mapped)));
    assert_eq!(
        reg.lookup(None, Some(ip(7)), 0),
        present("sid-m", Some("mmmm"))
    );
    assert_ne!(
        reg.lookup(None, Some(IpAddr::V6(Ipv6Addr::LOCALHOST)), 0),
        present("sid-m", Some("mmmm"))
    );
}

#[test]
fn a_recorded_stream_is_present_until_it_expires() {
    record_client_telemetry_stream(rec("sid-live", 1_000, Some("tg18-live"), Some(ip(101))));
    assert_eq!(
        client_telemetry_for(None, Some(ip(101)), 999),
        present("sid-live", Some("tg18-live"))
    );
    assert_eq!(
        client_telemetry_for(None, Some(ip(101)), 1_000),
        ClientTelemetry::Absent,
        "a stream is gone at its token's exp"
    );
}

#[test]
fn a_later_mint_replaces_the_earlier_stream() {
    record_client_telemetry_stream(rec(
        "sid-first",
        1_000,
        Some("tg18-relaunch"),
        Some(ip(102)),
    ));
    record_client_telemetry_stream(rec(
        "sid-second",
        2_000,
        Some("tg18-relaunch"),
        Some(ip(102)),
    ));
    assert_eq!(
        client_telemetry_for(None, Some(ip(102)), 0).session_id(),
        Some("sid-second")
    );
}

/// Mint A, mint B, then A's refresh arrives with an earlier `exp` than
/// B's: B must win. Without the guard the stale refresh would point the
/// join at A's dead stream.
#[test]
fn a_stale_refresh_does_not_overwrite_a_newer_stream() {
    let m = Some("tg18-stale");
    record_client_telemetry_stream(rec("sid-a", 1_000, m, Some(ip(103))));
    record_client_telemetry_stream(rec("sid-b", 2_000, m, Some(ip(103))));
    record_client_telemetry_stream(rec("sid-a", 1_500, m, Some(ip(103))));
    assert_eq!(
        client_telemetry_for(None, Some(ip(103)), 0).session_id(),
        Some("sid-b")
    );
    // The same stream's own refresh still extends it.
    record_client_telemetry_stream(rec("sid-b", 3_000, m, Some(ip(103))));
    assert_eq!(
        client_telemetry_for(None, Some(ip(103)), 2_500).session_id(),
        Some("sid-b")
    );
}

/// An unauthenticated caller cycling machine ids cannot grow the registry
/// past its ceiling. The fill expires at 1, earlier than any other test's
/// stream, so eviction takes the fill's own entries first.
#[test]
fn the_registry_stays_bounded_past_max_streams() {
    for i in 0..MAX_STREAMS + 100 {
        record_client_telemetry_stream(StreamRecord {
            account_name: Some(&format!("tg18-cap-acct-{i}")),
            ..rec("sid-cap", 1, Some(&format!("tg18-cap-{i}")), Some(ip(104)))
        });
    }
    let reg = registry().lock().unwrap();
    assert!(
        reg.by_machine.len() <= MAX_STREAMS,
        "{}",
        reg.by_machine.len()
    );
    assert!(
        reg.by_account.len() <= MAX_STREAMS,
        "{}",
        reg.by_account.len()
    );
    assert!(
        reg.by_machine
            .contains_key(&format!("tg18-cap-{}", MAX_STREAMS + 99)),
        "the newest entry survives"
    );
}

#[test]
fn eviction_drops_the_stream_closest_to_expiry() {
    let stream = |exp| Stream {
        session_id: "s".into(),
        machine_id: None,
        peer_ip: None,
        expires_at_unix: exp,
    };
    let mut map = HashMap::from([
        ("late".to_string(), stream(20)),
        ("early".to_string(), stream(10)),
    ]);
    evict_one(&mut map);
    assert!(map.contains_key("late"));
    assert!(!map.contains_key("early"));
}
