//! Shared server game-time clock, in the wire tick domain the client
//! derives from `BASEMSG_SET_GAME_TIME` / `BASEMSG_TICK_SYNC`.
//!
//! Mirrors the C++ reference server exactly:
//!
//! - `CellManager::ticks()` = milliseconds since service start divided by
//!   `tick_rate` (`deprecated/cpp/src/baseapp/cell_manager.cpp:198-201`),
//!   where `tick_rate` is **milliseconds per tick** (`100` in
//!   `BaseService.config`), so the counter advances 10 times a second.
//! - The login bundle writes that one server-wide counter into both
//!   `TICK_SYNC.gameTime` and `SET_GAME_TIME`, and `TICK_SYNC.tickRate`
//!   carries the same ms-per-tick value
//!   (`deprecated/cpp/src/baseapp/mercury/sgw/client_handler.cpp:44-63`).
//!   The ongoing heartbeat (`ClientHandler::gameTick`) sends the same
//!   counter, so a client never sees its clock jump between login and the
//!   first heartbeat.
//! - `Atrea.getGameTime()` = `ticks * tick_rate / 1000.0` seconds
//!   (`deprecated/cpp/src/baseapp/entity/base_py_util.cpp:176-181`). The
//!   Python reference emits `onTimerUpdate.BigWorldTimeComplete` as
//!   `getGameTime() + duration` (`AbilityManager.py`), i.e. an absolute
//!   time on this clock.
//!
//! [`game_time_tick`] feeds the wire tick fields; [`game_time_secs`] feeds
//! absolute timer endpoints. Both derive from one boot-stamped monotonic,
//! and `game_time_secs` is computed *from* the tick so the two cannot
//! disagree.

use std::sync::OnceLock;
use std::time::Instant;

/// Milliseconds per game tick — the `tickRate` field of `TICK_SYNC`
/// (C++ `tick_rate` config, `100`).
pub const TICK_INTERVAL_MS: u32 = 100;

/// `UPDATE_FREQUENCY_NOTIFICATION` payload: ticks per second
/// (C++ `1000 / tickRate()`).
pub const UPDATE_FREQUENCY_HZ: u8 = (1000 / TICK_INTERVAL_MS) as u8;

fn boot_instant() -> Instant {
    static BOOT: OnceLock<Instant> = OnceLock::new();
    *BOOT.get_or_init(Instant::now)
}

/// Server game time in wire ticks since boot, for `TICK_SYNC.gameTime` and
/// `SET_GAME_TIME`. Advances once per [`TICK_INTERVAL_MS`]; a `u32` wraps
/// after ~13.6 years of uptime.
pub fn game_time_tick() -> u32 {
    (boot_instant().elapsed().as_millis() / TICK_INTERVAL_MS as u128) as u32
}

/// Convert a wire tick count to seconds on the client's clock
/// (`ticks * tickRate / 1000`).
pub fn ticks_to_secs(ticks: u32) -> f32 {
    (f64::from(ticks) * f64::from(TICK_INTERVAL_MS) / 1000.0) as f32
}

/// Server game time in seconds, for absolute `onTimerUpdate`
/// `BigWorldTimeComplete` values. Quantised to the tick, like the C++
/// `PyUtil_GetGameTime`.
pub fn game_time_secs() -> f32 {
    ticks_to_secs(game_time_tick())
}

/// Test helper: block until the game clock has advanced past zero, so an
/// absolute expiry (`now + duration`) is distinguishable from a relative
/// one (`duration`). The clock starts on first use, so in a fresh test
/// process `now` would otherwise be `0.0`.
#[cfg(test)]
pub(crate) fn wait_for_nonzero_game_time() {
    while game_time_tick() < 2 {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `tickRate` is milliseconds per tick, not ticks per second: ten
    /// ticks are one second on the client's clock. Treating `100` as a
    /// rate would run the client clock 10x fast and every absolute expiry
    /// would already be in the past when it arrived.
    #[test]
    fn ticks_convert_at_ms_per_tick() {
        assert_eq!(ticks_to_secs(10), 1.0);
        assert_eq!(ticks_to_secs(36_000), 3600.0);
        assert_eq!(UPDATE_FREQUENCY_HZ, 10);
    }

    /// The domain-coupling invariant: the seconds value put in
    /// `BigWorldTimeComplete` is the tick value put on the wire, converted
    /// the way the client converts it.
    #[test]
    fn seconds_domain_is_derived_from_tick_domain() {
        let tick = game_time_tick();
        let secs = game_time_secs();
        let now_tick = game_time_tick();
        assert!(
            secs == ticks_to_secs(tick) || secs == ticks_to_secs(now_tick),
            "game_time_secs must be ticks_to_secs(game_time_tick()): tick={tick}, secs={secs}"
        );
    }

    #[test]
    fn game_time_is_monotonic() {
        let tick_a = game_time_tick();
        let secs_a = game_time_secs();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(game_time_tick() >= tick_a, "tick clock went backwards");
        assert!(game_time_secs() >= secs_a, "seconds clock went backwards");
    }
}
