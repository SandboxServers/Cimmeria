//! `CellToBaseMsg` — messages sent from CellApp to BaseApp.

use super::data::{MailOp, NpcAoIData};

/// Messages sent from CellApp to BaseApp.
#[derive(Debug)]
pub enum CellToBaseMsg {
    /// Notification that a space exists (sent at startup and on dynamic creation).
    SpaceData {
        space_id: u32,
        world_name: String,
    },

    /// Response to `CreateEntity` — entity placed in a space.
    EntityCreated {
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
    },

    /// An entity has moved (for relaying volatile updates to a specific witness via BaseApp).
    EntityMoved {
        witness_id: u32,
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
        direction: [f32; 3],
        velocity: [f32; 3],
    },

    /// A new entity entered a witness's Area of Interest.
    EnteredAoI {
        witness_id: u32,
        entity_id: u32,
        space_id: u32,
        class_id: u8,
        position: [f32; 3],
        direction: [f32; 3],
        /// Entity level (for `onLevelUpdate`). Defaults to 1.
        level: u32,
        /// NPC-specific data (faction, alignment, flags, name). None for players.
        npc_data: Option<NpcAoIData>,
    },

    /// An entity left a witness's Area of Interest.
    LeftAoI {
        witness_id: u32,
        entity_id: u32,
    },

    /// Send a server→client entity method call to the entity's client.
    ///
    /// The BaseApp looks up the entity's client address and sends the method
    /// call using `append_entity_method` encoding.
    EntityMethodCall {
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// Request mail headers for a player entity (forwarded to BaseApp for DB query).
    MailRequest {
        entity_id: u32,
        player_id: i32,
        /// Mail operation type.
        op: MailOp,
    },

    /// Request a gate travel (world transition) for a player entity.
    ///
    /// The CellService has already validated the stargate address and removed
    /// the entity from the old space. BaseApp must send RESET_ENTITIES to the
    /// client, then re-create the entity in the new world via the standard
    /// World entry flow (teardown -> create player -> enter world).
    GateTravel {
        entity_id: u32,
        target_world_name: String,
        position: [f32; 3],
        rotation: [f32; 3],
    },

    /// Persist a mission state change to the database.
    ///
    /// Uses `INSERT ... ON CONFLICT DO UPDATE` on `sgw_mission`.
    MissionUpdate {
        player_id: i32,
        mission_id: i32,
        status: i8,
        current_step_id: Option<i32>,
        completed_step_ids: Vec<i32>,
        completed_objective_ids: Vec<i32>,
        active_objective_ids: Vec<i32>,
        failed_objective_ids: Vec<i32>,
    },

    /// Grant XP to a player entity (from a mob kill).
    ///
    /// The CellService computes the XP amount from the mob's level and sends
    /// this to BaseApp, which updates the player's XP/level and sends client
    /// notifications.
    GrantXP {
        entity_id: u32,
        xp_amount: u64,
    },

    /// Grant an item to a player and persist to `sgw_inventory`.
    GrantItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        container_id: i32,
        count: i32,
    },

    /// Open a vendor store for a player using the vendor template lists.
    OpenVendorStore {
        entity_id: u32,
        player_id: i32,
        vendor_entity_id: i32,
        vendor_template_id: Option<i32>,
    },

    /// Purchase items from the currently-open vendor store.
    PurchaseVendorItems {
        entity_id: u32,
        player_id: i32,
        vendor_entity_id: i32,
        vendor_template_id: i32,
        items: Vec<(i32, i32)>,
    },

    /// Sell owned inventory items to the currently-open vendor store.
    SellVendorItems {
        entity_id: u32,
        player_id: i32,
        vendor_entity_id: i32,
        vendor_template_id: i32,
        items: Vec<(i32, i32)>,
    },

    /// Buy back recently-sold inventory items from the currently-open vendor store.
    BuybackVendorItems {
        entity_id: u32,
        player_id: i32,
        vendor_entity_id: i32,
        vendor_template_id: i32,
        items: Vec<(i32, i32)>,
    },

    /// Request a full inventory refresh from BaseApp.
    ListInventoryItems { entity_id: u32, player_id: i32 },

    /// Move an inventory item instance to another bag/slot.
    MoveInventoryItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        target_container_id: i32,
        target_slot_id: i32,
        quantity: i32,
    },

    /// Remove quantity from an inventory item instance.
    RemoveInventoryItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        quantity: i32,
    },

    /// Consume one charge/stack of an inventory item instance, then fire the
    /// content-engine `OnItemUse` event back to the cell with the resolved
    /// `type_id` (item design id). Mission progression that depends on item
    /// use is gated on the consumption committing successfully — if the row
    /// can't be found or the tx fails, no event fires and the mission does
    /// not advance.
    ///
    /// `item_id` is the inventory instance id from the wire (`useItem` arg
    /// per `SGWInventoryManager.def` — "Player inventory id"). The type_id is
    /// looked up server-side and returned via `BaseToCellMsg::ItemUsed`.
    UseInventoryItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        target_id: i32,
    },

    /// Repair an owned inventory item instance by a durability ratio.
    RepairInventoryItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        repair_ratio: f32,
    },

    /// Fully repair owned inventory item instances.
    RepairInventoryItems {
        entity_id: u32,
        player_id: i32,
        item_ids: Vec<i32>,
        vendor_template_id: Option<i32>,
    },

    /// Fully recharge owned inventory item instances.
    RechargeInventoryItems {
        entity_id: u32,
        player_id: i32,
        item_ids: Vec<i32>,
        vendor_template_id: Option<i32>,
    },

    /// Persist the player's active bandolier slot.
    ActiveSlotUpdate { player_id: i32, slot_id: i32 },

    /// Persist a single bandolier slot's per-slot ammo state.
    ///
    /// Sent by the cell on the batched cadence (reload completion, slot swap,
    /// `requestAmmoChange`, logout) — see Stage B/C/D in
    /// docs/gameplay/weapon-ammo-reload.md (TBD). `player_id` here matches the
    /// DB `character_id`, mirroring the field naming used by `ActiveSlotUpdate`.
    ///
    /// `expected_item_id` guards against TOCTOU: if the slot's item changes
    /// between the cell sending this message and the base writing the row,
    /// the SQL `WHERE type_id = $expected_item_id` clause skips the write
    /// rather than scribbling stale ammo onto the new weapon.
    BandolierAmmoUpdate {
        player_id: i32,
        slot_id: i32,
        expected_item_id: i32,
        current_ammo: i32,
        cur_ammo_type: i32,
    },

    /// Grant cash (naquadah) to a player and persist to the database.
    GrantCash {
        entity_id: u32,
        player_id: i32,
        amount: i32,
    },

    /// Respawn reload: send onClientMapLoad to trigger a loading screen, then
    /// set up pending_client_ready so the next onClientReady triggers a fresh
    /// mapLoaded sequence with cleared state. This is the only reliable way to
    /// reset ragdoll/death state on the client.
    RespawnReload {
        entity_id: u32,
        world_name: String,
        spawn_pos: [f32; 3],
    },

    /// Authoritative same-world teleport. The cell has already updated its
    /// spatial state via `update_entity_position`; the base must now push the
    /// new position to the player's own client. Engine-level snap, no loading
    /// screen, no world reload — used by ring transporters and any other
    /// in-world short-hop teleport.
    ///
    /// `space_id` is the cell's authoritative space — the base trusts this
    /// over its connection-cached `world_name`, which can lag during world
    /// transitions.
    ///
    /// Other witnesses receive their position update through the next AoI
    /// tick's `EntityMoved` broadcast — no extra fan-out is needed here.
    TeleportPlayer {
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
    },

    /// Send a ghost entity method call to a specific witness player.
    ///
    /// Used for broadcasting property updates (InteractionType, SetVisible, etc.)
    /// to players who have the entity in their AoI. The `entity_id` is the ghost
    /// entity the method is called on; `witness_id` is the player to send to.
    WitnessEntityMethod {
        witness_id: u32,
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// Send `BASEMSG_ENTITY_INVISIBLE (0x0B)` to a single witness without the
    /// follow-up `LEAVE_AOI (0x0C)` — visually hides the entity on that
    /// client while keeping it in AoI bookkeeping. Mirror of C++
    /// `ClientHandler::leaveAoI(entity_id, deleteEntity=false)`. Used by the
    /// ring-transport hide phase; pair with a `WitnessEntityMethod` calling
    /// `onVisible(1)` to restore.
    EntityInvisible {
        witness_id: u32,
        entity_id: u32,
    },

    /// Start a minigame session for a player (Cell → Base).
    StartMinigame {
        entity_id: u32,
        player_id: i32,
        game_name: String,
        difficulty: u32,
        on_victory_chains: Vec<i64>,
    },

    /// Minigame result callback (minigame server → Cell via Base).
    MinigameResult {
        entity_id: u32,
        result_code: u8,
        on_victory_chains: Vec<i64>,
    },
}
