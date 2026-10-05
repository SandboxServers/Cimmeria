//! The crafting request the cell forwards to the base
//! (sent in the `CellToBaseMsg::Plugin` envelope).
//!
//! The cell parses the client's arguments (methods 95-100, `SGWPlayer.def`)
//! and applies the station gate; the base owns every rule and every write.
//! The fields are the client's arguments verbatim: nothing here has been
//! validated beyond "the bytes parsed".

/// One crafting request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftRequest {
    /// The player's cell entity id; feedback goes to its client.
    pub entity_id: u32,
    /// `sgw_player.player_id`.
    pub player_id: i32,
    pub verb: CraftVerb,
    /// The `ECraftTypeFlags` mask of the verbs whose crafting station was in
    /// reach when the cell received the request.
    pub allowed: u8,
}

/// The six crafting cell methods and their arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftVerb {
    /// `spendAppliedSciencePoints(INT32 aDisciplineSeqId)` (95).
    Spend { discipline_id: i32 },
    /// `craft(INT32 aCraftId, ARRAY<ItemID> aItems, INT32 aQuantity)` (96).
    /// `items` are inventory instance ids, one per component type.
    Craft {
        blueprint_id: i32,
        items: Vec<i32>,
        quantity: i32,
    },
    /// `research(ItemID aItemId, ARRAY<ItemID> aKickers)` (97).
    Research { item_id: i32, kickers: Vec<i32> },
    /// `reverseEngineer(ItemID aItemId)` (98).
    ReverseEngineer { item_id: i32 },
    /// `alloying(INT32 aCraftId, ItemID aCurrentTierItemId,
    /// ARRAY<ItemID> aLowerTierItems)` (99).
    Alloy {
        blueprint_id: i32,
        current_tier_item_id: i32,
        lower_tier_items: Vec<i32>,
    },
    /// `respecCrafting()` (100).
    Respec,
}

impl CraftVerb {
    /// The cell-method index the verb arrived as (95-100).
    pub fn method_index(&self) -> u16 {
        use crate::cell::cell_methods::player::{
            ALLOYING, CRAFT, RESEARCH, RESPEC_CRAFTING, REVERSE_ENGINEER,
            SPEND_APPLIED_SCIENCE_POINTS,
        };
        match self {
            CraftVerb::Spend { .. } => SPEND_APPLIED_SCIENCE_POINTS,
            CraftVerb::Craft { .. } => CRAFT,
            CraftVerb::Research { .. } => RESEARCH,
            CraftVerb::ReverseEngineer { .. } => REVERSE_ENGINEER,
            CraftVerb::Alloy { .. } => ALLOYING,
            CraftVerb::Respec => RESPEC_CRAFTING,
        }
    }

    /// The client method name, for logs and the crafting `verb` label, from
    /// the generated table (`crate::names`).
    pub fn method_name(&self) -> &'static str {
        crate::names::player_cell_method(self.method_index()).unwrap_or("unknown")
    }
}
