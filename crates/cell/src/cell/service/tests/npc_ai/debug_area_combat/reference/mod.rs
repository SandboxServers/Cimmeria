//! The Z12 combat reference range (NC-03, `docs/content/debug-area.md`): one
//! pen each for the NID Guard (template 24), the Prisoner retrieval unit
//! (PRU, 4), the ScavDog (76) and the Praxis Jaffa pair (1380 and 1381), in
//! world 1300, spawns 14300-14319 (D-NC9). Later NC packets pin each
//! reference's behaviour in the sibling files; this file pins the seed rows.
//!
//! Layout and owners: `docs/analysis/npc-combat-reference/work-packets.md` C4.

mod jaffa;
mod melee_preference;
mod nid;
mod placeholder_policy;
mod pru;
mod regression_harset;
mod regression_tutorial;
mod scavdog;

use super::world_records;
use crate::cell::spawner::SpawnRecord;

/// The reference range's spawn block (contract C3, D-NC9).
pub(super) const REF_SPAWNS: std::ops::RangeInclusive<i32> = 14300..=14319;

/// The one world-1300 row tagged `tag`, from `super::world_records()`.
pub(super) fn ref_row<'a>(records: &'a [SpawnRecord], tag: &str) -> &'a SpawnRecord {
    let mut rows = records.iter().filter(|r| r.tag.as_deref() == Some(tag));
    let row = rows
        .next()
        .unwrap_or_else(|| panic!("no world-1300 row is tagged {tag:?}"));
    assert!(rows.next().is_none(), "more than one row is tagged {tag:?}");
    row
}

/// (tag, template). The Jaffa pair counts as one pen in [`PENS`].
const REFS: [(&str, i32); 5] = [
    ("DebugArea_Ref_NID", 24),
    ("DebugArea_Ref_PRU", 4),
    ("DebugArea_Ref_ScavDog", 76),
    ("DebugArea_Ref_JaffaGuard", 1380),
    ("DebugArea_Ref_JaffaLieutenant", 1381),
];

/// The reference rows in [`REF_SPAWNS`]: exactly the five tags, on the
/// templates the packet names, aggressive (no override) with a 20 s respawn.
/// Revert proof: dropping the spawnlist seed (or its `\ir` line) leaves no
/// rows; changing a template, tag, respawn or setting an override fails here.
#[test]
fn reference_range_holds_the_four_references() {
    let records = world_records();
    let mut in_block: Vec<&SpawnRecord> = records
        .iter()
        .filter(|r| REF_SPAWNS.contains(&r.spawn_id))
        .collect();
    in_block.sort_by_key(|r| r.spawn_id);
    let got: Vec<(&str, i32)> = in_block
        .iter()
        .map(|r| (r.tag.as_deref().unwrap_or(""), r.template_id))
        .collect();
    assert_eq!(got, REFS.to_vec(), "the block holds exactly the five rows");
    for r in in_block {
        assert_eq!(r.respawn_secs, Some(20), "{:?} respawns in 20 s", r.tag);
        assert_eq!(r.aggression_override, None, "{:?} is aggressive", r.tag);
        assert_eq!(r.world_name, super::WORLD);
    }
}

/// Pens: NID, PRU, ScavDog and the Jaffa pair as one.
fn pens(records: &[SpawnRecord]) -> Vec<(&'static str, Vec<&SpawnRecord>)> {
    vec![
        ("NID", vec![ref_row(records, "DebugArea_Ref_NID")]),
        ("PRU", vec![ref_row(records, "DebugArea_Ref_PRU")]),
        ("ScavDog", vec![ref_row(records, "DebugArea_Ref_ScavDog")]),
        (
            "Jaffa",
            vec![
                ref_row(records, "DebugArea_Ref_JaffaGuard"),
                ref_row(records, "DebugArea_Ref_JaffaLieutenant"),
            ],
        ),
    ]
}

/// No two pens are within 35 m (nearest members), so a fight in one never
/// pulls or rallies another. Revert proof: moving any pen within 35 m fails.
#[test]
fn reference_pens_are_apart() {
    let records = world_records();
    let pens = pens(&records);
    for (i, (a, ra)) in pens.iter().enumerate() {
        for (b, rb) in &pens[i + 1..] {
            let gap = ra
                .iter()
                .flat_map(|x| rb.iter().map(move |y| (x.x - y.x).hypot(x.z - y.z)))
                .fold(f32::MAX, f32::min);
            assert!(gap >= 35.0, "pens {a} and {b} are {gap:.1} m apart");
        }
    }
}

/// The abilities each reference fires today. NC-13 changes the ScavDog's.
/// Revert proof: a template's ability set changing fails here by name.
#[test]
fn reference_profiles_today() {
    let records = world_records();
    for (tag, abilities) in [
        ("DebugArea_Ref_NID", vec![559]),
        ("DebugArea_Ref_PRU", vec![221]),
        ("DebugArea_Ref_JaffaGuard", vec![584, 710]),
        ("DebugArea_Ref_JaffaLieutenant", vec![584, 710]),
        ("DebugArea_Ref_ScavDog", vec![]),
    ] {
        assert_eq!(ref_row(&records, tag).ability_ids, abilities, "{tag}");
    }
}

mod live_db {
    use super::*;
    use crate::cell::combat::HOSTILE_FACTION;
    use crate::cell::spawner::load_spawns_from_db;
    use crate::test_support::require_db_or_skip;

    /// The loaders see the five rows; the Jaffa clones are faction 10
    /// (damageable, hostile to players). Revert proof: dropping either seed
    /// file or its `\ir` line in db/database.sql leaves fewer rows or no
    /// templates 1380/1381.
    #[tokio::test]
    async fn live_db_debug_area_reference_rows_load() {
        let pool = require_db_or_skip!();
        let rows: Vec<SpawnRecord> = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| REF_SPAWNS.contains(&r.spawn_id))
            .collect();
        assert_eq!(rows.len(), 5, "five reference rows load");
        for tag in ["DebugArea_Ref_JaffaGuard", "DebugArea_Ref_JaffaLieutenant"] {
            let r = ref_row(&rows, tag);
            assert_eq!(r.faction, Some(HOSTILE_FACTION as i32), "{tag} faction");
        }
    }
}
