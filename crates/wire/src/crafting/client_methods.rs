//! Argument bytes for the crafting client methods (`SGWPlayer.def`
//! ClientMethods). Every integer is little-endian, and an `ARRAY` is a `u32`
//! element count followed by the elements, as for every other entity-method
//! array this server sends (`onUpdateItem`, `onAbilityTreeInfo`).

/// `onUpdateDiscipline(INT32 aDisciplineSeqId, INT32 aExpertise)` (136).
pub use cimmeria_entity::crafting::serialize_on_update_discipline as update_discipline_args;

use crate::cell::cell_methods::inventory::build_entity_property_args;

/// `GENERICPROPERTY_AppliedSciencePoints` in `entities/defs/enumerations.xml`.
pub const GENERICPROPERTY_APPLIED_SCIENCE_POINTS: i32 = 2;

/// `onEntityProperty(GENERICPROPERTY_AppliedSciencePoints, total)`: the
/// player's unspent ASP. Always the **total**, never a change (audit C-57):
/// the client shows the value as it arrives (Lua
/// `DisciplineTrainer.lua:49-54`).
pub fn applied_science_points_property_args(total: i32) -> Vec<u8> {
    build_entity_property_args(GENERICPROPERTY_APPLIED_SCIENCE_POINTS, total)
}

/// `onCraftingRespecPrompt(INT32 CostToRespec)` (112): the naquadah a
/// crafting respec costs.
pub fn crafting_respec_prompt_args(cost_to_respec: i32) -> Vec<u8> {
    cost_to_respec.to_le_bytes().to_vec()
}

/// `onDisciplineRespec()` (137): no arguments. The client zeroes the
/// expertise of every discipline it knows (audit C-37).
pub fn discipline_respec_args() -> Vec<u8> {
    Vec::new()
}

/// `onUpdateRacialParadigmLevel(INT32 aRacialParadigmId, INT8 aLevel)` (138):
/// five bytes.
pub fn racial_paradigm_level_args(racial_paradigm_id: i32, level: i8) -> Vec<u8> {
    let mut args = Vec::with_capacity(5);
    args.extend_from_slice(&racial_paradigm_id.to_le_bytes());
    args.extend_from_slice(&level.to_le_bytes());
    args
}

/// `onUpdateKnownCrafts(ARRAY<INT32> aCraftList)` (139): the full list of
/// known blueprint ids. The client can only request a blueprint listed here
/// (audit C-30).
pub fn known_crafts_args(blueprint_ids: &[i32]) -> Vec<u8> {
    let mut args = Vec::with_capacity(4 + blueprint_ids.len() * 4);
    write_i32_array(&mut args, blueprint_ids);
    args
}

/// `CraftingInfo` (`entities/defs/alias.xml`): the tools (`items`) and
/// machines (`entities`) that enable one verb. The client keeps only the
/// **last** id of each list (audit C-35).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftingInfo {
    /// Item ids of usable tools.
    pub items: Vec<i32>,
    /// Entity ids of usable machines (stations).
    pub entities: Vec<i32>,
}

/// `CraftingOptions` (`entities/defs/alias.xml`): one [`CraftingInfo`] per
/// verb. The default (every list empty) disables every crafting tab.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftingOptions {
    pub crafting: CraftingInfo,
    pub research: CraftingInfo,
    pub reverse_engineering: CraftingInfo,
    pub alloying: CraftingInfo,
}

/// `onUpdateCraftingOptions(CraftingOptions aOptions)` (140).
///
/// A `FIXED_DICT` goes on the wire as its fields in declaration order with
/// no header, so this is four `CraftingInfo`s in the `alias.xml` order
/// `crafting`, `research`, `reverseEngineering`, `alloying`, each being
/// `items` then `entities`: eight arrays, 32 bytes when all are empty.
pub fn crafting_options_args(options: &CraftingOptions) -> Vec<u8> {
    let sections = [
        &options.crafting,
        &options.research,
        &options.reverse_engineering,
        &options.alloying,
    ];
    let ids: usize = sections
        .iter()
        .map(|s| s.items.len() + s.entities.len())
        .sum();
    let mut args = Vec::with_capacity(32 + ids * 4);
    for section in sections {
        write_i32_array(&mut args, &section.items);
        write_i32_array(&mut args, &section.entities);
    }
    args
}

fn write_i32_array(out: &mut Vec<u8>, values: &[i32]) {
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}
