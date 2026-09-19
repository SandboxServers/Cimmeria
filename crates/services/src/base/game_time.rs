//! Shared server game-time clock, in the wire tick domain the client
//! derives from `BASEMSG_TICK_SYNC`.
//!
//! The client's "current time" — the value the cooldown handler
//! (`CooldownManager_HandleOnTimerUpdate`, `0x00ea6af0`) compares
//! `BigWorldTimeComplete` against — is its view of the server's game
//! time: `gameTime / tickRate` seconds, re-anchored on every tickSync
//! it receives. `onTimerUpdate`'s `BigWorldTimeComplete` field must be
//! an **absolute** expiry in that same domain (`now + duration`), which
//! is what the Python reference sends (`AbilityManager.py`:
//! `cooldown = now + abilityCooldown`, `onTimerUpdate(..., cooldown)`).
//!
//! One boot-stamped monotonic backs both sides of the split:
//!
//! - [`game_time_tick`] feeds the `gameTime` field of the ongoing
//!   `BASEMSG_TICK_SYNC` packets;
//! - [`game_time_secs`] feeds absolute timer endpoints (`onTimerUpdate`
//!   `BigWorldTimeComplete`).
//!
//! They agree by construction (`tick / TICK_RATE == secs`), which is the
//! coupling that keeps an absolute expiry **reachable** by the client's
//! clock for the whole session. An expiry outside the client's reachable
//! domain — say, an absolute value emitted while tickSync stayed pinned
//! at a per-session 0 — is classified by the client as never-elapsing,
//! wedging the cooldown bar "on cooldown" until reconnect.

use std::sync::OnceLock;
use std::time::Instant;

/// Ticks per second carried in the wire `tickRate` field. The client
/// converts `gameTime` ticks to seconds by dividing by this.
pub const TICK_RATE: u32 = 100;

/// Milliseconds per tick, derived from [`TICK_RATE`] so the two can
/// never drift apart in this file.
const MILLIS_PER_TICK: u128 = 1000 / TICK_RATE as u128;

fn boot_instant() -> Instant {
    static BOOT: OnceLock<Instant> = OnceLock::new();
    *BOOT.get_or_init(Instant::now)
}

/// Server game time in seconds since boot, in the client's tickSync
/// domain. `f32` to match the wire's `FLOAT BigWorldTimeComplete`.
pub fn game_time_secs() -> f32 {
    boot_instant().elapsed().as_secs_f32()
}

/// Server game time in wire ticks ([`TICK_RATE`] per second), for the
/// `gameTime` field of `BASEMSG_TICK_SYNC`. `game_time_tick() as f32 /
/// TICK_RATE as f32` equals [`game_time_secs`] to within one tick.
pub fn game_time_tick() -> u32 {
    (boot_instant().elapsed().as_millis() / MILLIS_PER_TICK) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The domain-coupling invariant: the tick value we put on the wire
    /// and the seconds value we put in `BigWorldTimeComplete` must be the
    /// same clock. A divergence here means an absolute expiry the client's
    /// clock can never reach (tickSync pinned low) or one already past
    /// (tickSync racing ahead).
    #[test]
    fn tick_domain_matches_seconds_domain() {
        let tick = game_time_tick();
        let secs = game_time_secs();
        let domain_seconds = tick as f32 / TICK_RATE as f32;
        assert!(
            (domain_seconds - secs).abs() < 0.02,
            "tick/TICK_RATE and game_time_secs diverged: tick={tick}, domain={domain_seconds}, secs={secs}"
        );
    }

    #[test]
    fn game_time_is_monotonic() {
        let secs_a = game_time_secs();
        let tick_a = game_time_tick();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let secs_b = game_time_secs();
        let tick_b = game_time_tick();
        assert!(secs_b >= secs_a, "seconds clock went backwards");
        assert!(tick_b >= tick_a, "tick clock went backwards");
    }
}
