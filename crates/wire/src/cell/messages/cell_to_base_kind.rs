//! Stable telemetry names for `CellToBaseMsg` variants (TG-DB-05).
//!
//! The base's cell-to-base dispatch loop logs slow dispatches with the
//! variant's name as `msg_kind`. The name is a fixed string per variant,
//! never a payload value, so it is safe as a throttle key.

use super::CellToBaseMsg;

impl CellToBaseMsg {
    /// Fixed snake_case name of this variant, used as `msg_kind` in
    /// telemetry. The match is exhaustive: a new variant must name itself.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SpaceData { .. } => "space_data",
            Self::EnterableWorlds { .. } => "enterable_worlds",
            Self::EntityCreated { .. } => "entity_created",
            Self::EntityMoved { .. } => "entity_moved",
            Self::EnteredAoI { .. } => "entered_aoi",
            Self::LeftAoI { .. } => "left_aoi",
            Self::EntityMethodCall { .. } => "entity_method_call",
            Self::EntityMethodCallBatch { .. } => "entity_method_call_batch",
            Self::MailRequest { .. } => "mail_request",
            Self::GateTravel { .. } => "gate_travel",
            Self::GrantStargateAddress { .. } => "grant_stargate_address",
            Self::ContainerLooted { .. } => "container_looted",
            Self::MissionUpdate { .. } => "mission_update",
            Self::GrantXP { .. } => "grant_xp",
            Self::TrainAbility { .. } => "train_ability",
            Self::ResetAbilities { .. } => "reset_abilities",
            Self::GrantItem { .. } => "grant_item",
            Self::OpenVendorStore { .. } => "open_vendor_store",
            Self::PurchaseVendorItems { .. } => "purchase_vendor_items",
            Self::SellVendorItems { .. } => "sell_vendor_items",
            Self::BuybackVendorItems { .. } => "buyback_vendor_items",
            Self::ListInventoryItems { .. } => "list_inventory_items",
            Self::MoveInventoryItem { .. } => "move_inventory_item",
            Self::RemoveInventoryItem { .. } => "remove_inventory_item",
            Self::RemoveInventoryItemByType { .. } => "remove_inventory_item_by_type",
            Self::UseInventoryItem { .. } => "use_inventory_item",
            Self::RepairInventoryItem { .. } => "repair_inventory_item",
            Self::RepairInventoryItems { .. } => "repair_inventory_items",
            Self::RechargeInventoryItems { .. } => "recharge_inventory_items",
            Self::PersistPosition { .. } => "persist_position",
            Self::ShutdownFlush { .. } => "shutdown_flush",
            Self::ActiveSlotUpdate { .. } => "active_slot_update",
            Self::SystemOptionsUpdate { .. } => "system_options_update",
            Self::RefreshAppearance { .. } => "refresh_appearance",
            Self::BandolierAmmoUpdate { .. } => "bandolier_ammo_update",
            Self::GrantCash { .. } => "grant_cash",
            Self::GrantTrainingPoints { .. } => "grant_training_points",
            Self::GmGrantAbility { .. } => "gm_grant_ability",
            Self::GmGiveAmmo { .. } => "gm_give_ammo",
            Self::GmAbilityBulk { .. } => "gm_ability_bulk",
            Self::ContentGrantAbilities { .. } => "content_grant_abilities",
            Self::RecordTutorialShown { .. } => "record_tutorial_shown",
            Self::ExecuteAuthoringSql { .. } => "execute_authoring_sql",
            Self::ConsoleSearch { .. } => "console_search",
            Self::GmSpawnNpc { .. } => "gm_spawn_npc",
            Self::ReanchorPlayer { .. } => "reanchor_player",
            Self::TeleportPlayer { .. } => "teleport_player",
            Self::WitnessEntityMethod { .. } => "witness_entity_method",
            Self::EntityInvisible { .. } => "entity_invisible",
            Self::StartMinigame { .. } => "start_minigame",
            Self::CancelMinigame { .. } => "cancel_minigame",
            Self::MinigameResult { .. } => "minigame_result",
            Self::ContactListCreate { .. } => "contact_list_create",
            Self::ContactListDelete { .. } => "contact_list_delete",
            Self::ContactListRename { .. } => "contact_list_rename",
            Self::ContactListFlagsUpdate { .. } => "contact_list_flags_update",
            Self::ContactListAddMembers { .. } => "contact_list_add_members",
            Self::ContactListRemoveMembers { .. } => "contact_list_remove_members",
            Self::ContactListPresenceEvent { .. } => "contact_list_presence_event",
            Self::ExecuteTrade { .. } => "execute_trade",
            Self::Org { .. } => "org",
            Self::Chat { .. } => "chat",
            Self::MailGm { .. } => "mail_gm",
            Self::Bank { .. } => "bank",
            Self::Account { .. } => "account",
            Self::ContentSystemMail { .. } => "content_system_mail",
            Self::BlackMarket { .. } => "black_market",
            Self::ConsumeItemForUse { .. } => "consume_item_for_use",
            Self::AmmoReserve { .. } => "ammo_reserve",
            Self::Plugin { .. } => "plugin",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CellToBaseMsg;

    /// Three variants with different shapes map to three different names,
    /// and the same variant always maps to the same name.
    #[test]
    fn kind_is_distinct_and_stable_for_three_variants() {
        let space = CellToBaseMsg::SpaceData {
            space_id: 1,
            world_name: "Castle".to_string(),
        };
        let left = CellToBaseMsg::LeftAoI {
            witness_id: 1,
            entity_id: 2,
        };
        let worlds = CellToBaseMsg::EnterableWorlds { worlds: Vec::new() };

        let kinds = [space.kind(), left.kind(), worlds.kind()];
        assert_eq!(kinds, ["space_data", "left_aoi", "enterable_worlds"]);
        assert_eq!(space.kind(), space.kind());
    }
}
