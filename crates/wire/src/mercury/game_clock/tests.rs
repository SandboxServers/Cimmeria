//! The clock's rate agrees with what the client is told, and a timer built
//! from it decodes to `now + duration` on the client's clock.

use std::time::Duration;

use cimmeria_entity::abilities::serialize_timer_update;

use super::*;

/// The three numbers the client derives its clock from must describe one
/// rate: `hertz` (update frequency), `tickRate` (ms per tick, sent in every
/// `tickSync`) and the interval the base sends `tickSync` at. If `hertz` and
/// `tickRate` disagree, the client's interpolation runs at one rate and its
/// tick count at another.
#[test]
fn declared_frequency_tick_period_and_send_interval_agree() {
    assert_eq!(
        u32::from(UPDATE_FREQUENCY_HZ) * TICK_PERIOD_MS,
        1000,
        "hertz * ms-per-tick must be one second"
    );
    assert_eq!(
        TICK_SYNC_INTERVAL.as_millis(),
        u128::from(TICK_PERIOD_MS),
        "the tick-sync loop sends once per declared tick"
    );
}

/// The tick count advances exactly one tick per send interval, at every
/// point in a long session. This is the invariant the old per-session
/// `tick += 1` counter only held by luck of the loop's sleep length.
#[test]
fn each_send_interval_advances_the_tick_count_by_exactly_one() {
    for n in [0u32, 1, 2, 9, 10, 599, 36_000, 864_000] {
        let start = TICK_SYNC_INTERVAL * n;
        assert_eq!(ticks_at(start), n, "tick at the start of interval {n}");
        assert_eq!(
            ticks_at(start + TICK_SYNC_INTERVAL - Duration::from_millis(1)),
            n,
            "still tick {n} just before the next send"
        );
        assert_eq!(
            ticks_at(start + TICK_SYNC_INTERVAL),
            n + 1,
            "one interval later is one tick later"
        );
    }
}

/// What the client reads after a `tickSync` (`ticks / hertz`) is the
/// server's seconds, rounded down to the tick.
#[test]
fn client_clock_for_the_sent_ticks_matches_server_seconds_within_one_tick() {
    let period = f64::from(TICK_PERIOD_MS) / 1000.0;
    for ms in [0u64, 99, 100, 1_050, 65_535_900, 3_600_000, 86_400_000] {
        let elapsed = Duration::from_millis(ms);
        let client = client_secs_for_ticks(ticks_at(elapsed));
        let server = f64::from(secs_at(elapsed));
        // f32 seconds round to about 8 ms at a day of uptime.
        let eps = 1e-2;
        assert!(
            client <= server + eps && server - client < period + eps,
            "at {ms} ms the client reads {client} s, the server {server} s"
        );
    }
}

/// A timer built as `game_time_secs() + d` carries `now + d` in its
/// `BigWorldTimeComplete` field (bytes 17..21 of `onTimerUpdate`), and the
/// client's `complete - clock` comes out as `d`, give or take the one tick
/// the client's clock lags between syncs.
#[test]
fn timer_built_from_game_time_decodes_to_now_plus_duration_on_the_client_clock() {
    const CRAFT_SECS: f32 = 3.0;
    let period = f64::from(TICK_PERIOD_MS) / 1000.0;
    // An hour of uptime: past the 16-bit `setGameTime` wrap (6553.5 s is
    // the next one) and well inside f32's sub-millisecond range.
    let elapsed = Duration::from_millis(3_723_450);

    let now = secs_at(elapsed);
    let expiry = now + CRAFT_SECS;
    let args = serialize_timer_update(900, 16, 1, 0, CRAFT_SECS, expiry);

    let wire_expiry = f32::from_le_bytes(args[17..21].try_into().unwrap());
    assert_eq!(
        wire_expiry, expiry,
        "BigWorldTimeComplete is the absolute expiry"
    );
    assert!(
        (wire_expiry - now - CRAFT_SECS).abs() < 1e-3,
        "expiry is now + duration, not the relative duration ({wire_expiry})"
    );

    let client_clock = client_secs_for_ticks(ticks_at(elapsed));
    let remaining = f64::from(wire_expiry) - client_clock;
    assert!(
        remaining >= f64::from(CRAFT_SECS) - 1e-3
            && remaining <= f64::from(CRAFT_SECS) + period + 1e-3,
        "the client computes {remaining} s remaining for a {CRAFT_SECS} s timer"
    );
}

/// The live clock reads the same instant in both units.
#[test]
fn live_ticks_and_seconds_read_the_same_clock() {
    init();
    let before = game_time_secs();
    let ticks = game_ticks();
    let after = game_time_secs();
    let from_ticks = client_secs_for_ticks(ticks);
    assert!(before <= after, "the clock never runs backwards");
    assert!(
        from_ticks <= f64::from(after) + 1e-3
            && f64::from(before) - from_ticks < f64::from(TICK_PERIOD_MS) / 1000.0 + 1e-3,
        "ticks {ticks} ({from_ticks} s) is outside [{before}, {after}]"
    );
}
