//! A MakeCAB-shaped cabinet set writer for tests on every host.
//!
//! `makecab.exe` exists only on Windows, so this writes `DATA*.CAB` and
//! `DATA.INF` itself. The MSZIP blocks come from a small fixed-Huffman deflate
//! encoder whose matches reach back into earlier blocks, as real MSZIP does:
//! a decoder that drops history between blocks or cabinets fails on them.
//! A folder that crosses a cabinet boundary can have its last block split
//! across the two cabinets, and carries on only with the file that crosses
//! it, as MakeCAB writes them. Every block but a folder's last is 32 KiB.

use std::collections::HashMap;
use std::path::Path;

use super::super::test_fixtures::{FIXTURE_DOS_DATE, FIXTURE_DOS_TIME};
use super::format::{CONTINUED_FROM_PREV, CONTINUED_PREV_AND_NEXT, CONTINUED_TO_NEXT};
use super::mszip::WINDOW;

#[derive(Debug, Clone, Copy)]
pub struct CabOptions {
    /// Data blocks per cabinet before the set moves on to the next one.
    pub blocks_per_cab: usize,
    /// Start a new folder before each file whose index is a multiple of this
    /// (0: one folder for everything).
    pub files_per_folder: usize,
    /// `typeCompress`: 0 stored, 1 MSZIP, 3 LZX (written, never decodable).
    pub compression: u16,
    /// Reserve sizes (header, per folder, per data block), written with
    /// `cfhdrRESERVE_PRESENT` (the header's as zeros, the others as `0xA5`);
    /// `None` writes none.
    pub reserve: Option<(u16, u8, u8)>,
}

impl Default for CabOptions {
    fn default() -> Self {
        Self {
            blocks_per_cab: 2,
            files_per_folder: 0,
            compression: 1,
            reserve: None,
        }
    }
}

/// What the writer produced, for tests to assert the fixture's shape.
#[derive(Debug)]
pub struct CabSetInfo {
    pub cabinets: Vec<String>,
    /// Back-references that reach into an earlier block.
    pub cross_block_matches: usize,
}

struct Block {
    folder: usize,
    /// Where the block's output starts in its folder's stream.
    start: u64,
    /// The block's whole output, also for either part of a split block.
    size: usize,
    data: Vec<u8>,
    /// `cbUncomp` as written: 0 for the first part of a split block.
    uncompressed: usize,
    /// The second part of a split block, opening the next cabinet.
    tail: bool,
}

/// What one cabinet holds of one folder: where the blocks that start in it
/// begin and end (`None` when it holds only a split block's tail), and the
/// data it completes.
#[derive(Clone, Copy)]
struct Share {
    starts: Option<(u64, u64)>,
    data: (u64, u64),
}

/// The blocks dealt into cabinets, each file's (folder, start, end), what each
/// cabinet covers of each folder's stream, and the cross-block match count.
struct Layout {
    cabs: Vec<Vec<Block>>,
    ranges: Vec<(usize, u64, u64)>,
    covered: Vec<HashMap<usize, Share>>,
    cross_block_matches: usize,
}

fn layout(files: &[(&str, Vec<u8>)], folders: &[Vec<usize>], opts: CabOptions) -> Layout {
    let mut ranges = vec![(0, 0u64, 0u64); files.len()];
    let mut blocks = Vec::new();
    let mut cross_block_matches = 0;
    for (f, members) in folders.iter().enumerate() {
        let mut stream = Vec::new();
        for &i in members {
            let start = stream.len() as u64;
            stream.extend_from_slice(&files[i].1);
            ranges[i] = (f, start, stream.len() as u64);
        }
        let mut encoder = Encoder::default();
        // Every block but a folder's last holds exactly 32 KiB: Windows' FDI
        // and 7-Zip refuse a folder of shorter blocks, which libmspack and
        // the portable decoder would accept.
        for chunk_start in (0..stream.len().max(1)).step_by(WINDOW) {
            let chunk_end = (chunk_start + WINDOW).min(stream.len());
            let data = match opts.compression {
                1 => encoder.block(&stream, chunk_start, chunk_end),
                _ => stream[chunk_start..chunk_end].to_vec(),
            };
            blocks.push(Block {
                folder: f,
                start: chunk_start as u64,
                size: chunk_end - chunk_start,
                data,
                uncompressed: chunk_end - chunk_start,
                tail: false,
            });
        }
        cross_block_matches += encoder.cross_block_matches;
    }

    // Deal blocks out to cabinets.
    let mut cabs: Vec<Vec<Block>> = Vec::new();
    for block in blocks {
        if cabs.last().is_none_or(|c| c.len() >= opts.blocks_per_cab) {
            cabs.push(Vec::new());
        }
        cabs.last_mut().unwrap().push(block);
    }
    // Split the last block of each cabinet whose folder carries on: the
    // first part stays with an uncompressed size of 0, and the rest, with
    // the block's real size, opens the next cabinet. libmspack and FDI both
    // count a carried-on folder's blocks that way, so MakeCAB always splits.
    for c in 0..cabs.len().saturating_sub(1) {
        let next_folder = cabs[c + 1][0].folder;
        let last = cabs[c].last_mut().unwrap();
        if last.folder != next_folder {
            continue;
        }
        assert!(last.data.len() >= 2, "fixture: a block too small to split");
        let tail = Block {
            folder: last.folder,
            start: last.start,
            size: last.size,
            data: last.data.split_off(last.data.len() / 2),
            uncompressed: std::mem::replace(&mut last.uncompressed, 0),
            tail: true,
        };
        cabs[c + 1].insert(0, tail);
    }

    let mut covered: Vec<HashMap<usize, Share>> = Vec::new();
    for cab in &cabs {
        let mut map: HashMap<usize, Share> = HashMap::new();
        for block in cab {
            let end = block.start + block.size as u64;
            let share = map.entry(block.folder).or_insert(Share {
                starts: None,
                data: (block.start, block.start),
            });
            if !block.tail {
                let lo = share.starts.map_or(block.start, |(lo, _)| lo);
                share.starts = Some((lo, end));
            }
            if block.uncompressed > 0 {
                share.data.1 = end;
            }
        }
        covered.push(map);
    }
    Layout {
        cabs,
        ranges,
        covered,
        cross_block_matches,
    }
}

/// MakeCAB's rule for a folder that reaches a cabinet boundary: it carries
/// on into the next cabinet only as far as the file that crosses the
/// boundary, and the next file starts a new folder (the 2009 seed's DATA2
/// and DATA3 each open with such a one-file folder). Windows' FDI refuses
/// more files in the continued part. Returns the folder grouping with that
/// break added at the first boundary that needs one, or `None` when none does.
fn break_at_boundary(folders: &[Vec<usize>], l: &Layout) -> Option<Vec<Vec<usize>>> {
    for c in 0..l.cabs.len().saturating_sub(1) {
        let f = l.cabs[c].last().unwrap().folder;
        if l.cabs[c + 1][0].folder != f {
            continue;
        }
        // Files that start in the next cabinet's own blocks don't belong in
        // the carried-on part. Zero-length ones need no data, so they never
        // make a folder of their own (an empty folder's only block would read
        // as the first half of a split block).
        let Some((_, hi)) = l.covered[c][&f].starts else {
            continue;
        };
        let members = &folders[f];
        let Some(keep) = members.iter().position(|&i| l.ranges[i].1 >= hi) else {
            continue;
        };
        if members[keep..]
            .iter()
            .any(|&i| l.ranges[i].2 > l.ranges[i].1)
        {
            let mut out = folders.to_vec();
            let rest = out[f].split_off(keep);
            out.insert(f + 1, rest);
            return Some(out);
        }
    }
    None
}

/// Write `DATA1.CAB`.. and `DATA.INF` for `files` into `dir`.
pub fn write_cab_set(dir: &Path, files: &[(&str, Vec<u8>)], opts: CabOptions) -> CabSetInfo {
    // Folders: file indices in each folder's stream.
    let mut folders: Vec<Vec<usize>> = Vec::new();
    for i in 0..files.len() {
        if folders.is_empty() || (opts.files_per_folder > 0 && i % opts.files_per_folder == 0) {
            folders.push(Vec::new());
        }
        folders.last_mut().unwrap().push(i);
    }
    let mut l = layout(files, &folders, opts);
    while let Some(split) = break_at_boundary(&folders, &l) {
        folders = split;
        l = layout(files, &folders, opts);
    }
    let Layout {
        cabs,
        ranges,
        covered,
        cross_block_matches,
    } = l;

    let names: Vec<String> = (1..=cabs.len()).map(|n| format!("DATA{n}.CAB")).collect();
    for (c, cab) in cabs.iter().enumerate() {
        let mut cab_folders: Vec<usize> = Vec::new();
        for block in cab {
            if cab_folders.last() != Some(&block.folder) {
                cab_folders.push(block.folder);
            }
        }
        let mut listed = Vec::new();
        for (i, (name, data)) in files.iter().enumerate() {
            let (f, start, end) = ranges[i];
            let Some(&share) = covered[c].get(&f) else {
                continue;
            };
            let last_cab_of_folder = covered[c + 1..].iter().all(|m| !m.contains_key(&f));
            // A file is listed where its data starts, and again in each
            // cabinet its data reaches; a split block's tail belongs to the
            // cabinet the block started in.
            let (starts_here, before) = match share.starts {
                Some((lo, hi)) => (
                    (start >= lo && start < hi) || (start == hi && last_cab_of_folder),
                    start < lo,
                ),
                None => (false, true),
            };
            let (data_lo, data_hi) = share.data;
            let after = end > data_hi;
            if !(starts_here || (before && end > data_lo)) {
                continue;
            }
            let slot = cab_folders.iter().position(|&x| x == f).unwrap() as u16;
            let i_folder = match (before, after) {
                (true, true) => CONTINUED_PREV_AND_NEXT,
                (true, false) => CONTINUED_FROM_PREV,
                (false, true) => CONTINUED_TO_NEXT,
                (false, false) => slot,
            };
            listed.push((name.to_string(), data.len() as u32, start as u32, i_folder));
        }
        // MakeCAB moves to the next cabinet mid-file, so a folder that
        // carries on always has a file marked as carrying on with it.
        let carries_on = c + 1 < cabs.len() && cabs[c + 1][0].folder == cab.last().unwrap().folder;
        let marked = listed
            .iter()
            .any(|f| matches!(f.3, CONTINUED_TO_NEXT | CONTINUED_PREV_AND_NEXT));
        assert_eq!(
            carries_on, marked,
            "fixture: cabinet {c} must break inside a file"
        );
        write_cabinet(
            &dir.join(&names[c]),
            c,
            cabs.len(),
            &names,
            &cab_folders,
            cab,
            &listed,
            opts.compression,
            opts.reserve,
        );
    }

    let mut inf = String::from("[disk list]\r\nDisk 1, Disc_1\r\n[cabinet list]\r\n");
    for (c, name) in names.iter().enumerate() {
        inf.push_str(&format!("Disk 1, Cabinet {}, {name}\r\n", c + 1));
    }
    inf.push_str("[file list]\r\n");
    for (i, (name, data)) in files.iter().enumerate() {
        inf.push_str(&format!("{}: Cabinet 1, {name}, {}\r\n", i + 1, data.len()));
    }
    std::fs::write(dir.join("DATA.INF"), inf).unwrap();
    CabSetInfo {
        cabinets: names,
        cross_block_matches,
    }
}

#[allow(clippy::too_many_arguments)]
fn write_cabinet(
    path: &Path,
    index: usize,
    count: usize,
    names: &[String],
    folders: &[usize],
    blocks: &[Block],
    files: &[(String, u32, u32, u16)],
    compression: u16,
    reserve: Option<(u16, u8, u8)>,
) {
    let mut flags = 0u16;
    // cbCFHeader, cbCFFolder and cbCFData, then the header's reserve, all
    // before the previous/next cabinet names.
    let mut reserved = Vec::new();
    let (folder_reserve, data_reserve) = match reserve {
        Some((header, folder, data)) => {
            flags |= 4;
            reserved.extend_from_slice(&header.to_le_bytes());
            reserved.push(folder);
            reserved.push(data);
            // Zeros, as reserve areas usually are: a parser that failed to
            // skip them would read them as empty cabinet names and lose its
            // place, where non-zero filler would merge into the next name.
            reserved.resize(4 + usize::from(header), 0);
            (usize::from(folder), usize::from(data))
        }
        None => (0, 0),
    };
    let mut links = Vec::new();
    if index > 0 {
        flags |= 1;
        links.extend_from_slice(names[index - 1].as_bytes());
        links.extend_from_slice(b"\0Disc_1\0");
    }
    if index + 1 < count {
        flags |= 2;
        links.extend_from_slice(names[index + 1].as_bytes());
        links.extend_from_slice(b"\0Disc_1\0");
    }
    let header_len = 36 + reserved.len() + links.len();
    let folders_len = (8 + folder_reserve) * folders.len();
    let files_offset = header_len + folders_len;
    let mut file_table = Vec::new();
    for (name, size, offset, i_folder) in files {
        file_table.extend_from_slice(&size.to_le_bytes());
        file_table.extend_from_slice(&offset.to_le_bytes());
        file_table.extend_from_slice(&i_folder.to_le_bytes());
        file_table.extend_from_slice(&FIXTURE_DOS_DATE.to_le_bytes());
        file_table.extend_from_slice(&FIXTURE_DOS_TIME.to_le_bytes());
        file_table.extend_from_slice(&0x20u16.to_le_bytes());
        file_table.extend_from_slice(name.as_bytes());
        file_table.push(0);
    }
    let mut data = Vec::new();
    let mut folder_table = Vec::new();
    let data_start = files_offset + file_table.len();
    for &f in folders {
        let offset = data_start + data.len();
        let mut n = 0u16;
        for block in blocks.iter().filter(|b| b.folder == f) {
            data.extend_from_slice(&0u32.to_le_bytes()); // no checksum
            data.extend_from_slice(&(block.data.len() as u16).to_le_bytes());
            data.extend_from_slice(&(block.uncompressed as u16).to_le_bytes());
            data.resize(data.len() + data_reserve, 0xA5);
            data.extend_from_slice(&block.data);
            n += 1;
        }
        folder_table.extend_from_slice(&(offset as u32).to_le_bytes());
        folder_table.extend_from_slice(&n.to_le_bytes());
        folder_table.extend_from_slice(&compression.to_le_bytes());
        folder_table.resize(folder_table.len() + folder_reserve, 0xA5);
    }
    let total = data_start + data.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"MSCF");
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(files_offset as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&[3, 1]); // version 1.3
    out.extend_from_slice(&(folders.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&0x1234u16.to_le_bytes()); // setID
    out.extend_from_slice(&(index as u16).to_le_bytes());
    out.extend_from_slice(&reserved);
    out.extend_from_slice(&links);
    out.extend_from_slice(&folder_table);
    out.extend_from_slice(&file_table);
    out.extend_from_slice(&data);
    std::fs::write(path, out).unwrap();
}

/// Fixed-Huffman deflate with greedy matching over the whole folder stream,
/// so matches cross block boundaries (MSZIP keeps a 32 KiB history).
#[derive(Default)]
struct Encoder {
    last_seen: HashMap<[u8; 3], usize>,
    cross_block_matches: usize,
}

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

impl Encoder {
    /// `CK` plus one final fixed-Huffman deflate block for `stream[from..to]`.
    fn block(&mut self, stream: &[u8], from: usize, to: usize) -> Vec<u8> {
        let mut bits = BitWriter::default();
        bits.put(1, 1); // BFINAL
        bits.put(1, 2); // BTYPE = fixed Huffman
        let mut pos = from;
        let mut matches = 0;
        while pos < to {
            let mut best = (0usize, 0usize);
            if pos + 3 <= to {
                let key = [stream[pos], stream[pos + 1], stream[pos + 2]];
                if let Some(&prev) = self.last_seen.get(&key) {
                    let dist = pos - prev;
                    if dist <= WINDOW {
                        let mut len = 0;
                        while len < 258 && pos + len < to && stream[prev + len] == stream[pos + len]
                        {
                            len += 1;
                        }
                        best = (len, dist);
                    }
                }
            }
            let step = if best.0 >= 3 {
                if pos - best.1 < from {
                    self.cross_block_matches += 1;
                    matches += 1;
                }
                emit_match(&mut bits, best.0, best.1);
                best.0
            } else {
                emit_literal(&mut bits, stream[pos]);
                1
            };
            for p in pos..pos + step {
                if p + 3 <= stream.len() {
                    self.last_seen
                        .insert([stream[p], stream[p + 1], stream[p + 2]], p);
                }
            }
            pos += step;
        }
        emit_code(&mut bits, 256);
        let mut out = b"CK".to_vec();
        out.extend(bits.finish());
        // Like MakeCAB, store a block that does not compress: MSZIP allows at
        // most 32 KiB + 12 bytes per block, which FDI and 7-Zip enforce.
        if out.len() > 2 + 5 + (to - from) {
            self.cross_block_matches -= matches;
            out.truncate(2);
            let len = (to - from) as u16;
            out.push(0b001); // BFINAL, BTYPE 00 (stored), then byte-aligned
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&(!len).to_le_bytes());
            out.extend_from_slice(&stream[from..to]);
        }
        out
    }
}

fn emit_code(bits: &mut BitWriter, sym: u16) {
    let (code, len) = match sym {
        0..=143 => (0x30 + sym, 8),
        144..=255 => (0x190 + sym - 144, 9),
        256..=279 => (sym - 256, 7),
        _ => (0xC0 + sym - 280, 8),
    };
    bits.put_reversed(u32::from(code), len);
}

fn emit_literal(bits: &mut BitWriter, byte: u8) {
    emit_code(bits, u16::from(byte));
}

fn emit_match(bits: &mut BitWriter, len: usize, dist: usize) {
    let li = LENGTH_BASE
        .iter()
        .rposition(|&b| usize::from(b) <= len)
        .unwrap();
    emit_code(bits, 257 + li as u16);
    bits.put(
        (len - usize::from(LENGTH_BASE[li])) as u32,
        LENGTH_EXTRA[li],
    );
    let di = DIST_BASE
        .iter()
        .rposition(|&b| usize::from(b) <= dist)
        .unwrap();
    bits.put_reversed(di as u32, 5);
    bits.put((dist - usize::from(DIST_BASE[di])) as u32, DIST_EXTRA[di]);
}

#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u8,
}

impl BitWriter {
    /// Write `len` bits of `value`, least significant first.
    fn put(&mut self, value: u32, len: u8) {
        for i in 0..len {
            self.acc |= ((value >> i) & 1) << self.n;
            self.n += 1;
            if self.n == 8 {
                self.out.push(self.acc as u8);
                self.acc = 0;
                self.n = 0;
            }
        }
    }

    /// Write a Huffman code, which deflate stores most significant bit first.
    fn put_reversed(&mut self, code: u32, len: u8) {
        for i in (0..len).rev() {
            self.put((code >> i) & 1, 1);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}
