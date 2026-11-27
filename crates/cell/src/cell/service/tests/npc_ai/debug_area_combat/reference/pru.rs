//! NC-14: the Prisoner Retrieval Unit (PRU, template 4) reference at Z12.
//! The PRU is the one reference that fires an integrated weapon: its
//! explicit ability set (221 Energy Shock) needs no `WP-` evidence under
//! either placeholder policy. These tests pin current behaviour; a failure
//! is a finding, not a fix. The CellBlock scripted activation is NC-23's job.

use super::{ref_row, world_records};
use cimmeria_cell_world::cell::combat::combat_profile::{
    resolve_npc_combat_profile, CombatProfileSource, PlaceholderPolicy, WeaponEvidence,
};

/// The PRU has an explicit ability set, so the profile is `ExplicitSet` with
/// no weapon evidence under both policies. Revert proof: if the row loses
/// 221, or the resolver starts requiring `WP-` evidence for an explicit set,
/// this fails.
#[test]
fn pru_reference_has_integrated_weapon() {
    let records = world_records();
    let row = ref_row(&records, "DebugArea_Ref_PRU");
    assert_eq!(row.template_id, 4, "the PRU is template 4");
    assert_eq!(row.ability_ids, vec![221], "the PRU fires Energy Shock");
    for policy in [
        PlaceholderPolicy::Legacy,
        PlaceholderPolicy::RequireWeaponEvidence,
    ] {
        let profile =
            resolve_npc_combat_profile(&row.ability_ids, row.components.as_deref(), policy);
        assert_eq!(
            profile.source,
            CombatProfileSource::ExplicitSet,
            "{policy:?}"
        );
        assert_eq!(profile.abilities, vec![221], "{policy:?}");
        assert_eq!(
            profile.weapon_evidence,
            WeaponEvidence::None,
            "{policy:?}: the PRU carries no WP- mesh"
        );
    }
}

mod live_db {
    use super::super::super::live_db::{fight_space, pass, set_pools, spawn};
    use super::super::super::PLAYER;
    use crate::cell::spawner::{load_ability_defs, load_effect_defs, load_spawn_templates};
    use crate::test_support::require_db_or_skip;
    use cimmeria_entity::stats::{FOCUS, HEALTH};
    use tokio::sync::mpsc;

    /// Ability 221 is ranged and fires effect 264; effect 264 is
    /// `RangedEnergyDamage` with HealthDamage 16 and FocusDamage 80. Revert
    /// proof: a changed script, NVP or ability link fails here by name.
    #[tokio::test]
    async fn live_db_reference_pru_energy_shock_defs() {
        let pool = require_db_or_skip!();
        let abilities = load_ability_defs(&pool).await.expect("ability defs");
        let effects = load_effect_defs(&pool).await.expect("effect defs");
        let ability = abilities.get(&221).expect("ability 221 is seeded");
        assert!(ability.is_ranged, "221 Energy Shock is ranged");
        assert!(
            ability.effect_ids.contains(&264),
            "221 fires effect 264: {:?}",
            ability.effect_ids
        );
        let effect = effects.get(&264).expect("effect 264 is seeded");
        assert_eq!(effect.script_name.as_deref(), Some("RangedEnergyDamage"));
        assert_eq!(effect.param_i32("HealthDamage"), 16);
        assert_eq!(effect.param_i32("FocusDamage"), 80);
    }

    /// A PRU at x = 10 fires Energy Shock at a player with 1000/1000 pools:
    /// over up to four AI passes the player loses Health plus Focus. Revert
    /// proof: dropping the PRU's ability set gives it the 592 fallback and
    /// fails the known-abilities assert; removing effect 264 leaves both pools
    /// full.
    #[tokio::test]
    async fn live_db_reference_pru_shock_damages_a_player() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates");
        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        const NPC: u32 = 300_400;
        spawn(&mut mgr, &templates, NPC, 4, 10.0, 0.0);
        assert_eq!(
            mgr.get_entity(NPC).unwrap().abilities.known_ability_ids(),
            vec![221],
            "the PRU fires only Energy Shock"
        );
        let _ = mgr.compute_aoi_changes();
        let (tx, mut rx) = mpsc::channel(65_536);
        for _ in 0..4 {
            pass(&mut mgr, &[NPC], &tx, &mut rx).await;
        }
        let p = mgr.get_entity(PLAYER).unwrap();
        let health = p.stats.get(HEALTH).unwrap().cur;
        let focus = p.stats.get(FOCUS).unwrap().cur;
        assert!(
            health + focus < 2000,
            "the PRU's shock must land; player left with {health} Health, {focus} Focus"
        );
    }
}
