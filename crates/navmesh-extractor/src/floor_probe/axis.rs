//! UE3 -> BigWorld axis algebra for the floor probe.
//!
//! Split out of `mod.rs` at the transform seam: this file answers
//! "which BigWorld axis reads which UE3 axis, with which sign", and
//! nothing here knows what a probe point or a triangle is. The probe
//! itself enumerates [`AxisMapping::all`] and asks the geometry which
//! one is right -- see the module docs on [`super`].

/// Centimetres per BigWorld unit — the `/ 100` in NavBuilder's `loadOBJ`.
pub const CM_PER_BW_UNIT: f32 = 100.0;

/// Which UE3 source axis (with sign) feeds one BigWorld axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisSource {
    /// 0 = UE3 X, 1 = UE3 Y, 2 = UE3 Z.
    pub ue3_axis: u8,
    pub negate: bool,
}

impl AxisSource {
    pub const fn new(ue3_axis: u8, negate: bool) -> Self {
        Self { ue3_axis, negate }
    }
}

/// A full UE3-cm → BigWorld-unit transform: an axis permutation, per-axis
/// signs, and the fixed cm→unit divide.
///
/// `bw[0]` sources BigWorld **x**, `bw[1]` BigWorld **y (up)**, `bw[2]`
/// BigWorld **z**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisMapping {
    pub bw: [AxisSource; 3],
}

impl AxisMapping {
    /// The CA05 worknote's calibrated mapping: `bw = (ue.Y, ue.Z, ue.X) / 100`.
    pub const CA05: AxisMapping = AxisMapping {
        bw: [
            AxisSource::new(1, false),
            AxisSource::new(2, false),
            AxisSource::new(0, false),
        ],
    };

    /// What NavBuilder's `loadOBJ` swizzle produces when handed an OBJ
    /// written in raw UE3 cm: `bw = (ue.Z, ue.Y, ue.X) / 100`. BigWorld's
    /// up axis ends up sourced from UE3's horizontal Y.
    pub const NAVBUILDER_ON_RAW_UE3: AxisMapping = AxisMapping {
        bw: [
            AxisSource::new(2, false),
            AxisSource::new(1, false),
            AxisSource::new(0, false),
        ],
    };

    /// Map one UE3-cm point into BigWorld units.
    pub fn apply(&self, ue3_cm: [f32; 3]) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for (i, src) in self.bw.iter().enumerate() {
            let v = ue3_cm[src.ue3_axis as usize] / CM_PER_BW_UNIT;
            out[i] = if src.negate { -v } else { v };
        }
        out
    }

    /// Compact label, e.g. `+Y+Z+X` for [`AxisMapping::CA05`] — read as
    /// "BigWorld x from +UE3 Y, BigWorld y from +UE3 Z, BigWorld z from
    /// +UE3 X".
    pub fn label(&self) -> String {
        const AXIS: [char; 3] = ['X', 'Y', 'Z'];
        let mut s = String::with_capacity(6);
        for src in &self.bw {
            s.push(if src.negate { '-' } else { '+' });
            s.push(AXIS[src.ue3_axis as usize]);
        }
        s
    }

    /// Parse a label produced by [`AxisMapping::label`]. Returns `None`
    /// for a malformed string or a non-permutation (repeated axis).
    pub fn from_label(label: &str) -> Option<Self> {
        let chars: Vec<char> = label.trim().chars().collect();
        if chars.len() != 6 {
            return None;
        }
        let mut bw = [AxisSource::new(0, false); 3];
        let mut seen = [false; 3];
        for i in 0..3 {
            let negate = match chars[i * 2] {
                '+' => false,
                '-' => true,
                _ => return None,
            };
            let axis = match chars[i * 2 + 1].to_ascii_uppercase() {
                'X' => 0u8,
                'Y' => 1,
                'Z' => 2,
                _ => return None,
            };
            if seen[axis as usize] {
                return None;
            }
            seen[axis as usize] = true;
            bw[i] = AxisSource::new(axis, negate);
        }
        Some(Self { bw })
    }

    /// All 48 candidates: 6 axis permutations × 8 sign combinations.
    pub fn all() -> Vec<AxisMapping> {
        const PERMS: [[u8; 3]; 6] = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let mut out = Vec::with_capacity(48);
        for perm in PERMS {
            for signs in 0u8..8 {
                out.push(AxisMapping {
                    bw: [
                        AxisSource::new(perm[0], signs & 1 != 0),
                        AxisSource::new(perm[1], signs & 2 != 0),
                        AxisSource::new(perm[2], signs & 4 != 0),
                    ],
                });
            }
        }
        out
    }
}
