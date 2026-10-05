//! Cimmeria-side additions to `CookedDataKismetSeqEvent.pak` (category 1).
//!
//! `onSequence` carries only an integer `KismetEventSetSeqID`. The client turns
//! it into a Kismet script path through the `_<id>` entry in its own cooked
//! catalogue — **not** through the server's `sequences` table. So a new row in
//! `db/resources/Events/Seed/sequences.sql` has zero effect on a client until
//! the catalogue carries the matching entry; an id the client has never seen is
//! a silent no-op.
//!
//! Same trick as [`super::dialog_overrides`]: add the entry in memory at server
//! startup and bump the category's metadata, so a client holding the shipped
//! table is resynced (`versionInfoRequest` → `onVersionInfo(InvalidateAll)` →
//! one `resourceFragment` per entry → the version stamp, #840) and receives
//! it. New keys have reached clients this way since dialog 3996.
//!
//! # Never edit the PAK's `MetaData` version on disk
//!
//! Category 1 used to have no override list, so any version the client did not
//! already hold made `cimmeria_base_session::base::cooked_data::handle_version_info_request` answer
//! `invalidate_all = true` and push nothing. The client does not lazy-fetch: it
//! emptied its sequence table, persisted the empty table to
//! `Cache.en-US/CookedDataKismetSeqEvent.pak`, and no Kismet sequence played
//! afterwards (ring transports, ability effects, VO, doors). That happened on
//! 2026-09-20. Since #840 a mismatch resyncs the whole category, so the wipe
//! cannot recur on a current build; the rule stands because the on-disk PAK is
//! the file clients already hold, and additions belong in overrides.
//!
//! The emitted XML is byte-for-byte the shape of the entries already in this
//! category (QA-build style: SOAP namespaces, `KismetScriptName` / `EventID` /
//! `KismetEventSetSeqID` order), since those are what the client demonstrably
//! parses for this element type.

/// One sequence the client must be able to resolve.
pub struct SequenceOverride {
    /// Cooked catalogue key (`_<sequence_id>`) and `sequences.sequence_id`.
    pub sequence_id: u32,
    /// `sequences.event_id`; 8000 / 8001 are ring Teleport Out / In.
    pub event_id: u32,
    /// Full object path of the Kismet sequence in a client package.
    pub kismet_script_name: &'static str,
}

/// The ring rig cloned onto the mission-688 Armory pad (region 33) by
/// `upk_patch`. The object only exists in a patched
/// `Castle_CellBlock-fffeffff.umap`; on an unpatched client the path does not
/// resolve and the sequence is a no-op.
const ARMORY_RING_RIG: &str =
    "Castle_Cellblock-fffeffff.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_0";

/// The eight ring rigs client patch `010-debug-area-rings` clones into
/// `Ihpet_Crater_Light-fff80002.umap` for the Debug Area (world 1300, DA-08),
/// in station order: Compound, Faction yard, AI slope, Arena rim, Arena pit,
/// Gallery west, Gallery east, Death yard. All eight are copies of region 3's
/// rig, so each copy's root sequence took the next free instance number under
/// `Main_Sequence.Prefabs`: the first has no suffix, the rest `_0` to `_6`.
/// Seeded as event sets 13810-13817 in
/// `db/resources/Events/Seed/debug_area_ring_events.sql`.
pub const DEBUG_AREA_RING_RIGS: [&str; 8] = [
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq",
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_0",
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_1",
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_2",
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_3",
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_4",
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_5",
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_6",
];

/// Teleport Out / Teleport In for Debug Area station `n`: ids `10189 + 2n` and
/// `10190 + 2n`.
const fn debug_area_ring(n: usize, event_id: u32) -> SequenceOverride {
    SequenceOverride {
        sequence_id: 10189 + 2 * n as u32 + (event_id - 8000),
        event_id,
        kismet_script_name: DEBUG_AREA_RING_RIGS[n],
    }
}

/// All Cimmeria-introduced sequences. Adding one:
///
///   1. Add the row to `db/resources/Events/Seed/sequences.sql` so the server's
///      catalogue (event sets, the ring FSM) agrees.
///   2. Add a `SequenceOverride` here so clients can resolve the id.
///
/// Ids must not collide with an entry the shipped PAK already has; the
/// `on_disk_kismet_sequence_pak_is_the_client_shipped_file` test enforces that.
pub const SEQUENCE_OVERRIDES: &[SequenceOverride] = &[
    SequenceOverride {
        sequence_id: 10187,
        event_id: 8000,
        kismet_script_name: ARMORY_RING_RIG,
    },
    SequenceOverride {
        sequence_id: 10188,
        event_id: 8001,
        kismet_script_name: ARMORY_RING_RIG,
    },
    debug_area_ring(0, 8000),
    debug_area_ring(0, 8001),
    debug_area_ring(1, 8000),
    debug_area_ring(1, 8001),
    debug_area_ring(2, 8000),
    debug_area_ring(2, 8001),
    debug_area_ring(3, 8000),
    debug_area_ring(3, 8001),
    debug_area_ring(4, 8000),
    debug_area_ring(4, 8001),
    debug_area_ring(5, 8000),
    debug_area_ring(5, 8001),
    debug_area_ring(6, 8000),
    debug_area_ring(6, 8001),
    debug_area_ring(7, 8000),
    debug_area_ring(7, 8001),
];

fn escape_xml_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Generate the `<COOKED_KISMET_EVENT_SEQUENCE>` entry for one override.
pub fn generate_sequence_xml(ov: &SequenceOverride) -> Vec<u8> {
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<COOKED_KISMET_EVENT_SEQUENCE",
            " xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\"",
            " xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\"",
            " xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"",
            " xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\"",
            " xmlns:CookedData1=\"SGW\"",
            " KismetScriptName=\"{}\" EventID=\"{}\" KismetEventSetSeqID=\"{}\">",
            "</COOKED_KISMET_EVENT_SEQUENCE>",
        ),
        escape_xml_attr(ov.kismet_script_name),
        ov.event_id,
        ov.sequence_id,
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shipped entry, verbatim from `CookedDataKismetSeqEvent.pak` (`_10015`).
    /// The generator must reproduce this shape exactly: it is the only format
    /// the client is known to parse for this element type.
    const SHIPPED_10015: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<COOKED_KISMET_EVENT_SEQUENCE xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
        "xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" ",
        "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
        "xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:CookedData1=\"SGW\" ",
        "KismetScriptName=\"Castle_Cellblock-fffefffd.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_0\" ",
        "EventID=\"8000\" KismetEventSetSeqID=\"10015\"></COOKED_KISMET_EVENT_SEQUENCE>",
    );

    #[test]
    fn generated_xml_matches_a_shipped_entry_byte_for_byte() {
        let xml = generate_sequence_xml(&SequenceOverride {
            sequence_id: 10015,
            event_id: 8000,
            kismet_script_name:
                "Castle_Cellblock-fffefffd.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_0",
        });
        assert_eq!(String::from_utf8(xml).unwrap(), SHIPPED_10015);
    }

    #[test]
    fn script_name_is_attribute_escaped() {
        let xml = generate_sequence_xml(&SequenceOverride {
            sequence_id: 1,
            event_id: 2,
            kismet_script_name: "A&B.\"C\"<D>",
        });
        let xml = String::from_utf8(xml).unwrap();
        assert!(
            xml.contains("KismetScriptName=\"A&amp;B.&quot;C&quot;&lt;D&gt;\""),
            "{xml}"
        );
    }

    #[test]
    fn override_ids_are_unique() {
        let mut ids: Vec<u32> = SEQUENCE_OVERRIDES.iter().map(|o| o.sequence_id).collect();
        ids.sort_unstable();
        let len = ids.len();
        ids.dedup();
        assert_eq!(
            ids.len(),
            len,
            "duplicate sequence_id in SEQUENCE_OVERRIDES"
        );
    }

    #[test]
    fn armory_ring_rig_has_teleport_out_and_in() {
        let events: Vec<(u32, u32)> = SEQUENCE_OVERRIDES
            .iter()
            .filter(|o| o.kismet_script_name == ARMORY_RING_RIG)
            .map(|o| (o.sequence_id, o.event_id))
            .collect();
        assert_eq!(events, vec![(10187, 8000), (10188, 8001)]);
    }

    /// Every Debug Area rig is reachable from a client: one Teleport Out and
    /// one Teleport In per station, at the ids
    /// `db/resources/Events/Seed/debug_area_ring_events.sql` seeds. A rig
    /// missing here never animates, because the client resolves the id only
    /// through its own catalogue.
    #[test]
    fn each_debug_area_rig_has_teleport_out_and_in_at_its_seeded_ids() {
        for (n, rig) in DEBUG_AREA_RING_RIGS.iter().enumerate() {
            let events: Vec<(u32, u32)> = SEQUENCE_OVERRIDES
                .iter()
                .filter(|o| o.kismet_script_name == *rig)
                .map(|o| (o.sequence_id, o.event_id))
                .collect();
            let out = 10189 + 2 * n as u32;
            assert_eq!(
                events,
                vec![(out, 8000), (out + 1, 8001)],
                "station {n}: {rig}"
            );
        }
    }

    /// The eight copies have eight object paths. Two stations sharing one
    /// path would make the second rig unreachable and fire the first.
    #[test]
    fn debug_area_rig_paths_are_distinct_and_in_the_patched_chunk() {
        let mut paths = DEBUG_AREA_RING_RIGS.to_vec();
        assert!(paths
            .iter()
            .all(|p| p.starts_with("Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.")));
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), 8);
    }

    /// `(sequence_id, event_id, kismet_script_name)` from every
    /// `INSERT INTO sequences` row of a seed file.
    fn seeded_sequences(rel: &str) -> Vec<(u32, u32, String)> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../db/resources/Events/Seed")
            .join(rel);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .lines()
            .filter(|l| l.starts_with("INSERT INTO sequences ("))
            .map(|l| {
                let v = &l[l.find("VALUES (").unwrap() + 8..l.rfind(");").unwrap()];
                let mut f = v.splitn(3, ", ");
                let id = f.next().unwrap().parse().unwrap();
                let event = f.next().unwrap().parse().unwrap();
                (id, event, f.next().unwrap().trim_matches('\'').to_string())
            })
            .collect()
    }

    /// The client plays what `SEQUENCE_OVERRIDES` says; the server's event
    /// sets resolve through the seeded `sequences` rows. Each override must
    /// match its seed row exactly, or the server would fire one id while the
    /// client plays another rig (a swapped pair passes every other test).
    #[test]
    fn every_override_matches_its_seeded_sequence_row() {
        let mut seeded = seeded_sequences("sequences.sql");
        seeded.extend(seeded_sequences("debug_area_ring_events.sql"));
        for ov in SEQUENCE_OVERRIDES {
            let rows: Vec<_> = seeded.iter().filter(|r| r.0 == ov.sequence_id).collect();
            assert_eq!(
                rows,
                vec![&(
                    ov.sequence_id,
                    ov.event_id,
                    ov.kismet_script_name.to_string()
                )],
                "sequence {} must be seeded once, as the override says",
                ov.sequence_id
            );
        }
        // And the Debug Area seed holds nothing the overrides do not deliver.
        for (id, _, path) in seeded_sequences("debug_area_ring_events.sql") {
            assert!(
                SEQUENCE_OVERRIDES.iter().any(|o| o.sequence_id == id),
                "seeded sequence {id} ({path}) never reaches a client"
            );
        }
    }
}
