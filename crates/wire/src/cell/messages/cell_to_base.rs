//! `CellToBaseMsg` — messages sent from CellApp to BaseApp.

use super::account_cell_to_base::AccountCellToBase;
use super::ammo_reserve::AmmoReserveRequest;
use super::bank_cell_to_base::BankCellToBase;
use super::black_market_cell_to_base::BlackMarketCellToBase;
use super::chat_cell_to_base::ChatCellToBase;
use super::content_mail_cell_to_base::ContentSystemMail;
use super::data::{MailOp, NpcAoIData, PlayerAoIData};
use super::item_use::ConsumeItemForUse;
use super::mail_gm_cell_to_base::MailGmCellToBase;
use super::org_cell_to_base::OrgCellToBase;
use super::plugin_msg::PluginMsg;
use super::shutdown_flush::ShutdownFlushRequest;
use crate::cell::vault::VaultAccess;

/// Messages sent from CellApp to BaseApp.
///
/// See the [internal Cell-to-Base message contract](https://github.com/SandboxServers/Cimmeria/blob/main/docs/protocol/internal-cell-to-base.md)
/// for ownership, ordering, and failure details of the message families below.
#[derive(Debug)]
pub enum CellToBaseMsg {
    /// Notification that a space exists (sent at startup and on dynamic creation).
    SpaceData { space_id: u32, world_name: String },

    /// Every world the cell can deliver a player to, including instanced worlds.
    EnterableWorlds { worlds: Vec<String> },

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
        /// Telemetry only, never on the wire: did this NPC move on the last
        /// movement tick (`None` for players and one-off relays)?
        npc_moved_since_last: Option<bool>,
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
        /// Live cell-side state of a player observee (state field, target,
        /// public stats, ammo type). `Some` exactly when the entering
        /// entity is a player; the base joins it with the observee's
        /// session identity to build the player-ghost cascade.
        player_data: Option<PlayerAoIData>,
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

    /// Ordered entity-method calls to one client in one reliable Mercury body.
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

    /// Validated world transition; Base resets the client and recreates its player.
    GateTravel {
        entity_id: u32,
        target_world_name: String,
        position: [f32; 3],
        rotation: [f32; 3],
        /// Cross-world ring destination; Base advances it after client-ready.
        destination_ring_id: Option<i32>,
        /// Exact instance, revalidated by Cell on arrival; `None` resolves by world.
        destination_space_id: Option<u32>,
    },

    /// Persist a cell-granted stargate address for this player.
    GrantStargateAddress {
        entity_id: u32,
        /// Base pairs this with the session account for the idempotent write.
        player_id: i32,
        stargate_id: i32,
    },

    /// Persist a once-per-character loot-container key.
    ContainerLooted {
        entity_id: u32,
        player_id: i32,
        container_key: String,
    },

    /// Persist cell-owned mission state, including the authoritative repeat count.
    MissionUpdate {
        player_id: i32,
        mission_id: i32,
        status: i8,
        current_step_id: Option<i32>,
        completed_step_ids: Vec<i32>,
        completed_objective_ids: Vec<i32>,
        active_objective_ids: Vec<i32>,
        failed_objective_ids: Vec<i32>,
        /// Cell-owned repeat count; omitting it on UPSERT resets mission gating.
        repeats: i32,
    },

    /// Grant XP computed by the cell; GM feedback goes to the caller after commit.
    GrantXP {
        entity_id: u32,
        xp_amount: u64,
        gm_feedback_to: Option<u32>,
    },

    /// Atomically buy an ability-tree node from Base-owned points and DB state.
    TrainAbility {
        entity_id: u32,
        /// Base verifies the current row and training-point debit atomically.
        player_id: i32,
        ability_id: i32,
        /// The node's `skill_point_cost` (`TrainPlan::cost`). Debited from
        /// `training_points` and added to `tree_points_spent`.
        cost: i32,
        /// The node's branch (`TrainPlan::tree_index`), for logs.
        tree_index: i32,
    },

    /// Atomically respec trainer-bought abilities and charge naquadah.
    ResetAbilities {
        entity_id: u32,
        player_id: i32,
        /// Naquadah to charge: `ability_tree::RESPEC_COST_NAQUADAH`.
        cost: i32,
    },

    /// Persist a granted item; report GM or loot refusal to the correct caller.
    GrantItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        container_id: i32,
        count: i32,
        /// Only GM grants set this; feedback follows the committed write.
        notify_gm: bool,
        /// Loot pickup source; a refused grant returns the item to Cell.
        loot: Option<super::LootGrantSource>,
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
        /// The cell's vault-session verdict for this move (BV-03).
        vault: VaultAccess,
    },

    /// Remove an item instance quantity; GM feedback follows commit.
    RemoveInventoryItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        quantity: i32,
        notify_gm: bool,
        /// The cell's vault-session verdict: an item in the vault (17) is
        /// removable only with a session open (BV-03).
        vault: VaultAccess,
    },

    /// Remove a quantity by design ID after Base resolves an owned instance.
    RemoveInventoryItemByType {
        entity_id: u32,
        player_id: i32,
        type_id: i32,
        count: i32,
        /// The cell's vault-session verdict: the vault (17) is searched
        /// only with a session open (BV-03).
        vault: VaultAccess,
    },

    /// Consume an item instance before Cell receives its OnItemUse event.
    UseInventoryItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        target_id: i32,
        /// The cell's vault-session verdict: an item in the vault (17) is
        /// usable only with a session open (BV-03).
        vault: VaultAccess,
    },

    /// Repair an owned inventory item instance by a durability ratio.
    RepairInventoryItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        repair_ratio: f32,
    },

    /// Fully repair owned inventory item instances at the open vendor, for
    /// payment. `vendor_template_id` is the server's (the player's vendor
    /// session); there is no free repair (CAT-E-01).
    RepairInventoryItems {
        entity_id: u32,
        player_id: i32,
        item_ids: Vec<i32>,
        vendor_template_id: i32,
    },

    /// Fully recharge owned inventory item instances at the open vendor,
    /// for payment. `vendor_template_id` is the server's; there is no free
    /// recharge (CAT-E-01).
    RechargeInventoryItems {
        entity_id: u32,
        player_id: i32,
        item_ids: Vec<i32>,
        vendor_template_id: i32,
    },

    /// Persist logout world and position for the next login.
    PersistPosition {
        player_id: i32,
        world_name: String,
        position: [f32; 3],
    },

    /// Persist positions and dirty ammo by the shutdown deadline.
    ShutdownFlush(ShutdownFlushRequest),

    /// Persist the active bandolier slot and refresh witness appearance.
    ActiveSlotUpdate {
        /// Base uses this to refresh `BeingAppearance` for witnesses.
        entity_id: u32,
        player_id: i32,
        slot_id: i32,
    },

    /// Persist the full cell-merged server-synced options block.
    SystemOptionsUpdate {
        player_id: i32,
        auto_reload: bool,
        reload_on_activate: bool,
    },

    /// Rebroadcast the current holster appearance to player witnesses.
    RefreshAppearance {
        entity_id: u32,
        player_id: i32,
        holstered: bool,
    },

    /// Persist ammo only while the expected inventory instance still matches.
    BandolierAmmoUpdate {
        player_id: i32,
        slot_id: i32,
        /// Inventory instance guard against stale ammo overwriting a new weapon.
        expected_instance_id: i32,
        current_ammo: i32,
        cur_ammo_type: i32,
    },

    /// Persist naquadah and send GM feedback to the caller after commit.
    GrantCash {
        entity_id: u32,
        player_id: i32,
        amount: i32,
        /// Caller entity, which may differ from the cash recipient.
        gm_feedback_to: Option<u32>,
    },

    /// Persist training points and return the Base-owned counter.
    GrantTrainingPoints {
        entity_id: u32,
        player_id: i32,
        amount: i32,
        gm_feedback_to: Option<u32>,
    },

    /// Persist a GM-granted ability outside trainer-bought abilities.
    GmGrantAbility {
        entity_id: u32,
        player_id: i32,
        ability_id: i32,
        gm_entity_id: u32,
        /// Base confirms this still owns the GM entity before feedback.
        gm_player_id: i32,
    },

    /// `.giveammo` for a player (ammo AM-06); see [`super::GmGiveAmmo`].
    GmGiveAmmo(super::GmGiveAmmo),

    /// `gmGiveAllAbilities` / `gmResetAbilities` for the calling GM (AB-N2);
    /// see [`super::GmAbilityBulk`]. Answered with
    /// [`crate::cell::messages::BaseToCellMsg::GmAbilitiesChanged`].
    GmAbilityBulk(super::GmAbilityBulk),

    /// The content `grant_ability` action (CS-01a); see
    /// [`super::ContentGrantAbilities`]. Answered with
    /// [`crate::cell::messages::BaseToCellMsg::ContentAbilitiesGranted`].
    ContentGrantAbilities(super::ContentGrantAbilities),

    /// The content `show_tutorial` action (CS-03); see
    /// [`super::RecordTutorialShown`]. Answered with
    /// [`crate::cell::messages::BaseToCellMsg::TutorialRecorded`].
    RecordTutorialShown(super::RecordTutorialShown),

    /// Run GM-gated, server-generated authoring SQL in Base.
    ExecuteAuthoringSql {
        entity_id: u32,
        label: String,
        /// Server-generated from GM-gated parsed input, never raw client text.
        sql: String,
    },

    /// Search Base-owned resource names for GM console feedback.
    ConsoleSearch {
        entity_id: u32,
        kind: u8,
        query: String,
    },

    /// Resolve a template in Base and return a spawn record to Cell.
    GmSpawnNpc {
        entity_id: u32,
        template_id: i32,
        space_id: u32,
        world_name: String,
        position: [f32; 3],
        heading: f32,
        /// Echoed on success or failure for `.spawnrandom` summaries.
        batch: Option<u32>,
    },

    /// Rebuild the local pawn without resetting the world or other entities.
    ReanchorPlayer {
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
        rotation: [f32; 3],
    },

    /// Snap a player within the same world after Cell updates spatial state.
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

    /// Send a ghost entity method to one witness.
    WitnessEntityMethod {
        witness_id: u32,
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
        /// Whether `entity_id` is a player entity. Controls the idbase used
        /// in the wire encoding: players use `IDBASE_SGW_PLAYER` (61),
        /// non-players use `IDBASE_NPC_DEFAULT` (62). Method indices ≥61
        /// encode differently under each idbase; wrong selection corrupts
        /// the wire byte for high-index methods.
        entity_is_player: bool,
    },

    /// Hide one entity from one witness without removing its AoI entry.
    EntityInvisible { witness_id: u32, entity_id: u32 },

    /// Start a minigame session for a player (Cell → Base).
    StartMinigame {
        entity_id: u32,
        player_id: i32,
        game_name: String,
        difficulty: u32,
        on_victory_chains: Vec<i64>,
        /// Fired on result codes 2 and 4 (D-MG5).
        on_failure_chains: Vec<i64>,
        /// From the player's cell entity; reaches the SWF's `joinOK`.
        player_level: u32,
        intelligence: u32,
        tech_competency: u32,
        abilities_mask: u32,
    },

    /// The session owner closed the minigame window; cancel its SmartFox task.
    CancelMinigame { entity_id: u32 },

    /// Ticket-only minigame result notification. Base redeems server-owned
    /// result authority (code, chains, `notify_client`) from the registry
    /// receipt before it emits a client close or Cell callback.
    MinigameResult { ticket: String },

    /// Create a new contact list for the player.
    ///
    /// Base assigns the server-side `list_id` (sequence), inserts the row,
    /// and echoes `onContactListUpdate` (CM 85) back to the client with the
    /// assigned id.
    ContactListCreate {
        entity_id: u32,
        player_id: i32,
        name: String,
        flags: u32,
    },

    /// Delete a contact list owned by the player.
    ///
    /// Base validates ownership, deletes the list (members cascade), and
    /// echoes `onContactListDelete` (CM 86).
    ContactListDelete {
        entity_id: u32,
        player_id: i32,
        list_id: i32,
    },

    /// Rename a contact list owned by the player.
    ///
    /// Base validates ownership, updates the name, and echoes
    /// `onContactListUpdate` (CM 85) with the new name.
    ContactListRename {
        entity_id: u32,
        player_id: i32,
        list_id: i32,
        name: String,
    },

    /// Update the flags bitmask on a contact list owned by the player.
    ///
    /// Base validates ownership, updates flags, and echoes
    /// `onContactListUpdate` (CM 85).
    ContactListFlagsUpdate {
        entity_id: u32,
        player_id: i32,
        list_id: i32,
        flags: u32,
    },

    /// Add members to a contact list owned by the player.
    ///
    /// Base validates ownership, inserts the member rows (ignoring duplicates),
    /// and echoes `onContactListAddMembers` (CM 87) with the names that were
    /// successfully added.
    ContactListAddMembers {
        entity_id: u32,
        player_id: i32,
        list_id: i32,
        names: Vec<String>,
    },

    /// Remove members from a contact list owned by the player.
    ///
    /// Base validates ownership, deletes the member rows, and echoes
    /// `onContactListRemoveMembers` (CM 88).
    ContactListRemoveMembers {
        entity_id: u32,
        player_id: i32,
        list_id: i32,
        names: Vec<String>,
    },

    /// Fan out a contact-list presence event to all online players who have
    /// `player_name` in any of their contact lists (CM 89, eventId = event_id).
    ///
    /// Generic cell→base hop for events that originate cell-side (e.g., Death)
    /// but need base context to reach the DB and the transport layer.
    ///
    /// `event_id` is one of the `EVENT_*` bitfield constants in
    /// `base::contact_list::wire` (1=LoggedInStatus, 2=GainLevel, 4=Death,
    /// 8=GateTravel). `data_value` is the event-specific payload (new level,
    /// 0 for death, destination world_id for gate travel).
    ContactListPresenceEvent {
        player_name: String,
        /// The subject's `sgw_player.player_id`, paired with `player_name`.
        player_id: i32,
        event_id: u32,
        data_value: i32,
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

    /// Organization traffic (Squads, Teams, Commands). One nested enum, so
    /// organization packets add variants in `org_cell_to_base.rs` instead
    /// of here (work-packets.md § Messages).
    Org(OrgCellToBase),

    /// Chat traffic (the GM broadcast today). One nested enum, so chat
    /// packets add variants in `chat_cell_to_base.rs` instead of here.
    Chat(ChatCellToBase),

    /// The GM mail tools (`.mail`, `.mailbox`, SS-U1). One nested enum, so
    /// they never touch `MailOp`, which the mail packets own.
    MailGm(MailGmCellToBase),

    /// Bank and vault traffic (the GM `.bankdump`, the vault expansion).
    /// One nested enum, so bank packets add variants in
    /// `bank_cell_to_base.rs` instead of here.
    Bank(BankCellToBase),

    /// The GM `.account` commands (client-tls-oidc C10), answered by the base.
    Account(AccountCellToBase),

    /// A content chain's `send_system_mail` action (SS-U3): one system mail
    /// to the chain's player, behind an optional per-player cooldown.
    ContentSystemMail(ContentSystemMail),
    /// Black Market auction traffic (cell methods 61-64). One nested enum, so
    /// later Black Market packets add variants in
    /// `black_market_cell_to_base.rs` instead of here.
    BlackMarket(BlackMarketCellToBase),

    /// Consume one unit of a native consumable (an item whose
    /// `items_event_sets` event-5 ability the cell applies itself) before
    /// its effect lands. The base answers with
    /// `BaseToCellMsg::ItemUseConsumed` only when the unit was taken. See
    /// `item_use.rs`.
    ConsumeItemForUse(ConsumeItemForUse),

    /// Special-ammo reserve traffic (AM-02): a reload draw or a switch
    /// return. See `ammo_reserve.rs`.
    AmmoReserve(AmmoReserveRequest),

    /// A migrated feature's message (#962, plugin ADR §3.4): a feature-owned
    /// payload the base plugin registered for consumes. New feature
    /// messages go here instead of growing this enum. See `plugin_msg.rs`.
    ///
    /// Crafting's payloads travel here (#962 step 5): the verbs
    /// (`crate::crafting::CraftRequest`), the station reports, the GM
    /// grants (`.allcraft`, `.craftkit`, `.learnblueprint`,
    /// `gmGiveExpertise`, `gmGiveAppliedSciencePoints`) and the respec
    /// opener, consumed by `cimmeria-base-crafting`.
    Plugin(PluginMsg),
}
