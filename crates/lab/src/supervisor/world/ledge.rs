//! The ledge guard for `move_to` (C14): while walking forward, trace down
//! ahead of the pawn and stop before stepping off an edge. A trace that
//! cannot see the floor turns the guard off for that walk instead.

use super::geometry::{Pose, Vec3};
use super::io::WorldIo;

/// How far ahead of the pawn the probe lands (UE3 units).
pub const LEDGE_AHEAD_UU: f64 = 150.0;
/// Half the pawn's height: the probe starts at the pawn's z and drops below its feet.
pub const LEDGE_HALF_HEIGHT_UU: f64 = 78.0;
/// Extra drop below the feet that counts as a ledge (2 m).
pub const LEDGE_DROP_UU: f64 = 200.0;
/// The probe runs at most this often while walking.
pub const LEDGE_EVERY_MS: u64 = 500;
/// Lift the trace start so it begins in open air even when the pose z is at the floor.
pub const LEDGE_LIFT_UU: f64 = 100.0;
/// Level geometry only; pawns are not hit.
const LEDGE_FLAGS: u32 = 0x2086;

/// The lower end of the down-trace from height `z`.
fn floor_z(z: f64) -> f64 {
    z - LEDGE_HALF_HEIGHT_UU - LEDGE_DROP_UU
}

/// The probe: a line from above the pawn, `LEDGE_AHEAD_UU` ahead along its yaw.
pub fn probe(pose: Pose) -> (Vec3, Vec3) {
    let x = pose.pos.x + LEDGE_AHEAD_UU * pose.yaw.cos();
    let y = pose.pos.y + LEDGE_AHEAD_UU * pose.yaw.sin();
    (
        Vec3::new(x, y, pose.pos.z + LEDGE_LIFT_UU),
        Vec3::new(x, y, floor_z(pose.pos.z)),
    )
}

/// The same span straight down under the pawn (the self-check).
pub fn under(pose: Pose) -> (Vec3, Vec3) {
    (
        Vec3::new(pose.pos.x, pose.pos.y, pose.pos.z + LEDGE_LIFT_UU),
        Vec3::new(pose.pos.x, pose.pos.y, floor_z(pose.pos.z)),
    )
}

/// The guard's state for one walk.
#[derive(Debug, Clone, PartialEq)]
pub enum Guard {
    /// Checking; `last_ms` is when the last probe ran.
    On { last_ms: Option<u64> },
    /// `allow_drops`: no probes.
    Off,
    /// A trace failed or cannot see the floor: no probes for this walk.
    Unavailable(String),
}

impl Guard {
    /// The `ledge_guard` evidence value.
    pub fn label(&self) -> String {
        match self {
            Guard::On { .. } => "on".into(),
            Guard::Off => "off".into(),
            Guard::Unavailable(e) => format!("unavailable: {e}"),
        }
    }
}

/// Before the first step: a trace straight down must find a floor, or the
/// guard is unavailable for this walk.
pub async fn self_check<W: WorldIo>(g: &mut Guard, io: &mut W, pose: Pose) {
    if !matches!(g, Guard::On { .. }) {
        return;
    }
    let (start, end) = under(pose);
    *g = match io.trace_line(start, end, LEDGE_FLAGS).await {
        Ok(Some(_)) => Guard::On { last_ms: None },
        Ok(None) => Guard::Unavailable("no floor under pawn".into()),
        Err(e) => Guard::Unavailable(e),
    };
    if let Guard::Unavailable(why) = &*g {
        tracing::info!(%why, "ledge guard unavailable for this walk");
    }
}

/// True when the walk must stop: the probe ahead finds no floor. Probes at
/// most every `LEDGE_EVERY_MS`; a failed trace makes the guard unavailable.
pub async fn check<W: WorldIo>(g: &mut Guard, io: &mut W, now: u64, pose: Pose) -> bool {
    let Guard::On { last_ms } = g else {
        return false;
    };
    if last_ms.is_some_and(|t| now.saturating_sub(t) < LEDGE_EVERY_MS) {
        return false;
    }
    *last_ms = Some(now);
    let (start, end) = probe(pose);
    match io.trace_line(start, end, LEDGE_FLAGS).await {
        Ok(None) => true,
        Ok(Some(_)) => false,
        Err(e) => {
            tracing::info!(%e, "ledge guard unavailable for this walk");
            *g = Guard::Unavailable(e);
            false
        }
    }
}
