//! [`CraftingPlugin`]: what crafting registers with the base at startup
//! (#962 step 5, plugin ADR §4.5).
//!
//! - [`consumers`]: the seven crafting payloads the cell sends in the
//!   `CellToBaseMsg::Plugin` envelope, each routed to the handler the
//!   central `CellToBaseMsg` arms used to call.
//! - [`hooks`]: the lifecycle and seam hooks, each at the line crafting's
//!   inline call occupied in core.

mod consumers;
mod hooks;

use cimmeria_base_session::base::plugin::{
    BasePlugin, BasePluginBuilder, InventoryHookPoint, ItemUseHookPoint, ProgressionHookPoint,
    SessionHookPoint, SessionStateHookPoint, WorldEntryHookPoint,
};
use cimmeria_wire::crafting::{
    CraftRequest, CraftingStations, GmAllCraft, GmCraftGrant, GmGrantAppliedSciencePoints,
    GmGrantExpertise, RespecCraftOpen,
};

/// Crafting as a base plugin. Holds no state: the induction queues are the
/// process-wide `session::crafting_sessions()`, and a session's crafting
/// options live in its `ConnectedClientState::extensions`.
pub struct CraftingPlugin;

impl BasePlugin for CraftingPlugin {
    fn name(&self) -> &'static str {
        "crafting"
    }

    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin
            .on_cell_message::<CraftRequest>(consumers::craft_request)
            .on_cell_message::<CraftingStations>(consumers::station_report)
            .on_cell_message::<GmAllCraft>(consumers::gm_all_craft)
            .on_cell_message::<GmCraftGrant>(consumers::gm_craft_grant)
            .on_cell_message::<RespecCraftOpen>(consumers::respec_open)
            .on_cell_message::<GmGrantExpertise>(consumers::grant_expertise)
            .on_cell_message::<GmGrantAppliedSciencePoints>(consumers::grant_applied_science)
            .session_hook(
                SessionHookPoint::LogOffAfterEntityUnmapped,
                hooks::drop_on_logout,
            )
            .session_hook(
                SessionHookPoint::DisconnectAfterEntityUnmapped,
                hooks::drop_on_logout,
            )
            .session_hook(
                SessionHookPoint::GateTravelBeforeActiveCharacterCheck,
                hooks::drop_on_world_change,
            )
            .session_state_hook(
                SessionStateHookPoint::PlayCharacterAfterEntryLatch,
                hooks::reset_on_play_character,
            )
            .session_state_hook(
                SessionStateHookPoint::GateTravelBeforeCreateEntity,
                hooks::begin_world_entry,
            )
            .world_entry_hook(
                WorldEntryHookPoint::ClientReadyAfterOrgRestore,
                hooks::login_sync,
            )
            .item_use_hook(ItemUseHookPoint::CraftingItem, hooks::use_crafting_item)
            .item_use_hook(ItemUseHookPoint::InstanceNotFound, hooks::use_missing_item)
            .inventory_hook(
                InventoryHookPoint::AfterFullInventoryUpdate,
                hooks::refresh_tools,
            )
            .progression_hook(
                ProgressionHookPoint::AfterAppliedScienceEarned,
                hooks::push_applied_science,
            );
    }
}
