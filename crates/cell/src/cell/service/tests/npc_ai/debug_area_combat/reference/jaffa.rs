//! NC-12: the Staff Melee AA (effect 736, ability 710) and the Mob Strike
//! (effect 1717, ability 1433) carry damage. Both payloads are
//! RECONSTRUCTED from each ability's own description (D-NC2), not recovered
//! original behaviour; the seed comments say so.

mod live_db {
    use super::super::super::live_db::{fight_space, pass, set_pools, spawn};
    use super::super::super::PLAYER;
    use cimmeria_entity::stats::{FOCUS, HEALTH};
    use tokio::sync::mpsc;

    use crate::cell::spawner::{load_effect_defs, load_spawn_templates};
    use crate::test_support::require_db_or_skip;

    /// Effect 736 (Staff Melee AA) and 1717 (Mob Strike): the script is
    /// `MeleePhysicalDamage` (as effect 656 has) and the NVPs are
    /// reconstructed from the description text "-100F -10H" and "-125F -15H".
    /// Revert proof: putting
    /// `NULL` back in `effects.sql` or dropping the NVP rows fails here.
    #[tokio::test]
    async fn live_db_reference_jaffa_melee_effects_carry_damage() {
        let pool = require_db_or_skip!();
        let effects = load_effect_defs(&pool).await.expect("effect defs load");
        for (effect_id, health, focus) in [(736, "10", "100"), (1717, "15", "125")] {
            let def = &effects[&effect_id];
            assert_eq!(
                def.script_name.as_deref(),
                Some("MeleePhysicalDamage"),
                "effect {effect_id} is bound to the melee damage script"
            );
            assert_eq!(
                def.params.get("HealthDamage").map(String::as_str),
                Some(health),
                "effect {effect_id} HealthDamage"
            );
            assert_eq!(
                def.params.get("FocusDamage").map(String::as_str),
                Some(focus),
                "effect {effect_id} FocusDamage"
            );
        }
    }

    /// The Jaffa Guard (1380) with its Staff Melee AA set, two metres from
    /// the player at 1000/1000 pools: within three AI passes the player loses
    /// Health or Focus. Revert proof: dropping the NVP rows (470-473) leaves
    /// `MeleePhysicalDamage` with `no_damage_nvp` and no damage lands. A NULL
    /// script alone is caught by `live_db_reference_jaffa_melee_effects_carry_damage`,
    /// not by this test.
    #[tokio::test]
    async fn live_db_reference_jaffa_staff_swing_damages_in_reach() {
        let pool = require_db_or_skip!();
        let mut templates = load_spawn_templates(&pool).await.expect("templates load");
        templates
            .get_mut(&1380)
            .expect("seeded template 1380")
            .ability_ids = vec![710];
        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        let id = 300_000;
        spawn(&mut mgr, &templates, id, 1380, 2.0, 0.0);
        let _ = mgr.compute_aoi_changes();
        let (tx, mut rx) = mpsc::channel(65_536);
        for _ in 0..3 {
            pass(&mut mgr, &[id], &tx, &mut rx).await;
        }
        let p = mgr.get_entity(PLAYER).unwrap();
        let health = p.stats.get(HEALTH).unwrap().cur;
        let focus = p.stats.get(FOCUS).unwrap().cur;
        assert!(
            health < 1000 || focus < 1000,
            "the staff swing reached the player: health {health}, focus {focus}"
        );
    }
}
