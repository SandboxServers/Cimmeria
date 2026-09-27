//! Tests for the Field Crafting Tool rule (D-CR21).

use super::*;
use crate::test_support::require_db_or_skip;

fn discipline(applied_science_id: i32, tech_competency: i32) -> Discipline {
    Discipline {
        discipline_id: 21,
        applied_science_id,
        racial_paradigm_id: 1,
        racial_paradigm_level: 1,
        tech_competency,
        required_discipline_ids: vec![],
        name: "test".into(),
    }
}

#[test]
fn prefixes_map_to_the_seeded_applied_sciences() {
    let science = |name| classify_tool(name, 5).map(|t| t.applied_science_id);
    assert_eq!(science("BMAS-5 Field Crafting Tool"), Some(1));
    assert_eq!(science("MAS-10 Field Crafting Tool"), Some(2));
    assert_eq!(science("PSAS-15 Field Crafting Tool"), Some(3));
    assert_eq!(science("EAS-55 Field Crafting Tool"), Some(4));
    assert_eq!(science("Efficient PSAS-35 Field Crafting Tool"), Some(3));
}

/// `MAS` is a suffix of `BMAS` and `PSAS`; the whole prefix must match.
#[test]
fn prefix_match_is_exact() {
    assert_eq!(classify_tool("XMAS-5 Field Crafting Tool", 5), None);
    assert_eq!(classify_tool("BMAS Field Crafting Tool", 5), None);
    assert_eq!(classify_tool("BMAS-5 Field Crafting Kit", 5), None);
    assert_eq!(
        classify_tool("Blueprint: BMAS-5 Field Crafting Tool", 5),
        None
    );
}

/// The tool's own `tech_comp` column decides its reach, not the number in
/// its name (item 8407 is "BMAS-10" with `tech_comp` 5).
#[test]
fn tech_comp_comes_from_the_column() {
    let tool = classify_tool("BMAS-10 Field Crafting Tool", 5).unwrap();
    assert_eq!(tool.tech_comp, 5);
    assert!(tool.covers(&discipline(1, 5)));
    assert!(!tool.covers(&discipline(1, 10)));
}

#[test]
fn a_tool_covers_its_science_up_to_its_tech_comp() {
    let tool = ToolSpec {
        applied_science_id: 3,
        tech_comp: 25,
    };
    assert!(tool.covers(&discipline(3, 1)));
    assert!(tool.covers(&discipline(3, 25)), "equal is covered");
    assert!(
        !tool.covers(&discipline(3, 26)),
        "above the tool's tech_comp"
    );
    assert!(!tool.covers(&discipline(1, 1)), "another science");
}

#[test]
fn only_tools_in_the_crafting_bag_count() {
    let table = HashMap::from([(
        8405,
        ToolSpec {
            applied_science_id: 3,
            tech_comp: 5,
        },
    )]);
    let rows = [
        (20_001, 8405, 1),  // tool in the main bag
        (20_002, 8405, 15), // tool in the crafting bag
        (20_003, 1234, 15), // something else in the crafting bag
        (20_004, 8405, 3),  // tool on the bandolier
    ];
    let tools = tools_in_crafting_bag(&table, rows);
    assert_eq!(
        tools,
        vec![HeldTool {
            instance_id: 20_002,
            spec: table[&8405],
        }]
    );
}

#[test]
fn best_tool_is_highest_tech_comp_then_lowest_instance() {
    let tool = |instance_id, tech_comp| HeldTool {
        instance_id,
        spec: ToolSpec {
            applied_science_id: 1,
            tech_comp,
        },
    };
    assert_eq!(best_tool(&[]), None);
    let tools = [tool(30, 10), tool(20, 35), tool(10, 35)];
    assert_eq!(best_tool(&tools), Some(&tool(10, 35)));
}

/// Against the seed: all 48 tools classify, and every prefix agrees with
/// the item's description ("Bio-Medical Engineering Field Crafting Tool",
/// ...), the only other place the science is written down.
#[tokio::test]
async fn every_seeded_tool_classifies_and_matches_its_description() {
    let pool = require_db_or_skip!();
    let table = tool_table(&pool).await.expect("tool table");
    assert_eq!(table.len(), 48, "audit C-26: 48 Field Crafting Tools");

    let rows: Vec<(i32, String, i32)> = sqlx::query_as(
        "SELECT item_id, description, tech_comp FROM resources.items \
         WHERE name LIKE '%Field Crafting Tool'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 48);
    for (item_id, description, tech_comp) in rows {
        let spec = table.get(&item_id).unwrap_or_else(|| panic!("{item_id}"));
        let expected = match description.as_str() {
            "Bio-Medical Engineering Field Crafting Tool" => 1,
            "Materials Engineering Field Crafting Tool" => 2,
            "Power Systems Engineering Field Crafting Tool" => 3,
            "Electronics Engineering Field Crafting Tool" => 4,
            other => panic!("item {item_id}: unexpected description {other:?}"),
        };
        assert_eq!(spec.applied_science_id, expected, "item {item_id}");
        assert_eq!(spec.tech_comp, tech_comp, "item {item_id}");
    }
}

/// Live-DB sentinels for the tool tests (`0x7000_Cxxx` crafting block;
/// `persistence.rs` uses up to `0x7000_CB55`, `handlers.rs` `0x7000_CC00`).
const TOOL_ACCOUNT: i32 = 0x7000_CE00;
const TOOL_PLAYER: i32 = 0x7000_CE01;
const TOOL_IN_CRAFTING_BAG: i32 = 0x7000_CE02;
const TOOL_IN_MAIN_BAG: i32 = 0x7000_CE03;

async fn cleanup(pool: &PgPool) {
    for id in [TOOL_IN_CRAFTING_BAG, TOOL_IN_MAIN_BAG] {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
            .bind(id)
            .execute(pool)
            .await;
    }
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(TOOL_PLAYER)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(TOOL_ACCOUNT)
        .execute(pool)
        .await;
}

/// Against the database: of two BMAS-5 tools (item 5369), only the one in
/// container 15 is held, read end to end from `sgw_inventory` through the
/// tool table.
#[tokio::test]
async fn load_held_tools_reads_only_the_crafting_bag() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(TOOL_ACCOUNT)
        .bind(format!("craft-tool-{TOOL_ACCOUNT}"))
        .execute(&pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0)",
    )
    .bind(TOOL_ACCOUNT)
    .bind(TOOL_PLAYER)
    .bind(format!("craft-tool-{TOOL_PLAYER}"))
    .execute(&pool)
    .await
    .expect("insert player");
    for (item_id, container_id) in [(TOOL_IN_CRAFTING_BAG, 15), (TOOL_IN_MAIN_BAG, 1)] {
        sqlx::query(
            "INSERT INTO sgw_inventory (item_id, stack_size, container_id, slot_id, type_id, character_id) \
             VALUES ($1, 1, $2, 0, 5369, $3)",
        )
        .bind(item_id)
        .bind(container_id)
        .bind(TOOL_PLAYER)
        .execute(&pool)
        .await
        .expect("insert tool");
    }

    let held = load_held_tools(&pool, TOOL_PLAYER).await;
    cleanup(&pool).await;

    assert_eq!(
        held.expect("load_held_tools"),
        vec![HeldTool {
            instance_id: TOOL_IN_CRAFTING_BAG,
            spec: ToolSpec {
                applied_science_id: 1,
                tech_comp: 5,
            },
        }]
    );
}
