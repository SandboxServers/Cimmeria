use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read, Seek, SeekFrom};

use crate::error::Result;

/// Binary reader with UE3-specific type support.
pub struct BinaryReader<R: Read + Seek> {
    inner: R,
}

impl<R: Read + Seek> BinaryReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn position(&mut self) -> io::Result<u64> {
        self.inner.stream_position()
    }

    pub fn seek(&mut self, pos: u64) -> io::Result<u64> {
        self.inner.seek(SeekFrom::Start(pos))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(self.inner.read_i32::<LittleEndian>()?)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(self.inner.read_u32::<LittleEndian>()?)
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        Ok(self.inner.read_u64::<LittleEndian>()?)
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        Ok(self.inner.read_f32::<LittleEndian>()?)
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; count];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read a UE3 FString: i32 length, then `abs(length)` chars.
    /// Positive length = ASCII (with null terminator), negative = UTF-16LE.
    pub fn read_fstring(&mut self) -> Result<String> {
        let length = self.read_i32()?;
        if length == 0 {
            return Ok(String::new());
        }
        if length < 0 {
            // UTF-16LE
            let char_count = (-length) as usize;
            let raw = self.read_bytes(char_count * 2)?;
            let chars: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&chars)
                .trim_end_matches('\0')
                .to_string())
        } else {
            let raw = self.read_bytes(length as usize)?;
            // Strip null terminator
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            Ok(String::from_utf8_lossy(&raw[..end]).to_string())
        }
    }

    /// Read an FName reference: (name_index: i32, instance_number: i32).
    pub fn read_fname(&mut self) -> Result<(i32, i32)> {
        let idx = self.read_i32()?;
        let num = self.read_i32()?;
        Ok((idx, num))
    }

    /// Get the underlying reader (for decompression).
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Get a mutable reference to the underlying reader.
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}
