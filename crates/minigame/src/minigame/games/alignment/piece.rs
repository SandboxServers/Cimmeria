//! One board cell of the Alignment board: a piece type and its power-up.

/// One board cell: a piece type (0 empty, 1-8 icons, 9 blocker) and a power-up
/// power 0-6 (0 = none). Spec section 3: `t + 0.p` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Piece {
    pub(crate) kind: u8,
    pub(crate) power: u8,
}

impl Piece {
    /// A piece with no power-up.
    pub(crate) const fn plain(kind: u8) -> Self {
        Self { kind, power: 0 }
    }

    /// The wire number, `kind + power / 10`.
    pub(crate) fn wire(self) -> f64 {
        f64::from(self.kind) + f64::from(self.power) / 10.0
    }
}
