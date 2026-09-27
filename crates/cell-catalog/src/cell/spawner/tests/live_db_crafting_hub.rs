//! Live-DB guards for the crafting corner of the stasis-room debug hub:
//! stations 310-313, the crafting supplies vendor 314, spawns 410-414 and
//! buy list 310 (`docs/content/debug-hub.md`).
//!
//! The seams, each of which loads without error and is still wrong:
//!
//! * a station without all four `ENTITYFLAG_Craft_*` bits leaves a crafting
//!   page "Disabled" beside it, because the station tick reports a station
//!   per verb from those bits alone;
//! * a station with an interaction bit gives the tester a cursor that
//!   nothing on the server answers;
//! * a spawn outside the stasis room, on the respawn spot, or on top of
//!   another hub NPC;
//! * a supplies list that lost a UAT item, names an item that does not
//!   exist, or costs anything but 1 naquadah.
//!
//! Each was proven to fail with the seed rows changed.
mod live_db {
    use std::collections::BTreeSet;

    use sqlx::Row;

    use crate::cell::spawner::*;
    use crate::crafting::{
        ENTITYFLAG_CRAFT_ALLOYING, ENTITYFLAG_CRAFT_CRAFT, ENTITYFLAG_CRAFT_RESEARCH,
        ENTITYFLAG_CRAFT_REV_ENG,
    };
    use crate::test_support::require_db_or_skip;

    const ALL_CRAFT_FLAGS: i64 = (ENTITYFLAG_CRAFT_CRAFT
        | ENTITYFLAG_CRAFT_RESEARCH
        | ENTITYFLAG_CRAFT_REV_ENG
        | ENTITYFLAG_CRAFT_ALLOYING) as i64;

    /// `INT_VendorGeneral`.
    const INT_VENDOR_GENERAL: i64 = 65536;

    const SUPPLIES: i32 = 314;
    const SUPPLIES_LIST: i32 = 310;

    /// `(template_id, name_id, the station's name)`.
    const STATIONS: [(i32, i32, &str); 4] = [
        (310, 27180, "BioMedical Crafting Station"),
        (311, 27182, "Electronics Crafting Station"),
        (312, 27184, "Power Systems Crafting Station"),
        (313, 27186, "Materials Crafting Station"),
    ];

    /// `(spawnlist.tag, template_id)` for every crafting-hub spawn.
    const SPAWNS: [(&str, i32); 5] = [
        ("CraftHub_Station_BioMedical", 310),
        ("CraftHub_Station_Electronics", 311),
        ("CraftHub_Station_PowerSystems", 312),
        ("CraftHub_Station_Materials", 313),
        ("CraftHub_Supplies", SUPPLIES),
    ];

    /// Everything buy list 310 sells: the UAT recipe components (5254,
    /// 5256, 5401, 5192, 5189), the research target 5481, the four kickers,
    /// the -5 and -50 Field Crafting Tool of each science, the five Racial
    /// Paradigm Guides and Blueprint item 6483.
    const SUPPLIES_ITEMS: [i32; 24] = [
        5254, 5256, 5401, 5192, 5189, 5481, 5668, 5669, 5670, 5671, 5369, 8415, 8402, 8441, 8405,
        8461, 8406, 8451, 7805, 7806, 7807, 7808, 7809, 6483,
    ];

    const ROOM: &str = "Castle_Cellblock.Region1";
    const STASIS_RESPAWNER: i32 = 8;
    const MIN_RESPAWNER_CLEARANCE: f32 = 5.0;
    const MIN_NPC_SPACING: f32 = 2.5;
    const FLOOR_Y: f32 = 73.472;

    fn xz_distance(a: &SpawnRecord, b: [f32; 3]) -> f32 {
        ((a.x - b[0]).powi(2) + (a.z - b[2]).powi(2)).sqrt()
    }

    /// The stations carry every craft bit and no interaction bit, and are
    /// named by the client's own station monikers; the vendor is a
    /// vendor-only NPC selling list 310 and is not a station.
    #[tokio::test]
    async fn crafting_hub_templates_are_stations_and_a_vendor() {
        let pool = require_db_or_skip!();
        let rows = sqlx::query(
            "SELECT t.template_id, t.flags::int8 AS flags, t.interaction_type, t.class, \
                    t.static_mesh, t.name_id, x.text, x.moniker_name, t.buy_item_list, \
                    t.sell_item_list, t.repair_item_list, t.recharge_item_list \
             FROM resources.entity_templates t \
             LEFT JOIN resources.texts x ON x.moniker_id = t.name_id AND x.language = 1033 \
             WHERE t.template_id BETWEEN 310 AND 314 ORDER BY t.template_id",
        )
        .fetch_all(&pool)
        .await
        .expect("entity_templates query must succeed");
        assert_eq!(rows.len(), 5, "templates 310-314 must all be seeded");

        for (row, (template_id, name_id, name)) in rows.iter().zip(STATIONS) {
            assert_eq!(row.get::<i32, _>("template_id"), template_id);
            let flags: i64 = row.get("flags");
            assert_eq!(
                flags & ALL_CRAFT_FLAGS,
                ALL_CRAFT_FLAGS,
                "station {template_id} must carry every ENTITYFLAG_Craft_* bit"
            );
            assert_eq!(
                row.get::<i64, _>("interaction_type"),
                0,
                "station {template_id}: no server handler answers a click on a station"
            );
            assert_eq!(row.get::<String, _>("class"), "being");
            assert!(row.get::<Option<String>, _>("static_mesh").is_some());
            assert_eq!(row.get::<Option<i32>, _>("name_id"), Some(name_id));
            assert_eq!(
                row.get::<Option<String>, _>("text").as_deref(),
                Some(name),
                "station {template_id}'s moniker must read as its science's station"
            );
            assert!(row
                .get::<Option<String>, _>("moniker_name")
                .is_some_and(|m| m.starts_with("DN_Cft_Ob_CraftingStation_")));
            assert_eq!(row.get::<Option<i32>, _>("buy_item_list"), None);
        }

        let vendor = &rows[4];
        assert_eq!(vendor.get::<i32, _>("template_id"), SUPPLIES);
        assert_eq!(
            vendor.get::<i64, _>("flags") & ALL_CRAFT_FLAGS,
            0,
            "the vendor is not a station"
        );
        assert_eq!(vendor.get::<i64, _>("interaction_type"), INT_VENDOR_GENERAL);
        assert_eq!(
            [
                vendor.get::<Option<i32>, _>("buy_item_list"),
                vendor.get::<Option<i32>, _>("sell_item_list"),
                vendor.get::<Option<i32>, _>("repair_item_list"),
                vendor.get::<Option<i32>, _>("recharge_item_list"),
            ],
            [Some(SUPPLIES_LIST), None, None, None]
        );
        assert_eq!(vendor.get::<Option<i32>, _>("name_id"), Some(27239));
    }

    /// Every crafting-hub spawn is a world 12 spawn inside the stasis room,
    /// on its floor, clear of the respawner and of every other spawn in the
    /// room (the rest of the hub included).
    #[tokio::test]
    async fn crafting_hub_spawns_sit_inside_the_stasis_room() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");
        let respawners = load_respawners(&pool)
            .await
            .expect("load_respawners must succeed");
        let room = regions
            .iter()
            .find(|r| r.name == ROOM)
            .unwrap_or_else(|| panic!("point set {ROOM} must be loaded"));
        let respawner = respawners
            .iter()
            .find(|r| r.respawner_id == STASIS_RESPAWNER)
            .expect("respawner 8 'Stasis Chamber' must be seeded");

        let in_room: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| {
                r.world_name == "Castle_CellBlock" && region_contains_xz(&room.points, r.x, r.z)
            })
            .collect();
        for (tag, template_id) in SPAWNS {
            let matches: Vec<&SpawnRecord> = records
                .iter()
                .filter(|r| r.tag.as_deref() == Some(tag))
                .collect();
            assert_eq!(matches.len(), 1, "tag {tag} must resolve to one spawn");
            let s = matches[0];
            assert_eq!(s.template_id, template_id, "{tag}");
            assert_eq!(
                s.world_name, "Castle_CellBlock",
                "{tag} must be in world 12"
            );
            assert!(
                region_contains_xz(&room.points, s.x, s.z),
                "{tag} at ({}, {}) must stand inside {ROOM}",
                s.x,
                s.z
            );
            assert!((s.y - FLOOR_Y).abs() < 0.5, "{tag} must stand on the floor");
            let clearance = xz_distance(s, respawner.pos);
            assert!(
                clearance >= MIN_RESPAWNER_CLEARANCE,
                "{tag} is {clearance:.2} from the respawner"
            );
            for other in in_room.iter().filter(|o| o.tag != s.tag) {
                let d = xz_distance(s, [other.x, other.y, other.z]);
                assert!(
                    d >= MIN_NPC_SPACING,
                    "{tag} and {:?} are {d:.2} units apart",
                    other.tag
                );
            }
        }
    }

    /// Buy list 310 sells exactly the UAT supplies, each a real item at
    /// quantity 1 for 1 naquadah and no item cost, and it covers set 1 of
    /// the UAT blueprints 25, 412 and 161, the alloy input of 42, and the
    /// Blueprint item that teaches 25.
    #[tokio::test]
    async fn crafting_supplies_list_resolves_at_one_naquadah() {
        let pool = require_db_or_skip!();
        let rows = sqlx::query(
            "SELECT ili.item_id, ili.design_id, ili.quantity, ili.naquadah, i.item_id AS resolved \
             FROM resources.item_list_items ili \
             LEFT JOIN resources.items i ON i.item_id = ili.design_id \
             WHERE ili.item_list_id = $1 ORDER BY ili.item_id",
        )
        .bind(SUPPLIES_LIST)
        .fetch_all(&pool)
        .await
        .expect("item_list_items query must succeed");
        let designs: Vec<i32> = rows.iter().map(|r| r.get("design_id")).collect();
        assert_eq!(
            designs.iter().copied().collect::<BTreeSet<_>>(),
            SUPPLIES_ITEMS.iter().copied().collect::<BTreeSet<_>>(),
            "buy list 310 must sell exactly the UAT supplies"
        );
        assert_eq!(designs.len(), SUPPLIES_ITEMS.len(), "no duplicate rows");
        for r in &rows {
            let design: i32 = r.get("design_id");
            assert_eq!(r.get::<Option<i32>, _>("resolved"), Some(design));
            assert_eq!(r.get::<i32, _>("quantity"), 1, "{design}");
            assert_eq!(r.get::<i32, _>("naquadah"), 1, "{design}");
        }
        let list_item_ids: Vec<i32> = rows.iter().map(|r| r.get("item_id")).collect();
        let costs: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.item_list_prices WHERE item_id = ANY($1)",
        )
        .bind(&list_item_ids)
        .fetch_one(&pool)
        .await
        .expect("item_list_prices query must succeed");
        assert_eq!(costs, 0, "no supplies row may cost an item");

        let needed: Vec<i32> = sqlx::query_scalar(
            "SELECT DISTINCT item_id FROM resources.blueprints_components \
             WHERE (blueprint_id IN (25, 412, 161) AND component_set_id = 1) \
                OR blueprint_id = 42 ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .expect("blueprints_components query must succeed");
        assert!(!needed.is_empty());
        for item in needed {
            assert!(designs.contains(&item), "UAT component {item} is not sold");
        }
        let teaches_25: Vec<i32> = sqlx::query_scalar(
            "SELECT item_id FROM resources.crafting_item_effects WHERE blueprint_id = 25",
        )
        .fetch_all(&pool)
        .await
        .expect("crafting_item_effects query must succeed");
        assert!(
            teaches_25.iter().any(|i| designs.contains(i)),
            "an item that teaches blueprint 25 must be sold"
        );
    }
}
