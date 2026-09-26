//! Wire decoders for the vendor request payloads: the `ARRAY<(item_id,
//! quantity)>` item list and the optional trailing vendor-template id.

pub(super) fn read_i32_array(args: &[u8], offset: &mut usize) -> Option<Vec<(i32, i32)>> {
    if args.len() < *offset + 4 {
        return None;
    }
    let count = u32::from_le_bytes([
        args[*offset],
        args[*offset + 1],
        args[*offset + 2],
        args[*offset + 3],
    ]) as usize;
    *offset += 4;

    // Bound count against remaining bytes BEFORE allocating: a malformed packet
    // with count=u32::MAX would otherwise reserve ~32 GiB up-front and OOM.
    let remaining = args.len().saturating_sub(*offset);
    if count.saturating_mul(8) > remaining {
        return None;
    }

    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        if args.len() < *offset + 8 {
            return None;
        }
        let item_id = i32::from_le_bytes([
            args[*offset],
            args[*offset + 1],
            args[*offset + 2],
            args[*offset + 3],
        ]);
        let quantity = i32::from_le_bytes([
            args[*offset + 4],
            args[*offset + 5],
            args[*offset + 6],
            args[*offset + 7],
        ]);
        *offset += 8;
        items.push((item_id, quantity));
    }
    Some(items)
}

pub(super) fn read_trailing_template_id(args: &[u8], offset: usize) -> Option<i32> {
    if args.len() >= offset + 4 {
        Some(i32::from_le_bytes([
            args[offset],
            args[offset + 1],
            args[offset + 2],
            args[offset + 3],
        ]))
    } else {
        None
    }
}

#[cfg(test)]
mod read_i32_array_tests {
    use super::read_i32_array;

    #[test]
    fn empty_array_returns_empty_vec() {
        let buf = 0u32.to_le_bytes();
        let mut off = 0;
        assert_eq!(read_i32_array(&buf, &mut off), Some(vec![]));
        assert_eq!(off, 4, "offset must advance past the count prefix");
    }

    #[test]
    fn single_pair_parses_in_order() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&12345i32.to_le_bytes());
        buf.extend_from_slice(&67i32.to_le_bytes());
        let mut off = 0;
        assert_eq!(read_i32_array(&buf, &mut off), Some(vec![(12345, 67)]));
        assert_eq!(off, 12);
    }

    #[test]
    fn multiple_pairs_preserve_input_order() {
        let pairs: [(i32, i32); 3] = [(10, 1), (20, 2), (30, 3)];
        let mut buf = Vec::new();
        buf.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
        for (id, q) in &pairs {
            buf.extend_from_slice(&id.to_le_bytes());
            buf.extend_from_slice(&q.to_le_bytes());
        }
        let mut off = 0;
        let parsed = read_i32_array(&buf, &mut off).expect("parse must succeed");
        assert_eq!(parsed, pairs.to_vec());
        assert_eq!(off, 4 + pairs.len() * 8);
    }

    #[test]
    fn returns_none_when_too_short_for_count_prefix() {
        let mut off = 0;
        assert!(read_i32_array(&[0, 1, 2], &mut off).is_none());
    }

    #[test]
    fn returns_none_when_payload_truncated_mid_pair() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&[0u8, 0u8]); // truncated 2nd pair
        let mut off = 0;
        assert!(read_i32_array(&buf, &mut off).is_none());
    }

    #[test]
    fn rejects_oversized_count_without_allocating() {
        // count = u32::MAX with only 4 bytes after the prefix would imply
        // ~32 GiB of payload. The function must reject up-front so a
        // malicious packet can't OOM the cell process via Vec::with_capacity.
        let mut buf = Vec::new();
        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        buf.extend_from_slice(&[0u8; 4]); // far less than u32::MAX * 8 bytes
        let mut off = 0;
        assert!(read_i32_array(&buf, &mut off).is_none());
    }

    #[test]
    fn parses_at_nonzero_offset() {
        // The function uses `*offset` for both bounds checks and indexing, so
        // a non-zero starting offset must work the same as a fresh slice.
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // junk prefix
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&7i32.to_le_bytes());
        buf.extend_from_slice(&8i32.to_le_bytes());
        let mut off = 4;
        assert_eq!(read_i32_array(&buf, &mut off), Some(vec![(7, 8)]));
        assert_eq!(off, 16);
    }
}
