//! The load grace's decision and the watchdog's per-poll step: kills,
//! graces, the sticky busy rule and the failure-count fallback.

use super::*;

const FAILS: u32 = 5;

/// Live regression (2026-10-05): a 19-42 s load of world 1300 failed
/// five heartbeats in a row and was terminated mid-load. A busy main
/// thread inside the grace waits.
#[test]
fn a_load_stall_within_the_grace_is_not_killed() {
    let v = decide(Poll::Failed { fails: 5 }, FAILS, true, 30_000, LOAD_GRACE);
    assert_eq!(v, Verdict::Grace { stalled_ms: 30_000 });
    let v = decide(Poll::Failed { fails: 19 }, FAILS, true, 119_999, LOAD_GRACE);
    assert_eq!(
        v,
        Verdict::Grace {
            stalled_ms: 119_999
        }
    );
}

/// The grace is a cap: a busy stall that outlasts it is killed, and
/// the kill says it was loading.
#[test]
fn a_load_stall_beyond_the_cap_is_killed() {
    let v = decide(Poll::Failed { fails: 20 }, FAILS, true, 120_000, LOAD_GRACE);
    assert_eq!(
        v,
        Verdict::Kill {
            loading: true,
            stalled_ms: 120_000
        }
    );
}

/// An idle stall (a deadlock or a crash dialog) dies at five failures,
/// as before, however short the stall; four is still a wait.
#[test]
fn an_idle_stall_still_dies_at_five_failures() {
    assert_eq!(
        decide(Poll::Failed { fails: 4 }, FAILS, false, 24_000, LOAD_GRACE),
        Verdict::Continue
    );
    assert_eq!(
        decide(Poll::Failed { fails: 5 }, FAILS, false, 30_000, LOAD_GRACE),
        Verdict::Kill {
            loading: false,
            stalled_ms: 30_000
        }
    );
}

/// The stale-Tick rule gets the same grace: answered polls with a
/// stopped counter wait while busy, die when idle or past the cap.
#[test]
fn the_stale_tick_rule_respects_the_grace() {
    let stale = Poll::Answered { stale: true };
    assert_eq!(
        decide(stale, FAILS, true, 9_000, LOAD_GRACE),
        Verdict::Grace { stalled_ms: 9_000 }
    );
    assert_eq!(
        decide(stale, FAILS, false, 9_000, LOAD_GRACE),
        Verdict::Kill {
            loading: false,
            stalled_ms: 9_000
        }
    );
    assert_eq!(
        decide(stale, FAILS, true, 121_000, LOAD_GRACE),
        Verdict::Kill {
            loading: true,
            stalled_ms: 121_000
        }
    );
    assert_eq!(
        decide(Poll::Answered { stale: false }, FAILS, false, 0, LOAD_GRACE),
        Verdict::Continue
    );
}

/// A grace of zero (`CIMMERIA_LAB_LOAD_GRACE_SECS=0`) restores the old
/// behaviour: a busy stall dies at the trip.
#[test]
fn a_zero_grace_turns_the_grace_off() {
    assert_eq!(
        decide(
            Poll::Failed { fails: 5 },
            FAILS,
            true,
            30_000,
            Duration::ZERO
        ),
        Verdict::Kill {
            loading: true,
            stalled_ms: 30_000
        }
    );
}

// --- StallTracker: the per-poll step the watchdog loop runs ---------

const POLL: i64 = 6_000; // a failed poll: 1 s sleep + 5 s timeout
const BUSY: Option<(i64, i64)> = Some((3_000, POLL));
const IDLE: Option<(i64, i64)> = Some((5, POLL));

fn tracker() -> StallTracker {
    StallTracker::new(FAILS, LOAD_GRACE)
}

/// A heartbeat that answered with the counter advancing at `now`: the
/// stall clock restarts there.
fn ticked(t: &mut StallTracker, now: i64) -> Step {
    t.step(Heartbeat::Answered { stale: false }, BUSY, now, 0)
}

/// The `n`th failed poll of a stall that began at `start`.
fn failed(t: &mut StallTracker, start: i64, n: i64, sample: Option<(i64, i64)>) -> Step {
    let now = start + n * POLL;
    let hb = Heartbeat::Failed {
        ms_since_last_ok: Some(now - start),
    };
    t.step(hb, sample, now, now - start)
}

/// Run failed polls `1..=n` with `sample(i)`; return the verdicts.
fn stall(
    t: &mut StallTracker,
    start: i64,
    n: i64,
    sample: impl Fn(i64) -> Option<(i64, i64)>,
) -> Vec<Verdict> {
    (1..=n)
        .map(|i| failed(t, start, i, sample(i)).verdict)
        .collect()
}

/// The safety fallback, end to end: an unreadable main thread (`None`)
/// is idle, so a stall dies at the fifth failure as before the grace.
#[test]
fn an_unreadable_main_thread_falls_back_to_the_old_kill() {
    let mut t = tracker();
    assert_eq!(ticked(&mut t, 0).verdict, Verdict::Continue);
    let v = stall(&mut t, 0, 5, |_| None);
    assert_eq!(&v[..4], &[Verdict::Continue; 4]);
    assert_eq!(
        v[4],
        Verdict::Kill {
            loading: false,
            stalled_ms: 30_000
        }
    );
}

/// A working main thread through the stall gets the grace at the trip.
#[test]
fn a_busy_stall_is_graced_at_the_trip() {
    let mut t = tracker();
    ticked(&mut t, 0);
    let v = stall(&mut t, 0, 5, |_| BUSY);
    assert_eq!(v[4], Verdict::Grace { stalled_ms: 30_000 });
}

/// Review finding (PR #1268): busy judged on the last interval alone
/// killed a load whose trip landed on one disk-bound window. Busy is
/// sticky for the stall, so idle windows after work keep the grace,
/// up to the cap.
#[test]
fn a_load_with_idle_windows_survives_until_the_cap() {
    let mut t = tracker();
    ticked(&mut t, 0);
    // Work in polls 2-3, then idle from the trip on.
    let v = stall(&mut t, 0, 21, |i| if i <= 3 { BUSY } else { IDLE });
    assert_eq!(v[4], Verdict::Grace { stalled_ms: 30_000 });
    assert_eq!(
        v[18],
        Verdict::Grace {
            stalled_ms: 114_000
        }
    );
    assert_eq!(
        v[19],
        Verdict::Kill {
            loading: true,
            stalled_ms: 120_000
        }
    );
}

/// A stall that never shows work dies at the fifth failure, ~30 s: the
/// pre-grace time.
#[test]
fn a_genuinely_idle_stall_dies_at_the_old_time() {
    let mut t = tracker();
    ticked(&mut t, 0);
    let v = stall(&mut t, 0, 5, |_| IDLE);
    assert_eq!(
        v[4],
        Verdict::Kill {
            loading: false,
            stalled_ms: 30_000
        }
    );
}

/// The first failed poll's interval still holds frames the thread
/// ticked before it stalled; its CPU must not make a hang look busy.
#[test]
fn the_onset_sample_does_not_count_as_busy() {
    let mut t = tracker();
    ticked(&mut t, 0);
    let v = stall(&mut t, 0, 5, |i| if i == 1 { BUSY } else { IDLE });
    assert_eq!(
        v[4],
        Verdict::Kill {
            loading: false,
            stalled_ms: 30_000
        }
    );
}

/// Busy belongs to one stall: after the Tick advances, a new idle stall
/// dies at the old rules even though the last one was a load.
#[test]
fn a_tick_advance_resets_busy() {
    let mut t = tracker();
    ticked(&mut t, 0);
    assert_eq!(
        stall(&mut t, 0, 5, |_| BUSY)[4],
        Verdict::Grace { stalled_ms: 30_000 }
    );
    ticked(&mut t, 40_000);
    let v = stall(&mut t, 40_000, 5, |_| IDLE);
    assert_eq!(
        v[4],
        Verdict::Kill {
            loading: false,
            stalled_ms: 30_000
        }
    );
}

/// A missed heartbeat while other calls complete is forgiven, through
/// the tracker's own failure count.
#[test]
fn a_busy_bridge_forgives_misses() {
    let mut t = tracker();
    ticked(&mut t, 0);
    for n in 1..=8 {
        let s = t.step(
            Heartbeat::Failed {
                ms_since_last_ok: Some(100),
            },
            IDLE,
            n * POLL,
            n * POLL,
        );
        assert_eq!(s.poll, Poll::Failed { fails: 0 });
        assert_eq!(s.verdict, Verdict::Continue);
    }
}

#[test]
fn the_env_override_parses_seconds() {
    assert_eq!(load_grace_from(None), LOAD_GRACE);
    assert_eq!(load_grace_from(Some("")), LOAD_GRACE);
    assert_eq!(load_grace_from(Some("abc")), LOAD_GRACE);
    assert_eq!(load_grace_from(Some(" 300 ")), Duration::from_secs(300));
    assert_eq!(load_grace_from(Some("0")), Duration::ZERO);
    // Clamped, so a huge value can't wrap negative in `decide`.
    assert_eq!(load_grace_from(Some("3601")), MAX_LOAD_GRACE);
    assert_eq!(
        load_grace_from(Some("18446744073709551615")),
        MAX_LOAD_GRACE
    );
}

/// Even an unclamped huge grace stays a grace, not a negative cap.
#[test]
fn a_huge_grace_does_not_wrap() {
    let v = decide(
        Poll::Failed { fails: 5 },
        FAILS,
        true,
        30_000,
        Duration::MAX,
    );
    assert_eq!(v, Verdict::Grace { stalled_ms: 30_000 });
}

/// 2 % of the interval is busy; a blocked thread's few ms are not.
#[test]
fn busy_is_a_share_of_the_interval() {
    assert!(main_thread_busy(120, 6_000));
    assert!(main_thread_busy(4_000, 6_000));
    assert!(!main_thread_busy(119, 6_000));
    assert!(!main_thread_busy(0, 6_000));
    assert!(!main_thread_busy(50, 0), "no interval, no verdict");
}

/// Before the bridge has answered once, a failed heartbeat inside the
/// boot grace is ignored; after the grace it counts again.
#[test]
fn a_boot_failure_inside_the_grace_is_ignored() {
    let start = 1_000_000;
    let boot = BootGrace::new(start, BOOT_GRACE);
    assert_eq!(boot.ignores_failure(start), Some(Duration::ZERO));
    assert_eq!(
        boot.ignores_failure(start + 89_000),
        Some(Duration::from_secs(89))
    );
    assert_eq!(
        boot.ignores_failure(start + 90_000),
        None,
        "the grace ends at 90 s"
    );
    assert_eq!(boot.ignores_failure(start + 91_000), None);
    // A clock that steps back reads as zero time since launch.
    assert_eq!(boot.ignores_failure(start - 5_000), Some(Duration::ZERO));
}

/// Once the bridge has answered, the boot grace is over, however young
/// the launch is: a client that answers and then hangs gets none.
#[test]
fn an_answered_bridge_has_no_boot_grace() {
    let start = 1_000_000;
    let mut boot = BootGrace::new(start, BOOT_GRACE);
    boot.answered();
    assert_eq!(boot.ignores_failure(start), None);
    assert_eq!(boot.ignores_failure(start + 10_000), None);
}
