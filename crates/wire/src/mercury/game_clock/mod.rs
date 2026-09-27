//! The server's one game clock, in the client's clock domain.
//!
//! The SGW client keeps a single game clock on its `ServerConnection`
//! (`FUN_00c6e220` -> `FUN_00dd6c60` in SGW.exe):
//!
//! ```text
//! clock_secs = (tickSync.gameTime + (now - prevTickArrival) / (tickSync.tickRate * 0.001))
//!              / updateFrequencyNotification.hertz
//! ```
//!
//! - `updateFrequencyNotification` (`0x02`, handler `0x00dd62a0`) stores its
//!   `u8` hertz as a float at `0x01e51cb8`.
//! - `tickSync` (`0x0D`, handler `0x00dd6d00`) stores `gameTime` in ticks and
//!   `tickRate * 0.001` (`DAT_01848ab8`) as the tick period, so `tickRate` is
//!   milliseconds per tick.
//! - `setGameTime` (`0x03`, handler `0x00dd6820`) stores only the low 16 bits
//!   of `gameTime` in the field the clock reads.
//!
//! So the client's clock is `ticks / hertz` seconds, and every
//! `onTimerUpdate.BigWorldTimeComplete` (cooldowns via `FUN_00c6d1c0`, the
//! crafting bar via `0x00e47800`) is compared against it. The legacy C++
//! server used the same shape: `tick_rate = 100` ms, `updateFreq = 1000 /
//! tick_rate`, and `ticks = (now - server start) / tick_rate`, one count
//! for the whole server (`deprecated/cpp/src/baseapp/cell_manager.cpp`).
//!
//! This module holds that clock for the Rust server: one process-wide epoch
//! (server start), one rate, and the helpers every sender uses. The epoch is
//! server start rather than Unix time because `BigWorldTimeComplete` is an
//! `f32`: seconds since 1970 would round expiries to about two minutes.

use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Ticks per second, sent as `updateFrequencyNotification.hertz`.
pub const UPDATE_FREQUENCY_HZ: u8 = 10;

/// Milliseconds per tick, sent as `tickSync.tickRate`. The client turns it
/// into its tick period (`tickRate * 0.001` s), so it must be
/// `1000 / UPDATE_FREQUENCY_HZ`, or the interpolated part of the client's
/// clock runs at a different rate from the tick count.
pub const TICK_PERIOD_MS: u32 = 1000 / UPDATE_FREQUENCY_HZ as u32;

/// How often the base's tick-sync loop sends `tickSync`. One tick per send,
/// so the client's interpolation never has to cover more than one period.
pub const TICK_SYNC_INTERVAL: Duration = Duration::from_millis(TICK_PERIOD_MS as u64);

static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Pins the epoch to now if nothing has read the clock yet.
///
/// Call once at server start, so the clock counts from start-up rather
/// than from the first login or the first timer.
pub fn init() {
    LazyLock::force(&EPOCH);
}

/// Game ticks since the epoch, as sent in `tickSync` and `setGameTime`.
pub fn game_ticks() -> u32 {
    ticks_at(EPOCH.elapsed())
}

/// Game time in seconds, in the client's clock domain.
///
/// Use it for absolute expiries:
/// `BigWorldTimeComplete = game_time_secs() + duration`.
pub fn game_time_secs() -> f32 {
    secs_at(EPOCH.elapsed())
}

/// Ticks for `elapsed` time since the epoch. Wraps after about 13.6 years.
pub fn ticks_at(elapsed: Duration) -> u32 {
    (elapsed.as_millis() / u128::from(TICK_PERIOD_MS)) as u32
}

/// Seconds for `elapsed` time since the epoch.
pub fn secs_at(elapsed: Duration) -> f32 {
    elapsed.as_secs_f32()
}

/// What the client's clock reads right after a `tickSync` carrying `ticks`,
/// before interpolation: `ticks / hertz` (`FUN_00dd6c40`).
pub fn client_secs_for_ticks(ticks: u32) -> f64 {
    f64::from(ticks) / f64::from(UPDATE_FREQUENCY_HZ)
}

#[cfg(test)]
mod tests;
