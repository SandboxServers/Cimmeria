//! Names for bitflag words in log lines (named telemetry, NT-31).
//!
//! A flag word logged as an integer (`state_flags=72`) sends the reader to
//! the enum to decode it by hand. Each flag set declares one [`FlagSet`]
//! table next to its constants, and a log site pairs the raw word with its
//! names under the same key plus `_names`:
//!
//! ```
//! use cimmeria_common::flag_names::FlagSet;
//!
//! const DEMO: FlagSet = FlagSet::new(&[(1, "A_First"), (8, "A_Fourth")]);
//! let flags: u32 = 0x29;
//! assert_eq!(DEMO.render(flags).to_string(), "A_First|A_Fourth|0x20");
//! // tracing::debug!(flags, flags_names = %DEMO.render(flags), "...");
//! ```
//!
//! The rendering is `A|B|C` in table order. Bits no entry names render
//! once, as a hex remainder at the end (`0x20`), so an unexpected bit shows
//! instead of vanishing. A zero word renders `0x0`. The name string is
//! rendered only when the event is enabled; an exporter that stores fields
//! as strings (the OTLP appender) still allocates one per exported event,
//! so keep `_names` fields off rows exported per packet or per tick.

use std::fmt;

/// A flag set's names: `(mask, name)` pairs in the order they render.
///
/// A mask is usually one bit. A multi-bit mask (the client's `EMailFlags`
/// ships two, 4092 and 8196) is an alias: a word exactly equal to it renders
/// as that one name. Otherwise a multi-bit mask names the word only when
/// every one of its bits is set, beside the single bits it overlaps. An
/// entry with mask 0 never matches.
#[derive(Debug, Clone, Copy)]
pub struct FlagSet {
    names: &'static [(u64, &'static str)],
}

impl FlagSet {
    pub const fn new(names: &'static [(u64, &'static str)]) -> Self {
        Self { names }
    }

    /// The `(mask, name)` table, for tests that pin it to the client's
    /// declaration.
    pub fn entries(&self) -> &'static [(u64, &'static str)] {
        self.names
    }

    /// The names of `bits`, for a `%` log field.
    pub fn render(&self, bits: impl FlagBits) -> FlagNames {
        FlagNames {
            set: *self,
            bits: bits.flag_bits(),
        }
    }
}

/// A flag word as raw bits. Signed words are reinterpreted, never
/// sign-extended, so an `i32` with bit 31 set reads as `0x80000000` and not
/// as 32 extra high bits.
pub trait FlagBits: Copy {
    fn flag_bits(self) -> u64;
}

macro_rules! flag_bits_unsigned {
    ($($t:ty),*) => {$(
        impl FlagBits for $t {
            fn flag_bits(self) -> u64 {
                u64::from(self)
            }
        }
    )*};
}
flag_bits_unsigned!(u8, u16, u32, u64);

macro_rules! flag_bits_signed {
    ($($t:ty => $u:ty),*) => {$(
        impl FlagBits for $t {
            fn flag_bits(self) -> u64 {
                u64::from(self as $u)
            }
        }
    )*};
}
flag_bits_signed!(i8 => u8, i16 => u16, i32 => u32, i64 => u64);

/// The `Display` form of one flag word; see [`FlagSet::render`].
#[derive(Debug, Clone, Copy)]
pub struct FlagNames {
    set: FlagSet,
    bits: u64,
}

impl fmt::Display for FlagNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bits == 0 {
            return f.write_str("0x0");
        }
        // An exact multi-bit alias is one value, not a set of single bits:
        // `EMailFlags` 4092 is `MAIL_ToCommandRank6`, not ten recipients.
        if let Some(&(_, name)) = self
            .set
            .names
            .iter()
            .find(|&&(mask, _)| mask == self.bits && mask.count_ones() > 1)
        {
            return f.write_str(name);
        }
        let mut named = 0u64;
        let mut first = true;
        for &(mask, name) in self.set.names {
            if mask != 0 && self.bits & mask == mask {
                if !first {
                    f.write_str("|")?;
                }
                f.write_str(name)?;
                named |= mask;
                first = false;
            }
        }
        let rest = self.bits & !named;
        if rest != 0 {
            if !first {
                f.write_str("|")?;
            }
            write!(f, "{rest:#x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: FlagSet = FlagSet::new(&[(1, "A"), (2, "B"), (8, "D"), (0, "NEVER")]);

    #[test]
    fn known_bits_render_in_table_order() {
        assert_eq!(SET.render(1u32).to_string(), "A");
        assert_eq!(SET.render(0b1011u32).to_string(), "A|B|D");
    }

    #[test]
    fn unknown_bits_render_as_one_hex_remainder() {
        assert_eq!(SET.render(0x4u8).to_string(), "0x4");
        assert_eq!(SET.render(0x31u32).to_string(), "A|0x30");
    }

    #[test]
    fn zero_renders_hex_zero() {
        assert_eq!(SET.render(0u64).to_string(), "0x0");
    }

    #[test]
    fn signed_words_are_not_sign_extended() {
        assert_eq!(SET.render(i32::MIN).to_string(), "0x80000000");
        assert_eq!(SET.render(-1i8).to_string(), "A|B|D|0xf4");
        assert_eq!(SET.render(i64::MIN).to_string(), "0x8000000000000000");
    }

    #[test]
    fn multi_bit_mask_needs_every_bit() {
        const MULTI: FlagSet = FlagSet::new(&[(4, "V"), (0b1100, "VT")]);
        assert_eq!(MULTI.render(4u32).to_string(), "V");
        assert_eq!(MULTI.render(0b1100u32).to_string(), "VT");
        assert_eq!(MULTI.render(0b1101u32).to_string(), "V|VT|0x1");
        assert_eq!(MULTI.render(0b1000u32).to_string(), "0x8");
    }
}
