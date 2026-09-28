//! The send natives against the Lua stack simulator and a recording
//! engine: argument rules, the bytes handed to the engine, the return
//! values, and registration.

use cimmeria_patch_wire::black_market::{
    BMCancelAuction, BMCreateAuction, BMPlaceBid, BMSearch, BMSearchOptions, BMStartWatchingItem,
    BMStopWatchingItem, CellCall, CellMethod,
};
use cimmeria_patch_wire::Encode;

use super::register::{ensure, Registration, VERSION_KEY};
use super::{run, send, Engine, Native, Refusal, NATIVE_TABLE, VERSION};
use crate::counters::Counters;
use crate::deliver::fake_lua::{FakeLua, V};
use std::sync::atomic::Ordering;

/// Records what a native hands the engine.
#[derive(Default)]
struct Recorder {
    off_main_thread: bool,
    refuse: Option<Refusal>,
    panic: bool,
    sends: Vec<(u8, Vec<u8>)>,
}

impl Engine for Recorder {
    fn on_main_thread(&self) -> bool {
        !self.off_main_thread
    }

    fn send(&mut self, sub_index: u8, payload: &[u8]) -> Result<(), Refusal> {
        if self.panic {
            panic!("engine stand-in panicked");
        }
        if let Some(refusal) = self.refuse.clone() {
            return Err(refusal);
        }
        self.sends.push((sub_index, payload.to_vec()));
        Ok(())
    }
}

fn s(text: &str) -> V {
    V::Str(text.into())
}

/// Call `native` with `args` and return the engine, the values it returned
/// to Lua, and the counters.
fn call(native: Native, args: Vec<V>) -> (Recorder, Vec<V>, Counters) {
    call_with(native, args, Recorder::default())
}

fn call_with(native: Native, args: Vec<V>, mut engine: Recorder) -> (Recorder, Vec<V>, Counters) {
    let arg_count = args.len();
    let mut lua = FakeLua::called_with(args);
    let counters = Counters::new();
    let returned = run(&mut lua, native, &mut engine, &counters);
    assert_eq!(
        lua.stack.len(),
        arg_count + returned as usize,
        "only the return values are left above the arguments"
    );
    let results = lua.stack.split_off(arg_count);
    (engine, results, counters)
}

fn expect_sent(native: Native, args: Vec<V>, expected: CellCall) {
    let (engine, results, counters) = call(native, args);
    assert_eq!(results, [V::Bool(true)], "{native:?} returns true");
    assert_eq!(
        engine.sends,
        [(expected.method().sub_index(), expected.to_bytes().unwrap())],
        "{native:?} sends {:?}",
        expected.method()
    );
    assert_eq!(counters.sent.load(Ordering::Relaxed), 1);
}

fn expect_refused(native: Native, args: Vec<V>, reason: &str) {
    let (engine, results, counters) = call(native, args.clone());
    assert_eq!(
        results,
        [V::Nil, s(reason)],
        "{native:?} with {args:?} returns nil, {reason}"
    );
    assert!(engine.sends.is_empty(), "nothing was sent");
    assert_eq!(counters.send_refused.load(Ordering::Relaxed), 1);
}

// ── what each native sends ───────────────────────────────────────────────

#[test]
fn bid_sends_place_bid_byte_exact() {
    let (engine, _, _) = call(Native::Bid, vec![V::Num(7), V::Num(1500)]);
    // BMPlaceBid: sub-index 2 (63 - 61), then INT32 sequenceId, INT32
    // bidAmount, little-endian.
    assert_eq!(engine.sends, [(2, vec![0x07, 0, 0, 0, 0xDC, 0x05, 0, 0])]);
    expect_sent(
        Native::Bid,
        vec![V::Num(7), V::Num(1500)],
        CellCall::PlaceBid(BMPlaceBid {
            sequence_id: 7,
            bid_amount: 1500,
        }),
    );
}

#[test]
fn cancel_sends_cancel_auction() {
    expect_sent(
        Native::Cancel,
        vec![V::Num(-3)],
        CellCall::CancelAuction(BMCancelAuction { sequence_id: -3 }),
    );
}

/// The Lua call takes (item, starting, buyout, length); the wire carries
/// the `.def` order (item, buyout, length, starting).
#[test]
fn create_reorders_its_arguments_into_def_order() {
    let (engine, _, _) = call(
        Native::Create,
        vec![V::Num(0x11), V::Num(0x22), V::Num(0x33), V::Num(4)],
    );
    assert_eq!(
        engine.sends,
        [(
            1,
            vec![
                0x11, 0, 0, 0, // itemInstanceId
                0x33, 0, 0, 0, // buyoutPrice
                4, // auctionLength
                0x22, 0, 0, 0, // startingPrice
            ]
        )]
    );
    assert_eq!(
        engine.sends[0].1.len(),
        BMCreateAuction::WIRE_LEN,
        "13 bytes"
    );
}

#[test]
fn create_without_a_buyout_sends_zero() {
    expect_sent(
        Native::Create,
        vec![V::Num(9), V::Num(100), V::Nil, V::Num(5)],
        CellCall::CreateAuction(BMCreateAuction {
            item_instance_id: 9,
            buyout_price: 0,
            auction_length: 5,
            starting_price: 100,
        }),
    );
}

#[test]
fn watch_picks_start_or_stop_by_lua_truthiness() {
    for enable in [V::Bool(true), V::Num(0), s("")] {
        expect_sent(
            Native::Watch,
            vec![V::Num(42), enable],
            CellCall::StartWatchingItem(BMStartWatchingItem { item_def_id: 42 }),
        );
    }
    for args in [
        vec![V::Num(42), V::Bool(false)],
        vec![V::Num(42), V::Nil],
        vec![V::Num(42)],
    ] {
        expect_sent(
            Native::Watch,
            args,
            CellCall::StopWatchingItem(BMStopWatchingItem { item_def_id: 42 }),
        );
    }
}

// ── search options ───────────────────────────────────────────────────────

fn defaults() -> BMSearchOptions {
    BMSearchOptions {
        quality: 2000,
        ..BMSearchOptions::default()
    }
}

fn run_in(
    mut lua: FakeLua,
    native: Native,
    args: Vec<V>,
    mut engine: Recorder,
) -> (Recorder, Vec<V>, Counters) {
    let arg_count = args.len();
    lua.stack = args;
    let counters = Counters::new();
    let returned = run(&mut lua, native, &mut engine, &counters);
    assert_eq!(lua.stack.len(), arg_count + returned as usize);
    let results = lua.stack.split_off(arg_count);
    (engine, results, counters)
}

/// A missing number is 0 and a missing string "", except `quality`, which
/// is the client's 2000; `nil` or no opts means every default.
#[test]
fn search_fills_defaults() {
    let expected = CellCall::Search(BMSearch {
        search_options: defaults(),
    })
    .to_bytes()
    .unwrap();
    for case in ["none", "nil", "empty table"] {
        let mut lua = FakeLua::called_with(Vec::new());
        let args = match case {
            "none" => vec![],
            "nil" => vec![V::Nil],
            _ => vec![lua.table(&[])],
        };
        let (engine, results, _) = run_in(lua, Native::Search, args, Recorder::default());
        assert_eq!(results, [V::Bool(true)], "{case}");
        assert_eq!(engine.sends, [(0, expected.clone())], "{case}");
    }
    // The default payload byte for byte.
    assert_eq!(
        expected,
        [
            0, // sortId
            0, 0, 0, 0, // clientKey
            0, 0, 0, 0, // sequenceId
            0, // bForward
            0, 0, 0, 0, // sellerName ""
            0, 0, 0, 0, // bidderName ""
            0, 0, 0, 0, // itemName ""
            0, 0, 0, 0, // minTC
            0, 0, 0, 0, // maxTC
            0xD0, 0x07, 0, 0, // quality 2000
            0, 0, 0, 0, // filterFlags
        ]
    );
}

/// Search with an opts table built from `fields`.
fn search(fields: &[(&str, V)]) -> (Recorder, Vec<V>, Counters) {
    let mut lua = FakeLua::called_with(Vec::new());
    let opts = lua.table(fields);
    run_in(lua, Native::Search, vec![opts], Recorder::default())
}

#[test]
fn search_sends_every_field_byte_exact() {
    let (engine, results, _) = search(&[
        ("sortId", V::Num(3)),
        ("clientKey", V::Num(1)),
        ("sequenceId", V::Num(77)),
        ("bForward", V::Bool(true)),
        ("sellerName", s("Vala")),
        ("bidderName", s("Bra'tac")),
        ("itemName", s("Zat")),
        ("minTC", V::Num(2)),
        ("maxTC", V::Num(9)),
        ("quality", V::Num(1500)),
        ("filterFlags", V::Num(0x10)),
        ("ignored", s("unknown keys are ignored")),
    ]);
    assert_eq!(results, [V::Bool(true)]);
    let expected = CellCall::Search(BMSearch {
        search_options: BMSearchOptions {
            sort_id: 3,
            client_key: 1,
            sequence_id: 77,
            b_forward: 1,
            seller_name: "Vala".into(),
            bidder_name: "Bra'tac".into(),
            item_name: "Zat".into(),
            min_tc: 2,
            max_tc: 9,
            quality: 1500,
            filter_flags: 0x10,
        },
    });
    assert_eq!(engine.sends, [(0, expected.to_bytes().unwrap())]);
    let (_, payload) = &engine.sends[0];
    assert_eq!(&payload[..10], [3, 1, 0, 0, 0, 77, 0, 0, 0, 1]);
    assert_eq!(&payload[10..18], [4, 0, 0, 0, b'V', b'a', b'l', b'a']);
}

#[test]
fn b_forward_takes_a_number_or_a_boolean() {
    for (value, wire) in [
        (V::Bool(false), 0),
        (V::Bool(true), 1),
        (V::Num(0), 0),
        (V::Num(1), 1),
    ] {
        let (engine, _, _) = search(&[("bForward", value)]);
        assert_eq!(engine.sends[0].1[9], wire);
    }
}

/// Non-ASCII names go out as UTF-8, and the 255-byte cap is on bytes.
#[test]
fn names_are_utf8_and_capped_in_bytes() {
    let (engine, results, _) = search(&[("itemName", s("Tök"))]);
    assert_eq!(results, [V::Bool(true)]);
    assert_eq!(
        &engine.sends[0].1[18..25],
        [3 + 1, 0, 0, 0, b'T', 0xC3, 0xB6]
    );

    let (_, results, _) = search(&[("sellerName", s(&"x".repeat(255)))]);
    assert_eq!(results, [V::Bool(true)], "255 bytes fit");
    let (engine, results, _) = search(&[("sellerName", s(&"ö".repeat(128)))]);
    assert_eq!(results, [V::Nil, s("bad_args")], "256 bytes do not");
    assert!(engine.sends.is_empty());
}

// ── refusals ─────────────────────────────────────────────────────────────

#[test]
fn wrong_types_are_bad_args_not_lua_errors() {
    expect_refused(Native::Bid, vec![s("7"), V::Num(1)], "bad_args");
    expect_refused(Native::Bid, vec![V::Num(7), V::Bool(true)], "bad_args");
    expect_refused(Native::Cancel, vec![], "bad_args");
    expect_refused(Native::Cancel, vec![V::Nil], "bad_args");
    expect_refused(Native::Watch, vec![s("42"), V::Bool(true)], "bad_args");
    expect_refused(Native::Search, vec![V::Num(1)], "bad_args");
    expect_refused(Native::Search, vec![s("opts")], "bad_args");
    expect_refused(
        Native::Create,
        vec![V::Num(1), V::Num(2), V::Num(3)],
        "bad_args",
    );
}

#[test]
fn numbers_must_be_integers_in_range() {
    expect_refused(Native::Bid, vec![V::Num(7), V::Float(2.5)], "bad_args");
    expect_refused(Native::Bid, vec![V::Num(7), V::Float(f64::NAN)], "bad_args");
    expect_refused(
        Native::Bid,
        vec![V::Num(7), V::Float(2_147_483_648.0)],
        "bad_args",
    );
    expect_refused(Native::Cancel, vec![V::Float(-2_147_483_649.0)], "bad_args");
    // The ends of the range are fine, and an integral float is an integer.
    expect_sent(
        Native::Bid,
        vec![V::Float(-2_147_483_648.0), V::Float(2_147_483_647.0)],
        CellCall::PlaceBid(BMPlaceBid {
            sequence_id: i32::MIN,
            bid_amount: i32::MAX,
        }),
    );
}

#[test]
fn search_field_ranges_are_checked() {
    for fields in [
        vec![("sortId", V::Num(256))],
        vec![("sortId", V::Num(-1))],
        vec![("bForward", V::Num(256))],
        vec![("clientKey", V::Num(3))],
        vec![("clientKey", V::Num(-1))],
        vec![("quality", s("2000"))],
        vec![("sellerName", V::Num(5))],
    ] {
        let (engine, results, _) = search(&fields);
        assert_eq!(results, [V::Nil, s("bad_args")], "{fields:?}");
        assert!(engine.sends.is_empty());
    }
}

/// `auctionLength` is a `UIAuctionTime`, 1 to 5; 0 is the 0-based mistake
/// the server's S5 fix guards against.
#[test]
fn auction_length_must_be_one_to_five() {
    for length in [0, 6, 255] {
        expect_refused(
            Native::Create,
            vec![V::Num(1), V::Num(2), V::Num(3), V::Num(length)],
            "bad_args",
        );
    }
    for length in 1..=5 {
        let (engine, results, _) = call(
            Native::Create,
            vec![V::Num(1), V::Num(2), V::Num(3), V::Num(length)],
        );
        assert_eq!(results, [V::Bool(true)]);
        assert_eq!(engine.sends[0].1[8], length as u8);
    }
}

#[test]
fn off_the_main_thread_nothing_is_read_or_sent() {
    let engine = Recorder {
        off_main_thread: true,
        ..Recorder::default()
    };
    // Even with bad arguments, the thread is the reason.
    let (engine, results, _) = call_with(Native::Bid, vec![s("bad")], engine);
    assert_eq!(results, [V::Nil, s("not_main_thread")]);
    assert!(engine.sends.is_empty());
}

#[test]
fn engine_refusals_map_to_their_reasons() {
    for (refusal, reason) in [
        (Refusal::Offline("no ServerConnection"), "offline"),
        (Refusal::EngineError("null bundle".into()), "engine_error"),
    ] {
        let engine = Recorder {
            refuse: Some(refusal),
            ..Recorder::default()
        };
        let (_, results, counters) = call_with(Native::Cancel, vec![V::Num(1)], engine);
        assert_eq!(results, [V::Nil, s(reason)]);
        assert_eq!(counters.send_refused.load(Ordering::Relaxed), 1);
        assert_eq!(counters.sent.load(Ordering::Relaxed), 0);
    }
}

/// A panic in the engine layer is contained and reported, not unwound
/// into Lua.
#[test]
fn a_panicking_engine_is_an_engine_error() {
    let engine = Recorder {
        panic: true,
        ..Recorder::default()
    };
    let (_, results, _) = call_with(Native::Cancel, vec![V::Num(1)], engine);
    assert_eq!(results, [V::Nil, s("engine_error")]);
}

#[test]
fn every_reason_is_one_of_the_contract_strings() {
    let reasons = [
        Refusal::NotMainThread,
        Refusal::BadArgs(String::new()),
        Refusal::Offline(""),
        Refusal::EngineError(String::new()),
    ]
    .map(|r| r.reason());
    assert_eq!(
        reasons,
        ["not_main_thread", "bad_args", "offline", "engine_error"]
    );
}

/// D7: `techCompetency` answers `nil` and never reaches the engine.
#[test]
fn tech_competency_is_nil() {
    let (engine, results, counters) = call(Native::TechCompetency, vec![V::Num(1234)]);
    assert_eq!(results, [V::Nil]);
    assert!(engine.sends.is_empty());
    assert_eq!(counters.tech_competency_nil.load(Ordering::Relaxed), 1);
    assert_eq!(counters.send_refused.load(Ordering::Relaxed), 0);
}

/// Every cell method is reachable from exactly one native call, at its own
/// sub-index.
#[test]
fn natives_cover_cell_methods_61_to_66() {
    let calls: [(Native, Vec<V>); 6] = [
        (Native::Search, vec![]),
        (
            Native::Create,
            vec![V::Num(1), V::Num(1), V::Nil, V::Num(3)],
        ),
        (Native::Bid, vec![V::Num(1), V::Num(1)]),
        (Native::Cancel, vec![V::Num(1)]),
        (Native::Watch, vec![V::Num(1), V::Bool(true)]),
        (Native::Watch, vec![V::Num(1), V::Bool(false)]),
    ];
    let mut seen = Vec::new();
    for (native, args) in calls {
        let mut lua = FakeLua::called_with(args);
        let mut engine = Recorder::default();
        let sent = send(&mut lua, native, &mut engine).unwrap();
        assert_eq!(engine.sends[0].0, sent.method.sub_index());
        seen.push(sent.method);
    }
    assert_eq!(seen, CellMethod::ALL);
}

// ── registration ─────────────────────────────────────────────────────────

fn registered_table(lua: &FakeLua) -> V {
    lua.global(NATIVE_TABLE).expect("the global is set")
}

#[test]
fn registration_builds_the_table_once() {
    let mut lua = FakeLua::new();
    lua.stack.push(V::Str("game".into()));
    assert_eq!(
        ensure(&mut lua),
        Registration::Registered { replaced: false }
    );
    assert_eq!(lua.stack, [V::Str("game".into())], "stack restored");

    let table = registered_table(&lua);
    for native in Native::ALL {
        assert_eq!(
            lua.field(&table, native.lua_name()),
            Some(V::Func(format!("native:{}", native.lua_name())))
        );
    }
    assert_eq!(lua.field(&table, VERSION_KEY), Some(s(VERSION)));
    assert_eq!(
        lua.render(&table).matches("fn native:").count(),
        6,
        "exactly the six functions"
    );

    // Idempotent: the same table stays, so a reference the overlay holds
    // keeps working.
    assert_eq!(ensure(&mut lua), Registration::AlreadyPresent);
    assert_eq!(registered_table(&lua), table);
    assert_eq!(lua.stack, [V::Str("game".into())]);
}

#[test]
fn registration_never_touches_the_overlay_table() {
    let mut lua = FakeLua::with_overlay(&["onOpen"]);
    let overlay = lua.global("CimmeriaBM");
    ensure(&mut lua);
    assert_eq!(lua.global("CimmeriaBM"), overlay);
    assert_eq!(lua.field(overlay.as_ref().unwrap(), "search"), None);
}

/// A UI reload that loses the global, a stale version, a missing function
/// or a foreign value each get a fresh table.
#[test]
fn registration_repairs_a_missing_or_stale_table() {
    let mut lua = FakeLua::new();
    ensure(&mut lua);

    lua.set_global(NATIVE_TABLE, V::Nil);
    assert_eq!(
        ensure(&mut lua),
        Registration::Registered { replaced: false }
    );

    let table = registered_table(&lua);
    lua.set_field_of(&table, VERSION_KEY, s("0.0.0-old"));
    assert_eq!(
        ensure(&mut lua),
        Registration::Registered { replaced: true }
    );
    assert_ne!(registered_table(&lua), table);

    let table = registered_table(&lua);
    lua.set_field_of(&table, "bid", V::Nil);
    assert_eq!(
        ensure(&mut lua),
        Registration::Registered { replaced: true }
    );
    assert_eq!(
        lua.field(&registered_table(&lua), "bid"),
        Some(V::Func("native:bid".into()))
    );

    lua.set_global(NATIVE_TABLE, s("not a table"));
    assert_eq!(
        ensure(&mut lua),
        Registration::Registered { replaced: true }
    );
    assert_eq!(ensure(&mut lua), Registration::AlreadyPresent);
}

/// Out of Lua memory part-way, the registration reports it, the stack is
/// restored, and the next attempt succeeds.
#[test]
fn registration_survives_running_out_of_memory() {
    let mut lua = FakeLua::new();
    lua.stack.push(V::Num(7));
    lua.alloc_budget = Some(5);
    assert!(matches!(
        ensure(&mut lua),
        Registration::SetupError { status: 4, .. }
    ));
    assert_eq!(lua.stack, [V::Num(7)]);
    assert_eq!(
        lua.global(NATIVE_TABLE),
        None,
        "nothing half-built is visible"
    );

    lua.alloc_budget = None;
    assert_eq!(
        ensure(&mut lua),
        Registration::Registered { replaced: false }
    );
}

#[test]
fn registration_reports_a_full_stack() {
    let mut lua = FakeLua::new();
    lua.room = 1;
    assert_eq!(ensure(&mut lua), Registration::NoStackSpace);
    assert!(lua.stack.is_empty());
}
