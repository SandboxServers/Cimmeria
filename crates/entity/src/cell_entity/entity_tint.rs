//! The colours an NPC template sends in `onEntityTint` (SGWBeing method 10).
//!
//! The method's three arguments are named `primaryColorId`,
//! `secondaryColorId` and `skinColorId`, but none of them is an id: the
//! client's handler (`GameEntity_ApplySkinTintColors`, `0x00e6f8b0`, reading
//! those three names from the event at `GameEntity.cpp:0x194-0x196`) unpacks
//! each `UINT32` as a packed `0xRRGGBB__` colour. It takes R, G and B from
//! bits 24, 16 and 8, drops the low byte, and forces alpha to 0xFF
//! (`docs/protocol/client-method-dispatch-table.md`, "onEntityTint").
//!
//! The seed stores the three in `bigint` columns
//! (`entity_templates.primary_color_id`, `secondary_color_id`, `skin_tint`),
//! and a colour with the top bit set (R >= 0x80) was saved as its signed
//! 32-bit value: skin tint -52773120 is `0xFCDABF00`, a pale skin tone. So
//! the wire value is the two's-complement low 32 bits. (The legacy Python
//! loader negated negative values instead, `EntityTemplate.py:35-43`, which
//! turns that skin tone into `0x03254100`, a dark blue.)
//!
//! Only a template that opts in with `entity_templates.send_tint` sends its
//! colours; every other NPC keeps `onEntityTint(0, 0, 0)`.

/// The three packed `0xRRGGBB__` colours of `onEntityTint`, in wire order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityTint {
    /// `primaryColorId`.
    pub primary: u32,
    /// `secondaryColorId`.
    pub secondary: u32,
    /// `skinColorId`.
    pub skin: u32,
}

impl EntityTint {
    /// The tint from the three `entity_templates` colour columns, or the
    /// first value that is not a 32-bit colour. A value in `i32` range is
    /// reinterpreted as its two's-complement `u32` (how the seed stores a
    /// colour with the top bit set); a value in `u32` range is taken as is.
    pub fn from_template_columns(primary: i64, secondary: i64, skin: i64) -> Result<Self, i64> {
        Ok(Self {
            primary: column_colour(primary)?,
            secondary: column_colour(secondary)?,
            skin: column_colour(skin)?,
        })
    }

    /// The 12 argument bytes of `onEntityTint`: three little-endian `u32`s.
    pub fn wire_args(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[0..4].copy_from_slice(&self.primary.to_le_bytes());
        out[4..8].copy_from_slice(&self.secondary.to_le_bytes());
        out[8..12].copy_from_slice(&self.skin.to_le_bytes());
        out
    }
}

/// One `bigint` colour column as its wire `u32`, or the value back if it
/// fits neither `i32` nor `u32`.
fn column_colour(value: i64) -> Result<u32, i64> {
    u32::try_from(value)
        .or_else(|_| i32::try_from(value).map(|v| v as u32))
        .map_err(|_| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seed's negative values are signed 32-bit colours: -52773120 is
    /// the pale skin `0xFCDABF00`, -627017216 is `0xDAA07A00`, -65536 is
    /// yellow `0xFFFF0000`. Positive values pass through. Revert proof:
    /// negate instead (the legacy Python loader) and the skin reads
    /// `0x03254100`.
    #[test]
    fn negative_columns_are_twos_complement_colours() {
        let t = EntityTint::from_template_columns(-65536, 16711680, -52773120).unwrap();
        assert_eq!(t.primary, 0xFFFF_0000);
        assert_eq!(t.secondary, 0x00FF_0000);
        assert_eq!(t.skin, 0xFCDA_BF00);
        let t = EntityTint::from_template_columns(0, 224256, -627017216).unwrap();
        assert_eq!((t.secondary, t.skin), (0x0003_6C00, 0xDAA0_7A00));
        let t = EntityTint::from_template_columns(0, 0, 2001679616).unwrap();
        assert_eq!(t.skin, 0x774F_3500);
        assert_eq!(
            EntityTint::from_template_columns(0, 0, u32::MAX as i64)
                .unwrap()
                .skin,
            u32::MAX
        );
    }

    /// A value outside both 32-bit ranges is refused, not truncated.
    #[test]
    fn a_value_wider_than_32_bits_is_refused() {
        assert_eq!(
            EntityTint::from_template_columns(0, 1 << 32, 0),
            Err(1 << 32)
        );
        assert_eq!(
            EntityTint::from_template_columns(i32::MIN as i64 - 1, 0, 0),
            Err(i32::MIN as i64 - 1)
        );
    }

    /// The wire args are primary, secondary, skin, each little-endian.
    #[test]
    fn wire_args_are_three_le_u32s_in_order() {
        let t = EntityTint {
            primary: 0xFFFF_0000,
            secondary: 0xFF00_0000,
            skin: 0xDAA0_7A00,
        };
        assert_eq!(
            t.wire_args(),
            [0, 0, 0xFF, 0xFF, 0, 0, 0, 0xFF, 0, 0x7A, 0xA0, 0xDA]
        );
    }
}
