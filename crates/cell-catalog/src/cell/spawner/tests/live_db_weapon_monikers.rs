//! Live-DB guards for the data the player weapon requirement reads
//! (Class Start v6 CS-07, OD-CS11): `abilities.item_monikers` loads into
//! `AbilityDef`, and `items.moniker_ids` loads per item.
mod live_db {
    use crate::cell::spawner::{load_ability_defs, load_weapon_monikers};
    use crate::test_support::require_db_or_skip;

    const ITEM_PISTOL: i64 = 2_445_422_768;
    const ITEM_AUTOMATIC_WEAPON: i64 = 3_175_425_141;
    const ITEM_LIGHT_MG: i64 = 1_115_110_575;
    const ITEM_STAFF: i64 = 1_383_013_887;
    const ITEM_RIBBON_DEVICE: i64 = 4_193_235_610;

    /// The starter rows load their requirement. Revert proof: with the
    /// loader not selecting `item_monikers` every list is empty.
    #[tokio::test]
    async fn seeded_ability_item_monikers_load() {
        let pool = require_db_or_skip!();
        let defs = load_ability_defs(&pool).await.expect("ability defs load");
        for (id, what, monikers) in [
            (592, "Pistol Shot", vec![ITEM_PISTOL]),
            (598, "Quick Burst", vec![ITEM_AUTOMATIC_WEAPON]),
            (1984, "Staff Swing", vec![ITEM_STAFF]),
            (
                1639,
                "Ribbon Device:Destruction Beam",
                vec![ITEM_RIBBON_DEVICE],
            ),
        ] {
            let def = defs
                .get(&id)
                .unwrap_or_else(|| panic!("ability {id} ({what}) must be seeded"));
            assert_eq!(def.item_monikers, monikers, "{id} {what}");
        }
        // An ability with no requirement loads an empty list.
        assert!(
            defs.get(&597).is_some_and(|d| d.item_monikers.is_empty()),
            "597 Heal Focus has no weapon requirement"
        );
    }

    /// The starter weapons load their monikers, and the SK37 LMG (3260)
    /// carries ITEM_LightMG but not ITEM_Automatic_Weapon.
    #[tokio::test]
    async fn seeded_item_monikers_load() {
        let pool = require_db_or_skip!();
        let map = load_weapon_monikers(&pool)
            .await
            .expect("item monikers load");
        let has = |item: i32, moniker: i64| map.get(&item).is_some_and(|m| m.contains(&moniker));
        assert!(has(55, ITEM_PISTOL), "55 SI 3 9mm Pistol");
        assert!(has(21, ITEM_AUTOMATIC_WEAPON), "21 SGHC 6 SMG");
        assert!(has(3260, ITEM_LIGHT_MG), "3260 SK37 LMG");
        assert!(
            !has(3260, ITEM_AUTOMATIC_WEAPON),
            "3260 is not an automatic weapon"
        );
        assert!(has(2797, ITEM_STAFF), "2797 Serpent Staff");
        assert!(has(4565, ITEM_RIBBON_DEVICE), "4565 Serpent Ribbon Device");
    }
}
