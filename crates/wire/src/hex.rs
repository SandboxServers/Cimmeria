//! Hex dumps of packet bytes for trace logs.
//!
//! The base's `UDP_OUT` traces and the wire firehose (`UDP_IN`,
//! `DECRYPT_OK`) print packets through this one formatter, so every hex
//! column in the logs reads the same. `cimmeria-services` re-exports it as
//! `base::helpers::to_hex`.

/// Format a byte slice as a hex string for trace logging.
pub fn to_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `to_hex` formats each byte as two uppercase hex digits, separated
    /// by single spaces. Pin the format so a refactor that swaps to
    /// lowercase or drops the separator doesn't silently change every
    /// trace log.
    #[test]
    fn to_hex_formats_bytes_as_uppercase_with_space_separator() {
        assert_eq!(to_hex(&[]), "");
        assert_eq!(to_hex(&[0x00]), "00");
        assert_eq!(to_hex(&[0xAB, 0xCD]), "AB CD");
        assert_eq!(
            to_hex(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]),
            "12 34 56 78 9A BC DE F0"
        );
    }

    /// `to_hex` zero-pads single-digit byte values. A regression that
    /// drops the `:02X` width specifier would emit "1 2" for [0x01, 0x02]
    /// instead of "01 02", breaking deterministic log diffs.
    #[test]
    fn to_hex_zero_pads_single_digit_bytes() {
        assert_eq!(to_hex(&[0x01, 0x02, 0x0F]), "01 02 0F");
    }
}
