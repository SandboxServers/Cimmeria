//! Reassembly caps (bundle count, pending bytes, earliest-arrival
//! eviction) and a differential check that the `BTreeMap` overlap index
//! decides exactly what the former full scan decided.

use std::collections::HashMap;

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;

use super::*;

const COUNT_CAP: usize = MAX_PENDING_FRAGMENTED_BUNDLES;
const BYTE_CAP: usize = MAX_PENDING_FRAGMENT_BYTES;

/// Open bundle `i` of a flood: a 2-fragment range far from every other
/// one, with only fragment 0 sent, so it never completes or overlaps.
fn open_partial(asm: &mut FragmentAssembler, i: u32, body: &[u8]) {
    let r = asm
        .add_fragment(i * 1000, 0, 2, Bytes::copy_from_slice(body))
        .unwrap();
    assert!(r.is_none(), "a lone fragment of a 2-fragment bundle waits");
}

#[test]
fn flood_of_disjoint_partial_bundles_stays_within_the_count_cap() {
    let mut asm = FragmentAssembler::new();
    for i in 0..1_000u32 {
        open_partial(&mut asm, i, b"x");
        assert!(
            asm.pending_count() <= COUNT_CAP,
            "pending {} exceeds the count cap after bundle {i}",
            asm.pending_count()
        );
    }
    assert_eq!(asm.pending_count(), COUNT_CAP);
    let hits = asm.take_cap_hits();
    assert_eq!(
        hits.count_evictions,
        1_000u64.saturating_sub(COUNT_CAP as u64)
    );
    assert_eq!(hits.byte_evictions + hits.oversize_drops, 0);
    assert!(asm.take_cap_hits().is_empty(), "take resets the tally");
}

#[test]
fn flood_of_large_partial_bundles_stays_within_the_byte_cap() {
    // Ten 64-fragment bundles, 63 fragments each of 1,400 bytes: 882,000
    // bytes if nothing were dropped, under the count cap throughout.
    let mut asm = FragmentAssembler::new();
    let body = vec![0xAB; 1_400];
    for b in 0..10u32 {
        for idx in 0..63u8 {
            let r = asm
                .add_fragment(b * 1_000, idx, 64, Bytes::copy_from_slice(&body))
                .unwrap();
            assert!(r.is_none());
            assert!(
                asm.pending_bytes() <= BYTE_CAP,
                "pending bytes {} exceed the cap",
                asm.pending_bytes()
            );
        }
    }
    let hits = asm.take_cap_hits();
    assert!(hits.byte_evictions > 0, "the byte cap must have evicted");
    assert_eq!(hits.count_evictions, 0);
    // The bundle being filled is never the one evicted: the last bundle
    // holds all 63 of its fragments.
    assert_eq!(asm.pending[&9_000].received_count, 63);
}

#[test]
fn a_single_bundle_larger_than_the_byte_cap_is_dropped() {
    let mut asm = FragmentAssembler::new();
    let body = vec![0u8; BYTE_CAP / 63 + 1];
    let mut completed = None;
    for idx in 0..64u8 {
        if let Some(b) = asm
            .add_fragment(500, idx, 64, Bytes::copy_from_slice(&body))
            .unwrap()
        {
            completed = Some(b);
        }
        assert!(asm.pending_bytes() <= BYTE_CAP);
    }
    assert!(completed.is_none(), "an oversize bundle never completes");
    let hits = asm.take_cap_hits();
    assert!(hits.oversize_drops >= 1);
    assert_eq!(
        hits.last.map(|(reason, seq, _, _)| (reason, seq)),
        Some((FragmentCapReason::OversizeBundle, 500))
    );
}

#[test]
fn interleaved_partial_bundles_under_the_cap_all_reassemble() {
    // COUNT_CAP bundles of 3 fragments each, open at once, fragments
    // delivered round-robin and out of order within each bundle.
    let mut asm = FragmentAssembler::new();
    let bundles: Vec<u32> = (0..COUNT_CAP as u32).map(|i| 100 + i * 10).collect();
    let mut done = HashMap::new();
    for idx in [2u8, 0, 1] {
        for &first in &bundles {
            let body = format!("{first}:{idx};");
            if let Some(b) = asm
                .add_fragment(first, idx, 3, Bytes::from(body.into_bytes()))
                .unwrap()
            {
                done.insert(first, b);
            }
        }
    }
    assert_eq!(done.len(), COUNT_CAP, "every bundle completes");
    for &first in &bundles {
        let want = format!("{first}:0;{first}:1;{first}:2;");
        assert_eq!(done[&first].as_ref(), want.as_bytes());
    }
    assert!(asm.take_cap_hits().is_empty(), "no cap was hit");
    assert_eq!((asm.pending_count(), asm.pending_bytes()), (0, 0));
}

#[test]
fn the_count_cap_evicts_the_earliest_arrival_not_the_lowest_sequence() {
    let mut asm = FragmentAssembler::new();
    // Arrival order runs from high sequence numbers to low ones.
    for i in (0..COUNT_CAP as u32).rev() {
        open_partial(&mut asm, i + 1, b"orphan");
    }
    let earliest = COUNT_CAP as u32 * 1000;
    assert!(asm.pending.contains_key(&earliest));

    // A new bundle completes despite the cap, and the earliest arrival
    // (highest sequence) is the one that made room.
    asm.add_fragment(900_000, 0, 2, Bytes::from_static(b"new-"))
        .unwrap();
    let body = asm
        .add_fragment(900_000, 1, 2, Bytes::from_static(b"bundle"))
        .unwrap()
        .expect("the new bundle reassembles at the cap");
    assert_eq!(body.as_ref(), b"new-bundle");
    assert!(!asm.pending.contains_key(&earliest));
    assert!(asm.pending.contains_key(&1000), "later arrivals survive");
    assert_eq!(asm.pending_count(), COUNT_CAP - 1);
    let hits = asm.take_cap_hits();
    assert_eq!(hits.count_evictions, 1);
    assert_eq!(
        hits.last,
        Some((FragmentCapReason::BundleCount, earliest, 1, 2))
    );
}

#[test]
fn bookkeeping_returns_to_zero_after_completion_and_overlap_eviction() {
    let mut asm = FragmentAssembler::new();
    asm.add_fragment(10, 0, 3, Bytes::from_static(b"abc"))
        .unwrap();
    assert_eq!(asm.pending_bytes(), 3);
    // Overlapping newer bundle evicts it; its bytes leave the tally.
    asm.add_fragment(12, 0, 2, Bytes::from_static(b"de"))
        .unwrap();
    assert_eq!((asm.pending_count(), asm.pending_bytes()), (1, 2));
    asm.add_fragment(12, 1, 2, Bytes::from_static(b"f"))
        .unwrap()
        .expect("completes");
    assert_eq!((asm.pending_count(), asm.pending_bytes()), (0, 0));
    assert!(asm.by_arrival.is_empty());
}

// ── Differential check against the former full-scan algorithm ─────

/// The pre-index algorithm, kept verbatim in shape: every decision scans
/// every pending entry. No caps.
#[derive(Default)]
struct FullScan {
    pending: HashMap<u32, (u8, Vec<Option<Vec<u8>>>)>,
}

impl FullScan {
    fn add(&mut self, first: u32, idx: u8, total: u8, data: &[u8]) -> Result<Option<Vec<u8>>> {
        let end = first.wrapping_add(total as u32 - 1);
        let stale = self.pending.iter().any(|(&e, (t, _))| {
            e != first
                && ranges_overlap_mod28(first, end, e, e.wrapping_add(*t as u32 - 1))
                && is_strictly_newer_mod28(e, first)
        });
        if stale {
            return Ok(None);
        }
        self.pending.retain(|&e, (t, _)| {
            e == first
                || !ranges_overlap_mod28(first, end, e, e.wrapping_add(*t as u32 - 1))
                || !is_strictly_newer_mod28(first, e)
        });
        let entry = self
            .pending
            .entry(first)
            .or_insert_with(|| (total, vec![None; total as usize]));
        if entry.0 != total {
            return Err(CimmeriaError::FragmentReassembly("conflict".into()));
        }
        if entry.1[idx as usize].is_none() {
            entry.1[idx as usize] = Some(data.to_vec());
        }
        if entry.1.iter().all(Option::is_some) {
            let (_, frags) = self.pending.remove(&first).unwrap();
            return Ok(Some(frags.into_iter().flatten().flatten().collect()));
        }
        Ok(None)
    }
}

#[test]
fn indexed_overlap_matches_the_full_scan_including_the_28_bit_wrap() {
    // Bundles of 36..=64 fragments inside a 512-wide arc that straddles
    // the wrap: at most 15 can coexist, so the count cap never fires and
    // any divergence is the index's.
    let mut rng = ChaCha20Rng::seed_from_u64(0x5EED_F4A6);
    let mut asm = FragmentAssembler::new();
    let mut reference = FullScan::default();
    let base = SEQUENCE_MASK - 255;
    for step in 0..20_000u32 {
        let first = base.wrapping_add(rng.random_range(0..512)) & SEQUENCE_MASK;
        // Reuse an open bundle's declared size most of the time, so
        // bundles actually complete; otherwise pick a fresh size.
        let total = match reference.pending.get(&first) {
            Some((t, _)) if rng.random_bool(0.9) => *t,
            _ => rng.random_range(36..=64u8),
        };
        let idx = rng.random_range(0..total);
        let data = step.to_le_bytes();
        let got = asm.add_fragment(first, idx, total, Bytes::copy_from_slice(&data));
        let want = reference.add(first, idx, total, &data);
        match (&got, &want) {
            (Ok(g), Ok(w)) => assert_eq!(
                g.as_ref().map(|b| b.to_vec()),
                *w,
                "step {step}: result differs"
            ),
            (Err(_), Err(_)) => {}
            _ => panic!("step {step}: {got:?} vs {want:?}"),
        }
        let mut got_keys: Vec<u32> = asm.pending.keys().copied().collect();
        let mut want_keys: Vec<u32> = reference.pending.keys().copied().collect();
        got_keys.sort_unstable();
        want_keys.sort_unstable();
        assert_eq!(got_keys, want_keys, "step {step}: pending set differs");
    }
    assert!(asm.take_cap_hits().is_empty());
}
