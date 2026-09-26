//! Invariants on the PAK files committed under `data/cache/`, and on how the
//! Kismet sequence overrides sit on top of them.

use super::super::ResourceCache;
use crate::base::sequence_overrides::{
    generate_sequence_xml, SequenceOverride, SEQUENCE_OVERRIDES,
};

/// Version of `CookedDataKismetSeqEvent.pak` that shipped clients hold.
const CLIENT_SHIPPED_KISMET_SEQ_VERSION: u32 = 7455;
const KISMET_SEQUENCES: u32 = 1;

fn data_dir() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache")
}

fn shipped_sequences() -> super::super::CategoryData {
    ResourceCache::load_pak(&format!("{}/CookedDataKismetSeqEvent.pak", data_dir()))
        .expect("committed Kismet sequence PAK loads")
}

/// The on-disk PAK must stay the file clients already hold: same version, no
/// Cimmeria entries. Additions belong in `sequence_overrides`, which delivers
/// them per key.
///
/// Editing the file's version instead is what broke clients on 2026-09-20: with
/// no override list, a mismatch answered `invalidate_all = true` and pushed
/// nothing, and the client emptied and persisted its whole sequence table.
#[test]
fn on_disk_kismet_sequence_pak_is_the_client_shipped_file() {
    let shipped = shipped_sequences();
    assert_eq!(
        shipped.metadata, CLIENT_SHIPPED_KISMET_SEQ_VERSION,
        "do not edit the PAK's MetaData; add a SequenceOverride instead"
    );
    for ov in SEQUENCE_OVERRIDES {
        assert!(
            !shipped.elements.contains_key(&ov.sequence_id),
            "sequence {} is in the on-disk PAK; overrides must not shadow shipped entries",
            ov.sequence_id
        );
    }
}

/// Guard for the regression itself: category 1 must carry an override list, so
/// `handle_version_info_request` takes the per-key branch
/// (`invalidate_all = false`, `InvalidKeys = [...]`, pushes) rather than the
/// destructive invalidate-all-and-push-nothing branch.
#[test]
fn kismet_sequences_take_the_per_key_handshake() {
    let cache = ResourceCache::load_all(data_dir()).expect("committed PAKs load");
    let mut expected: Vec<u32> = SEQUENCE_OVERRIDES.iter().map(|o| o.sequence_id).collect();
    expected.sort_unstable();
    assert!(!expected.is_empty());
    assert_eq!(
        cache.overridden_elements(KISMET_SEQUENCES),
        expected.as_slice()
    );

    let served = cache.category(KISMET_SEQUENCES).expect("category 1");
    // A client holding the shipped version must see a mismatch, or it never
    // learns the new ids.
    assert_ne!(served.metadata, CLIENT_SHIPPED_KISMET_SEQ_VERSION);
    let shipped = shipped_sequences();
    assert_eq!(
        served.elements.len(),
        shipped.elements.len() + expected.len()
    );
    for ov in SEQUENCE_OVERRIDES {
        assert_eq!(
            cache.get(KISMET_SEQUENCES, ov.sequence_id),
            Some(&generate_sequence_xml(ov))
        );
    }
    // Shipped entries are served untouched.
    assert_eq!(
        cache.get(KISMET_SEQUENCES, 1951),
        shipped.elements.get(&1951)
    );
}

/// The generator must emit exactly what the client already parses for this
/// element type. Checked against a real shipped entry rather than a transcribed
/// literal.
#[test]
fn generated_sequence_xml_matches_a_shipped_entry() {
    let shipped = shipped_sequences();
    let actual = shipped
        .elements
        .get(&10015)
        .expect("_10015 ships in the PAK");
    let generated = generate_sequence_xml(&SequenceOverride {
        sequence_id: 10015,
        event_id: 8000,
        kismet_script_name:
            "Castle_Cellblock-fffefffd.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_0",
    });
    assert_eq!(
        String::from_utf8_lossy(&generated),
        String::from_utf8_lossy(actual)
    );
}
