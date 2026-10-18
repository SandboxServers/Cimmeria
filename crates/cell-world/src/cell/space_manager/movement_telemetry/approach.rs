//! The approach ring: the last few accepted positions of each player,
//! kept server-side so `interaction.approach` (player-capture-bots, idea
//! 2b) can show how a player walked up to an interaction.
//!
//! The ring is fed from the accept path at 1 Hz, independent of the 5 s
//! position sample, and nothing here changes a movement decision. A step
//! longer than [`APPROACH_JUMP_M`] is a teleport, so the trail is dropped
//! there rather than drawn as a line across the map.

use std::time::{Duration, Instant};

use super::MovementTelemetry;

/// How many accepted positions are kept per player.
pub(crate) const APPROACH_POINTS: usize = 8;

/// Minimum gap between two ring entries for one player.
pub(crate) const APPROACH_MIN_INTERVAL: Duration = Duration::from_secs(1);

/// Entries older than this are left out of a trail.
pub(crate) const APPROACH_MAX_AGE: Duration = Duration::from_secs(30);

/// A step longer than this, in metres, is a teleport and clears the ring.
pub(crate) const APPROACH_JUMP_M: f32 = 10.0;

/// One ring entry as it appears in a trail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApproachPoint {
    /// World position of the player when the point was accepted.
    pub pos: [f32; 3],
    /// Player yaw in degrees, wrapped to `[0, 360)`.
    pub yaw_deg: f32,
    /// Seconds between the point and the moment the trail was read.
    pub age_s: f32,
}

/// The points leading up to an interaction, oldest first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApproachTrail {
    /// Points in arrival order, oldest first.
    pub points: Vec<ApproachPoint>,
}

impl ApproachTrail {
    /// Seconds covered by the trail, or 0.0 with fewer than two points.
    pub fn span_s(&self) -> f32 {
        match (self.points.first(), self.points.last()) {
            (Some(first), Some(last)) if self.points.len() >= 2 => first.age_s - last.age_s,
            _ => 0.0,
        }
    }

    /// Length of the walked path, summing 3D distances between consecutive
    /// points, in metres.
    pub fn path_len_m(&self) -> f32 {
        self.points
            .windows(2)
            .map(|w| dist3(w[0].pos, w[1].pos))
            .sum()
    }

    /// Wire form: points joined with `;`, each as
    /// `x,y,z,yaw_deg,age_s` with the precision the trail needs and no
    /// more.
    pub fn encode(&self) -> String {
        self.points
            .iter()
            .map(|p| {
                format!(
                    "{:.2},{:.2},{:.2},{:.1},{:.1}",
                    p.pos[0], p.pos[1], p.pos[2], p.yaw_deg, p.age_s
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

impl MovementTelemetry {
    /// Record one accepted position in the player's ring.
    ///
    /// Skips the point when the last one is less than
    /// [`APPROACH_MIN_INTERVAL`] old, clears the ring on a teleport, and
    /// evicts the oldest entry beyond [`APPROACH_POINTS`].
    pub(crate) fn note_approach_point(
        &mut self,
        entity_id: u32,
        pos: [f32; 3],
        yaw_deg: f32,
        now: Instant,
    ) {
        let ring = self.approach.entry(entity_id).or_default();
        if let Some(&(at, last, _)) = ring.back() {
            if now.saturating_duration_since(at) < APPROACH_MIN_INTERVAL {
                return;
            }
            if dist3(last, pos) > APPROACH_JUMP_M {
                // A teleport breaks the trail.
                ring.clear();
            }
        }
        ring.push_back((now, pos, yaw_deg));
        while ring.len() > APPROACH_POINTS {
            ring.pop_front();
        }
    }

    /// The player's trail as of `now`: points no older than
    /// [`APPROACH_MAX_AGE`], oldest first. `None` when fewer than two
    /// points remain, since one point is not a trail.
    pub fn approach_trail(&self, entity_id: u32, now: Instant) -> Option<ApproachTrail> {
        let ring = self.approach.get(&entity_id)?;
        let points: Vec<ApproachPoint> = ring
            .iter()
            .filter_map(|&(at, pos, yaw_deg)| {
                let age = now.saturating_duration_since(at);
                (age <= APPROACH_MAX_AGE).then_some(ApproachPoint {
                    pos,
                    yaw_deg,
                    age_s: age.as_secs_f32(),
                })
            })
            .collect();
        (points.len() >= 2).then_some(ApproachTrail { points })
    }
}

/// Euclidean distance between two world positions.
fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    (dx * dx + dy * dy + dz * dz).sqrt()
}
