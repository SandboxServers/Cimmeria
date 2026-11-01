//! The fake `trace_line` for [`Sim`]: a hit at the end of the line, unless the
//! floor ends before the line starts. Split out of `sim.rs` for its size limit.

use super::super::geometry::Vec3;
use super::super::io::TraceHit;
use super::Sim;

impl Sim {
    /// Answer one trace. `floor_end_x` ends the floor: a line that starts past
    /// it finds nothing. `trace_error` fails the call.
    pub(super) fn trace(&mut self, start: Vec3, end: Vec3) -> Result<Option<TraceHit>, String> {
        self.trace_calls += 1;
        if let Some(e) = &self.trace_error {
            return Err(e.clone());
        }
        if self.floor_end_x.is_some_and(|x| start.x > x) {
            return Ok(None);
        }
        Ok(Some(TraceHit {
            time: 1.0,
            location: end,
            item: 0,
            actor: 0,
        }))
    }
}
