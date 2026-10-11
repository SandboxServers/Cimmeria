//! Wire decoders for the five vendor cell methods (78-82).
//!
//! Per `docs/reverse-engineering/findings/inventory-wire-formats.md` §Store
//! Operations (verified against the stock client 2026-10-10), every method
//! carries `ARRAY<INT32>` arguments only, each a `u32` count then bare
//! little-endian `i32`s:
//!
//! - `purchaseItems` (78): `ItemIndices`, `Quantities`
//! - `sellItems` (79), `buybackItems` (80): `ItemIds`, `Quantities`
//! - `repairItems` (81), `rechargeItems` (82): `ItemIds`
//!
//! The two-array methods are parallel arrays, not `(id, qty)` pairs, and no
//! method sends a vendor template id: the vendor comes from the server's own
//! session. The old pair-stream reader took a stock purchase
//! `[1][idx][1][qty]` as one pair `(idx, 1)` plus a trailing "template id"
//! `qty`, and refused every real request as a template mismatch.

/// Why a vendor payload was refused. [`WireReject::reason`] is the `reason`
/// field of the `malformed_args` refusal row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WireReject {
    /// Fewer than four bytes left where an array count belongs.
    TruncatedCount,
    /// The count promises more elements than the payload holds.
    CountExceedsPayload,
    /// Bytes left over after the last array.
    TrailingBytes,
    /// The id and quantity arrays differ in length.
    LengthMismatch,
    /// A quantity of zero or less. The client resolves `-1` to the stack
    /// count before sending, so a stock client never sends one.
    NonPositiveQuantity,
}

impl WireReject {
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::TruncatedCount => "truncated_count",
            Self::CountExceedsPayload => "count_exceeds_payload",
            Self::TrailingBytes => "trailing_bytes",
            Self::LengthMismatch => "length_mismatch",
            Self::NonPositiveQuantity => "non_positive_quantity",
        }
    }
}

/// One `ARRAY<INT32>` at `*offset`.
fn read_i32_array(args: &[u8], offset: &mut usize) -> Result<Vec<i32>, WireReject> {
    let Some(prefix) = args.get(*offset..*offset + 4) else {
        return Err(WireReject::TruncatedCount);
    };
    let count = u32::from_le_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]) as usize;
    *offset += 4;

    // Bound the count against the remaining bytes BEFORE allocating: a forged
    // count of u32::MAX would otherwise reserve ~16 GiB up front.
    let remaining = args.len() - *offset;
    if count > remaining / 4 {
        return Err(WireReject::CountExceedsPayload);
    }
    let values = args[*offset..*offset + count * 4]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| i32::from_le_bytes(*c))
        .collect();
    *offset += count * 4;
    Ok(values)
}

fn expect_end(args: &[u8], offset: usize) -> Result<(), WireReject> {
    if offset == args.len() {
        Ok(())
    } else {
        Err(WireReject::TrailingBytes)
    }
}

/// `repairItems` / `rechargeItems`: `u32 n, n x i32 ItemIds`.
pub(super) fn read_item_ids(args: &[u8]) -> Result<Vec<i32>, WireReject> {
    let mut offset = 0;
    let ids = read_i32_array(args, &mut offset)?;
    expect_end(args, offset)?;
    Ok(ids)
}

/// `purchaseItems` / `sellItems` / `buybackItems`:
/// `u32 n, n x i32 ids, u32 m, m x i32 Quantities`, zipped into
/// `(id, quantity)` lines. `n == m` and every quantity is positive.
pub(super) fn read_id_quantity_lines(args: &[u8]) -> Result<Vec<(i32, i32)>, WireReject> {
    let mut offset = 0;
    let ids = read_i32_array(args, &mut offset)?;
    let quantities = read_i32_array(args, &mut offset)?;
    expect_end(args, offset)?;
    if ids.len() != quantities.len() {
        return Err(WireReject::LengthMismatch);
    }
    if quantities.iter().any(|&q| q <= 0) {
        return Err(WireReject::NonPositiveQuantity);
    }
    Ok(ids.into_iter().zip(quantities).collect())
}

#[cfg(test)]
mod tests {
    //! Byte-exact payloads built from the documented layouts. Each one is
    //! something the old `(id, qty)`-pair-plus-trailing-template parser got
    //! wrong: it either misread the lines or refused a valid request.
    use super::*;

    /// Encode one `ARRAY<INT32>`.
    fn array(values: &[i32]) -> Vec<u8> {
        let mut out = (values.len() as u32).to_le_bytes().to_vec();
        for v in values {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    fn two_arrays(ids: &[i32], quantities: &[i32]) -> Vec<u8> {
        let mut out = array(ids);
        out.extend(array(quantities));
        out
    }

    /// The stock right-click buy: row 4, one unit. 16 bytes, spelled out so
    /// the layout is pinned independently of the `array` helper.
    #[test]
    fn stock_purchase_of_one_line_parses_both_arrays() {
        let bytes = [
            0x01, 0x00, 0x00, 0x00, // n = 1
            0x04, 0x00, 0x00, 0x00, // ItemIndices[0] = 4
            0x01, 0x00, 0x00, 0x00, // m = 1
            0x05, 0x00, 0x00, 0x00, // Quantities[0] = 5
        ];
        assert_eq!(read_id_quantity_lines(&bytes), Ok(vec![(4, 5)]));
    }

    /// The multi-select buy: three rows, element i of Quantities belongs to
    /// element i of ItemIndices.
    #[test]
    fn stock_purchase_of_three_lines_pairs_by_position() {
        let bytes = two_arrays(&[0, 7, 12], &[1, 20, 3]);
        assert_eq!(bytes.len(), 4 + 12 + 4 + 12);
        assert_eq!(
            read_id_quantity_lines(&bytes),
            Ok(vec![(0, 1), (7, 20), (12, 3)])
        );
    }

    #[test]
    fn stock_sell_parses_item_ids_and_quantities() {
        let bytes = two_arrays(&[0x0123_4567], &[10]);
        assert_eq!(read_id_quantity_lines(&bytes), Ok(vec![(0x0123_4567, 10)]));
    }

    #[test]
    fn stock_buyback_of_two_lines_parses() {
        let bytes = two_arrays(&[901, 902], &[1, 4]);
        assert_eq!(read_id_quantity_lines(&bytes), Ok(vec![(901, 1), (902, 4)]));
    }

    /// A one-item repair is 8 bytes: the old pair reader needed 12 and
    /// refused it as malformed.
    #[test]
    fn stock_repair_of_one_item_parses() {
        let bytes = [
            0x01, 0x00, 0x00, 0x00, // n = 1
            0x2A, 0x00, 0x00, 0x00, // ItemIds[0] = 42
        ];
        assert_eq!(read_item_ids(&bytes), Ok(vec![42]));
    }

    #[test]
    fn stock_repair_of_three_items_parses() {
        assert_eq!(read_item_ids(&array(&[42, 43, 44])), Ok(vec![42, 43, 44]));
    }

    #[test]
    fn stock_recharge_parses() {
        assert_eq!(read_item_ids(&array(&[77, 78])), Ok(vec![77, 78]));
    }

    #[test]
    fn empty_arrays_parse_to_no_lines() {
        assert_eq!(read_item_ids(&array(&[])), Ok(vec![]));
        assert_eq!(read_id_quantity_lines(&two_arrays(&[], &[])), Ok(vec![]));
    }

    #[test]
    fn short_count_prefix_is_truncated_count() {
        assert_eq!(read_item_ids(&[1, 0, 0]), Err(WireReject::TruncatedCount));
        // The second array's count is missing.
        assert_eq!(
            read_id_quantity_lines(&array(&[4])),
            Err(WireReject::TruncatedCount)
        );
    }

    #[test]
    fn count_beyond_the_payload_is_refused_without_allocating() {
        let mut bytes = u32::MAX.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0; 4]);
        assert_eq!(read_item_ids(&bytes), Err(WireReject::CountExceedsPayload));
        // A count one past the elements present.
        let mut bytes = 2u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&9i32.to_le_bytes());
        assert_eq!(read_item_ids(&bytes), Err(WireReject::CountExceedsPayload));
    }

    /// The old parser read a trailing i32 as the vendor template id; the
    /// stock client never sends one.
    #[test]
    fn trailing_bytes_are_refused() {
        let mut bytes = two_arrays(&[4], &[1]);
        bytes.extend_from_slice(&31i32.to_le_bytes());
        assert_eq!(
            read_id_quantity_lines(&bytes),
            Err(WireReject::TrailingBytes)
        );
        let mut bytes = array(&[42]);
        bytes.push(0);
        assert_eq!(read_item_ids(&bytes), Err(WireReject::TrailingBytes));
    }

    #[test]
    fn mismatched_array_lengths_are_refused() {
        assert_eq!(
            read_id_quantity_lines(&two_arrays(&[1, 2], &[1])),
            Err(WireReject::LengthMismatch)
        );
    }

    #[test]
    fn non_positive_quantities_are_refused() {
        for q in [0, -1] {
            assert_eq!(
                read_id_quantity_lines(&two_arrays(&[1], &[q])),
                Err(WireReject::NonPositiveQuantity),
                "quantity {q}"
            );
        }
    }

    #[test]
    fn reasons_are_stable_telemetry_values() {
        assert_eq!(
            WireReject::CountExceedsPayload.reason(),
            "count_exceeds_payload"
        );
        assert_eq!(WireReject::TrailingBytes.reason(), "trailing_bytes");
        assert_eq!(WireReject::LengthMismatch.reason(), "length_mismatch");
    }
}
