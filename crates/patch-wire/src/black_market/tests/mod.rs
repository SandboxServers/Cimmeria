//! Tests for the Black Market codecs.
//!
//! - `golden`: hand-written byte strings for every method, both ways.
//! - `robustness`: truncation at every byte, the caps, bad UTF-8, trailing
//!   bytes, and the pull-source discipline the DLL relies on.
//! - `def_order`: the encoders against the `.def`, `alias.xml`, the
//!   dispatch tables and the seeded enums, read from the repo.

mod def_order;
mod golden;
mod robustness;

use super::AuctionItem;
use crate::{ByteSource, SourceExhausted};

/// An `AuctionItem` with a distinct value in every field, including a
/// negative one, so a swapped or mis-signed field shows in the bytes.
fn sample_item() -> AuctionItem {
    AuctionItem {
        sequence_id: 0x11,
        item_def_id: 0x22,
        stack_size: 3,
        durability: 100,
        charges: -1,
        current_bid: 400,
        buyout_price: 1000,
        end_time_value: 4,
        next_min_bid_price: 420,
        seller_name: "Bo".to_string(),
    }
}

/// [`sample_item`] on the wire, written out by hand: 39 bytes.
const SAMPLE_ITEM_BYTES: &[u8] = &[
    0x11, 0x00, 0x00, 0x00, // sequenceId = 17
    0x22, 0x00, 0x00, 0x00, // itemDefId = 34
    0x03, 0x00, 0x00, 0x00, // stackSize = 3
    0x64, 0x00, 0x00, 0x00, // durability = 100
    0xFF, 0xFF, 0xFF, 0xFF, // charges = -1
    0x90, 0x01, 0x00, 0x00, // currentBid = 400
    0xE8, 0x03, 0x00, 0x00, // buyoutPrice = 1000
    0x04, // endTimeValue = 4 (Long)
    0xA4, 0x01, 0x00, 0x00, // nextMinBidPrice = 420
    0x02, 0x00, 0x00, 0x00, b'B', b'o', // sellerName = "Bo"
];

/// A pull source over a slice that behaves like the client's
/// `BinaryIStream` contract the DLL wraps: it records a violation if a
/// decoder calls `read_into` without first asking `remaining`, or asks for
/// more than `remaining` said was there.
struct StrictSource<'a> {
    bytes: &'a [u8],
    pos: usize,
    checked: std::cell::Cell<bool>,
    violations: Vec<String>,
}

impl<'a> StrictSource<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            checked: std::cell::Cell::new(false),
            violations: Vec::new(),
        }
    }
}

impl ByteSource for StrictSource<'_> {
    fn remaining(&self) -> usize {
        self.checked.set(true);
        self.bytes.len() - self.pos
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<(), SourceExhausted> {
        if !self.checked.replace(false) {
            self.violations.push(format!(
                "read of {} at {} without remaining()",
                out.len(),
                self.pos
            ));
        }
        let left = self.bytes.len() - self.pos;
        if out.len() > left {
            self.violations.push(format!(
                "over-read of {} at {} with {left} left",
                out.len(),
                self.pos
            ));
            return Err(SourceExhausted);
        }
        out.copy_from_slice(&self.bytes[self.pos..self.pos + out.len()]);
        self.pos += out.len();
        Ok(())
    }
}
