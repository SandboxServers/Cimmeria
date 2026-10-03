//! How many piggybacked ACKs one outgoing datagram can carry.
//!
//! The client's Mercury socket reader (`FUN_0158a200`) gives `recvfrom` a
//! [`PACKET_MAX_SIZE`] (1472-byte) buffer. A larger datagram fails with
//! `WSAEMSGSIZE` before Mercury sees it, and a retransmit resends the same
//! cached bytes, so one oversized reliable packet wedges the client's
//! reliable stream for good. A 2026-09-29 colo session stalled on 1488-byte
//! datagrams exactly this way.
//!
//! Bodies are sized up front (bundles cut at
//! [`FRAGMENT_BODY_SIZE`](super::FRAGMENT_BODY_SIZE), resource fragments at
//! 1406 bytes); piggybacked ACKs were the one part of a packet nobody
//! bounded, and every send drained the whole pending list onto itself. Every
//! send now takes its ACKs through [`take_piggyback_acks`]; ACKs that do not
//! fit stay pending for the next packet or the 10 Hz tickSync.
//!
//! [`PACKET_MAX_SIZE`]: crate::consts::PACKET_MAX_SIZE

use crate::consts::PACKET_MAX_SIZE;
use crate::encryption::EncryptionVersion;

/// Largest plaintext a data packet occupies before its ACK footers: the
/// flags byte, the body, and every other footer. The worst case is a
/// cooked-data resource fragment (1 flags + 1406 body + 4 seq = 1411,
/// Mercury's `MAX_BODY_LENGTH`); a fragmented bundle comes to
/// 1 + 1300 + 8 frag + 4 seq + 2 request offset = 1315.
pub const MAX_DATA_PLAINTEXT_BEFORE_ACKS: usize = 1411;

/// Plaintext bound for tickSync, an unreliable packet with a tiny body.
pub const TICK_SYNC_PLAINTEXT_BEFORE_ACKS: usize = 32;

const AES_BLOCK: usize = 16;
const MAC_LEN: usize = 16;
const V2_PREFIX_LEN: usize = 1 + 16;
const ACK_LEN: usize = 4;
const ACK_COUNT_LEN: usize = 1;

/// Encrypted datagram size for a plaintext of `len` bytes. PKCS7 always
/// adds 1 to 16 bytes, so an aligned plaintext grows by a whole block.
pub const fn encrypted_len(len: usize, version: EncryptionVersion) -> usize {
    let padded = (len / AES_BLOCK + 1) * AES_BLOCK;
    match version {
        EncryptionVersion::V1 => padded + MAC_LEN,
        EncryptionVersion::V2 => V2_PREFIX_LEN + padded + MAC_LEN,
    }
}

/// Most ACKs that fit after `plaintext_before_acks` bytes while keeping the
/// encrypted datagram within [`PACKET_MAX_SIZE`]. Also capped at 255, the
/// most the one-byte ACK count footer can express.
pub const fn ack_budget(plaintext_before_acks: usize, version: EncryptionVersion) -> usize {
    let mut n = u8::MAX as usize;
    loop {
        let len = plaintext_before_acks + ACK_COUNT_LEN + n * ACK_LEN;
        if encrypted_len(len, version) <= PACKET_MAX_SIZE || n == 0 {
            return n;
        }
        n -= 1;
    }
}

/// ACK budget for any data packet: 10 under v1, 2 under v2.
pub const fn data_ack_budget(version: EncryptionVersion) -> usize {
    ack_budget(MAX_DATA_PLAINTEXT_BEFORE_ACKS, version)
}

/// Remove the oldest pending ACKs that fit on a data packet and return them.
/// The rest stay in `pending`.
pub fn take_piggyback_acks(pending: &mut Vec<u32>, version: EncryptionVersion) -> Vec<u32> {
    take_acks(pending, data_ack_budget(version))
}

/// ACKs for a packet whose plaintext before ACKs is known exactly (flags,
/// body and every other footer). Use it where the body can exceed the
/// [`MAX_DATA_PLAINTEXT_BEFORE_ACKS`] assumption, such as a long feedback
/// line or a large enter-world packet; a body that already overflows on its
/// own gets no ACKs. Capped at the data budget like every other send.
pub fn take_acks_for_plaintext(
    pending: &mut Vec<u32>,
    plaintext_before_acks: usize,
    version: EncryptionVersion,
) -> Vec<u32> {
    let max = ack_budget(plaintext_before_acks, version).min(data_ack_budget(version));
    take_acks(pending, max)
}

/// Remove up to `max` of the oldest pending ACKs and return them.
pub fn take_acks(pending: &mut Vec<u32>, max: usize) -> Vec<u32> {
    let n = pending.len().min(max);
    pending.drain(..n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::MercuryEncryption;
    use crate::packet::{build_outgoing, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE, FLAG_RELIABLE};

    const VERSIONS: [EncryptionVersion; 2] = [EncryptionVersion::V1, EncryptionVersion::V2];

    fn enc(version: EncryptionVersion) -> MercuryEncryption {
        MercuryEncryption::from_session_key_versioned([0x5A; 32], version)
    }

    /// The largest data packet, with its full ACK budget, encrypts to at
    /// most 1472 bytes, and one ACK more does not. Real cipher, real
    /// builder: this fails if either the budget or the size model drifts.
    #[test]
    fn largest_data_packet_with_full_ack_budget_fits_the_client_buffer() {
        let body = vec![0xAB; MAX_DATA_PLAINTEXT_BEFORE_ACKS - 1 - 4];
        let flags = FLAG_RELIABLE | FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS;
        for version in VERSIONS {
            let budget = data_ack_budget(version);
            let acks: Vec<u32> = (1..=budget as u32 + 1).collect();
            let fits = build_outgoing(flags, &body, Some(7), &acks[..budget], None);
            assert_eq!(fits.len(), MAX_DATA_PLAINTEXT_BEFORE_ACKS + 1 + 4 * budget);
            let wire = enc(version).encrypt(&fits).unwrap();
            assert_eq!(wire.len(), encrypted_len(fits.len(), version));
            assert!(wire.len() <= PACKET_MAX_SIZE, "{version:?}: {}", wire.len());

            let over = build_outgoing(flags, &body, Some(7), &acks, None);
            let wire = enc(version).encrypt(&over).unwrap();
            assert!(
                wire.len() > PACKET_MAX_SIZE,
                "{version:?}: budget is not tight"
            );
        }
    }

    #[test]
    fn budgets_are_pinned() {
        assert_eq!(data_ack_budget(EncryptionVersion::V1), 10);
        assert_eq!(data_ack_budget(EncryptionVersion::V2), 2);
        // tickSync can carry a full ACK count byte's worth.
        for version in VERSIONS {
            assert_eq!(ack_budget(TICK_SYNC_PLAINTEXT_BEFORE_ACKS, version), 255);
        }
    }

    /// The 2026-09-29 stall: a full 1300-byte bundle fragment with every
    /// pending ACK drained onto it came to 1488 encrypted bytes. Draining
    /// through the budget keeps it under the cap and leaves the rest queued.
    #[test]
    fn draining_a_long_ack_backlog_leaves_the_overflow_pending() {
        let mut pending: Vec<u32> = (100..140).collect();
        let taken = take_piggyback_acks(&mut pending, EncryptionVersion::V1);
        assert_eq!(taken, (100..110).collect::<Vec<_>>());
        assert_eq!(pending, (110..140).collect::<Vec<_>>());

        let body = vec![0xCD; crate::packet::FRAGMENT_BODY_SIZE];
        let flags = FLAG_RELIABLE | FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS;
        let uncapped: Vec<u32> = (100..140).collect();
        let old = enc(EncryptionVersion::V1)
            .encrypt(&build_outgoing(flags, &body, Some(1), &uncapped, None))
            .unwrap();
        assert!(
            old.len() > PACKET_MAX_SIZE,
            "the old drain overflowed: {}",
            old.len()
        );
        let new = enc(EncryptionVersion::V1)
            .encrypt(&build_outgoing(flags, &body, Some(1), &taken, None))
            .unwrap();
        assert!(new.len() <= PACKET_MAX_SIZE);
    }

    /// A body past the 1411-byte assumption gets a smaller budget, and
    /// one that cannot fit even one ACK gets none.
    #[test]
    fn exact_plaintext_budget_shrinks_for_a_large_body() {
        let v2 = EncryptionVersion::V2;
        let mut pending: Vec<u32> = (1..=20).collect();
        // 1420 bytes before ACKs under v2: one ACK pads to 1440 + 33.
        assert!(take_acks_for_plaintext(&mut pending, 1420, v2).is_empty());
        assert_eq!(take_acks_for_plaintext(&mut pending, 1415, v2), vec![1]);
        assert_eq!(take_acks_for_plaintext(&mut pending, 100, v2).len(), 2);
        let body = vec![0x11; 1420 - 1 - 4];
        let flags = FLAG_RELIABLE | FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS;
        let wire = enc(v2)
            .encrypt(&build_outgoing(flags, &body, Some(9), &[1], None))
            .unwrap();
        assert!(
            wire.len() > PACKET_MAX_SIZE,
            "one ACK would overflow: {}",
            wire.len()
        );
    }

    #[test]
    fn take_acks_on_a_short_list_takes_everything() {
        let mut pending = vec![1, 2];
        assert_eq!(take_acks(&mut pending, 10), vec![1, 2]);
        assert!(pending.is_empty());
    }
}
