//! `InterpTrackMove` keyframes, and what they say about where an actor
//! spends its time.
//!
//! A Matinee move track stores two curves: `PosTrack` (an
//! `InterpCurveVector` of positions, UE3 cm) and `EulerTrack` (the same
//! shape, rotations in degrees). `MoveFrame` says what the keys are
//! relative to:
//!
//! - `IMF_World` (0, the default, so usually absent from the cooked
//!   tagged-property block) — keys are absolute world values.
//! - `IMF_RelativeToInitial` (1) — keys are offsets from wherever the
//!   actor stood when the sequence started.
//!
//! Every cooked SGW mover measured by NA40 uses the relative frame
//! except a single `CloseDoor` sequence in Castle_CellBlock.

use cimmeria_upk::{parse_tagged_properties, parse_tagged_properties_with_end, PropValue};
use cimmeria_upk::{NameEntry, TaggedProperty};

/// What a move track's keys are relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFrame {
    /// Absolute world position and rotation.
    World,
    /// Offsets from the pose the actor had when the sequence started.
    RelativeToInitial,
}

/// One `InterpTrackMove`, reduced to its key values.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveTrack {
    pub frame: MoveFrame,
    /// `PosTrack` key values, UE3 cm, in key order.
    pub positions: Vec<[f32; 3]>,
    /// `EulerTrack` key values, degrees, in key order.
    pub rotations: Vec<[f32; 3]>,
}

/// A move track measured against the actor's cooked pose.
///
/// "Offset" is always the distance from that pose: for a relative
/// track the key value itself, for a world track the key minus the
/// actor's cooked `Location`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveTrackSummary {
    /// Offset of the first position key, cm.
    pub first_offset_cm: f32,
    /// Offset of the last position key, cm.
    pub last_offset_cm: f32,
    /// Largest horizontal (UE3 X/Y) offset of any key, cm.
    pub max_horizontal_cm: f32,
    /// Largest vertical (UE3 Z) offset of any key, cm.
    pub max_vertical_cm: f32,
    /// Largest rotation of any key, degrees, on any axis. For a world
    /// track the rotations are measured from the first key, since the
    /// cooked rotator and the Euler keys do not share an axis order.
    pub max_rotation_deg: f32,
}

impl MoveTrack {
    /// Parse an `InterpTrackMove` export's tagged-property block.
    pub fn from_props(props: &[TaggedProperty], names: &[NameEntry]) -> Self {
        let frame = match props.iter().find(|p| p.name == "MoveFrame") {
            Some(TaggedProperty {
                value: PropValue::Byte(b),
                ..
            }) if b.first() == Some(&1) => MoveFrame::RelativeToInitial,
            Some(TaggedProperty {
                value: PropValue::Int(1),
                ..
            }) => MoveFrame::RelativeToInitial,
            _ => MoveFrame::World,
        };
        Self {
            frame,
            positions: curve_points(props, "PosTrack", names),
            rotations: curve_points(props, "EulerTrack", names),
        }
    }

    /// Measure this track against a cooked `Location` (UE3 cm).
    pub fn summarize(&self, cooked_location: [f32; 3]) -> MoveTrackSummary {
        let origin = match self.frame {
            MoveFrame::RelativeToInitial => [0.0; 3],
            MoveFrame::World => cooked_location,
        };
        let offsets: Vec<[f32; 3]> = self
            .positions
            .iter()
            .map(|p| [p[0] - origin[0], p[1] - origin[1], p[2] - origin[2]])
            .collect();
        let length = |v: &[f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let rotation_origin = match self.frame {
            MoveFrame::RelativeToInitial => [0.0; 3],
            MoveFrame::World => self.rotations.first().copied().unwrap_or([0.0; 3]),
        };
        MoveTrackSummary {
            first_offset_cm: offsets.first().map(length).unwrap_or(0.0),
            last_offset_cm: offsets.last().map(length).unwrap_or(0.0),
            max_horizontal_cm: offsets
                .iter()
                .map(|o| (o[0] * o[0] + o[1] * o[1]).sqrt())
                .fold(0.0, f32::max),
            max_vertical_cm: offsets.iter().map(|o| o[2].abs()).fold(0.0, f32::max),
            max_rotation_deg: self
                .rotations
                .iter()
                .flat_map(|r| (0..3).map(move |i| (r[i] - rotation_origin[i]).abs()))
                .fold(0.0, f32::max),
        }
    }
}

/// `InterpCurveVector.Points[*].OutVal`, for the struct property `name`.
///
/// A missing curve, or a point without an `OutVal`, reads as no keys
/// rather than as a zero key: a zero would claim the actor sits at its
/// cooked pose, which is the one thing a malformed track must not do.
fn curve_points(props: &[TaggedProperty], name: &str, names: &[NameEntry]) -> Vec<[f32; 3]> {
    let Some(PropValue::Struct { data, .. }) =
        props.iter().find(|p| p.name == name).map(|p| &p.value)
    else {
        return Vec::new();
    };
    let curve = parse_tagged_properties(data, 0, names);
    let Some(PropValue::Array(points)) =
        curve.iter().find(|p| p.name == "Points").map(|p| &p.value)
    else {
        return Vec::new();
    };
    struct_array_elements(points, names)
        .into_iter()
        .filter_map(|point| {
            point.iter().find_map(|p| match (&p.name[..], &p.value) {
                ("OutVal", PropValue::Vector { x, y, z }) => Some([*x, *y, *z]),
                _ => None,
            })
        })
        .collect()
}

/// Split an `ArrayProperty` of tagged-property structs into its
/// elements: `i32 count`, then each element's properties up to its own
/// `None`.
pub(crate) fn struct_array_elements(bytes: &[u8], names: &[NameEntry]) -> Vec<Vec<TaggedProperty>> {
    let Some(count) = bytes
        .get(..4)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut pos = 4usize;
    for _ in 0..count.max(0) {
        if pos >= bytes.len() {
            break;
        }
        let (props, end) = parse_tagged_properties_with_end(bytes, pos, names);
        if end <= pos {
            break;
        }
        pos = end;
        out.push(props);
    }
    out
}
