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
    DestroyEntity { entity_id: u32 },

    /// Mark an entity as having a client controller (player).
    /// Sent after world entry packets are delivered to the client.
    ConnectEntity { entity_id: u32 },

    /// Remove client controller from an entity (player disconnected).
    DisconnectEntity { entity_id: u32 },

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
        /// Player archetype id from `sgw_player.archetype`. Drives any
        /// archetype-keyed lookups on the cell side — currently the
        /// `Item_Equip`/`Item_Unequip`/`Item_Reload`/`Item_Use` event
        /// set resolution (`ARCHETYPE_ITEM_EVENT_SETS` in the python
        /// source) via [`crate::cell::spawner::archetype_item_event_set`].
        /// Without this, the cell entity's `archetype_id` stays `None`
        /// and the reload animation lookup falls through silently —
        /// see follow-up notes.
        archetype_id: i32,
        /// Saved missions loaded from DB, to be restored before content engine fires.
        saved_missions: Vec<SavedMission>,
        /// Player's known ability IDs (from sgw_player.abilities column).
        abilities: Vec<i32>,
        /// Active bandolier slot from `sgw_player.bandolier_slot` (0-based).
        active_bandolier_slot: i32,
        /// Bandolier slot contents loaded from `sgw_inventory` + `resources.items`.
        bandolier_items: Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
        /// Server-synced client options from `sgw_player.auto_reload` and
        /// `sgw_player.reload_on_activate`. Populates `CellEntity::system_options`
        /// so the auto-reload and reload-on-activate triggers honour the
        /// player's saved preferences instead of falling back to defaults.
        system_options: cimmeria_entity::cell_entity::SystemOptions,
        /// Persisted user-preference state bits from `sgw_player.state_field`
        /// (today: `BSF_AutoCycling` only — see `PERSISTED_STATE_FIELD_MASK`).
        /// The handler masks again on restore, ORs the bits onto
        /// `CellEntity::state_field`, re-arms `abilities.auto_cycle`, and
        /// re-broadcasts `onStateFieldUpdate` so the client's button
        /// highlight survives the relog. (#412)
        state_field: u32,
        /// Account access level (0=Player … 4=Developer) from the login
        /// session (`ConnectedClientState.access_level`, itself sourced from
        /// the `account.accesslevel` DB column). Stored on
        /// `CellEntity::access_level` so the cell-method GM gate can reject
        /// `gm*`/debug methods from non-privileged callers. Authoritative
        /// server-side value — never client-supplied. (#475 / CAT-N-03)
        access_level: u32,
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
    /// `item_id` is the inventory instance row id (per `sgw_inventory.item_id`);
    /// `type_id` is the item design id, needed by content chains keyed on
    /// `item_equipped::<type_id>`.
    InventoryItemMoveApplied {
        entity_id: u32,
        item_id: i32,
        type_id: i32,
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

    /// Base confirmed: ability trained, training point debited, persisted
    /// to `sgw_player.abilities`. Sent in response to
    /// [`crate::cell::messages::CellToBaseMsg::TrainAbility`] after the
    /// DB UPDATE succeeded with `rows_affected == 1` and the
    /// `ConnectedClientState.player_training_points` was decremented.
    ///
    /// On receipt, the cell adds `ability_id` to `entity.abilities` and
    /// broadcasts `onKnownAbilitiesUpdate` so the player's hotbar
    /// refreshes. `training_points_remaining` is informational — useful
    /// to send to the client as a `feedback` line ("You have N training
    /// points left").
    ///
    ///
    AbilityGranted {
        entity_id: u32,
        ability_id: i32,
        training_points_remaining: i32,
    },

    /// Inventory item was used by the player (in response to
    /// `CellToBaseMsg::UseInventoryItem` after base verified ownership).
    /// The cell fires the `OnItemUse` content event with `type_id` (item
    /// design id) so chains conditioned on `item_use::<type_id>` can run.
    ///
    /// `instance_id` is the inventory row id the client clicked — passed
    /// through so the chain context can record which exact instance
    /// initiated the use. `Action::RemoveItem` reads this to remove
    /// THAT specific stack rather than the player's first-by-type
    /// instance, which is the difference between "consume the slappack
    /// you clicked" and "consume the leftmost slappack in the bag."
    ///
    /// Note: base does NOT consume the item before sending this — chains
    /// decide via `Action::RemoveItem`. The historical comment about
    /// "consumption tx" pre-dated the chain-decides-consumption design.
    ItemUsed {
        entity_id: u32,
        instance_id: i32,
        type_id: i32,
        target_id: i32,
    },

    /// Cross-world ring transport: signal the destination cell that a
    /// player has finished loading on the new world and the destination
    /// ring's FSM should advance out of `RemoteLoadWait`. Sent by base's
    /// `handle_client_ready` after a `GateTravel` whose
    /// `destination_ring_id` field was `Some(_)` — i.e. only for
    /// `Effect::TeleportCrossWorld` flows, not stargate dial.
    AdvanceRingDestination { entity_id: u32, region_id: i32 },

    /// Reload the content engine from the database (triggered by admin API / Content Editor).
    ReloadContentEngine,

    /// Minigame result callback (forwarded from BaseApp after minigame server reports).
    MinigameResult {
        entity_id: u32,
        result_code: u8,
        on_victory_chains: Vec<i64>,
    },

    /// Client→server `requestEntityUpdate` (msg `0x07`): the client believes it
    /// is missing or has stale state for one or more entities and is asking the
    /// server to re-emit them. This is the canonical recovery path when a
    /// `createEntity` (`0x09`) for an NPC gets dropped on the wire past the
    /// 20-retry lifetime cap — otherwise the NPC stays permanently invisible
    /// on that client.
    ///
    /// The cell re-emits a synthetic `CellToBaseMsg::EnteredAoI` for each
    /// requested `entity_id` that is currently in `witness_id`'s AoI. Entities
    /// not in the witness's witness set are dropped silently — the client must
    /// not be able to probe arbitrary entity ids.
    RequestEntityUpdate {
        witness_id: u32,
        entity_ids: Vec<u32>,
    },

    /// Base resolved a `gmSpawnByCmd` template into a `SpawnRecord` and is
    /// handing it back to the cell to spawn. Response to
    /// [`crate::cell::messages::CellToBaseMsg::GmSpawnNpc`] — the round-trip
    /// exists because only the base side can query `resources.entity_templates`
    /// to build the record (the cell has no template cache). The cell allocates
    /// an NPC id and calls `spawn_npc_from_record_in_space(id, &record,
    /// space_id)`; AoI fanout handles client visibility on the next tick, so no
    /// extra send is needed. `record.x/y/z` already carry the computed spawn
    /// position from the original command.
    ///
    /// `record` is boxed because `SpawnRecord` is ~380 bytes — large enough
    /// that carrying it inline would balloon every `BaseToCellMsg` variant
    /// (clippy `large_enum_variant`). Boxing keeps the channel cheap to move.
    ///
    /// `requester_entity_id` is the GM entity that issued `gmSpawnByCmd`. The
    /// cell carries it through so it can send the *definitive* "spawned npc
    /// <id>" feedback line to the GM only after `spawn_npc_from_record_in_space`
    /// actually succeeds (the cell is the layer that knows the new NPC id and
    /// whether the spawn took).
    GmSpawnNpcReady {
        record: Box<crate::cell::spawner::SpawnRecord>,
        space_id: u32,
        requester_entity_id: u32,
    },
}
