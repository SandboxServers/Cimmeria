//! Independent uncompressed CAB fixture for the real Windows FDI adapter.
//! This builds archive bytes, never an extractor or an executable game.
pub(super) fn cabinet(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut files = Vec::new();
    let mut data = Vec::new();
    for (name, bytes) in entries {
        files.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        files.extend_from_slice(&(data.len() as u32).to_le_bytes());
        files.extend_from_slice(&0u16.to_le_bytes()); // folder index
        files.extend_from_slice(&crate::unpack::test_fixtures::FIXTURE_DOS_DATE.to_le_bytes());
        files.extend_from_slice(&crate::unpack::test_fixtures::FIXTURE_DOS_TIME.to_le_bytes());
        files.extend_from_slice(&0x20u16.to_le_bytes());
        files.extend_from_slice(name.replace('/', "\\").as_bytes());
        files.push(0);
        data.extend_from_slice(bytes);
    }
    let mut blocks = Vec::new();
    for block in data.chunks(32768) {
        blocks.extend_from_slice(&0u32.to_le_bytes()); // checksum omitted by format
        blocks.extend_from_slice(&(block.len() as u16).to_le_bytes());
        blocks.extend_from_slice(&(block.len() as u16).to_le_bytes());
        blocks.extend_from_slice(block);
    }
    let mut out = b"MSCF".to_vec();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((44 + files.len() + blocks.len()) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&44u32.to_le_bytes()); // CFFILE offset
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&[3, 1]); // cabinet version 1.3
    out.extend_from_slice(&1u16.to_le_bytes()); // one folder
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // no continuation/reserved fields
    out.extend_from_slice(&123u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // cabinet index
    out.extend_from_slice(&((44 + files.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(data.len().div_ceil(32768) as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // uncompressed folder
    out.extend_from_slice(&files);
    out.extend_from_slice(&blocks);
    out
}
