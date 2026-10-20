//! UO-39: the Asgard trainer (template and spawn 63809, trainer list 1) and the
//! four Asgard Programs vendors T1-T4 (templates 63843-63846, spawns
//! 63824-63827, lists 63821-63824). D-UO8 B, D-UO30, D-UO38.
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * the trainer losing its trainer bit or its list 1 link, or picking up a
//!   shop list (the click then opens a vendor or nothing);
//! * list 1 offering no Asgard tree (the window opens bare and the server
//!   WARNs `trainer_empty_offering`);
//! * a program vendor without the weapons bit (D-UO38);
//! * an Asgard template still wearing the human body (D-UO37);
//! * a program list that breaks the D-UO38 price or tier rule;
//! * an Asgard spawn that stands on another Omega spawn or off its place.

mod live_db {
    use sqlx::Row;

    use super::super::live_db_union_omega_vendor_common::*;
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `INT_Trainer`.
    const INT_TRAINER: i64 = 128;
    /// `INT_VENDOR_WEAPONS`: Asgard programs carry the weapons bit (D-UO38).
    const INT_VENDOR_WEAPONS: i64 = 16384;
    const TRAINER: i32 = 63809;
    const TRAINER_NAME: i32 = 27218;
    const LIST: i32 = 1;
    const ASGARD_BODY: &str = "BS_Asgard.BS_Asgard";
    /// (template, moniker, list, tier)
    const PROGRAMS: [(i32, i32, i32, i32); 4] = [
        (63843, 27252, 63821, 1),
        (63844, 27253, 63822, 2),
        (63845, 27254, 63823, 3),
        (63846, 27255, 63824, 4),
    ];
    /// (spawn, template, tag)
    const SPAWNS: [(i32, i32, &str); 5] = [
        (63809, 63809, "UnionOmega_AsgardTrainer"),
        (63824, 63843, "UnionOmega_AsgardProgramsT1"),
        (63825, 63844, "UnionOmega_AsgardProgramsT2"),
        (63826, 63845, "UnionOmega_AsgardProgramsT3"),
        (63827, 63846, "UnionOmega_AsgardProgramsT4"),
    ];

    async fn archetype_index(pool: &sqlx::PgPool, label: &str) -> i32 {
        sqlx::query_scalar(
            "SELECT array_position(enum_range(NULL::resources.\"EArchetype\"), \
                    $1::resources.\"EArchetype\") - 1",
        )
        .bind(label)
        .fetch_one(pool)
        .await
        .expect("EArchetype index query")
    }

    /// The Asgard trainer is a placed trainer NPC: trainer bit, list 1, the
    /// Asgard moniker that resolves to text, and no shop, loot or ability set.
    /// Its list 1 offers the Asgard tree.
    #[tokio::test]
    async fn omega_asgard_trainer_serves_the_asgard_tree() {
        let pool = require_db_or_skip!();
        let row = sqlx::query(
            "SELECT t.class, t.faction, t.interaction_type, t.name_id, \
                    t.trainer_ability_list_id, t.buy_item_list, t.sell_item_list, \
                    t.repair_item_list, t.recharge_item_list, t.loot_table_id, \
                    t.ability_set_id, x.text \
               FROM resources.entity_templates t \
               LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
              WHERE t.template_id = $1",
        )
        .bind(TRAINER)
        .fetch_optional(&pool)
        .await
        .expect("entity_templates query")
        .unwrap_or_else(|| panic!("template {TRAINER} must be seeded"));

        let got = (
            row.get::<String, _>("class"),
            row.get::<Option<i32>, _>("faction"),
            row.get::<i64, _>("interaction_type"),
            row.get::<Option<i32>, _>("name_id"),
            row.get::<Option<i32>, _>("trainer_ability_list_id"),
            row.get::<Option<i32>, _>("buy_item_list"),
            row.get::<Option<i32>, _>("sell_item_list"),
            row.get::<Option<i32>, _>("repair_item_list"),
            row.get::<Option<i32>, _>("recharge_item_list"),
            row.get::<Option<i32>, _>("loot_table_id"),
            row.get::<Option<i32>, _>("ability_set_id"),
        );
        assert_eq!(
            got,
            (
                "mob".to_string(),
                Some(1),
                INT_TRAINER,
                Some(TRAINER_NAME),
                Some(LIST),
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            "template {TRAINER} (Asgard trainer) trainer columns"
        );
        let name: Option<String> = row.get("text");
        assert!(
            name.as_deref().is_some_and(|n| !n.trim().is_empty()),
            "template {TRAINER}: name_id {TRAINER_NAME} must resolve to non-empty text, got {name:?}"
        );

        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        assert_eq!(
            lists.get(&TRAINER),
            Some(&LIST),
            "the startup loader must map template {TRAINER} to list {LIST}"
        );

        let offered = load_trainer_abilities(&pool)
            .await
            .expect("load_trainer_abilities must succeed");
        let archetype = archetype_index(&pool, "ARCHETYPE_Asgard").await;
        let abilities = offered.get(&(LIST, archetype));
        assert!(
            abilities.is_some_and(|a| !a.is_empty()),
            "template {TRAINER}: list {LIST} must offer a tree to ARCHETYPE_Asgard (index {archetype}), got {abilities:?}"
        );
    }

    /// Each Asgard Programs vendor carries the weapons bit, its moniker, its
    /// own list and no trainer or static interaction (D-UO38).
    #[tokio::test]
    async fn omega_asgard_programs_carry_the_weapons_bit() {
        let pool = require_db_or_skip!();
        for (template, moniker, list, _) in PROGRAMS {
            assert_vendor_template(&pool, template, INT_VENDOR_WEAPONS, moniker, list).await;
        }
    }

    /// The trainer and the four program vendors wear the Asgard body with
    /// skin tint 0 (D-UO37), not the human body of template 1302.
    #[tokio::test]
    async fn omega_asgard_templates_wear_the_asgard_body() {
        let pool = require_db_or_skip!();
        let mut ids = vec![TRAINER];
        ids.extend(PROGRAMS.iter().map(|(template, _, _, _)| *template));
        let rows = sqlx::query(
            "SELECT template_id, body_set, skin_tint FROM resources.entity_templates \
              WHERE template_id = ANY($1)",
        )
        .bind(&ids)
        .fetch_all(&pool)
        .await
        .expect("entity_templates body query");
        assert_eq!(
            rows.len(),
            ids.len(),
            "templates {ids:?} must all be seeded"
        );
        for row in rows {
            let template: i32 = row.get("template_id");
            let got = (
                row.get::<Option<String>, _>("body_set"),
                row.get::<Option<i64>, _>("skin_tint"),
            );
            assert_eq!(
                got,
                (Some(ASGARD_BODY.to_string()), Some(0)),
                "template {template} must wear {ASGARD_BODY} with skin tint 0 (D-UO37)"
            );
        }
    }

    /// Each program list follows D-UO38: 200 x tech_comp, the tier's own
    /// programs, and 1 to 20 rows.
    #[tokio::test]
    async fn omega_asgard_programs_lists_follow_d_uo38() {
        let pool = require_db_or_skip!();
        for (_, _, list, tier) in PROGRAMS {
            let rows = load_list(&pool, list).await;
            assert_list(list, &rows, Price::Weapon, Some(tier));
        }
    }

    /// The trainer and each program vendor stand clear of every other Omega
    /// spawn and of every Omega respawner.
    #[tokio::test]
    async fn omega_asgard_trainer_and_programs_stand_clear() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }
}
