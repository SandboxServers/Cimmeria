//! Input latency and coasting for the test `Sim`: a key change can reach the
//! game late, and forward motion can run on for a while after `W` goes up.
//! Pure: the `Sim` passes its own clock in, and nothing here sleeps.

/// The lag model. The defaults change nothing: keys apply at once and the
/// pawn stops the moment `W` goes up.
#[derive(Debug, Clone, Default)]
pub(crate) struct Lag {
    /// Milliseconds a key change waits before it applies (0: at once).
    pub input_latency_ms: u64,
    /// Seconds the pawn keeps walking forward after `W` goes up (0: stops).
    pub coast_s: f64,
    /// Changes not yet applied, `(due_ms, key, down)`, in arrival order.
    pending: Vec<(u64, String, bool)>,
    /// When the current coast ends, if one is running.
    coast_until_ms: Option<u64>,
}

impl Lag {
    /// A key change at `now`: `Some((key, down))` to apply now, or `None`
    /// when it was queued for `input_latency_ms` later.
    pub fn key(&mut self, now: u64, key: &str, down: bool) -> Option<(String, bool)> {
        if self.input_latency_ms == 0 {
            return Some((key.to_string(), down));
        }
        self.pending
            .push((now + self.input_latency_ms, key.to_string(), down));
        None
    }

    /// Queued changes due at `now`, in order.
    pub fn due(&mut self, now: u64) -> Vec<(String, bool)> {
        let mut out = Vec::new();
        let mut kept = Vec::with_capacity(self.pending.len());
        for (at, key, down) in self.pending.drain(..) {
            if at <= now {
                out.push((key, down));
            } else {
                kept.push((at, key, down));
            }
        }
        self.pending = kept;
        out
    }

    /// Forward was released at `now`: start a coast of `coast_s`.
    pub fn released_forward(&mut self, now: u64) {
        self.coast_until_ms = (self.coast_s > 0.0).then(|| now + secs_to_ms(self.coast_s));
    }

    /// True while the pawn should keep walking forward with `W` up.
    pub fn coasting(&self, now: u64) -> bool {
        self.coast_until_ms.is_some_and(|until| now <= until)
    }

    /// Forward was pressed again: stop any coast.
    pub fn pressed_forward(&mut self) {
        self.coast_until_ms = None;
    }
}

fn secs_to_ms(s: f64) -> u64 {
    (s * 1000.0).round() as u64
}
