//! MSZIP block decoding.
//!
//! Each MSZIP `CFDATA` block is `CK` followed by a complete raw deflate
//! stream that inflates to at most 32 KiB. Blocks are not independent: a
//! back-reference may reach up to 32 KiB into the output of earlier blocks of
//! the same folder, so the decoder keeps that much history and inflates each
//! block into a buffer that starts with it.

use miniz_oxide::inflate::core::{decompress, inflate_flags, DecompressorOxide};
use miniz_oxide::inflate::TINFLStatus;

/// Deflate's window, and MSZIP's largest uncompressed block.
pub(super) const WINDOW: usize = 32 * 1024;

pub(super) struct Mszip {
    inflater: Box<DecompressorOxide>,
    /// The history (at most [`WINDOW`] bytes) followed by the last block.
    buf: Vec<u8>,
}

impl Mszip {
    pub fn new() -> Self {
        Self {
            inflater: Box::default(),
            buf: Vec::with_capacity(2 * WINDOW),
        }
    }

    /// Inflate one block that must produce exactly `uncompressed` bytes.
    pub fn block(&mut self, data: &[u8], uncompressed: usize) -> Result<&[u8], String> {
        if uncompressed > WINDOW {
            return Err(format!("an MSZIP block claims {uncompressed} bytes"));
        }
        let Some(stream) = data.strip_prefix(b"CK") else {
            return Err("an MSZIP block lacks its CK signature".into());
        };
        // Keep only the last WINDOW bytes of output as history.
        let keep = self.buf.len().min(WINDOW);
        self.buf.drain(..self.buf.len() - keep);
        self.buf.resize(keep + uncompressed, 0);
        self.inflater.init();
        let (status, _read, written) = decompress(
            &mut self.inflater,
            stream,
            &mut self.buf,
            keep,
            inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
        );
        match status {
            TINFLStatus::Done if written == uncompressed => Ok(&self.buf[keep..]),
            TINFLStatus::Done => Err(format!(
                "an MSZIP block inflated to {written} bytes, not {uncompressed}"
            )),
            TINFLStatus::HasMoreOutput => Err(format!(
                "an MSZIP block inflates to more than {uncompressed} bytes"
            )),
            other => Err(format!("an MSZIP block does not inflate ({other:?})")),
        }
    }
}
