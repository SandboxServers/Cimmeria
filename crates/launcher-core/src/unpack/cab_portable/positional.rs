//! Reads at an explicit offset. Folders expanding on several threads share
//! each cabinet's open handle, so no reader may depend on the handle's
//! cursor: every read names its offset.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

/// Fill `buf` from `offset` of `file`.
#[cfg(unix)]
pub(super) fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

/// Fill `buf` from `offset` of `file`. Windows has no `read_exact_at`, and
/// `seek_read` moves the handle's cursor, which is harmless: nothing in the
/// expand path reads through the cursor.
#[cfg(windows)]
pub(super) fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        let n = file.seek_read(buf, offset)?;
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        buf = &mut buf[n..];
        offset += n as u64;
    }
    Ok(())
}

/// A `Read` and `Seek` cursor over a shared handle, with its own position.
/// `len` is the file's length: reads stop there rather than fail, as a
/// `BufReader` over the file would see it.
pub(super) struct PositionalReader<'a> {
    file: &'a File,
    pos: u64,
    len: u64,
}

impl<'a> PositionalReader<'a> {
    pub fn new(file: &'a File, len: u64, pos: u64) -> Self {
        Self { file, pos, len }
    }
}

impl Read for PositionalReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = usize::try_from(self.len.saturating_sub(self.pos)).unwrap_or(usize::MAX);
        let n = left.min(buf.len());
        read_exact_at(self.file, &mut buf[..n], self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for PositionalReader<'_> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let next = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
        };
        let next = next
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before the start"))?;
        self.pos = next;
        Ok(next)
    }
}
