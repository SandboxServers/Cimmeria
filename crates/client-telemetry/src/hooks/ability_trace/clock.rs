//! The client's game clock, read from memory the way the client computes
//! it, so an applied row can say how long a timer has left.
//!
//! `BigWorldTimeComplete` and every client timer are on this clock
//! (`docs/protocol/client-method-dispatch-table.md`, `onTimerUpdate`).
//! The effect-bar handler (`0x00e09160`) reads it through
//! `0x00c6f870` (the connection holder's singleton, `[0x01ef2264]`) and
//! `0x00c6e220`, which jumps to `0x00dd6c60` with the `ServerConnection` at
//! `[holder+0x28]`. That function, disassembled from the QA `SGW.exe`
//! (2026-10-04):
//!
//! ```text
//! rate = f64 [conn+0x344]
//! frac = rate > 0 ? (f64 *[conn+0x34c] - f64 [conn+0x334]) / rate : 0
//! time = (u32 [conn+0x32c] + frac) / f32 [0x01e51cb8]
//! ```
//!
//! `[conn+0x32c]` is the tick count (`fild` then `+ 2^32` when negative:
//! unsigned), `0x01e51cb8` is the tick rate (`10.0` in the image). The
//! hook reads the same words and calls nothing.

use crate::hooks::entity_trace::map::Mem;

/// Holder of the `ServerConnection` (`0x00c6f870` returns it).
pub(crate) const CONNECTION_HOLDER: u32 = 0x01ef_2264;
/// The `ServerConnection` inside the holder.
pub(crate) const HOLDER_CONNECTION: u32 = 0x28;
/// Server ticks received, `u32`.
pub(crate) const CONN_TICKS: u32 = 0x32c;
/// Local time of the last tick, `f64`.
pub(crate) const CONN_LAST_TICK_TIME: u32 = 0x334;
/// Local seconds per tick, `f64`.
pub(crate) const CONN_TICK_PERIOD: u32 = 0x344;
/// Pointer to the local clock, an `f64`.
pub(crate) const CONN_NOW_PTR: u32 = 0x34c;
/// Ticks per second, `f32`.
pub(crate) const HERTZ: u32 = 0x01e5_1cb8;

fn f64_at(mem: &dyn Mem, addr: u32) -> Option<f64> {
    let lo = mem.u32_at(addr)?;
    let hi = mem.u32_at(addr.wrapping_add(4))?;
    Some(f64::from_bits(u64::from(hi) << 32 | u64::from(lo)))
}

/// The game time in seconds, or `None` before the connection exists.
pub(crate) fn game_time(mem: &dyn Mem) -> Option<f64> {
    let holder = mem.u32_at(CONNECTION_HOLDER).filter(|&h| h != 0)?;
    let conn = mem
        .u32_at(holder.wrapping_add(HOLDER_CONNECTION))
        .filter(|&c| c != 0)?;
    let ticks = mem.u32_at(conn.wrapping_add(CONN_TICKS))?;
    let period = f64_at(mem, conn.wrapping_add(CONN_TICK_PERIOD))?;
    let frac = if period > 0.0 {
        let now_ptr = mem.u32_at(conn.wrapping_add(CONN_NOW_PTR))?;
        let now = f64_at(mem, now_ptr)?;
        let last = f64_at(mem, conn.wrapping_add(CONN_LAST_TICK_TIME))?;
        (now - last) / period
    } else {
        0.0
    };
    let hertz = f64::from(f32::from_bits(mem.u32_at(HERTZ)?));
    (hertz > 0.0).then(|| (f64::from(ticks) + frac) / hertz)
}

/// Seconds left until `complete`, rounded to milliseconds: negative when
/// it is already past.
pub(crate) fn remaining(complete: f32, now: f64) -> f64 {
    ((f64::from(complete) - now) * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::entity_trace::map::fake::FakeMem;

    fn set_f64(mem: &mut FakeMem, addr: u32, v: f64) {
        let b = v.to_bits();
        mem.set(addr, b as u32);
        mem.set(addr + 4, (b >> 32) as u32);
    }

    /// 12345 ticks at 10 Hz, a quarter of a tick since the last one.
    #[test]
    fn the_clock_is_ticks_plus_the_fraction_over_hertz() {
        let mut m = FakeMem::default();
        m.set(CONNECTION_HOLDER, 0x1000);
        m.set(0x1000 + HOLDER_CONNECTION, 0x2000);
        m.set(0x2000 + CONN_TICKS, 12345);
        set_f64(&mut m, 0x2000 + CONN_TICK_PERIOD, 0.1);
        set_f64(&mut m, 0x2000 + CONN_LAST_TICK_TIME, 50.0);
        m.set(0x2000 + CONN_NOW_PTR, 0x3000);
        set_f64(&mut m, 0x3000, 50.025);
        m.set(HERTZ, 10.0f32.to_bits());
        let t = game_time(&m).unwrap();
        assert!((t - 1234.525).abs() < 1e-9, "{t}");
        assert_eq!(remaining(1240.0, t), 5.475);
        assert_eq!(remaining(1000.0, t), -234.525);
    }

    /// With no tick period yet the fraction is 0, as in the game.
    #[test]
    fn no_period_means_whole_ticks() {
        let mut m = FakeMem::default();
        m.set(CONNECTION_HOLDER, 0x1000);
        m.set(0x1000 + HOLDER_CONNECTION, 0x2000);
        m.set(0x2000 + CONN_TICKS, 100);
        set_f64(&mut m, 0x2000 + CONN_TICK_PERIOD, 0.0);
        m.set(HERTZ, 10.0f32.to_bits());
        assert_eq!(game_time(&m), Some(10.0));
    }

    #[test]
    fn no_connection_means_no_clock() {
        let mut m = FakeMem::default();
        assert_eq!(game_time(&m), None);
        m.set(CONNECTION_HOLDER, 0);
        assert_eq!(game_time(&m), None);
    }
}
