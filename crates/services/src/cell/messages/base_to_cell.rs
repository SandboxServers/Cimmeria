//! `BaseToCellMsg` — messages sent from BaseApp to CellApp.

use super::data::SavedMission;

/// Messages sent from BaseApp to CellApp.
// Cannot derive Debug because oneshot::Sender doesn't implement Debug.
// Manual impl would be possible but not worth the boilerplate.
pub enum BaseToCellMsg {
    /// Create a cell entity in the named world at the given position/rotation.
    /// The `reply_tx` oneshot returns the resolved `space_id` so the caller
    /// can `.await` it before building the world-entry wire packet.
    CreateEntity {
        entity_id: u32,
        world_name: String,
        position: [f32; 3],
        rotation: [f32; 3],
        reply_tx: tokio::sync::oneshot::Sender<u32>,
    },

    /// Destroy a cell entity (player left, entity despawned).
    DestroyEntity {
        entity_id: u32,
    },

    /// Mark an entity as having a client controller (player).
    /// Sent after world entry packets are delivered to the client.
    ConnectEntity {
        entity_id: u32,
    },

    /// Remove client controller from an entity (player disconnected).
    DisconnectEntity {
        entity_id: u32,
    },

    /// Client position/movement update forwarded from `avatarUpdateExplicit`.
    EntityMove {
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    },

    /// Client→server cell entity method call forwarded from BaseApp.
    ///
    /// `method_index` is the flattened EXPOSED CellMethod index for the
    /// SGWPlayer entity type (0 = setTargetID, 1 = setMovementType, etc.).
    /// `args` contains the raw method arguments (after entity_id extraction).
    CellMethodCall {
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// Chat message from a player, forwarded from BaseApp for spatial distribution.
    ///
    /// The CellService broadcasts to witnesses based on channel type:
    /// say/emote (nearby), yell (wider range).
    ChatMessage {
        entity_id: u32,
        speaker_name: String,
        speaker_flags: u8,
        channel: u8,
        text: String,
    },

    /// Initialize player state after world entry (missions, etc.).
    /// Sent after ConnectEntity so the CellService can populate per-player data.
    InitPlayerState {
        entity_id: u32,
        player_id: i32,
        world_name: String,
        /// Saved missions loaded from DB, to be restored before content engine fires.
        saved_missions: Vec<SavedMission>,
        /// Player's known ability IDs (from sgw_player.abilities column).
        abilities: Vec<i32>,
        /// Active bandolier slot from `sgw_player.bandolier_slot` (0-based).
        active_bandolier_slot: i32,
        /// Bandolier slot contents loaded from `sgw_inventory` + `resources.items`.
        bandolier_items: Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
    },

    /// Update one bandolier slot after a runtime item grant.
    ///
    /// BaseApp persists inventory changes and sends the client inventory update,
    /// while CellApp owns combat state. This keeps the cell-side weapon cache in
    /// sync without waiting for relog/world entry.
    UpdateBandolierItem {
        entity_id: u32,
        slot_id: i32,
        item: cimmeria_entity::cell_entity::BandolierItem,
        make_active: bool,
    },

    /// Replace the whole cell-side bandolier cache after inventory move/remove.
    SyncBandolierItems {
        entity_id: u32,
        active_bandolier_slot: i32,
        bandolier_items: Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
    },

    /// Inventory move committed in BaseApp after DB validation.
    ///
    /// CellApp uses the source/target transition to fire item equip/unequip
    /// event abilities only after the move is known to have persisted.
    InventoryItemMoveApplied {
        entity_id: u32,
        item_id: i32,
        source_container_id: i32,
        target_container_id: i32,
        swapped_item_id: Option<i32>,
    },

    /// Inventory item instance was fully removed after DB validation.
    InventoryItemRemoved {
        entity_id: u32,
        item_id: i32,
        source_container_id: i32,
    },

    /// Inventory item was granted and persisted in BaseApp.
    InventoryItemGranted {
        entity_id: u32,
        item_id: i32,
        container_id: i32,
        slot_id: i32,
        quantity: i32,
    },

    /// Inventory item was consumed atomically by base (in response to
    /// `CellToBaseMsg::UseInventoryItem`). The cell should fire the
    /// `OnItemUse` content event with `type_id` (item design id). Only sent
    /// after the consumption tx commits — if the player didn't own that
    /// instance or the tx failed, this message is not emitted.
    ItemUsed {
        entity_id: u32,
        type_id: i32,
        target_id: i32,
    },

    /// Reload the content engine from the database (triggered by admin API / Content Editor).
    ReloadContentEngine,

    /// Minigame result callback (forwarded from BaseApp after minigame server reports).
    MinigameResult {
        entity_id: u32,
        result_code: u8,
        on_victory_chains: Vec<i64>,
    },
}
