//! Wire types for the Black Market / Auction House (`SGWBlackMarket`).
//!
//! These mirror the recovered client emitters in `SGW.exe`. Names use STRING
//! (4-byte LE length prefix + N UTF-8 bytes), **not** WSTRING/UTF-16 — unlike
//! most other SGW social systems. See the restoration findings doc for the
//! evidence chain.

/// Search filters sent by the client's `BMSearch` (emitter `FUN_00e59f70`).
///
/// Fields are listed in **wire packing order**. The 11th field, `filter_flags`,
/// was recovered from the emitter at `puVar1+0x15`; an older doc mislabelled
/// that slot `monikerCRC`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BMSearchOptions {
    /// Sort column selector (`EBlackMarketSortType`).
    pub sort_id: u8,
    /// Opaque pagination token echoed back to the client in results.
    pub client_key: i32,
    /// Pagination cursor (last-seen auction sequence id).
    pub sequence_id: i32,
    /// Non-zero = paginate forward, zero = backward.
    pub b_forward: u8,
    /// Seller-name filter (empty = no filter).
    pub seller_name: String,
    /// Bidder-name filter (empty = no filter).
    pub bidder_name: String,
    /// Item-name substring filter (empty = no filter).
    pub item_name: String,
    /// Minimum trade-credits price.
    pub min_tc: i32,
    /// Maximum trade-credits price.
    pub max_tc: i32,
    /// Item-quality filter (`EItemQuality`).
    pub quality: i32,
    /// Category / faction / mode filter bitfield (`EBlackMarketFilter`).
    pub filter_flags: i32,
}

/// Read a 4-byte LE length prefix + N UTF-8 bytes starting at `buf[0]`.
///
/// Returns the decoded string and the total number of bytes consumed
/// (4 + length). Returns `None` if the buffer is too short for the prefix or
/// the declared body, or if the body is not valid UTF-8.
fn read_string(buf: &[u8]) -> Option<(String, usize)> {
    if buf.len() < 4 {
        return None;
    }
    let len = u32::from_le_bytes(buf[0..4].try_into().ok()?) as usize;
    let end = 4usize.checked_add(len)?;
    if buf.len() < end {
        return None;
    }
    let s = std::str::from_utf8(&buf[4..end]).ok()?.to_owned();
    Some((s, end))
}

/// Read a 4-byte LE `i32` starting at `buf[off]`, advancing `off`.
fn read_i32(buf: &[u8], off: &mut usize) -> Option<i32> {
    let end = off.checked_add(4)?;
    if buf.len() < end {
        return None;
    }
    let v = i32::from_le_bytes(buf[*off..end].try_into().ok()?);
    *off = end;
    Some(v)
}

/// Read a single `u8` starting at `buf[off]`, advancing `off`.
fn read_u8(buf: &[u8], off: &mut usize) -> Option<u8> {
    let v = *buf.get(*off)?;
    *off += 1;
    Some(v)
}

impl BMSearchOptions {
    /// Deserialize the 11-field `BMSearchOptions` from its wire encoding.
    ///
    /// Wire order: `UINT8 sortId, INT32 clientKey, INT32 sequenceId,
    /// UINT8 bForward, STRING sellerName, STRING bidderName, STRING itemName,
    /// INT32 minTC, INT32 maxTC, INT32 quality, INT32 filterFlags`.
    ///
    /// Returns `None` on any truncation or invalid-UTF-8 string body.
    pub fn from_wire(buf: &[u8]) -> Option<Self> {
        let mut off = 0usize;

        let sort_id = read_u8(buf, &mut off)?;
        let client_key = read_i32(buf, &mut off)?;
        let sequence_id = read_i32(buf, &mut off)?;
        let b_forward = read_u8(buf, &mut off)?;

        let (seller_name, n) = read_string(&buf[off..])?;
        off += n;
        let (bidder_name, n) = read_string(&buf[off..])?;
        off += n;
        let (item_name, n) = read_string(&buf[off..])?;
        off += n;

        let min_tc = read_i32(buf, &mut off)?;
        let max_tc = read_i32(buf, &mut off)?;
        let quality = read_i32(buf, &mut off)?;
        let filter_flags = read_i32(buf, &mut off)?;

        Some(BMSearchOptions {
            sort_id,
            client_key,
            sequence_id,
            b_forward,
            seller_name,
            bidder_name,
            item_name,
            min_tc,
            max_tc,
            quality,
            filter_flags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: pack a STRING (4B LE len prefix + UTF-8 body).
    fn push_string(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }

    /// Build a full, valid 11-field `BMSearchOptions` wire buffer.
    fn build_search_wire(opts: &BMSearchOptions) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(opts.sort_id);
        buf.extend_from_slice(&opts.client_key.to_le_bytes());
        buf.extend_from_slice(&opts.sequence_id.to_le_bytes());
        buf.push(opts.b_forward);
        push_string(&mut buf, &opts.seller_name);
        push_string(&mut buf, &opts.bidder_name);
        push_string(&mut buf, &opts.item_name);
        buf.extend_from_slice(&opts.min_tc.to_le_bytes());
        buf.extend_from_slice(&opts.max_tc.to_le_bytes());
        buf.extend_from_slice(&opts.quality.to_le_bytes());
        buf.extend_from_slice(&opts.filter_flags.to_le_bytes());
        buf
    }

    /// Round-trip all 11 fields, including non-empty strings and the
    /// `filter_flags` 11th INT32. Guards against any field being dropped or
    /// reordered.
    #[test]
    fn search_options_round_trip_all_eleven_fields() {
        let expected = BMSearchOptions {
            sort_id: 2,
            client_key: 0x1111_2222,
            sequence_id: 0x3333_4444,
            b_forward: 1,
            seller_name: "Sheppard".to_string(),
            bidder_name: "McKay".to_string(),
            item_name: "Naquadah".to_string(),
            min_tc: 100,
            max_tc: 50_000,
            quality: 3,
            filter_flags: 0x0F0F_0F0F,
        };

        let wire = build_search_wire(&expected);
        let decoded = BMSearchOptions::from_wire(&wire).expect("should deserialize");

        assert_eq!(decoded.sort_id, 2);
        assert_eq!(decoded.client_key, 0x1111_2222);
        assert_eq!(decoded.sequence_id, 0x3333_4444);
        assert_eq!(decoded.b_forward, 1);
        assert_eq!(decoded.seller_name, "Sheppard");
        assert_eq!(decoded.bidder_name, "McKay");
        assert_eq!(decoded.item_name, "Naquadah");
        assert_eq!(decoded.min_tc, 100);
        assert_eq!(decoded.max_tc, 50_000);
        assert_eq!(decoded.quality, 3);
        // The 11th INT32. If a deserializer stops at 10 fields (the stale
        // `monikerCRC`-in-slot-10 layout), this assert fails.
        assert_eq!(decoded.filter_flags, 0x0F0F_0F0F);

        assert_eq!(decoded, expected);
    }

    /// Names are STRING (UTF-8), not WSTRING (UTF-16LE). A WSTRING reader would
    /// interpret the 1-byte-per-char UTF-8 body as 2-bytes-per-char and either
    /// mangle the text or over-read into the trailing INT32s. Pin the exact
    /// byte length to catch a WSTRING misread.
    #[test]
    fn search_options_strings_are_utf8_not_wstring() {
        let opts = BMSearchOptions {
            sort_id: 0,
            client_key: 7,
            sequence_id: 9,
            b_forward: 0,
            seller_name: "ABCD".to_string(),
            bidder_name: String::new(),
            item_name: String::new(),
            min_tc: -1,
            max_tc: -1,
            quality: 0,
            filter_flags: 42,
        };
        let wire = build_search_wire(&opts);

        // Fixed scalars: 1 + 4 + 4 + 1 = 10; three STRINGs: (4+4)+(4+0)+(4+0)=16;
        // trailing four INT32s: 16. UTF-8 "ABCD" is 4 bytes, not 8.
        assert_eq!(wire.len(), 10 + 16 + 16);

        let decoded = BMSearchOptions::from_wire(&wire).expect("should deserialize");
        assert_eq!(decoded.seller_name, "ABCD");
        assert_eq!(decoded.bidder_name, "");
        assert_eq!(decoded.item_name, "");
        // If seller_name had been read as WSTRING the cursor would be off by
        // 4 bytes and filter_flags would not land on 42.
        assert_eq!(decoded.filter_flags, 42);
    }

    /// Truncated buffers must fail cleanly, not panic.
    #[test]
    fn search_options_truncated_returns_none() {
        assert!(BMSearchOptions::from_wire(&[]).is_none());
        // Header present but strings/trailers missing.
        assert!(BMSearchOptions::from_wire(&[0u8; 10]).is_none());
        // A declared string length that overruns the buffer.
        let mut bad = vec![0u8; 10];
        bad.extend_from_slice(&1000u32.to_le_bytes()); // seller_name len = 1000
        assert!(BMSearchOptions::from_wire(&bad).is_none());
    }
}
