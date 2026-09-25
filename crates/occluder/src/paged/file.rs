//! The paged `.occ` file.
//!
//! ```text
//!   0   [u8; 4]  magic "CMOP"
//!   4   u16      version (1)
//!   6   u16      reserved (0)
//!   8   f32      page edge, metres
//!  12   i32 i32  page coordinates of page 0 (x, z); page (x, z) covers
//!                [x * edge, (x + 1) * edge) in world metres
//!  20   u32 u32  pages across x, z
//!  28   u64      source hash (FNV-1a 64 over the input triangles)
//!  36   u16      label length, then that many UTF-8 bytes
//!       u32      page count
//!       per page, 16 bytes: u32 index (row-major by z), u32 offset (from the
//!                start of the blob area), u32 length, u32 unpacked RAM bytes
//!       blob area: each page is a complete single-page `.occ`
//!                (`crate::format`, magic "CMOC") over that page's tiles
//! ```
//!
//! A page is unpacked by decoding its blob; nothing else in the file needs
//! decompressing to load it.

use crate::format::OccluderError;
use crate::grid::fnv1a64;

use super::PageGrid;

/// Paged file magic.
pub const PAGED_MAGIC: [u8; 4] = *b"CMOP";
/// Paged format version.
pub const PAGED_VERSION: u16 = 1;

const MAX_PAGES: u64 = 1 << 20;

/// One page's blob in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageRef {
    pub offset: u32,
    pub len: u32,
    pub ram_bytes: u32,
}

/// A parsed paged file: the grid, the page table and the raw bytes the
/// blobs live in.
#[derive(Debug)]
pub(crate) struct PagedFile {
    pub grid: PageGrid,
    pub source_hash: u64,
    pub label: String,
    /// `(index, blob)`, sorted by index.
    pub pages: Vec<(u32, PageRef)>,
    pub blob_start: usize,
    pub bytes: Vec<u8>,
    pub content_hash: u64,
}

impl PagedFile {
    pub fn blob(&self, r: &PageRef) -> &[u8] {
        let a = self.blob_start + r.offset as usize;
        &self.bytes[a..a + r.len as usize]
    }
}

/// Encode a page grid and its encoded page blobs.
pub(crate) fn encode(
    grid: &PageGrid,
    source_hash: u64,
    label: &str,
    pages: &[(u32, Vec<u8>, usize)],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&PAGED_MAGIC);
    out.extend_from_slice(&PAGED_VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&grid.size.to_le_bytes());
    out.extend_from_slice(&grid.px0.to_le_bytes());
    out.extend_from_slice(&grid.pz0.to_le_bytes());
    out.extend_from_slice(&grid.nx.to_le_bytes());
    out.extend_from_slice(&grid.nz.to_le_bytes());
    out.extend_from_slice(&source_hash.to_le_bytes());
    let label = label.as_bytes();
    let label = &label[..label.len().min(u16::MAX as usize)];
    out.extend_from_slice(&(label.len() as u16).to_le_bytes());
    out.extend_from_slice(label);
    out.extend_from_slice(&(pages.len() as u32).to_le_bytes());
    let mut offset = 0u32;
    for (idx, blob, ram) in pages {
        out.extend_from_slice(&idx.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        out.extend_from_slice(&(*ram as u32).to_le_bytes());
        offset += blob.len() as u32;
    }
    for (_, blob, _) in pages {
        out.extend_from_slice(blob);
    }
    out
}

fn rd<const N: usize>(b: &[u8], at: &mut usize) -> Result<[u8; N], OccluderError> {
    let end = at
        .checked_add(N)
        .filter(|&e| e <= b.len())
        .ok_or(OccluderError::Malformed("truncated"))?;
    let mut out = [0u8; N];
    out.copy_from_slice(&b[*at..end]);
    *at = end;
    Ok(out)
}

/// Parse a paged file's header and page table; the blobs stay packed.
pub(crate) fn parse(bytes: Vec<u8>) -> Result<PagedFile, OccluderError> {
    let b = &bytes;
    let mut at = 0usize;
    if rd::<4>(b, &mut at)? != PAGED_MAGIC {
        return Err(OccluderError::BadMagic);
    }
    let version = u16::from_le_bytes(rd(b, &mut at)?);
    if version != PAGED_VERSION {
        return Err(OccluderError::Version(version));
    }
    rd::<2>(b, &mut at)?;
    let size = f32::from_le_bytes(rd(b, &mut at)?);
    let px0 = i32::from_le_bytes(rd(b, &mut at)?);
    let pz0 = i32::from_le_bytes(rd(b, &mut at)?);
    let nx = u32::from_le_bytes(rd(b, &mut at)?);
    let nz = u32::from_le_bytes(rd(b, &mut at)?);
    if !(size.is_finite() && size > 0.0) || nx == 0 || nz == 0 || nx as u64 * nz as u64 > MAX_PAGES
    {
        return Err(OccluderError::Malformed("page grid"));
    }
    let source_hash = u64::from_le_bytes(rd(b, &mut at)?);
    let label_len = u16::from_le_bytes(rd(b, &mut at)?) as usize;
    let label_end = at
        .checked_add(label_len)
        .filter(|&e| e <= b.len())
        .ok_or(OccluderError::Malformed("truncated"))?;
    let label = String::from_utf8(b[at..label_end].to_vec())
        .map_err(|_| OccluderError::Malformed("label is not UTF-8"))?;
    at = label_end;
    let n = u32::from_le_bytes(rd(b, &mut at)?) as u64;
    if n > nx as u64 * nz as u64 {
        return Err(OccluderError::Malformed("page count"));
    }
    let mut pages = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let idx = u32::from_le_bytes(rd(b, &mut at)?);
        let r = PageRef {
            offset: u32::from_le_bytes(rd(b, &mut at)?),
            len: u32::from_le_bytes(rd(b, &mut at)?),
            ram_bytes: u32::from_le_bytes(rd(b, &mut at)?),
        };
        if idx as u64 >= nx as u64 * nz as u64 {
            return Err(OccluderError::Malformed("page index"));
        }
        if pages
            .last()
            .is_some_and(|&(p, _): &(u32, PageRef)| p >= idx)
        {
            return Err(OccluderError::Malformed("page order"));
        }
        pages.push((idx, r));
    }
    let blob_start = at;
    let blob_len = b.len() - blob_start;
    let mut expect = 0u64;
    for (_, r) in &pages {
        if r.offset as u64 != expect {
            return Err(OccluderError::Malformed("page offset"));
        }
        expect += r.len as u64;
    }
    if expect != blob_len as u64 {
        return Err(OccluderError::Malformed("blob area length"));
    }
    let content_hash = fnv1a64(&bytes);
    Ok(PagedFile {
        grid: PageGrid {
            size,
            px0,
            pz0,
            nx,
            nz,
        },
        source_hash,
        label,
        pages,
        blob_start,
        bytes,
        content_hash,
    })
}
