//! Field Crafting Tools (D-CR21): portable stand-ins for a crafting station.
//!
//! A tool counts only in the crafting bag (`INV_Crafting`, container 15),
//! the only player bag its `container_sets` `{17,15}` allows. It enables
//! crafting, research and reverse engineering for disciplines of its applied
//! science whose `tech_competency` is at most the tool's `tech_comp`.
//! Alloying needs a station. Tools are not consumed.
//!
//! Nothing in the seed or the cooked data says an item is a tool or which
//! science it serves: `applied_science_id` is NULL on every tool and the
//! cooked `AppliedScienceID` is 0 (CR-E2 Q3). The science is in the name
//! prefix only, so [`classify_tool`] reads it from there, and
//! [`tool_table`] builds the table once from `resources.items`.

use std::collections::HashMap;
use std::sync::Arc;

use cimmeria_cell_catalog::crafting::Discipline;
use sqlx::PgPool;
use tokio::sync::OnceCell;

/// `INV_Crafting`: the only bag a tool counts in.
pub const CRAFTING_BAG: i32 = 15;

/// What a Field Crafting Tool enables: one applied science up to a tech
/// competency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolSpec {
    /// `resources.applied_science.id`: 1 Biomedical, 2 Materials,
    /// 3 Power Systems, 4 Electronic.
    pub applied_science_id: i32,
    /// The item's `tech_comp`.
    pub tech_comp: i32,
}

impl ToolSpec {
    /// Whether this tool covers work in `discipline` (D-CR21).
    pub fn covers(&self, discipline: &Discipline) -> bool {
        discipline.applied_science_id == self.applied_science_id
            && discipline.tech_competency <= self.tech_comp
    }
}

/// One tool in a player's crafting bag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldTool {
    /// The inventory instance id (`sgw_inventory.item_id`), the id
    /// `CraftingInfo.items` names: the client resolves it through its own
    /// inventory (`getItemSlot`, `Crafting.lua:45`).
    pub instance_id: i32,
    pub spec: ToolSpec,
}

/// Classify an item by name: `<PREFIX>-<n> Field Crafting Tool`, optionally
/// preceded by `Efficient `. The prefix names the science (BMAS
/// Biomedical, MAS Materials, PSAS Power Systems, EAS Electronic; ids from
/// `db/resources/Abilities/Seed/applied_science.sql`). Anything else is not
/// a tool.
pub fn classify_tool(name: &str, tech_comp: i32) -> Option<ToolSpec> {
    let stem = name.strip_suffix(" Field Crafting Tool")?;
    let stem = stem.strip_prefix("Efficient ").unwrap_or(stem);
    let (prefix, _grade) = stem.split_once('-')?;
    let applied_science_id = match prefix {
        "BMAS" => 1,
        "MAS" => 2,
        "PSAS" => 3,
        "EAS" => 4,
        _ => return None,
    };
    Some(ToolSpec {
        applied_science_id,
        tech_comp,
    })
}

/// The best tool for the crafting-window label: the highest `tech_comp`,
/// then the lowest instance id, so the choice is stable.
pub fn best_tool(tools: &[HeldTool]) -> Option<&HeldTool> {
    tools
        .iter()
        .max_by_key(|t| (t.spec.tech_comp, std::cmp::Reverse(t.instance_id)))
}

/// Pick the tools out of a player's inventory rows (instance id, type id,
/// container id): items of a tool type sitting in the crafting bag.
pub fn tools_in_crafting_bag(
    table: &HashMap<i32, ToolSpec>,
    rows: impl IntoIterator<Item = (i32, i32, i32)>,
) -> Vec<HeldTool> {
    let mut tools: Vec<HeldTool> = rows
        .into_iter()
        .filter(|&(_, _, container_id)| container_id == CRAFTING_BAG)
        .filter_map(|(instance_id, type_id, _)| {
            table
                .get(&type_id)
                .map(|&spec| HeldTool { instance_id, spec })
        })
        .collect();
    tools.sort_by_key(|t| t.instance_id);
    tools
}

static TOOL_TABLE: OnceCell<Arc<HashMap<i32, ToolSpec>>> = OnceCell::const_new();

/// Every Field Crafting Tool item type, keyed by `resources.items.item_id`,
/// loaded once per process.
pub async fn tool_table(pool: &PgPool) -> Result<Arc<HashMap<i32, ToolSpec>>, sqlx::Error> {
    TOOL_TABLE
        .get_or_try_init(|| async {
            let rows: Vec<(i32, String, i32)> = sqlx::query_as(
                "SELECT item_id, name, tech_comp FROM resources.items \
                 WHERE name LIKE '%Field Crafting Tool'",
            )
            .fetch_all(pool)
            .await?;
            let table: HashMap<i32, ToolSpec> = rows
                .into_iter()
                .filter_map(|(id, name, tc)| classify_tool(&name, tc).map(|s| (id, s)))
                .collect();
            tracing::info!(
                target: "crafting",
                event = "tool_table_loaded",
                tools = table.len(),
                "Field Crafting Tool table loaded"
            );
            Ok(Arc::new(table))
        })
        .await
        .cloned()
}

/// The tools in `player_id`'s crafting bag, read from the database.
pub async fn load_held_tools(pool: &PgPool, player_id: i32) -> Result<Vec<HeldTool>, sqlx::Error> {
    let table = tool_table(pool).await?;
    let rows: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT item_id, type_id, container_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2",
    )
    .bind(player_id)
    .bind(CRAFTING_BAG)
    .fetch_all(pool)
    .await?;
    Ok(tools_in_crafting_bag(&table, rows))
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
