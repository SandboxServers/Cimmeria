//! Live-DB guards for the Castle return-hub quartermaster (CS-05a, CS-05S2a):
//! spawn 13804 and template 415 in World 8, the sealed stock on its lists,
//! and the absent hub medic (template 414, spawn 13803, OD-4).
//!
//! The quartermaster sells the heals from list 1300, the Debug Area list it
//! shares with template 1302. Its position is checked against every hostile
//! aggro reach, so a faction-10 spawn placed within reach of the hub fails
//! here. Revert proofs: set template 415's `interaction_type` to 0 and the buy
//! list to NULL by hand; each test below fails.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;
    use cimmeria_entity::interaction_flags::INT_VENDOR_GENERAL;

    /// The quartermaster's spawn and template ids (CS-05S2a, CD-6).
    const HUB_SPAWN: i32 = 13804;
    const HUB_TEMPLATE: i32 = 415;
    /// The sealed transform of spawn 13804 in World 8.
    const HUB_POS: [f32; 3] = [440.0, 70.22, 992.0];
    /// The sealed heading: the seed's 1.5708 is a quarter turn, within the
    /// test's 1e-4 tolerance of `FRAC_PI_2`.
    const HUB_HEADING: f32 = std::f32::consts::FRAC_PI_2;
    /// The quartermaster's buy list (1300) and its three service lists (2).
    const HUB_LISTS: (i32, i32, i32, i32) = (1300, 2, 2, 2);
    /// Rows the sealed stock on list 1300 holds (the Debug Area munitions).
    const STOCK_ROWS: i64 = 19;
    /// Horizontal slack between a hostile's reach and the quartermaster.
    const HUB_CLEARANCE: f32 = 5.0;

    /// `cimmeria_entity`'s aggro default, mirrored as `live_db_castle_population`
    /// does (cell-world depends on this crate).
    const DEFAULT_AGGRO_RADIUS: f32 = 18.0;

    /// Every point a hostile can stand on while idle: its spawn, plus its
    /// patrol loop sampled every tenth of a leg.
    fn idle_positions(r: &SpawnRecord) -> Vec<[f32; 3]> {
        let mut out = vec![[r.x, r.y, r.z]];
        let path = &r.patrol_path;
        for i in 0..path.len() {
            let (a, b) = (path[i], path[(i + 1) % path.len()]);
            for k in 0..=10 {
                let t = k as f32 / 10.0;
                out.push([
                    a.x + (b.x - a.x) * t,
                    a.y + (b.y - a.y) * t,
                    a.z + (b.z - a.z) * t,
                ]);
            }
        }
        out
    }

    #[tokio::test]
    async fn live_db_castle_hub_service_is_seeded_safe_and_stocked() {
        let pool = require_db_or_skip!();
        let spawns = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        let hub = spawns
            .iter()
            .find(|r| r.spawn_id == HUB_SPAWN)
            .expect("spawn 13804 must load");
        assert_eq!(hub.world_name, "Castle", "spawn 13804 is in World 8");
        assert_eq!(
            hub.template_id, HUB_TEMPLATE,
            "spawn 13804 uses template 415"
        );
        assert_eq!(
            [hub.x, hub.y, hub.z],
            HUB_POS,
            "spawn 13804 sits at the sealed transform"
        );
        assert!(
            (hub.heading - HUB_HEADING).abs() < 1e-4,
            "spawn 13804 heading {} != {HUB_HEADING}",
            hub.heading
        );
        assert!(hub.is_stationary, "the quartermaster never walks");

        assert_eq!(
            hub.interaction_type & INT_VENDOR_GENERAL,
            INT_VENDOR_GENERAL,
            "template 415 interaction_type must carry INT_VendorGeneral (65536); \
             cell-world derives NpcInteractionType::Vendor from this bit"
        );
        assert_eq!(hub.faction, Some(1), "the quartermaster is a friendly");

        let lists: (i32, i32, i32, i32) = sqlx::query_as(
            "SELECT buy_item_list, sell_item_list, repair_item_list, recharge_item_list \
               FROM resources.entity_templates WHERE template_id = $1",
        )
        .bind(HUB_TEMPLATE)
        .fetch_one(&pool)
        .await
        .expect("template 415 must load its lists");
        assert_eq!(
            lists, HUB_LISTS,
            "template 415 lists (buy, sell, repair, recharge)"
        );

        // No hostile (faction 10, World 8) may reach the quartermaster: its
        // aggro radius, its wander radius and the clearance together stay
        // short of the hub's position.
        let mut checked = 0;
        for h in spawns
            .iter()
            .filter(|r| r.world_name == "Castle" && r.faction == Some(10))
        {
            checked += 1;
            let reach =
                h.aggro_radius.unwrap_or(DEFAULT_AGGRO_RADIUS) + h.wander_radius + HUB_CLEARANCE;
            for p in idle_positions(h) {
                let d = ((p[0] - HUB_POS[0]).powi(2) + (p[2] - HUB_POS[2]).powi(2)).sqrt();
                assert!(
                    d > reach,
                    "hostile spawn {} ({:?}) reaches the quartermaster: {d:.2} u \
                     <= reach {reach:.2} u",
                    h.spawn_id,
                    h.tag
                );
            }
        }
        assert!(
            checked > 0,
            "no faction-10 World 8 spawn loaded; the probe is broken"
        );
    }

    #[tokio::test]
    async fn live_db_castle_quartermaster_lists_match_the_sealed_stock() {
        let pool = require_db_or_skip!();
        let stock: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resources.item_list_items WHERE item_list_id = 1300",
        )
        .fetch_one(&pool)
        .await
        .expect("list 1300 count");
        assert_eq!(stock, STOCK_ROWS, "list 1300 holds the sealed stock rows");

        // Each of template 415's lists is set, and none is the Debug Area's
        // list 1 (the cheap munitions list that must not be reused here).
        let lists: (Option<i32>, Option<i32>, Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT buy_item_list, sell_item_list, repair_item_list, recharge_item_list \
               FROM resources.entity_templates WHERE template_id = $1",
        )
        .bind(HUB_TEMPLATE)
        .fetch_one(&pool)
        .await
        .expect("template 415 lists");
        for (name, list) in [
            ("buy", lists.0),
            ("sell", lists.1),
            ("repair", lists.2),
            ("recharge", lists.3),
        ] {
            let list = list.unwrap_or_else(|| panic!("template 415 {name} list is NULL"));
            assert_ne!(list, 1, "template 415 {name} list must not be list 1");
        }
    }

    #[tokio::test]
    async fn live_db_castle_medic_is_absent() {
        let pool = require_db_or_skip!();
        let templates: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resources.entity_templates WHERE template_id = 414",
        )
        .fetch_one(&pool)
        .await
        .expect("template 414 count");
        assert_eq!(templates, 0, "the hub medic (template 414) waits on OD-4");

        let spawns: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM resources.spawnlist WHERE spawn_id = 13803")
                .fetch_one(&pool)
                .await
                .expect("spawn 13803 count");
        assert_eq!(spawns, 0, "the hub medic (spawn 13803) waits on OD-4");
    }
}
