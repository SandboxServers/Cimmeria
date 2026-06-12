//! `CellToBaseMsg` — messages sent from CellApp to BaseApp.

use super::data::{MailOp, NpcAoIData};

/// Messages sent from CellApp to BaseApp.
#[derive(Debug)]
pub enum CellToBaseMsg {
    /// Notification that a space exists (sent at startup and on dynamic creation).
    SpaceData { space_id: u32, world_name: String },

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
    LeftAoI { witness_id: u32, entity_id: u32 },

    /// Send a server→client entity method call to the entity's client.
    ///
    /// The BaseApp looks up the entity's client address and sends the method
    /// call using `append_entity_method` encoding.
    EntityMethodCall {
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// Send a batch of entity-method calls to the **same** target entity's
    /// client, packed into a single Mercury packet body via [`ChannelBundle`].
    ///
    /// Use when the cell emits a burst of small same-target methods on a
    /// single tick (e.g., world-entry region registrations, initial stat
    /// seed). The base side packs all methods into one packet body, so the
    /// client sees a single UDP datagram with N method-call records instead
    /// of N separate datagrams to ACK.
    ///
    /// Wire format on the client side is identical to N separate
    /// `EntityMethodCall`s — only the transport layer collapses the burst.
    /// Reliable channel; ordering within the batch is preserved.
    ///
    /// Introduced for the freeze fix (PR #410, 2026-05-26): the world-entry
    /// region-hint burst was firing 22 separate UDP packets in <1 ms, each
    /// needing an individual ACK. The combined ACK pressure stalled some
    /// clients past their render-thread budget. Bundling drops it to one
    /// datagram → one ACK.
    EntityMethodCallBatch {
        entity_id: u32,
        /// Ordered list of `(method_index, args)` to pack into one packet.
        calls: Vec<(u16, Vec<u8>)>,
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
    ///
    /// `destination_ring_id` carries through cross-world ring transports
    /// (`Effect::TeleportCrossWorld`). When `Some`, base stashes it on the
    /// connected-client state and sends it back via
    /// `BaseToCellMsg::AdvanceRingDestination` once the destination world
    /// finishes its `onClientReady` handshake — that's the deferred hook
    /// the destination ring's FSM waits on to advance out of
    /// `RemoteLoadWait`. Stargate-driven gate travel leaves it `None`.
    GateTravel {
        entity_id: u32,
        target_world_name: String,
        position: [f32; 3],
        rotation: [f32; 3],
        destination_ring_id: Option<i32>,
    },

    /// Persist a mission state change to the database.
    ///
    /// Uses `INSERT ... ON CONFLICT DO UPDATE` on `sgw_mission`. The
    /// `repeats` field MUST be carried through every update — the cell
    /// holds the authoritative count (bumped by `MissionInstance::complete`/
    /// `fail`), and skipping it on UPSERT silently resets the counter on
    /// the row, breaking repeatable-mission gating (#118).
    MissionUpdate {
        player_id: i32,
        mission_id: i32,
        status: i8,
        current_step_id: Option<i32>,
        completed_step_ids: Vec<i32>,
        completed_objective_ids: Vec<i32>,
        active_objective_ids: Vec<i32>,
        failed_objective_ids: Vec<i32>,
        repeats: i32,
    },

    /// Grant XP to a player entity (from a mob kill).
    ///
    /// The CellService computes the XP amount from the mob's level and sends
    /// this to BaseApp, which updates the player's XP/level and sends client
    /// notifications.
    GrantXP { entity_id: u32, xp_amount: u64 },

    /// Train a new ability for a player — debit one training point and
    /// persist the new ability to `sgw_player.abilities`. The base side
    /// owns `training_points` (it lives on `ConnectedClientState`) and
    /// the DB UPDATE, so this round-trip is the only correct path.
    ///
    /// Cell pre-validates: ability exists, ability is in player's
    /// archetype tree, level requirement met, prereqs known, not already
    /// known. Base only validates training_points >= 1 (the cell's
    /// per-player state isn't authoritative for that) and the DB UPDATE
    /// returning `rows_affected == 1`.
    ///
    /// On success, base responds with
    /// [`crate::cell::messages::BaseToCellMsg::AbilityGranted`] so the
    /// cell can add to `entity.abilities` and broadcast
    /// `onKnownAbilitiesUpdate`.
    TrainAbility {
        entity_id: u32,
        player_id: i32,
        ability_id: i32,
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

    /// Remove `count` of an item by **design id** (`type_id`) — chains know
    /// design ids, not instance ids. Base resolves the player's first
    /// matching instance and applies the same wire-update sequence as
    /// `RemoveInventoryItem`. Used by `Action::RemoveItem` in the cell
    /// content executor (e.g., chain 1034 consumes the Ambernol vial after
    /// `OnItemUse` fires).
    RemoveInventoryItemByType {
        entity_id: u32,
        player_id: i32,
        type_id: i32,
        count: i32,
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
    ///
    /// `entity_id` is carried so the base handler can broadcast a fresh
    /// `BEING_APPEARANCE` to the player's witnesses after the slot is
    /// persisted — without the appearance refresh, the player's model
    /// keeps the previously-equipped weapon visual until the next login.
    ActiveSlotUpdate {
        entity_id: u32,
        player_id: i32,
        slot_id: i32,
    },

    /// Persist the player's server-synced client options after a cell-side
    /// `updateSystemOptions` (player method 93) applies. Mirrors the same
    /// "cell mutates in-memory state, base persists to DB" split as
    /// `ActiveSlotUpdate`. Only the columns the client sent get the
    /// updated values; defaults are pre-merged on the cell side so we
    /// always send the *full* SystemOptions block (a partial update
    /// would require column-aware SQL we don't need at scale today).
    SystemOptionsUpdate {
        player_id: i32,
        auto_reload: bool,
        reload_on_activate: bool,
    },

    /// Persist the user-preference bits of the player's `state_field`
    /// after a toggle (today: `setAutoCycle`, player method 83, flipping
    /// `BSF_AutoCycling`). Same cell-mutates / base-persists split as
    /// `SystemOptionsUpdate`. The cell side sends the value already
    /// masked to `PERSISTED_STATE_FIELD_MASK`; the base-side handler
    /// masks again defensively so transient combat bits (BSF_Dead,
    /// BSF_InCombat, BSF_MovementLock) can never reach the DB even if a
    /// future send site forgets. Restored onto the entity (and
    /// re-broadcast to the client) by `InitPlayerState` on the next
    /// world entry. (#412)
    StateFieldUpdate { player_id: i32, state_field: u32 },

    /// Re-broadcast `BeingAppearance` to the player's AoI with a fresh
    /// holster state. Used by the combat enter/exit path (and any other
    /// runtime holster toggle, e.g. the `requestHolsterWeapon` button)
    /// to draw or holster the weapon without requiring an inventory
    /// change.
    ///
    /// The base handler updates `ConnectedClientState::weapon_holstered`
    /// so any subsequent appearance refresh (item equip, slot swap,
    /// world re-entry) inherits the live holster state instead of the
    /// hardcoded spawn-holstered default. It then re-queries the
    /// player's appearance and broadcasts the resulting packet — same
    /// path as `ActiveSlotUpdate`.
    ///
    /// **Phase 2 of the holster work**: Phase 1 made spawn
    /// always holstered; this message is what flips it back on enter
    /// combat.
    RefreshAppearance {
        entity_id: u32,
        player_id: i32,
        holstered: bool,
    },

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

    /// Re-anchor the local pawn to a fresh actor without `RESET_ENTITIES`.
    ///
    /// BaseApp's `handle_reanchor_player` sends two packets to the client:
    /// 1. A burst combining `BASEMSG_CREATE_BASE_PLAYER` +
    ///    `BASEMSG_SPACE_VIEWPORT_INFO` + `BASEMSG_CREATE_CELL_PLAYER` +
    ///    `BASEMSG_FORCED_POSITION`.
    /// 2. A separate bundle with `BeingAppearance` + `onEntityTint`
    ///    pulled from the client's cached world-entry args.
    ///
    /// `CREATE_BASE_PLAYER` (0x05) is the load-bearing piece — it invokes
    /// the client's `createBasePlayer` callback (the same hook used on
    /// initial login), which destroys the existing pawn actor (carrying
    /// the ragdoll physics state from the `Entity_Death` kismet) and
    /// instantiates a fresh standing one. The trailing VIEWPORT/CELL/
    /// FORCED_POSITION keep the client's space tables consistent with
    /// the new pawn, and the property replay repopulates its visuals.
    ///
    /// **No `RESET_ENTITIES` is sent**, so all other client-side state —
    /// AoI entities, kismet sequence state (door open/closed, triggered
    /// events), the level itself — survives the respawn untouched.
    ///
    /// Used by `handle_respawn`. Cross-world respawn falls back to `GateTravel`
    /// since the player is leaving the space anyway.
    ///
    /// Why it's separate from `TeleportPlayer`: TeleportPlayer sends only
    /// `BASEMSG_FORCED_POSITION` (a position snap), which doesn't re-create
    /// the pawn actor and so doesn't clear ragdoll state. ReanchorPlayer
    /// is the stronger primitive that drives a full pawn rebuild.
    ReanchorPlayer {
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
        rotation: [f32; 3],
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
        /// The entity's last-known position before the snap. Becomes the
        /// `forcedPosition` previous-position reference vector (offsets 24-35,
        /// NOT velocity — see `build_forced_position`). Senders must capture
        /// this *before* calling `space_mgr.update_entity_position(...)`.
        prev_pos: [f32; 3],
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
    EntityInvisible { witness_id: u32, entity_id: u32 },

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

    /// Both players in a trade reached `LockedAndConfirmed` — base must
    /// now perform the atomic swap (items + cash) inside a single sqlx
    /// transaction, then send `onTradeResults` to both clients.
    ///
    /// Cell has already torn down the in-memory session state by the
    /// time this fires — base is the final source of truth for the
    /// commit outcome. Base sends one of:
    /// - `Completed` to both on success
    /// - `Cancelled` to both if any validation (item ownership, cash
    ///   availability, slot capacity) fails inside the tx
    ///
    /// `p1_item_instance_ids` and `p2_item_instance_ids` carry only the
    /// inventory instance IDs from each side's proposal. Base re-fetches
    /// the full row (FOR UPDATE) to re-validate ownership and stack size
    /// — the cell-side TradeProposal is stale by the time this arrives.
    ExecuteTrade {
        entity_id: u32,
        player_id: i32,
        partner_entity_id: u32,
        partner_player_id: i32,
        p1_item_instance_ids: Vec<i32>,
        p1_cash: i32,
        p2_item_instance_ids: Vec<i32>,
        p2_cash: i32,
    },
}
