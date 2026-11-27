//! The ScavDog reference (template 76, `DebugArea_Ref_ScavDog`, spawn 14302).
//! NC-13 gives it ability set 6 = {1433 Mob Strike} (D-NC5), RECONSTRUCTED from
//! the Tier-3 set link. It is melee only: it bites in reach and never knows
//! 592 Pistol Shot. Ability 983 Bite stays unassigned.

use cimmeria_cell_world::cell::combat::combat_profile::{
    resolve_npc_combat_profile, CombatProfileSource, PlaceholderPolicy,
};

use super::super::world_records;
use super::ref_row;

/// The ScavDog row holds set 6 alone: [1433] resolves `ExplicitSet` under both
/// placeholder policies, and 592 is not among the abilities. Revert proof:
/// a NULL set on template 76 leaves the row empty, so the profile falls back
/// to Pistol Shot and this fails.
#[test]
fn scavdog_reference_is_melee_only() {
    let records = world_records();
    let dog = ref_row(&records, "DebugArea_Ref_ScavDog");
    assert_eq!(dog.template_id, 76, "the ScavDog is template 76");
    assert_eq!(dog.ability_ids, vec![1433], "the ScavDog knows Mob Strike");
    for policy in [
        PlaceholderPolicy::Legacy,
        PlaceholderPolicy::RequireWeaponEvidence,
    ] {
        let p = resolve_npc_combat_profile(&dog.ability_ids, None, policy);
        assert_eq!(p.source, CombatProfileSource::ExplicitSet, "{policy:?}");
        assert_eq!(p.abilities, vec![1433], "{policy:?}");
        assert!(!p.abilities.contains(&592), "{policy:?}: no Pistol Shot");
    }
}

mod live_db {
    use cimmeria_entity::stats::{FOCUS, HEALTH};
    use tokio::sync::mpsc;

    use super::super::super::live_db::{fight_space, pass, set_pools, spawn};
    use super::super::super::PLAYER;
    use crate::cell::spawner::load_spawn_templates;
    use crate::test_support::require_db_or_skip;

    /// The ScavDog (76) at 15 m is out of Mob Strike's reach: it closes or
    /// holds, and the player is untouched after two passes. Placed at 2 m, it
    /// bites within three passes. Through both fights it never knows 592.
    /// Revert proof: a NULL set on template 76 leaves [592], which fires at
    /// 15 m, so the first assertion fails.
    #[tokio::test]
    async fn live_db_reference_scavdog_bites_in_reach_only() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates load");
        const NPC: u32 = 300_200;
        let (tx, mut rx) = mpsc::channel(65_536);

        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        spawn(&mut mgr, &templates, NPC, 76, 15.0, 0.0);
        let _ = mgr.compute_aoi_changes();
        for _ in 0..2 {
            pass(&mut mgr, &[NPC], &tx, &mut rx).await;
            let known = mgr.get_entity(NPC).unwrap().abilities.known_ability_ids();
            assert!(!known.contains(&592), "the ScavDog knows {known:?}");
        }
        let p = mgr.get_entity(PLAYER).unwrap();
        let (health, focus) = (
            p.stats.get(HEALTH).unwrap().cur,
            p.stats.get(FOCUS).unwrap().cur,
        );
        assert_eq!((health, focus), (1000, 1000), "no bite lands at 15 m");

        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        spawn(&mut mgr, &templates, NPC, 76, 2.0, 0.0);
        let _ = mgr.compute_aoi_changes();
        for _ in 0..3 {
            pass(&mut mgr, &[NPC], &tx, &mut rx).await;
            let known = mgr.get_entity(NPC).unwrap().abilities.known_ability_ids();
            assert!(!known.contains(&592), "the ScavDog knows {known:?}");
        }
        let p = mgr.get_entity(PLAYER).unwrap();
        let (health, focus) = (
            p.stats.get(HEALTH).unwrap().cur,
            p.stats.get(FOCUS).unwrap().cur,
        );
        assert!(
            health < 1000 || focus < 1000,
            "the bite reached the player at 2 m: health {health}, focus {focus}"
        );
    }

    /// Ability set 6 in the seed is exactly Mob Strike (1433).
    /// Revert proof: dropping the `(6, 1433)` row fails here.
    #[tokio::test]
    async fn live_db_reference_scavdog_set_6_loads() {
        let pool = require_db_or_skip!();
        let abilities: Vec<i32> = sqlx::query_scalar(
            "SELECT ability_id FROM resources.ability_set_abilities \
             WHERE ability_set_id = 6 ORDER BY ability_id",
        )
        .fetch_all(&pool)
        .await
        .expect("set 6 query");
        assert_eq!(abilities, vec![1433], "ability set 6 is Mob Strike alone");
    }
}
