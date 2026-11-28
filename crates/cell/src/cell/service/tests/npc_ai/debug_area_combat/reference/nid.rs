//! The NID Guard reference (template 24, `DebugArea_Ref_NID`, spawn 14300): the
//! SMG evidence (`WP-Human.WP_SMG_1A`), the cover take, and the live fight and
//! loot/respawn rows. These pin current behaviour; a failure is a finding, not a
//! fix (`docs/analysis/npc-combat-reference/work-packets-w1-w2.md` NC-11).

use cimmeria_cell_world::cell::combat::combat_profile::{
    resolve_npc_combat_profile, CombatProfileSource, PlaceholderPolicy, WeaponEvidence,
};

use super::super::live_db::{fight_space, pass, set_pools, spawn};
use super::super::{opt, seed_rows, text, world_records, PLAYER};
use super::{ref_row, REF_SPAWNS};

/// The `components` column of template `template_id` from the entity template
/// seeds: `world_records` leaves components unparsed.
fn template_components(template_id: i32) -> Vec<String> {
    let row = seed_rows(
        "db/resources/Entities/Seed",
        "entity_templates",
        "entity_templates",
    )
    .into_iter()
    .find(|t| opt::<i32>(t, "template_id") == Some(template_id))
    .unwrap_or_else(|| panic!("template {template_id} is seeded"));
    text(&row, "components")
        .expect("components column")
        .trim_matches(|c| c == '{' || c == '}')
        .split(',')
        .map(str::to_string)
        .collect()
}

/// The NID row is template 24 and its template carries `WP-Human.WP_SMG_1A`.
/// An explicit set needs no weapon evidence, so under both placeholder policies
/// the profile is `ExplicitSet` [559] with the SMG as its evidence. Revert
/// proof: dropping the SMG from template 24 or the 559 from its set fails here.
#[test]
fn nid_reference_carries_smg_evidence() {
    let records = world_records();
    let nid = ref_row(&records, "DebugArea_Ref_NID");
    assert_eq!(nid.template_id, 24, "the NID Guard is template 24");
    let components = template_components(nid.template_id);
    for policy in [
        PlaceholderPolicy::Legacy,
        PlaceholderPolicy::RequireWeaponEvidence,
    ] {
        let p = resolve_npc_combat_profile(&nid.ability_ids, Some(&components), policy);
        assert_eq!(p.source, CombatProfileSource::ExplicitSet, "{policy:?}");
        assert_eq!(p.abilities, vec![559], "{policy:?}");
        assert_eq!(
            p.weapon_evidence,
            WeaponEvidence::Component("WP-Human.WP_SMG_1A".to_string()),
            "{policy:?}"
        );
    }
}

/// The NID row takes cover (`use_cover`), as the riflemen do.
/// Revert proof: clearing template 24's `use_cover` fails here.
#[test]
fn nid_reference_takes_cover() {
    let records = world_records();
    let nid = ref_row(&records, "DebugArea_Ref_NID");
    assert_eq!(nid.use_cover, Some(true), "the NID Guard takes cover");
}

mod live_db {
    use super::*;
    use cimmeria_entity::stats::{FOCUS, HEALTH};
    use tokio::sync::mpsc;

    use crate::cell::spawner::{
        load_ability_defs, load_effect_defs, load_spawn_templates, load_spawns_from_db, SpawnRecord,
    };
    use crate::test_support::require_db_or_skip;

    /// The NID Guard's SMG damages a player 10 m away: with pools of 1000 and
    /// 1000, four AI passes take Health plus Focus down, and the NPC knows only
    /// 559. Revert proof: dropping the SMG effect or its ability set fails here.
    #[tokio::test]
    async fn live_db_reference_nid_smg_damages_a_player() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates");
        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        const NPC: u32 = 300_100;
        spawn(&mut mgr, &templates, NPC, 24, 10.0, 0.0);
        let _ = mgr.compute_aoi_changes();
        let (tx, mut rx) = mpsc::channel(65_536);
        for _ in 0..4 {
            pass(&mut mgr, &[NPC], &tx, &mut rx).await;
        }
        let p = mgr.get_entity(PLAYER).unwrap();
        let (health, focus) = (
            p.stats.get(HEALTH).unwrap().cur,
            p.stats.get(FOCUS).unwrap().cur,
        );
        assert!(
            health + focus < 2000,
            "the SMG must land: {health} Health, {focus} Focus left"
        );
        assert_eq!(
            mgr.get_entity(NPC).unwrap().abilities.known_ability_ids(),
            vec![559],
            "the NID Guard knows only 559"
        );
    }

    /// The Ref NID row, as the loader builds it, resolves loot table 2 (from
    /// template 24) and a 20 s respawn (from the spawn row).
    /// Revert proof: dropping the spawn's `respawn_secs` fails here.
    #[tokio::test]
    async fn live_db_reference_nid_loot_and_respawn() {
        let pool = require_db_or_skip!();
        let rows: Vec<SpawnRecord> = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| REF_SPAWNS.contains(&r.spawn_id))
            .collect();
        let nid = ref_row(&rows, "DebugArea_Ref_NID");
        assert_eq!(nid.loot_table_id, Some(2), "the NID Guard's loot table");
        assert_eq!(nid.respawn_secs, Some(20), "the NID Guard respawns in 20 s");
    }

    /// Ability 559 (the NID's SMG auto attack) executes its own effect 5310, the
    /// `-100F -10H` of its tooltip (RECONSTRUCTED, D-NC8, D-NC11). Giza effect
    /// 621 (`-200F -20H`) is kept unchanged on purpose and no ability uses it.
    /// Revert proofs: (i) `'{621}'` back in `abilities.sql` fails on
    /// `effect_ids`; (ii) nvp 17 set to `'100'` (a rewrite of 621) fails on 621.
    #[tokio::test]
    async fn live_db_reference_nid_559_uses_reconstructed_effect_5310() {
        let pool = require_db_or_skip!();
        let desc = sqlx::query_scalar::<_, String>(
            "SELECT description FROM resources.abilities WHERE ability_id = 559",
        )
        .fetch_one(&pool)
        .await
        .expect("ability 559 is seeded");
        let lines: Vec<&str> = desc.lines().map(str::trim).collect();
        let num = |suffix: char| -> i32 {
            lines
                .iter()
                .find_map(|l| l.strip_prefix('-')?.strip_suffix(suffix))
                .unwrap_or_else(|| panic!("559's tooltip has a -<n>{suffix} line"))
                .parse()
                .expect("a number")
        };
        let (focus, health) = (num('F'), num('H'));
        assert_eq!((focus, health), (100, 10), "559's tooltip");

        let abilities = load_ability_defs(&pool).await.expect("ability defs load");
        let effects = load_effect_defs(&pool).await.expect("effect defs load");
        assert_eq!(abilities[&559].effect_ids, vec![5310], "559 fires 5310");
        let e = &effects[&5310];
        assert_eq!(e.ability_id, 559, "effect 5310 belongs to 559");
        assert_eq!(e.script_name.as_deref(), Some("RangedPhysicalDamage"));
        assert_eq!(e.event_set_id, Some(289), "effect 5310's event set");
        assert_eq!(e.param_i32("FocusDamage"), focus, "5310 Focus = tooltip -F");
        assert_eq!(
            e.param_i32("HealthDamage"),
            health,
            "5310 Health = tooltip -H"
        );

        let giza = &effects[&621];
        assert_eq!(giza.param_i32("FocusDamage"), 200, "Giza 621 is kept");
        assert_eq!(giza.param_i32("HealthDamage"), 20, "Giza 621 is kept");
        assert_eq!(giza.script_name.as_deref(), Some("RangedPhysicalDamage"));
    }
}
