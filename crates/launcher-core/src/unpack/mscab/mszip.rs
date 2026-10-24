//! MSZIP block decoding.
//!
//! Each MSZIP `CFDATA` block is `CK` followed by a raw deflate stream that
//! inflates to at most 32 KiB. Blocks are not independent: a back-reference
//! may reach up to 32 KiB into the output of the blocks before it in the same
//! logical folder. So the decoder keeps the last block's output as a
//! dictionary and inflates each block with it. The decoder is `flate2`, with
//! its `zlib-rs` backend, because `set_dictionary` needs that backend.

use flate2::{Decompress, FlushDecompress, Status};

use crate::unpack::UnpackError;

/// MSZIP's largest uncompressed block, and deflate's window.
pub(in crate::unpack) const BLOCK_MAX: usize = 32_768;

/// The inflater and the output of the last block of the current folder.
pub(in crate::unpack) struct MsZip {
    /// The last block's output, used as the dictionary of the next block.
    history: Vec<u8>,
    inflater: Decompress,
}

impl MsZip {
    pub(super) fn new() -> Self {
        Self {
            history: Vec::new(),
            inflater: Decompress::new(false),
        }
    }

    /// Forget the history. Call this at the start of each logical folder.
    pub(super) fn reset(&mut self) {
        self.history.clear();
        self.inflater.reset(false);
    }

    /// Inflate one block into `out`, which the caller has cleared. The block
    /// must inflate to exactly `uncompressed_len` bytes.
    pub(super) fn block(
        &mut self,
        data: &[u8],
        uncompressed_len: usize,
        out: &mut Vec<u8>,
    ) -> Result<(), UnpackError> {
        if uncompressed_len > BLOCK_MAX {
            return Err(cab_error(format!(
                "MSZIP block claims {uncompressed_len} bytes"
            )));
        }
        let Some(stream) = data.strip_prefix(b"CK") else {
            return Err(cab_error("MSZIP block without CK".into()));
        };
        self.inflater.reset(false);
        if !self.history.is_empty() {
            self.inflater
                .set_dictionary(&self.history)
                .map_err(|e| cab_error(format!("MSZIP: {e}")))?;
        }
        out.reserve(uncompressed_len);
        let status = self
            .inflater
            .decompress_vec(stream, out, FlushDecompress::Finish)
            .map_err(|e| cab_error(format!("MSZIP: {e}")))?;
        // A stream that needs more room than `uncompressed_len` stops short of
        // StreamEnd, so this also catches output that would be cut off.
        if status != Status::StreamEnd {
            return Err(cab_error("MSZIP block does not end where it should".into()));
        }
        if out.len() != uncompressed_len {
            return Err(cab_error(format!(
                "MSZIP block inflated to {}, expected {uncompressed_len}",
                out.len()
            )));
        }
        self.history.clear();
        self.history.extend_from_slice(out);
        Ok(())
    }
}

fn cab_error(message: String) -> UnpackError {
    UnpackError::Cab(message)
}

#[cfg(test)]
mod tests {
    use flate2::{Compress, Compression, FlushCompress};

    use super::*;

    /// A raw deflate stream of `data`, optionally using `dict` as history,
    /// with the `CK` signature in front.
    fn ck_block(data: &[u8], dict: Option<&[u8]>) -> Vec<u8> {
        let mut deflate = Compress::new(Compression::best(), false);
        if let Some(dict) = dict {
            deflate.set_dictionary(dict).unwrap();
        }
        // compress_vec writes only into spare capacity, so leave room for it.
        let mut out = Vec::with_capacity(2 * data.len() + 64);
        out.extend_from_slice(b"CK");
        deflate
            .compress_vec(data, &mut out, FlushCompress::Finish)
            .unwrap();
        out
    }

    /// Pseudo-random bytes, so a block can only shrink by matching history.
    fn noise(len: usize, seed: u32) -> Vec<u8> {
        let mut state = seed;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect()
    }

    fn inflate(mszip: &mut MsZip, block: &[u8], len: usize) -> Result<Vec<u8>, UnpackError> {
        let mut out = Vec::new();
        mszip.block(block, len, &mut out)?;
        Ok(out)
    }

    #[test]
    fn two_blocks_where_the_second_repeats_the_first_round_trip() {
        // Half a block, so the repeat sits within deflate's reach of the window.
        let first = noise(BLOCK_MAX / 2, 1);
        let second = first.clone();
        let mut mszip = MsZip::new();
        let got = inflate(&mut mszip, &ck_block(&first, None), first.len()).unwrap();
        assert_eq!(got, first);
        let block = ck_block(&second, Some(&first));
        // The second block only fits in a small stream because it refers back.
        assert!(
            block.len() < 1000,
            "the second block has {} bytes",
            block.len()
        );
        let got = inflate(&mut mszip, &block, second.len()).unwrap();
        assert_eq!(got, second);
    }

    #[test]
    fn the_second_block_fails_or_differs_after_reset_drops_the_history() {
        let first = noise(BLOCK_MAX / 2, 2);
        let second = first.clone();
        let block = ck_block(&second, Some(&first));
        let mut mszip = MsZip::new();
        inflate(&mut mszip, &ck_block(&first, None), first.len()).unwrap();
        mszip.reset();
        let result = inflate(&mut mszip, &block, second.len());
        assert!(!matches!(result, Ok(out) if out == second));
    }

    #[test]
    fn a_missing_ck_signature_is_a_cab_error() {
        let mut mszip = MsZip::new();
        let mut block = ck_block(b"hello", None);
        block[0] = b'X';
        assert!(matches!(
            inflate(&mut mszip, &block, 5),
            Err(UnpackError::Cab(msg)) if msg.contains("CK")
        ));
    }

    #[test]
    fn a_wrong_uncompressed_length_is_a_cab_error() {
        let mut mszip = MsZip::new();
        let block = ck_block(b"hello world", None);
        assert!(inflate(&mut mszip, &block, 11 - 1).is_err());
        assert!(inflate(&mut mszip, &block, 11 + 1).is_err());
    }

    #[test]
    fn a_length_over_the_block_maximum_is_refused() {
        let mut mszip = MsZip::new();
        let block = ck_block(b"hello", None);
        assert!(inflate(&mut mszip, &block, BLOCK_MAX + 1).is_err());
    }
}
