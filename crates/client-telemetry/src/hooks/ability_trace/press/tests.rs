use super::super::field;
use super::*;

fn targets(outs: &[Out]) -> Vec<&'static str> {
    outs.iter().map(|o| o.target).collect()
}

fn reason(outs: &[Out]) -> Option<String> {
    outs.iter()
        .find(|o| o.target == TARGET_DROPPED)
        .and_then(|o| field(&o.fields, "reason"))
        .and_then(|v| v.as_str().map(str::to_owned))
}

fn site_of(outs: &[Out]) -> Option<String> {
    outs.iter()
        .find(|o| o.target == TARGET_DROPPED)
        .and_then(|o| field(&o.fields, "drop_site"))
        .and_then(|v| v.as_str().map(str::to_owned))
}

/// The hotbar path to a post: one press row, no drop, and a pending send
/// for `useAbility` carrying the press id.
#[test]
fn a_known_hotbar_press_announces_once_and_leaves_a_pending_send() {
    let mut p = Press::begin(Source::Hotbar, 1, 0);
    p.slot_entered(3, false);
    let outs = p.lookup_entered(597, 1234);
    assert_eq!(targets(&outs), vec![TARGET_PRESS]);
    let f = &outs[0].fields;
    assert_eq!(field(f, "source"), Some(&json!("hotbar")));
    assert_eq!(field(f, "slot"), Some(&json!(3)));
    assert_eq!(field(f, "ability_id"), Some(&json!(597)));
    assert_eq!(field(f, "target_id"), Some(&json!(1234)));
    assert_eq!(field(f, "self_cast"), Some(&json!(false)));
    let pending = p.send_entered(Some(false), 10);
    assert_eq!(pending.method, "useAbility");
    assert_eq!(pending.press_id, 1);
    assert!(p.lookup_left().is_empty());
    assert!(p.slot_left(Some(0xAAAA)).is_empty());
    assert!(p.thunk_left().is_empty());
    assert!(p.resolved());
}

/// Row 5: the ability is not in the `AbilitySet`, so `FUN_00d2ae40` is
/// never reached and the press dies at `0x00d2afcf`.
#[test]
fn an_unknown_ability_drops_at_the_set_lookup() {
    let mut p = Press::begin(Source::Hotbar, 2, 0);
    p.slot_entered(1, false);
    p.lookup_entered(597, 0);
    let outs = p.lookup_left();
    assert_eq!(targets(&outs), vec![TARGET_DROPPED]);
    assert_eq!(reason(&outs).as_deref(), Some("not_known"));
    assert_eq!(site_of(&outs).as_deref(), Some("0x00d2afcf"));
    assert!(p.slot_left(Some(0xAAAA)).is_empty(), "exactly one answer");
    assert!(p.thunk_left().is_empty());
}

/// Row 3: an empty slot, so no executor ran: a press with no ability and
/// a `no_action` drop.
#[test]
fn an_empty_slot_is_no_action() {
    let mut p = Press::begin(Source::Hotbar, 3, 2);
    p.slot_entered(12, false);
    let outs = p.slot_left(Some(0));
    assert_eq!(targets(&outs), vec![TARGET_PRESS, TARGET_DROPPED]);
    assert_eq!(field(&outs[0].fields, "ability_id"), Some(&Value::Null));
    assert_eq!(field(&outs[0].fields, "pending_expired"), Some(&json!(2)));
    assert_eq!(reason(&outs).as_deref(), Some("no_action"));
    assert_eq!(site_of(&outs).as_deref(), Some("0x00ad959e"));
    assert!(p.thunk_left().is_empty());
}

/// A slot that holds an item or macro is not an ability press: nothing.
#[test]
fn a_non_ability_action_reports_nothing() {
    let mut p = Press::begin(Source::Hotbar, 4, 0);
    p.slot_entered(5, false);
    assert!(p.slot_left(Some(0xBBBB_0000)).is_empty());
    assert!(p.slot_left(None).is_empty(), "unreadable slot: no guess");
    assert!(p.thunk_left().is_empty());
}

/// Row 2: the binding's check failed and it raised a Lua error before the
/// next step; the thunk's exit reports it.
#[test]
fn a_bad_lua_call_is_bad_args_for_either_binding() {
    let mut hot = Press::begin(Source::Hotbar, 5, 0);
    let outs = hot.thunk_left();
    assert_eq!(targets(&outs), vec![TARGET_PRESS, TARGET_DROPPED]);
    assert_eq!(reason(&outs).as_deref(), Some("bad_args"));
    assert_eq!(site_of(&outs).as_deref(), Some("0x00aa9569"));

    let mut lua = Press::begin(Source::Lua, 6, 0);
    let outs = lua.thunk_left();
    assert_eq!(reason(&outs).as_deref(), Some("bad_args"));
    assert_eq!(site_of(&outs).as_deref(), Some("0x00aa2997"));
    assert_eq!(field(&outs[0].fields, "source"), Some(&json!("lua")));
}

/// The Lua path reaches the lookup directly; a hit is not `bad_args`.
#[test]
fn a_lua_press_that_reached_the_lookup_is_not_bad_args() {
    let mut p = Press::begin(Source::Lua, 7, 0);
    p.lookup_entered(4000, 99);
    p.send_entered(None, 0);
    assert!(p.thunk_left().is_empty());
}

#[test]
fn a_ground_ability_waits_longer_for_its_send() {
    let mut p = Press::begin(Source::Hotbar, 8, 0);
    p.slot_entered(1, false);
    p.lookup_entered(42, 0);
    let pending = p.send_entered(Some(true), 0);
    assert_eq!(pending.method, "useAbilityOnGroundTarget");
    assert_eq!(pending.ttl_ms, GROUND_TTL_MS);
}

/// Row 4 and the three GamePet branches, each with its own reason.
#[test]
fn pet_presses_drop_on_each_pet_branch() {
    let mut missing = Press::begin(Source::Hotbar, 9, 0);
    missing.slot_entered(2, false);
    let outs = missing.pet_entered(11, 900);
    assert_eq!(field(&outs[0].fields, "pet_id"), Some(&json!(900)));
    let outs = missing.pet_left();
    assert_eq!(reason(&outs).as_deref(), Some("pet_missing"));
    assert_eq!(site_of(&outs).as_deref(), Some("0x00e3cfb1"));

    let cases = [
        (
            PetGate {
                ready: Some(false),
                known: Some(true),
                allowed: Some(true),
            },
            "pet_state_flag",
            "0x00d3a84a",
        ),
        (
            PetGate {
                ready: Some(true),
                known: Some(false),
                allowed: None,
            },
            "not_known",
            "0x00d3a862",
        ),
        (
            PetGate {
                ready: Some(true),
                known: Some(true),
                allowed: Some(false),
            },
            "pet_ability_flag",
            "0x00d3a875",
        ),
    ];
    for (gate, want, at) in cases {
        let mut p = Press::begin(Source::Hotbar, 10, 0);
        p.pet_entered(11, 900);
        let (outs, pending) = p.pet_send_entered(gate, 5, 0);
        assert!(pending.is_none(), "{want}");
        assert_eq!(reason(&outs).as_deref(), Some(want));
        assert_eq!(site_of(&outs).as_deref(), Some(at));
        assert!(p.pet_left().is_empty(), "{want}: one answer");
    }

    let mut ok = Press::begin(Source::Hotbar, 11, 0);
    ok.pet_entered(11, 900);
    let gate = PetGate {
        ready: Some(true),
        known: Some(true),
        allowed: None,
    };
    let (outs, pending) = ok.pet_send_entered(gate, 5, 0);
    assert!(outs.is_empty());
    assert_eq!(pending.unwrap().method, "petInvokeAbility");
    assert!(ok.pet_left().is_empty());
}

/// The router claims the oldest press for its method and ability.
#[test]
fn the_pending_table_matches_method_and_ability_in_order() {
    let mut t = PendingTable::default();
    let mk = |id, method, ability, at| PendingSend {
        press_id: id,
        source: Source::Hotbar,
        method,
        ability_id: Some(ability),
        at_ms: at,
        ttl_ms: SEND_TTL_MS,
    };
    t.push(mk(1, "useAbility", 5, 0));
    t.push(mk(2, "useAbility", 6, 0));
    t.push(mk(3, "useAbility", 5, 1));
    assert_eq!(
        t.take("useAbility", Some(5), 2).map(|p| p.press_id),
        Some(1)
    );
    assert_eq!(
        t.take("useAbility", Some(5), 2).map(|p| p.press_id),
        Some(3)
    );
    assert_eq!(t.take("petInvokeAbility", Some(6), 2), None);
    assert_eq!(
        t.take("useAbility", Some(6), 2).map(|p| p.press_id),
        Some(2)
    );
    assert_eq!(t.take_expired(2), 0);
}

#[test]
fn unclaimed_presses_expire_and_are_counted() {
    let mut t = PendingTable::default();
    t.push(PendingSend {
        press_id: 1,
        source: Source::Lua,
        method: "useAbility",
        ability_id: Some(1),
        at_ms: 0,
        ttl_ms: SEND_TTL_MS,
    });
    assert_eq!(t.take("useAbility", Some(1), SEND_TTL_MS + 1), None);
    assert_eq!(t.take_expired(SEND_TTL_MS + 1), 1);
    assert_eq!(t.take_expired(SEND_TTL_MS + 1), 0, "drained");
    for i in 0..(MAX_PENDING as u32 + 3) {
        t.push(PendingSend {
            press_id: i,
            source: Source::Lua,
            method: "useAbility",
            ability_id: Some(i as i32),
            at_ms: 0,
            ttl_ms: SEND_TTL_MS,
        });
    }
    assert_eq!(t.take_expired(0), 3, "overflow counts as expired");
    assert_eq!(t.take("useAbility", Some(0), 0), None, "oldest evicted");
}

/// Rows 6 to 8 come from memory; with all three passing, a router that
/// reached no `start*Message` refused the class (rows 9 and 10).
#[test]
fn the_router_outcome_names_the_first_failed_row() {
    let ok = RoutePre {
        has_connection: Some(true),
        connected: Some(true),
        local_player_found: Some(true),
    };
    assert_eq!(route_outcome(true, ok), RouteOutcome::Sent);
    assert_eq!(
        route_outcome(true, RoutePre::default()),
        RouteOutcome::Sent,
        "an observed send wins over an unreadable pre-check"
    );
    let no_conn = RoutePre {
        has_connection: Some(false),
        ..ok
    };
    assert_eq!(
        route_outcome(false, no_conn),
        RouteOutcome::Dropped(DropReason::NotConnected, site::NO_CONNECTION)
    );
    let offline = RoutePre {
        connected: Some(false),
        ..ok
    };
    assert_eq!(
        route_outcome(false, offline),
        RouteOutcome::Dropped(DropReason::NotConnected, site::NOT_CONNECTED)
    );
    let gone = RoutePre {
        local_player_found: Some(false),
        ..ok
    };
    assert_eq!(
        route_outcome(false, gone),
        RouteOutcome::Dropped(DropReason::NotConnected, site::NO_LOCAL_PLAYER)
    );
    assert_eq!(
        route_outcome(false, ok),
        RouteOutcome::Dropped(DropReason::ClassMismatch, site::CLASS)
    );
}

#[test]
fn a_router_drop_without_a_press_has_a_null_press_id() {
    let out = route_dropped(
        "gmDebugCombat",
        None,
        None,
        DropReason::ClassMismatch,
        site::CLASS,
    );
    assert_eq!(out.target, TARGET_DROPPED);
    assert_eq!(field(&out.fields, "press_id"), Some(&Value::Null));
    assert_eq!(field(&out.fields, "method"), Some(&json!("gmDebugCombat")));
    assert_eq!(field(&out.fields, "route_rows"), Some(&json!("9|10")));
    assert_eq!(out.key, "client.ability.press_dropped:class_mismatch");
}

/// Only the finding's reasons (plus the GamePet branches read for AB-C2)
/// exist; the client has no cooldown, range, target or death check.
#[test]
fn no_invented_reasons() {
    let all = [
        DropReason::NoAction,
        DropReason::BadArgs,
        DropReason::NotKnown,
        DropReason::PetMissing,
        DropReason::PetStateFlag,
        DropReason::PetAbilityFlag,
        DropReason::NotConnected,
        DropReason::ClassMismatch,
    ];
    for r in all {
        let s = r.as_str();
        for banned in ["cooldown", "range", "dead", "target"] {
            assert!(!s.contains(banned), "{s}");
        }
    }
}
