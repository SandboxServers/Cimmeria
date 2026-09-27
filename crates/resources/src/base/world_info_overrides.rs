//! Cimmeria-side additions to `CookedWorldInfo.pak` (category 12).
//!
//! The historical CellBlock worlds (1201–1207, see
//! [`cimmeria_wire::mercury::world_data::historical_cellblocks`]) are world
//! ids the shipped catalogue has never had. The client's world table comes
//! from this catalogue, so the server adds the seven entries in memory at
//! startup and the cooked-data wire path (`versionInfoRequest` →
//! `onVersionInfo(InvalidKeys=[1201..1207])` → `resourceFragment(_key, XML)`)
//! pushes them. That is the path that already delivers new ids for dialogs
//! (3996) and Kismet sequences (10187/10188).
//!
//! # Never edit the PAK on disk
//!
//! `data/cache/CookedWorldInfo.pak` stays byte-identical to the file clients
//! ship with (`MetaData` 5959). The version bump lives only in memory, and
//! it is only safe because this category now carries an override list: a
//! category without one answers a version mismatch with
//! `invalidate_all = true` and pushes nothing, and the client empties its
//! whole table (the 2026-09-20 Kismet sequence wipe; see
//! `super::sequence_overrides`).
//!
//! The emitted XML reproduces the shipped entries byte for byte (QA-build
//! shape: SOAP namespaces; attribute order `Flags`, `MinPerDay`,
//! `MinToRealMin`, `ClientMap`, `World`, `WorldID`; an explicit end tag),
//! since that is what the client demonstrably parses for this element type.

use cimmeria_wire::mercury::world_data::historical_cellblocks::HISTORICAL_CELLBLOCKS;

/// One `COOKED_WORLD_INFO` entry the client must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldInfoOverride {
    /// Cooked catalogue key (`_<world_id>`) and `resources.worlds.world_id`.
    pub world_id: u32,
    /// `World`: the world name the server keys the world by.
    pub world: &'static str,
    /// `ClientMap`: the client package the world loads.
    pub client_map: &'static str,
    pub flags: u32,
    pub min_per_day: u32,
    pub min_to_real_min: u32,
}

/// `Flags`, `MinPerDay` and `MinToRealMin` of the stock `Castle_CellBlock`
/// entry (`_12`). The historical worlds are older states of the same map, so
/// they carry its values.
const CELLBLOCK_FLAGS: u32 = 1;
const CELLBLOCK_MIN_PER_DAY: u32 = 1440;
const CELLBLOCK_MIN_TO_REAL_MIN: u32 = 1;

/// Every world Cimmeria adds to the catalogue: one per historical CellBlock
/// world, built from the wire crate's table so the ids, names and client
/// maps cannot drift from what `onClientMapLoad` sends.
///
/// Ids must not collide with an entry the shipped PAK already has; the
/// `on_disk_world_info_pak_is_the_client_shipped_file` test enforces that.
pub const WORLD_INFO_OVERRIDES: &[WorldInfoOverride] = &historical_cellblock_world_info();

const fn historical_cellblock_world_info() -> [WorldInfoOverride; HISTORICAL_CELLBLOCKS.len()] {
    let mut out = [WorldInfoOverride {
        world_id: 0,
        world: "",
        client_map: "",
        flags: 0,
        min_per_day: 0,
        min_to_real_min: 0,
    }; HISTORICAL_CELLBLOCKS.len()];
    let mut i = 0;
    while i < out.len() {
        let world = &HISTORICAL_CELLBLOCKS[i];
        out[i] = WorldInfoOverride {
            world_id: world.world_id as u32,
            world: world.world,
            client_map: world.client_map,
            flags: CELLBLOCK_FLAGS,
            min_per_day: CELLBLOCK_MIN_PER_DAY,
            min_to_real_min: CELLBLOCK_MIN_TO_REAL_MIN,
        };
        i += 1;
    }
    out
}

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

/// Generate the `<COOKED_WORLD_INFO>` entry for one override.
pub fn generate_world_info_xml(ov: &WorldInfoOverride) -> Vec<u8> {
    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<COOKED_WORLD_INFO",
            " xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\"",
            " xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\"",
            " xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"",
            " xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\"",
            " xmlns:CookedData1=\"SGW\"",
            " Flags=\"{}\" MinPerDay=\"{}\" MinToRealMin=\"{}\"",
            " ClientMap=\"{}\" World=\"{}\" WorldID=\"{}\">",
            "</COOKED_WORLD_INFO>",
        ),
        ov.flags,
        ov.min_per_day,
        ov.min_to_real_min,
        escape_xml_attr(ov.client_map),
        escape_xml_attr(ov.world),
        ov.world_id,
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shipped entry, verbatim from `CookedWorldInfo.pak` (`_12`, the stock
    /// CellBlock). `committed_paks` checks the generator against the PAK
    /// itself; this literal keeps the expected shape readable here.
    const SHIPPED_12: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<COOKED_WORLD_INFO xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
        "xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" ",
        "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
        "xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:CookedData1=\"SGW\" ",
        "Flags=\"1\" MinPerDay=\"1440\" MinToRealMin=\"1\" ClientMap=\"Castle_CellBlock\" ",
        "World=\"Castle_CellBlock\" WorldID=\"12\"></COOKED_WORLD_INFO>",
    );

    #[test]
    fn generated_xml_matches_the_stock_cellblock_entry_byte_for_byte() {
        let xml = generate_world_info_xml(&WorldInfoOverride {
            world_id: 12,
            world: "Castle_CellBlock",
            client_map: "Castle_CellBlock",
            flags: 1,
            min_per_day: 1440,
            min_to_real_min: 1,
        });
        assert_eq!(String::from_utf8(xml).unwrap(), SHIPPED_12);
    }

    #[test]
    fn attributes_are_escaped() {
        let xml = generate_world_info_xml(&WorldInfoOverride {
            world_id: 1,
            world: "A&B",
            client_map: "\"C\"<D>",
            flags: 0,
            min_per_day: 0,
            min_to_real_min: 0,
        });
        let xml = String::from_utf8(xml).unwrap();
        assert!(xml.contains("World=\"A&amp;B\""), "{xml}");
        assert!(
            xml.contains("ClientMap=\"&quot;C&quot;&lt;D&gt;\""),
            "{xml}"
        );
    }

    /// The overrides are exactly the seven historical CellBlock worlds, each
    /// with its own client map and the stock CellBlock time/flag values.
    #[test]
    fn overrides_are_the_historical_cellblocks() {
        let expected: [(u32, &str, &str); 7] = [
            (1201, "CellBlock43", "C43485_CellBlock"),
            (1202, "CellBlock55", "C55124_CellBlock"),
            (1203, "CellBlock57", "C57050_CellBlock"),
            (1204, "CellBlock58", "C58674_CellBlock"),
            (1205, "CellBlock60", "C60130_CellBlock"),
            (1206, "CellBlock62", "C62429_CellBlock"),
            (1207, "CellBlock63", "C63682_CellBlock"),
        ];
        let actual: Vec<(u32, &str, &str)> = WORLD_INFO_OVERRIDES
            .iter()
            .map(|o| (o.world_id, o.world, o.client_map))
            .collect();
        assert_eq!(actual, expected);
        for ov in WORLD_INFO_OVERRIDES {
            assert_eq!(
                (ov.flags, ov.min_per_day, ov.min_to_real_min),
                (1, 1440, 1),
                "{}",
                ov.world
            );
        }
    }

    #[test]
    fn override_ids_are_unique() {
        let mut ids: Vec<u32> = WORLD_INFO_OVERRIDES.iter().map(|o| o.world_id).collect();
        ids.sort_unstable();
        let len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len, "duplicate world_id in WORLD_INFO_OVERRIDES");
    }
}
