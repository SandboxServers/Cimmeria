//! The pull-based byte source every decoder reads through.
//!
//! The client-patch DLL cannot hand the decoder a slice: the arguments sit
//! in the client's `BinaryIStream`, and the only safe way to read them is
//! `remainingLength()` followed by `retrieve(n)`. [`ByteSource`] is that
//! contract, so the same decoder runs over the live stream in `SGW.exe`,
//! over a `&[u8]` on the server ([`SliceSource`]), and over mock streams in
//! tests.

/// A source that could not supply bytes it was asked for, although it
/// reported them as remaining. A well-behaved source never returns this:
/// the decoders ask [`ByteSource::remaining`] first and only request what
/// is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceExhausted;

/// Pull-based reader for a message's argument bytes.
///
/// Decoders call [`remaining`](Self::remaining) before every
/// [`read_into`](Self::read_into) and never request more than it reported,
/// which is the rule the client's `BinaryIStream::retrieve` needs:
/// `retrieve` past the end does not fail cleanly.
pub trait ByteSource {
    /// Number of unread bytes left.
    fn remaining(&self) -> usize;

    /// Consume exactly `out.len()` bytes into `out`. Must either fill all of
    /// `out` or fail without a short read.
    fn read_into(&mut self, out: &mut [u8]) -> Result<(), SourceExhausted>;
}

impl<S: ByteSource + ?Sized> ByteSource for &mut S {
    fn remaining(&self) -> usize {
        (**self).remaining()
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<(), SourceExhausted> {
        (**self).read_into(out)
    }
}

/// A [`ByteSource`] over a byte slice.
#[derive(Debug, Clone)]
pub struct SliceSource<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> SliceSource<'a> {
    /// A source positioned at the start of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Bytes consumed so far.
    pub fn position(&self) -> usize {
        self.pos
    }
}

impl ByteSource for SliceSource<'_> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<(), SourceExhausted> {
        let end = self.pos.checked_add(out.len()).ok_or(SourceExhausted)?;
        let src = self.bytes.get(self.pos..end).ok_or(SourceExhausted)?;
        out.copy_from_slice(src);
        self.pos = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_source_reads_in_order_and_tracks_position() {
        let mut src = SliceSource::new(&[1, 2, 3, 4, 5]);
        let mut a = [0u8; 2];
        src.read_into(&mut a).unwrap();
        assert_eq!(a, [1, 2]);
        assert_eq!(src.position(), 2);
        assert_eq!(src.remaining(), 3);
        let mut b = [0u8; 3];
        src.read_into(&mut b).unwrap();
        assert_eq!(b, [3, 4, 5]);
        assert_eq!(src.remaining(), 0);
    }

    /// A read longer than what is left fails and consumes nothing, so a
    /// caller that ignores `remaining` still cannot desynchronise.
    #[test]
    fn slice_source_rejects_over_read_without_consuming() {
        let mut src = SliceSource::new(&[1, 2, 3]);
        let mut out = [0u8; 4];
        assert_eq!(src.read_into(&mut out), Err(SourceExhausted));
        assert_eq!(src.position(), 0, "a failed read must not advance");
        assert_eq!(src.remaining(), 3);
    }

    #[test]
    fn empty_read_always_succeeds() {
        let mut src = SliceSource::new(&[]);
        assert_eq!(src.read_into(&mut []), Ok(()));
    }
}
