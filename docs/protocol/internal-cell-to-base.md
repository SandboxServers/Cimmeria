# Internal Cell-to-Base message contract

> **Type:** Reference. **Audience:** Server implementers and reviewers.
> **Last updated:** 2026-10-14.
> **Companions:** [Protocol index](README.md), [documentation index](../readme.md), [source enum](../../crates/wire/src/cell/messages/cell_to_base.rs), [Castle prep packet](../analysis/castle-continuation/worknotes/CS-02b-prep.md).

`CellToBaseMsg` values are in-process `tokio::mpsc` messages from CellApp to BaseApp. They are **not client wire opcodes**, Mercury message IDs, or recovered 2009 layouts. Some Base handlers subsequently encode client Mercury messages; those client sends are separate from this internal channel. This reference preserves the explanatory contract from the [source enum](../../crates/wire/src/cell/messages/cell_to_base.rs); field types and concise safety comments remain beside each variant.

## Space, world entry, AoI, persistence and progression

### `EnterableWorlds`

Every world the cell can deliver a player to (a startup space or an
instanced world), sent once at startup after the `SpaceData` burst.
Character creation refuses a start profile whose world is not in it
(Class Start v6, lock L3); an instanced world such as
`Castle_CellBlock` has no `SpaceData` until someone enters it.

### `EntityMethodCallBatch`

Send a batch of entity-method calls to the **same** target entity's
client, packed into a single Mercury packet body via `ChannelBundle`.

Use when the cell emits a burst of small same-target methods on a
single tick (e.g., world-entry region registrations, initial stat
seed). The base side packs all methods into one packet body, so the
client sees a single UDP datagram with N method-call records instead
of N separate datagrams to ACK.

Wire format on the client side is identical to N separate
`EntityMethodCall`s — only the transport layer collapses the burst.
Reliable channel; ordering within the batch is preserved.

Introduced for the freeze fix (PR #410, 2026-05-26): the world-entry
region-hint burst was firing 22 separate UDP packets in <1 ms, each
needing an individual ACK. The combined ACK pressure stalled some
clients past their render-thread budget. Bundling drops it to one
datagram → one ACK.

### `GateTravel`

Request a gate travel (world transition) for a player entity.

The CellService has already validated the stargate address and removed
the entity from the old space. BaseApp must send RESET_ENTITIES to the
client, then re-create the entity in the new world via the standard
World entry flow (teardown -> create player -> enter world).

`destination_ring_id` carries through cross-world ring transports
(`Effect::TeleportCrossWorld`). When `Some`, base stashes it on the
connected-client state and sends it back via
`BaseToCellMsg::AdvanceRingDestination` once the destination world
finishes its `onClientReady` handshake — that's the deferred hook
the destination ring's FSM waits on to advance out of
`RemoteLoadWait`. Stargate-driven gate travel leaves it `None`.

`destination_space_id` names an *exact* already-loaded space instance
to join, rather than letting the cell pick one by world name. Used by
the GM cross-instance transfer primitive
(`cimmeria_cell_interactions::cell::space_transfer`) so `.goto <player>` lands in the
target's actual instance instead of a freshly-created private one
(`find_or_create_space` always allocates a NEW space for an instanced
world). `None` keeps the historical behavior — resolve by world name.
The id is re-validated cell-side on arrival, because the instance can
be destroyed (last player left) while this message is in flight.

### `GrantStargateAddress`

Persist one learned stargate address onto `sgw_player.known_stargates`.

Sent by the content executor's `Action::GrantStargateAddress` arm
after it has already appended to the player's *in-memory*
`CellEntity::known_stargates` and told the client
(`updateStargateAddress`, client method 66). This message is the
persistence leg only: nothing on the cell waits for it, and its
failure costs the player the address on their next login, not this
session.

The base resolves `account_id` from the session and uses it as the
ownership predicate alongside `player_id`, the same fail-closed pair
`cimmeria_base_world_entry::base::world_entry::gate_travel`'s arrival persistence uses.
The append is idempotent, so a retried or duplicated message cannot
double-append — `resources.stargates.stargate_id` carries no
uniqueness constraint and the column is a bare `integer[]`.

### `ContainerLooted`

Persist one once-per-character loot container onto
`sgw_player.looted_containers` (the content `open_loot` action).

Same shape as `Self::GrantStargateAddress`: the cell has already
added the key to `CellEntity::looted_containers` and opened the loot
window, so this is the persistence leg only. A lost write costs the
flag on the next login (the chest could be opened once more), never
this session. The base resolves `account_id` from the session as the
ownership predicate, and the append is idempotent.

### `MissionUpdate`

Persist a mission state change to the database.

Uses `INSERT ... ON CONFLICT DO UPDATE` on `sgw_mission`. The
`repeats` field MUST be carried through every update — the cell
holds the authoritative count (bumped by `MissionInstance::complete`/
`fail`), and skipping it on UPSERT silently resets the counter on
the row, breaking repeatable-mission gating (#118).

### `GrantXP`

Grant XP to a player entity (from a mob kill).

The CellService computes the XP amount from the mob's level and sends
this to BaseApp, which updates the player's XP/level and sends client
notifications.

`gm_feedback_to`: when `Some(gm_entity_id)`, the base sends a
definitive GM-feedback line to that caller entity — which is NOT
necessarily `entity_id` (the XP recipient); `.givexp` grants to a
selected target while the caller receives the feedback — after the
write commits. Only the GM `gmGiveXp` / `.givexp` paths set this;
non-GM senders (mob-kill XP) leave it `None`.

### `TrainAbility`

Buy an ability-tree node for a player: debit the node's cost, add
it to the archetype-wide spend, and persist the ability to
`sgw_player.abilities` and `trained_abilities`, all in one `UPDATE`.
The base owns `training_points` and the DB row, so this round-trip
is the only correct path.

The cell has already passed `ability_tree::evaluate_train` (every
node, spend and point gate). The base re-checks only what the row
itself decides atomically: `training_points >= cost` and the
ability not already known.

On success, base responds with
`crate::cell::messages::BaseToCellMsg::AbilityGranted`.

### `ResetAbilities`

Respec (`resetMyAbilities`, AT-08): remove and refund every
trainer-bought ability, reset the spend, and charge `cost` naquadah,
all in one guarded `UPDATE`.

The cell has already checked that the player stands at a pinned
trainer in range. The base decides the rest from the row: whether
anything is trainer-bought and whether the player can pay. It always
answers with `crate::cell::messages::BaseToCellMsg::AbilitiesReset`,
except when the session no longer plays `player_id`.

### `GrantItem`

Grant an item to a player and persist to `sgw_inventory`.

`notify_gm`: when true, the base sends a definitive GM-feedback line to
`entity_id` after the write commits. Only the GM `gmGiveItem` path sets
this; non-GM senders (loot pickup, content-chain `Action::GrantItem`)
leave it false. `loot` is set only by loot pickup: a refused grant is
answered with `BaseToCellMsg::LootGrantRefused` so the item goes back.

### `RemoveInventoryItem`

Remove quantity from an inventory item instance.

`notify_gm`: when true, the base sends a definitive GM-feedback line to
`entity_id` after the write commits. Only the GM `gmRemoveItem` path
sets this; non-GM senders (content-chain `Action::RemoveItem`, the
`removeItem` inventory method) leave it false.

### `RemoveInventoryItemByType`

Remove `count` of an item by **design id** (`type_id`) — chains know
design ids, not instance ids. Base resolves the player's first
matching instance and applies the same wire-update sequence as
`RemoveInventoryItem`. Used by `Action::RemoveItem` in the cell
content executor (e.g., chain 1034 consumes the Ambernol vial after
`OnItemUse` fires).

### `UseInventoryItem`

Consume one charge/stack of an inventory item instance, then fire the
content-engine `OnItemUse` event back to the cell with the resolved
`type_id` (item design id). Mission progression that depends on item
use is gated on the consumption committing successfully — if the row
can't be found or the tx fails, no event fires and the mission does
not advance.

`item_id` is the inventory instance id from the wire (`useItem` arg
per `SGWInventoryManager.def` — "Player inventory id"). The type_id is
looked up server-side and returned via `BaseToCellMsg::ItemUsed`.

## Position, appearance, equipment, currency and abilities

### `PersistPosition`

Persist the player's last known world + position when the session
ends (sent from the `DisconnectEntity` arm, before teardown), so the
next login resumes where the player logged out.

Until this existed, `sgw_player.world_location` / `pos_*` were
written only by gate travel and the GM teleport, so a returning
character spawned at the last gate arrival — or the creation point —
regardless of where they logged out. Same cell-mutates /
base-persists split as `SystemOptionsUpdate`; the base handler resolves
`world_id` from `resources.worlds` the way gate travel does.

### `ShutdownFlush`

Server shutdown (TG-DB-02): persist every in-world player's position
and dirty bandolier ammo in parallel, by the request's deadline, and
answer with the players saved. Sent once, from the cell loop's
shutdown arm; the base handles it inline, so every message queued
before it has already been handled when the answer comes back.

### `ActiveSlotUpdate`

Persist the player's active bandolier slot.

`entity_id` is carried so the base handler can broadcast a fresh
`BEING_APPEARANCE` to the player's witnesses after the slot is
persisted — without the appearance refresh, the player's model
keeps the previously-equipped weapon visual until the next login.

### `SystemOptionsUpdate`

Persist the player's server-synced client options after a cell-side
`updateSystemOptions` (player method 93) applies. Mirrors the same
"cell mutates in-memory state, base persists to DB" split as
`ActiveSlotUpdate`. Only the columns the client sent get the
updated values; defaults are pre-merged on the cell side so we
always send the *full* SystemOptions block (a partial update
would require column-aware SQL we don't need at scale today).

### `RefreshAppearance`

Re-broadcast `BeingAppearance` to the player's AoI with a fresh
holster state. Used by the combat enter/exit path (and any other
runtime holster toggle, e.g. the `requestHolsterWeapon` button)
to draw or holster the weapon without requiring an inventory
change.

The base handler updates `ConnectedClientState::weapon_holstered`
so any subsequent appearance refresh (item equip, slot swap,
world re-entry) inherits the live holster state instead of the
hardcoded spawn-holstered default. It then re-queries the
player's appearance and broadcasts the resulting packet — same
path as `ActiveSlotUpdate`.

**Phase 2 of the holster work**: Phase 1 made spawn
always holstered; this message is what flips it back on enter
combat.

### `BandolierAmmoUpdate`

Persist a single bandolier slot's per-slot ammo state.

Sent by the cell on the batched cadence (reload completion, slot swap,
`requestAmmoChange`, logout) — see Stage B/C/D in
docs/gameplay/weapon-ammo-reload.md (TBD). `player_id` here matches the
DB `character_id`, mirroring the field naming used by `ActiveSlotUpdate`.

`expected_instance_id` guards against TOCTOU: it is the
`sgw_inventory.item_id` per-row instance id (NOT the design id) of the
slot's item at the time the writeback was computed.
If the slot's physical item changes between the cell sending this
message and the base writing the row, the SQL
`WHERE item_id = $expected_instance_id` clause skips the write rather
than scribbling stale ammo onto the new weapon — even when the new
weapon shares the same design as the old one.

### `GrantCash`

Grant cash (naquadah) to a player and persist to the database.

`gm_feedback_to`: when `Some(gm_entity_id)`, the base sends a
definitive GM-feedback line to that caller entity — which is NOT
necessarily `entity_id` (the cash recipient); `.givecash` grants to a
selected target while the caller receives the feedback — after the
write commits. Only the GM `gmGiveCash` / `.givecash` paths set this;
non-GM senders (loot pickup) leave it `None`.

### `GrantTrainingPoints`

Grant training points and persist to the database
(`gmGiveTrainingPoints`). The base adds `amount` to
`sgw_player.training_points` in one guarded `UPDATE`, refreshes its
point cache, answers the cell with
`crate::cell::messages::BaseToCellMsg::TrainingPointsGranted` (which
mirrors the points and sends the client counter), and sends the
feedback line to `gm_feedback_to`. `amount` is validated `> 0`
cell-side; the base re-checks it.

### `GmGrantAbility`

Grant one ability and persist it (the GM `.giveability` console
command, pets campaign PT-07). **Only `cimmeria-cell-console` builds
this**: the base does not re-check GM rights, it trusts that the
console's channel gate (`is_gm` on the server-side `access_level`)
already ran.

The base appends `ability_id` to `sgw_player.abilities` in one guarded
`UPDATE` (a no-op when already known), never to `trained_abilities`,
so a trainer respec keeps it and refunds nothing for it. It answers
the cell with
`crate::cell::messages::BaseToCellMsg::GmAbilityGranted` and sends
the outcome line to the GM, but only while `gm_entity_id`'s session
still plays `gm_player_id` (an entity id is recycled on relog).

## GM, authoring, teleport and witnesses

### `ExecuteAuthoringSql`

Execute a server-generated authoring SQL statement against the live DB
(`.`-console). The cell has no DB pool, so the spawn/patrol
authoring commands hand their `INSERT`/`UPDATE`/`DELETE` to the base,
which runs it and reports the row count back to the GM via the feedback
channel. The write is intentionally transient — it lets the developer
see the change hold across reconnects within the current deploy, but the
next deploy rebuilds from `db/resources/` seeds and wipes it (the
durable artifact is the seed SQL emitted by `.seedconfirm`).

**Trust model:** the `.`-channel is GM-gated server-side, and `sql` is
*server-generated* — numeric values are formatted from cell-parsed
`i32`/`f32` and strings are escaped through
`cimmeria_cell_console::cell::console::seed::sql_str`, so no raw client text is
concatenated into the statement. This mirrors the legacy
`Atrea.dbQuery` authoring path. `label` is a short human tag for the
feedback line (e.g. `"savespawn"`).

### `ConsoleSearch`

Run a read-only name search against a resource table and feed the
matches back to the GM (`.searchitem` / `.searchmission` /
`.searchtemplate`). The cell caches resource ids but not their display
names (those live only in the base-side DB), so the search runs base-side
and reports `id: name` lines on the feedback channel. `kind` selects the
table (`0` items, `1` missions, `2` entity_templates); `query` is the
case-insensitive substring (used as a parameterized `ILIKE` bind — no
string concatenation).

### `GmSpawnNpc`

Spawn an NPC by template id at a computed position (`gmSpawnByCmd`).

Round-trip sink, mirroring `TrainAbility` → `AbilityGranted`: the cell
cannot build a `SpawnRecord` for an arbitrary `template_id` (it has no
template cache), so it hands the request to the base, which queries
`resources.entity_templates`, materializes a `SpawnRecord`, and sends it
back via `crate::cell::messages::BaseToCellMsg::GmSpawnNpcReady`. The
cell then allocates an NPC id and spawns it into `space_id`. `position`
is the final spawn position (caller position + the command's X/Z
offsets, already validated finite cell-side).

`heading` is the spawn yaw in radians, written straight into the
materialized `SpawnRecord.heading` (which
`spawn_npc_from_record_into` turns into `direction = (0, heading, 0)`).
The dot-console `.spawn` / `.spawnrandom` send the caller's own facing
here, matching legacy `Resource.spawnEntity`'s
`space.createEntity(template, player.position, player.rotation)`. The
native `gmSpawnByCmd` keeps sending `0.0` — its wire signature has no
rotation argument, so there is nothing to forward.

`batch` is `Some` only for a dot-console `.spawnrandom` copy (lab-chaos
CH-13): the cell's spawn batch id, echoed back on
`crate::cell::messages::BaseToCellMsg::GmSpawnNpcReady` or
`crate::cell::messages::BaseToCellMsg::GmSpawnNpcFailed` so the cell
can report every spawned id in one summary line. `None` (`.spawn`, the
native `gmSpawnByCmd`) keeps the per-spawn feedback lines.

### `ReanchorPlayer`

Re-anchor the local pawn to a fresh actor without `RESET_ENTITIES`.

BaseApp's `handle_reanchor_player` sends two packets to the client:

1. A burst combining `BASEMSG_CREATE_BASE_PLAYER` +
   `BASEMSG_SPACE_VIEWPORT_INFO` + `BASEMSG_CREATE_CELL_PLAYER` +
   `BASEMSG_FORCED_POSITION`.
2. A separate bundle with `BeingAppearance` + `onEntityTint`
   pulled from the client's cached world-entry args.

`CREATE_BASE_PLAYER` (0x05) is the load-bearing piece — it invokes
the client's `createBasePlayer` callback (the same hook used on
initial login), which destroys the existing pawn actor (carrying
the ragdoll physics state from the `Entity_Death` kismet) and
instantiates a fresh standing one. The trailing VIEWPORT/CELL/
FORCED_POSITION keep the client's space tables consistent with
the new pawn, and the property replay repopulates its visuals.

**No `RESET_ENTITIES` is sent**, so all other client-side state —
AoI entities, kismet sequence state (door open/closed, triggered
events), the level itself — survives the respawn untouched.

Used by `handle_respawn`. Cross-world respawn falls back to `GateTravel`
since the player is leaving the space anyway.

Why it's separate from `TeleportPlayer`: TeleportPlayer sends only
`BASEMSG_FORCED_POSITION` (a position snap), which doesn't re-create
the pawn actor and so doesn't clear ragdoll state. ReanchorPlayer
is the stronger primitive that drives a full pawn rebuild.

### `TeleportPlayer`

Authoritative same-world teleport. The cell has already updated its
spatial state via `update_entity_position`; the base must now push the
new position to the player's own client. Engine-level snap, no loading
screen, no world reload — used by ring transporters and any other
in-world short-hop teleport.

`space_id` is the cell's authoritative space — the base trusts this
over its connection-cached `world_name`, which can lag during world
transitions.

Other witnesses receive their position update through the next AoI
tick's `EntityMoved` broadcast — no extra fan-out is needed here.

### `WitnessEntityMethod`

Send a ghost entity method call to a specific witness player.

Used for broadcasting property updates (InteractionType, SetVisible, etc.)
to players who have the entity in their AoI. The `entity_id` is the ghost
entity the method is called on; `witness_id` is the player to send to.

### `EntityInvisible`

Send `BASEMSG_ENTITY_INVISIBLE (0x0B)` to a single witness without the
follow-up `LEAVE_AOI (0x0C)` — visually hides the entity on that
client while keeping it in AoI bookkeeping. Mirror of C++
`ClientHandler::leaveAoI(entity_id, deleteEntity=false)`. Used by the
ring-transport hide phase; pair with a `WitnessEntityMethod` calling
`onVisible(1)` to restore.

## Minigame boundary

`StartMinigame` carries the launch request and `CancelMinigame` names the
entity to close. `MinigameResult { ticket }` is now an internal notification
only. The minigame registry holds the one-use terminal receipt with the
original Base owner, result and registered victory chains. Base redeems it
before any client close or Cell forward. This internal change does not alter
client Mercury or SmartFox bytes and does not authorize a Castle callback;
the Cell launch-incarnation check remains in [CS-02b2](../analysis/castle-continuation/work-packets.md).
