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
//! startup and let the cooked-data wire path (`versionInfoRequest` →
//! `onVersionInfo(InvalidKeys=[...])` → `resourceFragment(_key, XML)`) push it.
//! The new-key case is the proven one: dialog 3996 reaches clients this way.
//!
//! # Never edit the PAK's `MetaData` version on disk
//!
//! Category 1 used to have no override list, so any version the client did not
//! already hold made [`super::cooked_data::handle_version_info_request`] answer
//! `invalidate_all = true` and push nothing. The client does not lazy-fetch: it
//! emptied its sequence table, persisted the empty table to
//! `Cache.en-US/CookedDataKismetSeqEvent.pak`, and no Kismet sequence played
//! afterwards (ring transports, ability effects, VO, doors). That happened on
//! 2026-09-20. With an override list present, a mismatch takes the per-key path
//! instead, which only touches the listed ids.
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
}
