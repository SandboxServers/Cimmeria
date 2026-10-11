//! `RunChaos` parsing, bounds and headers (lab-chaos CH-08).

use super::*;

fn run(texts: &[&str]) -> Result<RunChaos, String> {
    let texts: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
    RunChaos::parse(&texts, 7)
}

#[test]
fn burst_is_refused_with_the_deferral() {
    let err = run(&["burst"]).unwrap_err();
    assert!(err.contains(BURST_DEFERRED), "{err}");
    assert!(run(&["cpu", "burst:count=5"]).is_err());
}

#[test]
fn a_profile_named_twice_and_split_seeds_are_refused() {
    assert!(run(&["net", "net:break"])
        .unwrap_err()
        .contains("named twice"));
    assert!(run(&["net:seed=1", "flush:reorder:seed=2"])
        .unwrap_err()
        .contains("one server seed"));
    assert!(run(&["net:seed=3", "flush:reorder:seed=3"]).is_ok());
    assert!(run(&["net:seed=1", "flush:shift:seed=2"])
        .unwrap_err()
        .contains("one server seed"));
}

/// Regression guard: a flush mode that reads no seed (pack, delay, a
/// fixed shift) does not have to match the net's; the set carries the
/// net's.
#[test]
fn an_unseeded_flush_mode_does_not_clash_with_the_net_seed() {
    for flush in ["flush:pack", "flush:delay", "flush:shift:shift=4"] {
        let chaos = run(&["net:seed=1", flush]).unwrap_or_else(|e| panic!("{flush}: {e}"));
        assert_eq!(chaos.server_set_args(1, 60).unwrap().unwrap()["seed"], 1);
    }
}

/// One case per server bound (base-session `lab_chaos/bounds.rs`): the
/// edge is accepted and one past it refused, so the plan refuses what
/// the set would.
#[test]
fn net_latency_bound() {
    assert!(run(&["net:latency=5000"]).is_ok());
    assert!(run(&["net:latency=5001"])
        .unwrap_err()
        .contains("over the server's 5000"));
}

#[test]
fn net_jitter_bound() {
    assert!(run(&["net:jitter=5000ms"]).is_ok());
    assert!(run(&["net:jitter=5001ms"])
        .unwrap_err()
        .contains("over the server's 5000"));
}

#[test]
fn net_loss_bound() {
    assert!(run(&["net:loss=100"]).is_ok());
    assert!(run(&["net:loss=100.1"]).unwrap_err().contains("0 to 100"));
}

#[test]
fn net_dup_bound() {
    assert!(run(&["net:dup=100%"]).is_ok());
    assert!(run(&["net:dup=101%"]).unwrap_err().contains("0 to 100"));
}

#[test]
fn net_reorder_bound() {
    assert!(run(&["net:reorder=1000"]).is_ok());
    assert!(run(&["net:reorder=1001"])
        .unwrap_err()
        .contains("over the server's 1000"));
}

#[test]
fn flush_delay_bound() {
    assert!(run(&["flush:delay:delay_ms=60000"]).is_ok());
    assert!(run(&["flush:delay:delay_ms=60001"])
        .unwrap_err()
        .contains("over the server's 60000"));
}

#[test]
fn flush_pack_records_bound() {
    assert!(run(&["flush:pack:records=1"]).is_ok());
    assert!(run(&["flush:pack:records=34"]).is_ok());
    for bad in ["flush:pack:records=0", "flush:pack:records=35"] {
        assert!(run(&[bad]).unwrap_err().contains("not 1 to 34"), "{bad}");
    }
}

#[test]
fn flush_shift_bound() {
    for ok in [
        "flush:shift:shift=0",
        "flush:shift:shift=2",
        "flush:shift:shift=38",
    ] {
        assert!(run(&[ok]).is_ok(), "{ok}");
    }
    for bad in ["flush:shift:shift=1", "flush:shift:shift=39"] {
        assert!(
            run(&[bad]).unwrap_err().contains("not 0 or 2 to 38"),
            "{bad}"
        );
    }
}

/// A control character would reach the server's label, which refuses it.
#[test]
fn a_control_character_is_refused() {
    assert!(run(&["net:latency=1\t0"])
        .unwrap_err()
        .contains("control character"));
    // Outer whitespace is trimmed, as for every text.
    assert!(run(&["net:latency=10\n"]).is_ok());
}

/// The plan does not know the run's seed: a seed the text named stays,
/// the run's own reads `<per run>`.
#[test]
fn the_plan_header_names_only_seeds_the_text_named() {
    let chaos = run(&["cpu", "flush:reorder:seed=9"]).unwrap();
    assert_eq!(
        chaos.plan_headers(),
        [
            "chaos=cpu:safe seed=<per run>",
            "chaos=flush:reorder seed=9"
        ]
    );
    assert_eq!(chaos.headers()[0], "chaos=cpu:safe seed=7");
    // A named seed equal to the default is still named.
    let same = run(&["net:seed=7"]).unwrap();
    assert!(same.plan_headers()[0].ends_with(" seed=7"));
}

#[test]
fn knobs_the_server_would_refuse_fail_at_parse() {
    for bad in [
        "net:latency=fast",
        "net:loss=0.05",
        "net:loss=-1",
        "net:latency=0,jitter=0,loss=0,dup=0,reorder=0",
        "flush:pack:records=70000",
        "flush:pack:records=-1",
        "flush:shift:shift=wide",
        "cpu:cores=0",
    ] {
        assert!(run(&[bad]).is_err(), "{bad} should be refused");
    }
}

#[test]
fn net_and_flush_go_in_one_set_call() {
    let chaos = run(&["net:latency=120ms,loss=1%", "flush:pack"]).unwrap();
    let args = chaos.server_set_args(42, 360).unwrap().unwrap();
    assert_eq!(args["entity_id"], 42);
    assert_eq!(args["ttl_s"], 360);
    assert_eq!(args["net"]["latency"], 120);
    assert_eq!(args["net"]["loss"], 1.0);
    assert_eq!(args["flush"], json!({ "mode": "pack", "records": 24 }));
    assert_eq!(args["seed"], 7, "net reads the seed");
    assert_eq!(
        args["label"],
        "net:latency=120ms,loss=1% seed=7 flush:pack seed=7"
    );
}

/// The tool refuses a seed nothing reads, so a pack-only set has none,
/// and a reorder carries it.
#[test]
fn the_seed_goes_only_where_it_is_read() {
    let pack = run(&["flush:pack"]).unwrap();
    let args = pack.server_set_args(1, 60).unwrap().unwrap();
    assert!(args.get("seed").is_none());
    let reorder = run(&["flush:reorder"]).unwrap();
    assert_eq!(reorder.server_set_args(1, 60).unwrap().unwrap()["seed"], 7);
    let sweep = run(&["flush:shift"]).unwrap();
    let args = sweep.server_set_args(1, 60).unwrap().unwrap();
    assert_eq!(args["flush"]["shift"], "sweep");
    assert_eq!(args["seed"], 7);
}

#[test]
fn cpu_alone_sends_nothing_to_the_server() {
    let chaos = run(&["cpu"]).unwrap();
    assert!(!chaos.has_server_profile());
    assert_eq!(chaos.server_set_args(1, 60).unwrap(), None);
    assert!(chaos.cpu_stress().unwrap().is_some());
    assert_eq!(
        chaos.header_line().as_deref(),
        Some(header(&chaos.specs[0]).as_str())
    );
}

#[test]
fn no_chaos_has_no_header_and_empty_texts_are_skipped() {
    let chaos = run(&["", "  "]).unwrap();
    assert!(chaos.is_empty());
    assert_eq!(chaos.header_line(), None);
}

#[test]
fn the_run_seed_is_stable_and_differs_by_run() {
    assert_eq!(seed_of("abc"), seed_of("abc"));
    assert_ne!(seed_of("abc"), seed_of("abd"));
}
