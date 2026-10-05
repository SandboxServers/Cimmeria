//! One group's schedule: which exchange is next, and when its next line is
//! due. Pure state over [`Instant`]s, so the timing is tested without a
//! world.
//!
//! A group waits until its quiet time is over, then starts its next
//! exchange only if a player is in earshot (otherwise it looks again after
//! [`IDLE_RECHECK`] and keeps the same exchange, so the next listener hears a
//! scene from its start). Once an exchange starts, its lines are spoken in
//! order, each after its own delay, whoever is still listening; after the
//! last line the group is quiet for its `exchange_gap`, then moves on to the
//! following exchange, wrapping round to the first.

use std::time::{Duration, Instant};

use cimmeria_cell_catalog::cell::spawner::ChatterGroup;

/// How long a group with nobody in earshot waits before looking again.
pub(crate) const IDLE_RECHECK: Duration = Duration::from_secs(2);

/// Where a group is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Quiet until `until`; then the next exchange may start.
    Waiting { until: Instant },
    /// Exchange `exchange` is under way; line `line` is due at `due`.
    Speaking {
        exchange: usize,
        line: usize,
        due: Instant,
    },
}

/// What a group has due now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Due {
    /// The quiet time is over: start `exchange` if anyone is listening.
    Start { exchange: usize },
    /// Speak `line` of `exchange`.
    Line { exchange: usize, line: usize },
}

/// One group's run state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupRun {
    /// The exchange the group starts next.
    pub(crate) next_exchange: usize,
    pub(crate) phase: Phase,
}

impl GroupRun {
    /// A group that may start its first exchange at `now`.
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            next_exchange: 0,
            phase: Phase::Waiting { until: now },
        }
    }

    /// When the group next has something to do.
    pub(crate) fn due_at(&self) -> Instant {
        match self.phase {
            Phase::Waiting { until } => until,
            Phase::Speaking { due, .. } => due,
        }
    }

    /// What the group has due at `now`, if anything.
    pub(crate) fn due(&self, now: Instant) -> Option<Due> {
        match self.phase {
            Phase::Waiting { until } if until <= now => Some(Due::Start {
                exchange: self.next_exchange,
            }),
            Phase::Speaking {
                exchange,
                line,
                due,
            } if due <= now => Some(Due::Line { exchange, line }),
            _ => None,
        }
    }

    /// Nobody was in earshot when the exchange was due: look again after
    /// [`IDLE_RECHECK`], keeping the same exchange.
    pub(crate) fn no_audience(&mut self, now: Instant) {
        self.phase = Phase::Waiting {
            until: now + IDLE_RECHECK,
        };
    }

    /// Start the next exchange at `now`. Its first line is due after that
    /// line's own delay.
    pub(crate) fn start(&mut self, now: Instant, group: &ChatterGroup) {
        let exchange = self.next_exchange % group.exchanges.len();
        self.next_exchange = (exchange + 1) % group.exchanges.len();
        let first = group.exchanges[exchange]
            .lines
            .first()
            .map_or(Duration::ZERO, |l| l.delay);
        self.phase = Phase::Speaking {
            exchange,
            line: 0,
            due: now + first,
        };
    }

    /// The due line was spoken (or skipped) at `now`: the next line is due
    /// after its delay, or, after the last line, the group is quiet for its
    /// `exchange_gap`.
    pub(crate) fn line_done(&mut self, now: Instant, group: &ChatterGroup) {
        let Phase::Speaking { exchange, line, .. } = self.phase else {
            return;
        };
        let lines = &group.exchanges[exchange].lines;
        self.phase = match lines.get(line + 1) {
            Some(next) => Phase::Speaking {
                exchange,
                line: line + 1,
                due: now + next.delay,
            },
            None => Phase::Waiting {
                until: now + group.exchange_gap,
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use cimmeria_cell_catalog::cell::spawner::{ChatterExchange, ChatterLine};

    use super::*;

    fn group(exchanges: &[&[u64]]) -> ChatterGroup {
        ChatterGroup {
            group_id: 1,
            world_id: 1300,
            name: "test".into(),
            hear_radius: 20.0,
            exchange_gap: Duration::from_secs(30),
            exchanges: exchanges
                .iter()
                .enumerate()
                .map(|(i, delays)| ChatterExchange {
                    exchange_id: i as i32 + 1,
                    lines: delays
                        .iter()
                        .map(|&ms| ChatterLine {
                            speaker_tag: "A".into(),
                            delay: Duration::from_millis(ms),
                            text: "x".into(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// A fresh group is due to start at once, and does nothing before its
    /// time.
    #[test]
    fn a_new_group_is_due_to_start_its_first_exchange() {
        let t0 = Instant::now();
        let run = GroupRun::new(t0);
        assert_eq!(run.due(t0), Some(Due::Start { exchange: 0 }));
        assert_eq!(run.due_at(), t0);
        let waiting = GroupRun {
            next_exchange: 0,
            phase: Phase::Waiting {
                until: t0 + Duration::from_secs(1),
            },
        };
        assert_eq!(waiting.due(t0), None);
    }

    /// With nobody listening the group keeps its exchange and looks again
    /// after IDLE_RECHECK: a player who walks up later hears the scene from
    /// its first line, not from wherever an empty room had got to.
    #[test]
    fn no_audience_keeps_the_exchange_and_rechecks() {
        let t0 = Instant::now();
        let mut run = GroupRun::new(t0);
        run.no_audience(t0);
        assert_eq!(run.due(t0), None);
        assert_eq!(run.due(t0 + IDLE_RECHECK), Some(Due::Start { exchange: 0 }));
    }

    /// One exchange end to end: each line due after its own delay, then the
    /// exchange gap, then the next exchange, wrapping round to the first.
    #[test]
    fn lines_follow_their_delays_then_the_gap_then_the_next_exchange() {
        let g = group(&[&[0, 4000, 3000], &[500]]);
        let t0 = Instant::now();
        let mut run = GroupRun::new(t0);

        run.start(t0, &g);
        assert_eq!(
            run.due(t0),
            Some(Due::Line {
                exchange: 0,
                line: 0
            })
        );
        run.line_done(t0, &g);
        let t1 = t0 + Duration::from_millis(3999);
        assert_eq!(run.due(t1), None, "line 1 waits its 4 s");
        let t1 = t0 + Duration::from_millis(4000);
        assert_eq!(
            run.due(t1),
            Some(Due::Line {
                exchange: 0,
                line: 1
            })
        );
        run.line_done(t1, &g);
        let t2 = t1 + Duration::from_millis(3000);
        assert_eq!(
            run.due(t2),
            Some(Due::Line {
                exchange: 0,
                line: 2
            })
        );
        run.line_done(t2, &g);

        // The last line out: quiet for the 30 s gap.
        assert_eq!(run.due(t2 + Duration::from_secs(29)), None);
        let t3 = t2 + Duration::from_secs(30);
        assert_eq!(run.due(t3), Some(Due::Start { exchange: 1 }));

        // Exchange 2's first line has its own 500 ms delay.
        run.start(t3, &g);
        assert_eq!(run.due(t3), None);
        let t4 = t3 + Duration::from_millis(500);
        assert_eq!(
            run.due(t4),
            Some(Due::Line {
                exchange: 1,
                line: 0
            })
        );
        run.line_done(t4, &g);
        let t5 = t4 + Duration::from_secs(30);
        assert_eq!(
            run.due(t5),
            Some(Due::Start { exchange: 0 }),
            "after the last exchange the group starts again from the first"
        );
    }
}
