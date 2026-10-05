//! Live-DB guards for the Visual NPC Lineup's switchable groups (DA-10,
//! world 1300): the seed's five spawn sets, loaded the way `startup` loads
//! them.
//!
//! * every lineup actor (spawns 13870-14079) is in exactly one group, and
//!   no other row is in a lineup group;
//! * every group is a `visual_lineup` set of world 1300 no bigger than
//!   [`MAX_GROUP`], the size the lab measured as safe for the 32-bit client;
//! * after [`partition_spawn_sets`] no lineup actor is left to spawn at
//!   boot, every group is off, and the six attendants still spawn.
//!
//! Revert proofs (each run against the seed): clear spawn 13870's
//! `set_name` and `lineup_live_db_every_actor_is_in_one_group_off_at_boot`
//! names it; move spawn 13911 from Humans into Jaffa male and the size bound
//! names the 45-actor group.

use std::collections::BTreeMap;

use crate::cell::space_manager::partition_spawn_sets;
use crate::cell::spawner::{load_spawn_sets, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

const ACTORS: std::ops::RangeInclusive<i32> = 13870..=14079;
const ATTENDANTS: std::ops::RangeInclusive<i32> = 14090..=14095;
const ACTOR_COUNT: usize = 161;
const GROUPS: usize = 5;
/// The biggest group the lab loaded cleanly on the 32-bit client (DA-10 lab,
/// 2026-10-05; docs/content/debug-area.md, Arrival load).
const MAX_GROUP: usize = 44;
const KIND: &str = "visual_lineup";
const WORLD_ID: i32 = 1300;

#[tokio::test]
async fn lineup_live_db_every_actor_is_in_one_group_off_at_boot() {
    let pool = require_db_or_skip!();
    let defs = load_spawn_sets(&pool).await.expect("load_spawn_sets");
    let lineup: Vec<_> = defs.iter().filter(|d| d.kind == KIND).collect();
    assert_eq!(lineup.len(), GROUPS, "five lineup groups: {lineup:#?}");

    let mut owner: BTreeMap<i32, i32> = BTreeMap::new();
    let mut errors = Vec::new();
    for d in &lineup {
        if d.world_id != WORLD_ID {
            errors.push(format!("set {} is in world {}", d.set_id, d.world_id));
        }
        if d.spawn_ids.is_empty() || d.spawn_ids.len() > MAX_GROUP {
            errors.push(format!(
                "set {} {:?} has {} actors (1..={MAX_GROUP})",
                d.set_id,
                d.name,
                d.spawn_ids.len()
            ));
        }
        for &s in &d.spawn_ids {
            if !ACTORS.contains(&s) {
                errors.push(format!(
                    "set {} holds spawn {s}, not a lineup actor",
                    d.set_id
                ));
            }
            if let Some(other) = owner.insert(s, d.set_id) {
                errors.push(format!("spawn {s} is in sets {other} and {}", d.set_id));
            }
        }
    }

    let mut records = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db");
    let actors: Vec<i32> = records
        .iter()
        .map(|r| r.spawn_id)
        .filter(|s| ACTORS.contains(s))
        .collect();
    assert_eq!(actors.len(), ACTOR_COUNT, "every lineup row loads");
    for s in &actors {
        if !owner.contains_key(s) {
            errors.push(format!("lineup spawn {s} is in no group"));
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");

    let catalog = partition_spawn_sets(&mut records, defs);
    let at_boot: Vec<i32> = records
        .iter()
        .map(|r| r.spawn_id)
        .filter(|s| ACTORS.contains(s))
        .collect();
    assert!(
        at_boot.is_empty(),
        "lineup actors left to spawn at boot: {at_boot:?}"
    );
    assert_eq!(
        records
            .iter()
            .filter(|r| ATTENDANTS.contains(&r.spawn_id))
            .count(),
        ATTENDANTS.count(),
        "the attendants always spawn"
    );
    let held: usize = catalog
        .of_kind(KIND, WORLD_ID)
        .iter()
        .map(|s| s.records.len())
        .sum();
    assert_eq!(held, ACTOR_COUNT, "the catalog holds every actor");
    for set in catalog.iter() {
        assert!(!set.is_active(), "set {} is on at boot", set.set_id);
        assert_eq!(
            set.world_name.as_deref(),
            Some("DebugArea"),
            "set {}",
            set.set_id
        );
    }
}
