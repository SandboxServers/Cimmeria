//! The chunked LZO container the cooked packages store bulk data in.
//!
//! ```text
//! u32 tag 0x9E2A83C1 | u32 block size | u32 total compressed | u32 total uncompressed
//! (u32 compressed, u32 uncompressed) per block, then the compressed blocks
//! ```
//!
//! Compression is `lzokay-native`, pinned to an exact version in `Cargo.toml`
//! because a patch result hash depends on its output.

use crate::error::{Result, UpkError};

const TAG: u32 = 0x9E2A_83C1;
/// The block size the stock packages use.
pub const BLOCK: usize = 0x20000;

fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| UpkError::Parse("LZO chunk header is cut short".into()))
}

/// Decompress a chunked payload.
pub fn unpack(payload: &[u8]) -> Result<Vec<u8>> {
    if u32_at(payload, 0)? != TAG {
        return Err(UpkError::Parse("LZO chunk tag is wrong".into()));
    }
    let block = u32_at(payload, 4)? as usize;
    let total = u32_at(payload, 12)? as usize;
    if block == 0 {
        return Err(UpkError::Parse("LZO chunk block size is zero".into()));
    }
    let blocks = total.div_ceil(block);
    let mut at = 16 + 8 * blocks;
    let mut out = Vec::with_capacity(total);
    for i in 0..blocks {
        let comp = u32_at(payload, 16 + 8 * i)? as usize;
        let uncomp = u32_at(payload, 20 + 8 * i)? as usize;
        let src = payload
            .get(at..at + comp)
            .ok_or_else(|| UpkError::Parse("LZO block runs past its payload".into()))?;
        out.extend(
            lzokay_native::decompress_all(src, Some(uncomp))
                .map_err(|e| UpkError::Parse(format!("LZO block {i}: {e}")))?,
        );
        at += comp;
    }
    Ok(out)
}

/// Compress `data` into a chunked payload.
pub fn pack(data: &[u8]) -> Result<Vec<u8>> {
    let mut table = Vec::new();
    let mut body = Vec::new();
    for part in data.chunks(BLOCK) {
        let c = lzokay_native::compress(part)
            .map_err(|e| UpkError::Parse(format!("LZO compress: {e}")))?;
        table.extend_from_slice(&(c.len() as u32).to_le_bytes());
        table.extend_from_slice(&(part.len() as u32).to_le_bytes());
        body.extend_from_slice(&c);
    }
    let mut out = Vec::with_capacity(16 + table.len() + body.len());
    out.extend_from_slice(&TAG.to_le_bytes());
    out.extend_from_slice(&(BLOCK as u32).to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&table);
    out.extend_from_slice(&body);
    Ok(out)
}
