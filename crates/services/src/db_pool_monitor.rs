//! The database pool sampler (TG-DB-06).
//!
//! Every 15 s it reads the pool's `size` (open connections) and `idle`
//! (connections nobody holds) and writes a DEBUG row on target `db.pool`.
//! When every connection is open and none is idle, the next query waits in
//! sqlx's acquire queue (up to the 30 s acquire timeout) and the server
//! stalls on it, so that sample is also a WARN `event = "db_pool_saturated"`.
//!
//! The WARN is Pattern D (negative-logging convention): the first saturated
//! sample warns at once, then at most one WARN per 60 s while the pool stays
//! saturated, carrying `suppressed`, the saturated samples swallowed since
//! the last WARN. The first unsaturated sample ends the episode, so the next
//! saturation warns at once again. There is one pool, so one throttle key.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

/// How often the sampler reads the pool.
pub(crate) const SAMPLE_INTERVAL: Duration = Duration::from_secs(15);

/// A saturated episode warns again every this many samples (60 s at 15 s).
pub(crate) const WARN_EVERY_SAMPLES: u32 = 4;

/// The Pattern D window in seconds, reported on the WARN.
const WARN_WINDOW_SECS: u64 = SAMPLE_INTERVAL.as_secs() * WARN_EVERY_SAMPLES as u64;

/// The sampler's throttle state between samples.
#[derive(Debug, Default)]
pub(crate) struct SaturationState {
    /// `true` from the first saturated sample until an unsaturated one.
    in_episode: bool,
    /// Saturated samples since the last WARN, that WARN's own included.
    samples_since_warn: u32,
    /// Saturated samples swallowed since the last WARN.
    suppressed: u32,
}

/// Judge one sample of the pool: `size` open connections, `idle` of them
/// free, `max` the pool's ceiling. Writes the DEBUG sample row and, on
/// saturation, the throttled WARN.
pub(crate) fn sample(size: u32, idle: usize, max: u32, state: &mut SaturationState) {
    let idle = idle as u64;
    tracing::debug!(
        target: "db.pool",
        size,
        idle,
        max_connections = max,
        "database pool sample"
    );

    // `size` never exceeds `max` (sqlx holds a semaphore of `max` permits);
    // `>=` keeps a saturated pool saturated if that ever changes.
    let saturated = idle == 0 && size >= max;
    if !saturated {
        *state = SaturationState::default();
        return;
    }
    if state.in_episode && state.samples_since_warn < WARN_EVERY_SAMPLES {
        state.samples_since_warn += 1;
        state.suppressed += 1;
        return;
    }
    tracing::warn!(
        target: "db.pool",
        event = "db_pool_saturated",
        size,
        idle,
        max_connections = max,
        suppressed = state.suppressed,
        window_secs = WARN_WINDOW_SECS,
        "database pool saturated: every connection is in use, so queries wait for a free one"
    );
    state.in_episode = true;
    state.samples_since_warn = 1;
    state.suppressed = 0;
}

/// Start the sampler for `pool`, whose ceiling is `max_connections`. It runs
/// until the pool is closed or the returned handle is aborted
/// ([`crate::database::DatabasePool`] aborts it when dropped).
pub(crate) fn spawn(pool: PgPool, max_connections: u32) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut state = SaturationState::default();
        loop {
            ticker.tick().await;
            if pool.is_closed() {
                break;
            }
            sample(pool.size(), pool.num_idle(), max_connections, &mut state);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    fn saturated_warns(capture: &crate::test_support::LogCaptureGuard) -> Vec<String> {
        capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::WARN
                    && c.target == "db.pool"
                    && c.has_field("event", "db_pool_saturated")
            })
            .map(|c| c.fields.get("suppressed").cloned().unwrap_or_default())
            .collect()
    }

    /// Saturation WARNs at once; the next saturated sample is suppressed.
    /// Fails if the WARN is removed or the throttle lets every sample warn.
    #[test]
    fn saturation_warns_then_the_next_sample_is_suppressed() {
        let capture = LogCapture::install();
        let mut state = SaturationState::default();

        sample(10, 0, 10, &mut state);
        assert_eq!(saturated_warns(&capture), vec!["0".to_string()]);
        let warn = capture
            .all()
            .into_iter()
            .find(|c| c.level == Level::WARN)
            .expect("saturation must warn");
        assert!(warn.has_field("size", "10"));
        assert!(warn.has_field("idle", "0"));
        assert!(warn.has_field("max_connections", "10"));

        sample(10, 0, 10, &mut state);
        assert_eq!(
            saturated_warns(&capture).len(),
            1,
            "the second saturated sample must be suppressed"
        );

        // Every sample writes its DEBUG row, warned or not.
        let debug_rows = capture
            .all()
            .into_iter()
            .filter(|c| c.level == Level::DEBUG && c.target == "db.pool")
            .count();
        assert_eq!(debug_rows, 2);
    }

    /// A burst: a pool saturated for five samples warns on the first and
    /// again on the fifth (60 s later), which carries `suppressed = 3`.
    #[test]
    fn a_long_saturation_rewarns_each_window_with_the_suppressed_count() {
        let capture = LogCapture::install();
        let mut state = SaturationState::default();
        for _ in 0..5 {
            sample(10, 0, 10, &mut state);
        }
        assert_eq!(
            saturated_warns(&capture),
            vec!["0".to_string(), "3".to_string()]
        );
    }

    /// The first unsaturated sample resets the state, so the next
    /// saturation warns at once instead of being swallowed.
    #[test]
    fn an_unsaturated_sample_resets_the_throttle() {
        let capture = LogCapture::install();
        let mut state = SaturationState::default();

        sample(10, 0, 10, &mut state);
        sample(10, 1, 10, &mut state); // a connection came free
        sample(10, 0, 10, &mut state);
        assert_eq!(
            saturated_warns(&capture),
            vec!["0".to_string(), "0".to_string()],
            "saturation after a free sample must warn again at once"
        );
    }

    /// A pool below its ceiling with nothing idle is busy, not saturated:
    /// sqlx opens another connection for the next query.
    #[test]
    fn a_busy_pool_below_its_ceiling_is_not_saturated() {
        let capture = LogCapture::install();
        let mut state = SaturationState::default();
        sample(4, 0, 10, &mut state);
        assert!(saturated_warns(&capture).is_empty());
    }
}
