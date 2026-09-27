//! The client's `BinaryIStream` as a [`ByteSource`], so the
//! `cimmeria-patch-wire` decoders read the arguments straight from it.

use core::ffi::c_void;

use cimmeria_patch_wire::{ByteSource, SourceExhausted};

use crate::addresses::{ISTREAM_REMAINING_LENGTH, ISTREAM_RETRIEVE};
use crate::memory::{is_readable, MemoryReader, ProcessMemory};

/// `const void* BinaryIStream::retrieve(int n)`: the next `n` bytes, consumed.
type RetrieveFn = unsafe extern "thiscall" fn(this: *mut c_void, n: i32) -> *const u8;

/// `int BinaryIStream::remainingLength()`.
type RemainingLengthFn = unsafe extern "thiscall" fn(this: *mut c_void) -> i32;

/// A borrowed `BinaryIStream*`. `retrieve` past the end does not fail
/// cleanly in the client, so [`read_into`](ByteSource::read_into) asks
/// `remainingLength` first every time, on top of the decoders doing the
/// same.
pub(crate) struct ClientStream {
    this: *mut c_void,
    retrieve: RetrieveFn,
    remaining_length: RemainingLengthFn,
}

impl ClientStream {
    /// Resolve the stream's `retrieve` and `remainingLength` through its
    /// vtable. `None` if the object or its vtable is unreadable.
    ///
    /// # Safety
    ///
    /// `stream` must be the `BinaryIStream*` a dispatcher call received, used
    /// on that call's thread while the call is running.
    pub(crate) unsafe fn open(stream: usize) -> Option<Self> {
        let mem = ProcessMemory;
        let vtable = mem.read_ptr(stream)?;
        let retrieve = mem.read_ptr(vtable.checked_add(ISTREAM_RETRIEVE)?)?;
        let remaining_length = mem.read_ptr(vtable.checked_add(ISTREAM_REMAINING_LENGTH)?)?;
        // SAFETY: both slots hold the stream class's own virtual methods,
        // whose signatures are the ones declared above (see `addresses`).
        unsafe {
            Some(Self {
                this: stream as *mut c_void,
                retrieve: core::mem::transmute::<usize, RetrieveFn>(retrieve),
                remaining_length: core::mem::transmute::<usize, RemainingLengthFn>(
                    remaining_length,
                ),
            })
        }
    }
}

impl ByteSource for ClientStream {
    fn remaining(&self) -> usize {
        // SAFETY: `open`'s contract: a live stream on its own thread.
        let n = unsafe { (self.remaining_length)(self.this) };
        usize::try_from(n).unwrap_or(0)
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<(), SourceExhausted> {
        if out.is_empty() {
            return Ok(());
        }
        if self.remaining() < out.len() {
            return Err(SourceExhausted);
        }
        let n = i32::try_from(out.len()).map_err(|_| SourceExhausted)?;
        // SAFETY: `open`'s contract, and `n` bytes are known to remain.
        let p = unsafe { (self.retrieve)(self.this, n) };
        if p.is_null() || !is_readable(p as usize, out.len()) {
            return Err(SourceExhausted);
        }
        // SAFETY: `out.len()` readable bytes at `p`, checked above.
        unsafe { core::ptr::copy_nonoverlapping(p, out.as_mut_ptr(), out.len()) };
        Ok(())
    }
}

/// These run only on the real target: they build a C++-shaped stream object
/// with a vtable of `thiscall` functions and read it through
/// [`ClientStream`], which pins the vtable slots and the calling convention.
#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_patch_wire::black_market::{AuctionItem, ClientCall, ClientMethod, OnBMAuctions};
    use cimmeria_patch_wire::{DecodeError, Encode};

    /// `BinaryIStream`'s layout as far as the DLL uses it: a vtable pointer
    /// first, then whatever the class keeps.
    #[repr(C)]
    struct FakeStream {
        vtable: &'static FakeVtable,
        bytes: Vec<u8>,
        pos: usize,
        over_reads: usize,
    }

    #[repr(C)]
    struct FakeVtable {
        destructor: usize,
        retrieve: unsafe extern "thiscall" fn(*mut FakeStream, i32) -> *const u8,
        remaining_length: unsafe extern "thiscall" fn(*mut FakeStream) -> i32,
    }

    /// A scratch buffer handed back on an over-read, the way a real stream
    /// hands back garbage instead of failing.
    static mut GARBAGE: [u8; 256] = [0; 256];

    unsafe extern "thiscall" fn fake_retrieve(this: *mut FakeStream, n: i32) -> *const u8 {
        // SAFETY: `this` is the live FakeStream the test passed in.
        let s = unsafe { &mut *this };
        let n = n as usize;
        if s.pos + n > s.bytes.len() {
            s.over_reads += 1;
            return &raw const GARBAGE as *const u8;
        }
        let p = s.bytes[s.pos..].as_ptr();
        s.pos += n;
        p
    }

    unsafe extern "thiscall" fn fake_remaining(this: *mut FakeStream) -> i32 {
        // SAFETY: as above.
        let s = unsafe { &*this };
        (s.bytes.len() - s.pos) as i32
    }

    static VTABLE: FakeVtable = FakeVtable {
        destructor: 0,
        retrieve: fake_retrieve,
        remaining_length: fake_remaining,
    };

    fn fake(bytes: Vec<u8>) -> FakeStream {
        FakeStream {
            vtable: &VTABLE,
            bytes,
            pos: 0,
            over_reads: 0,
        }
    }

    #[test]
    fn decodes_through_the_vtable() {
        let call = OnBMAuctions {
            auction_items: vec![AuctionItem {
                sequence_id: 5,
                seller_name: "Vala".into(),
                ..Default::default()
            }],
            total_results: 1,
            client_key: 2,
        };
        let mut s = fake(call.to_bytes().unwrap());
        let len = s.bytes.len();
        // SAFETY: a live fake stream on this thread.
        let mut stream = unsafe { ClientStream::open(&raw mut s as usize) }.unwrap();
        let decoded = ClientCall::decode_from(ClientMethod::OnBMAuctions, &mut stream).unwrap();
        assert_eq!(decoded, ClientCall::Auctions(call));
        assert_eq!(s.pos, len, "exactly the arguments consumed");
        assert_eq!(s.over_reads, 0);
    }

    /// A short stream is an error, and `retrieve` is never asked for more
    /// than `remainingLength` said was there.
    #[test]
    fn a_short_stream_never_over_retrieves() {
        let mut s = fake(vec![1, 0]);
        // SAFETY: as above.
        let mut stream = unsafe { ClientStream::open(&raw mut s as usize) }.unwrap();
        assert!(matches!(
            ClientCall::decode_from(ClientMethod::OnBMError, &mut stream),
            Err(DecodeError::Truncated { .. })
        ));
        let mut raw = [0u8; 4];
        assert_eq!(stream.read_into(&mut raw), Err(SourceExhausted));
        assert_eq!(s.over_reads, 0);
        assert_eq!(s.pos, 0);
    }

    #[test]
    fn an_unreadable_stream_does_not_open() {
        // SAFETY: `open` only reads through the page checks.
        assert!(unsafe { ClientStream::open(0) }.is_none());
        assert!(unsafe { ClientStream::open(0x10) }.is_none());
    }
}
