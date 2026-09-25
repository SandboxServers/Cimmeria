//! The 2009 faction reaction table: how a viewer faction regards a target
//! faction, as an `EMobAggressionLevel` (NA13, D-NA01).
//!
//! # Source
//!
//! `FACTION_REACTION_TABLE` in `entities/defs/enumerations.xml` (the client's
//! own copy of the table, with row labels), identical value for value to
//! `FACTION_REACTION_TABLE` in `deprecated/python/Atrea/enums.py`, which
//! `SGWPlayer.getAggressionLevel` (`deprecated/python/cell/SGWPlayer.py`)
//! indexed as `[viewer.faction][target.faction]`. The test at the bottom of
//! this file re-parses `enumerations.xml` and fails on any drift.
//!
//! # Why a constant table in code and not a seed table
//!
//! The table is engine data the client ships, not per-zone content: no
//! designer tunes it per world, the scan reads it for every candidate every
//! AI tick, and a seed table would add a loader, a DB round trip and a cache
//! for 1,936 values that never change. Pinning it to the client's XML in a
//! unit test gives the same single source of truth a seed row would. A
//! per-spawn exception is a `spawnlist.aggression_override`, not a table
//! edit.

use cimmeria_entity::cell_entity::MobAggression;

/// Number of factions in the table (rows and columns), `0 Undefined` to
/// `43 Hostile_To_Players`.
pub const FACTION_COUNT: usize = 44;

/// `REACTION[viewer][target]`, `EMobAggressionLevel` values (1 hostile ...
/// 4 friendly). Row labels are the XML's `<Label>`s.
#[rustfmt::skip]
const REACTION: [[u8; FACTION_COUNT]; FACTION_COUNT] = [
    [4, 4, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 0 Undefined
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4], // 1 World Object
    [3, 4, 4, 3, 4, 3, 4, 3, 3, 4, 1, 1, 4, 1, 3, 4, 3, 3, 4, 1, 3, 1, 3, 3, 4, 1, 1, 1, 1, 1, 3, 4, 1, 4, 1, 4, 4, 3, 3, 3, 3, 1, 3, 3], // 2 SGU
    [3, 4, 3, 4, 3, 4, 4, 3, 3, 4, 1, 1, 4, 1, 3, 4, 3, 1, 4, 3, 3, 1, 1, 3, 1, 1, 1, 4, 1, 4, 1, 4, 1, 4, 1, 4, 4, 3, 3, 3, 3, 1, 3, 3], // 3 Praxis
    [3, 4, 4, 3, 4, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 4 SGU_MissionGiver
    [3, 4, 3, 4, 3, 4, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 5 Praxis_MissionGiver
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3], // 6 Neutral_MissionGiver
    [3, 4, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 7 Neutral_Ambient
    [1, 4, 1, 1, 3, 3, 3, 3, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3], // 8 Aggro_Ambient
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3], // 9 Friendly_Ambient
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 4, 1, 1, 1, 1, 1, 3, 3, 3, 3, 1, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 1, 4, 3, 3, 3, 3, 3, 3, 3, 3], // 10 Straegis
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 1, 4, 4, 3, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 11 Agnos_Laro
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 12 Ancients
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 1, 3, 1, 3, 3, 3, 3, 3, 3, 3], // 13 Bataur
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 4, 3, 3, 3, 3, 3, 3, 4, 3, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3], // 14 Burtonol
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 15 Furling
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 16 Ihpet_Unas
    [3, 4, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 1, 1, 3, 3, 3, 3], // 17 Jaffa_Beleth
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 18 Jaffa_Dakara
    [3, 4, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 19 Jaffa_DakorFollower
    [3, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 20 Jaffa_NinePeaks
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 4, 1, 1, 1, 3, 3, 3, 3, 3, 3, 2, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 21 Jaffa_Ra
    [3, 4, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 1, 1, 3, 3, 3, 3], // 22 Jaffa_Svarog
    [3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 1, 3, 4, 3, 3, 3, 1, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 23 Lucia_Ambient
    [3, 4, 4, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 4, 1, 1, 1, 1, 1, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3], // 24 Lucia_Blue
    [3, 4, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 3, 3, 3, 3, 2, 3, 3, 2, 4, 4, 4, 2, 2, 3, 3, 3, 3, 3, 3, 2, 3, 3, 3, 3, 3, 3, 3], // 25 Lucia_DrugAddict
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 1, 4, 4, 4, 1, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3], // 26 Lucia_DrugDealer
    [3, 4, 1, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 1, 3, 3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3], // 27 Lucia_Green
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 1, 1, 3, 3, 1, 4, 1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3], // 28 Lucia_Red
    [3, 4, 1, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 1, 1, 3, 3, 1, 1, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 29 Lucia_Yellow
    [3, 4, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 30 NID
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 31 Nox
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 32 Replicator
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 33 Sha_Friendly
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3], // 34 Sha_Hostile
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3], // 35 Straegis_Prime
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 4, 1, 1, 1, 1, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3], // 36 TechConGroup
    [3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 3, 3, 3, 3], // 37 Tollan_Ambient
    [3, 4, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 1, 3, 3, 3, 3], // 38 Tollan_Narim
    [3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 1, 4, 3, 3, 3, 3], // 39 Tollan_Travell
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3], // 40 Vokos_Aesir
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3], // 41 Vokos_Alterr
    [3, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 3], // 42 Vokos_Nix
    [3, 4, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4], // 43 Hostile_To_Players
];

/// How `viewer_faction` regards `target_faction`.
///
/// A faction outside the table (python would have raised `IndexError`) reads
/// as [`MobAggression::Neutral`], the reference's fallback for anything that
/// is not a being.
pub fn reaction(viewer_faction: u8, target_faction: u8) -> MobAggression {
    REACTION
        .get(viewer_faction as usize)
        .and_then(|row| row.get(target_faction as usize))
        .and_then(|&v| MobAggression::from_level(v as i32))
        .unwrap_or(MobAggression::Neutral)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the ported table to the client's `enumerations.xml`, row by row.
    /// A hand edit here, or a changed table in the defs, fails this test.
    #[test]
    fn table_matches_enumerations_xml() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../entities/defs/enumerations.xml");
        let xml = std::fs::read_to_string(&path).expect("read enumerations.xml");
        let start = xml
            .find("<FACTION_REACTION_TABLE>")
            .expect("FACTION_REACTION_TABLE in enumerations.xml");
        let end = start
            + xml[start..]
                .find("</FACTION_REACTION_TABLE>")
                .expect("closing tag");
        let rows: Vec<Vec<u8>> = xml[start..end]
            .split("<Data>")
            .skip(1)
            .map(|chunk| {
                let data = &chunk[..chunk.find("</Data>").expect("</Data>")];
                data.split(',')
                    .map(|v| v.trim().parse::<u8>().expect("numeric cell"))
                    .collect()
            })
            .collect();
        assert_eq!(rows.len(), FACTION_COUNT, "row count");
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.as_slice(), REACTION[i].as_slice(), "row {i}");
        }
    }

    /// The pairs the seeds actually use. Players are faction 3 on the wire
    /// (`mercury::aoi::PLAYER_FACTION`).
    #[test]
    fn seeded_faction_pairs() {
        assert_eq!(reaction(3, 10), MobAggression::Hostile, "Straegis/NID");
        assert_eq!(reaction(3, 1), MobAggression::Friendly, "world object");
        assert_eq!(reaction(3, 3), MobAggression::Friendly, "own faction");
        assert_eq!(reaction(3, 0), MobAggression::Neutral, "undefined/NULL");
    }

    #[test]
    fn out_of_table_factions_read_neutral() {
        assert_eq!(reaction(3, 44), MobAggression::Neutral);
        assert_eq!(reaction(200, 10), MobAggression::Neutral);
    }
}
