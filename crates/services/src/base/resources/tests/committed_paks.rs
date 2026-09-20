//! Invariants on the PAK files committed under `data/cache/`.

use super::super::ResourceCache;

/// Version of `CookedDataKismetSeqEvent.pak` that shipped clients hold.
const CLIENT_SHIPPED_KISMET_SEQ_VERSION: u32 = 7455;

/// The Kismet sequence PAK's `MetaData` version must equal the copy clients
/// already hold.
///
/// Category 1 has no override list, so a version mismatch makes
/// `handle_version_info_request` answer `invalidate_all = true` with nothing
/// pushed. The client never lazy-fetches: it drops its whole sequence table,
/// rewrites `Cache.en-US/CookedDataKismetSeqEvent.pak` as an empty archive
/// stamped with the new version, and from then on no Kismet sequence plays
/// (ring transports, ability effects, VO, doors) until that file is restored by
/// hand. Bumping this version broke exactly that on 2026-09-20.
///
/// New sequences reach clients the way the existing custom ones (10015+) did:
/// the identical PAK, at the same version, on both sides. If the client copy is
/// ever re-issued at a new version, change both together and update this test.
#[test]
fn kismet_sequence_pak_version_matches_the_copy_clients_hold() {
    let data_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
    let cache = ResourceCache::load_all(data_dir).expect("committed PAKs load");
    let sequences = cache.category(1).expect("category 1 (Kismet sequences)");
    assert_eq!(
        sequences.metadata, CLIENT_SHIPPED_KISMET_SEQ_VERSION,
        "bumping this wipes every connecting client's sequence table; see the doc comment"
    );
    // The ring transport sequences the Armory rig needs are still served.
    assert!(sequences.elements.contains_key(&10187));
    assert!(sequences.elements.contains_key(&10188));
    assert!(sequences.elements.contains_key(&1951));
}
