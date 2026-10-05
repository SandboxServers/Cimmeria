---
title: "ADR: feature plugins over CellEntity and SpaceManager (no ECS)"
type: explanation
audience: engineers
last_updated: 2026-09-28
---

# ADR: feature plugins over `CellEntity` and `SpaceManager` (no ECS)

> **Status:** Accepted (owner decision, 2026-09-28, [#962](https://github.com/SandboxServers/Cimmeria/issues/962)). Pilot: pets (`cimmeria-cell-pets`), measured in §4.1. Step 2: duels (`cimmeria-cell-duel`), §4.2. Step 3: squads and org creation (`cimmeria-cell-org`), §4.3. Step 4: effect scripts (`cimmeria-cell-effect-scripts`, a registry move), §4.4. Step 5: the `BasePlugin` core and crafting (`cimmeria-base-crafting`), §4.5. Amends [services-crate-split.md](services-crate-split.md).
> **Type:** Architecture decision record
> **Owner:** Server architecture
> **Companion docs:** [services-crate-split.md](services-crate-split.md) (the layer split this builds on), [build-system.md](build-system.md) (the build lane and the rebuild measurements), [negative-logging-convention.md](negative-logging-convention.md) (the hook-miss logs), [scaling-analysis.md](scaling-analysis.md) (why the base/cell split buys us nothing), [../protocol/cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md) (the method indices plugins register against)

## TL;DR

- **Features become plugins.** A `CellPlugin` (and, later, a `BasePlugin`) registers its method-index handlers, its event hooks and its per-entity state at startup. The server's core (`SpaceManager`, the cell loop, the router, the Mercury layer) stops naming features.
- **No ECS.** `CellEntity` and `SpaceManager` stay. Per-entity feature state moves into a type-keyed extension map on `CellEntity`, so the struct stops growing a field per feature. `bevy_ecs` was rejected for now.
- **The wire stays byte-identical.** Plugins register against the method indices the client already uses, emit on the same FIFO channel as today, and fire from the exact positions the inline calls had. The existing wire-format, chain-replay, wire-level replay and live-DB suites are the proof.
- **The composition root owns the plugin table.** `cimmeria-services` lists the plugins in a fixed order. Startup fails if a method index that belongs to plugins has no handler, has two, or is not a client method.
- **Migration is incremental.** Pets is the pilot. Other features move at their campaign close-outs, never mid-campaign.

## 1. Context

The services split ([services-crate-split.md](services-crate-split.md)) cut an edit from a ~240k-line recompile to one crate and the handful above it. It split the code by BigWorld's layers: the cell track is a chain (world, combat, content, interactions, methods, console, cell). The campaigns (crafting, pets, orgs, social, bank, black market) cut across every layer, so the long tail stayed. #962 measured it after the split:

- Of the 82 PRs merged since the split (`83154f8b`, 2026-09-26), the median rebuilds **73%** of the workspace's lines, and 53 of 82 rebuild more than 60%.
- The rebuild weight by crate (production-code changes times lines rebuilt above the crate) is `wire` 26%, `entity` 15%, `cell-world` 11%, `cell-catalog` 8%, `base-session` 7%.
- A PR touches a median of about 5 crates. `base-world-entry` and `wire` change together in 81% of `base-world-entry`'s commits.

The reason is structural, not a matter of more splitting:

- **Every feature edits the central types.** `CellToBaseMsg` / `BaseToCellMsg` (named in about 540 and 170 files across 16 crates), `CellEntity` (a field per feature: `pet`, `vault_session`, `trade_proposal`, `crafting_stations`, …) and `ConnectedClientState` sit at the bottom of the graph.
- **`SpaceManager` is a hub because it owns and calls the features.** It holds the duel, pet, squad, org-creation, ring and cover registries as fields, and calls into them on disconnect, AoI enter, space teardown and spawn.
- **The static routers name every feature.** `dispatch_cell_method` walks one arm per interface; `cell_methods::player::dispatch` matches index ranges per feature; the cell loop calls every feature's tick by name.

A feature edit therefore touches a low crate, and everything above it rebuilds.

## 2. Client compatibility: what the plugin model must not change

The client is immutable. This section is the BigWorld engine advisor's confirmation of the list in #962, with the detail a plugin author needs. Everything not listed here is server organization and is free to change.

| # | Constraint | Confirmed | What it means for a plugin |
|---|---|---|---|
| C1 | Mercury framing, encryption, acks, fragmentation | Yes | Plugins never touch it. It stays in `cimmeria-mercury` and the base's connection loop. |
| C2 | Message ids, entity type ids (the clientIndex, `<ServerOnly/>` skipped: `Account = 0x07`), method indices | Yes | Indices come from the `.def` parse order (Implements, then Properties, then Methods; exposed methods only). A plugin registers the constant from `cimmeria-wire` and never computes an index. |
| C3 | The client addresses base and cell methods through different paths | Yes | Cell methods arrive as `index \| 0x80` (direct, 0-60) or `0xBD` plus a sub-index (61+); base methods as `index \| 0xC0` ([message-catalog.md](../protocol/message-catalog.md), [cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md)). The client picks the path from the `.def`, so the cell-method and base-method index spaces are separate registries. Index 88 on the cell is `petInvokeAbility`; index 88 on the base is something else. Decoding (the `0x80` / `0xBD` split, the 4-byte entity-id prefix) stays in core; plugins see the flattened index and the argument bytes. Which server code handles a call is free; the path it arrives on is not. |
| C4 | Property-sync formats and ordering | Yes, with a caveat | Property ids follow the `.def` parse order (1-byte ids 0-59, 2-byte from 60; [entity-property-sync.md](../protocol/entity-property-sync.md)). The 436 properties in `entities/defs/` use only `CELL_PRIVATE`, `BASE` and `CELL_PUBLIC`; no `OWN_CLIENT`, `OTHER_CLIENTS` or `ALL_CLIENTS` flag appears anywhere. So nothing replicates to the client on its own: every property and list the client sees is an explicit client-method send from server code. **Moving state into the extension map cannot change what the client sees**, because nothing syncs implicitly. What can change it is the *order* of sends; see C6. |
| C5 | Position and forced-position formats | Yes | Core. `forcedPosition` is 49 bytes in SGW (velocity and flags added). Plugins call the existing builders. |
| C6 | Message order on the wire | Yes | The client orders the reliable stream (a 512-packet window, adopting the first sequence number) and delivers unreliable packets on arrival. Within one channel, the order the server queues messages is the order the client applies them. Everything a cell handler sends goes through one FIFO `mpsc` channel (`CellToBaseMsg`) to the base, which bundles it for the client. **A plugin must emit on that same channel, in the same order as the inline code did.** A per-feature channel, or a hook that fires at a different point in a tick, reorders property updates and method calls against each other. |
| C7 | AoI enter/leave ordering | Yes | `CREATE_ENTITY` (the `createOnClient` property cascade) comes before any method call or property update for that entity, and owner-only lists (a pet's ability and stance lists) come after it. The client does buffer early messages for an unknown id, but the server must not rely on that. An AoI-enter hook appends to the event list at the position the inline call had. |
| C8 | World entry (`resetEntities` / `ENABLE_ENTITIES`, the 8-byte SGW payload, the entry phases) | Yes (added) | Core. No plugin hooks inside the world-entry handshake. |

The base/cell split itself is BigWorld's, not the client's. The client knows only the two addressing paths (C3) and that the player's base and cell entity share one id. We run one process, and base and cell talk over in-process channels. How the server divides work behind the socket is free.

## 3. Decision

A plugin architecture over the existing `CellEntity` and `SpaceManager`, without an ECS.

### 3.1 The `Plugin` trait

```rust
// cimmeria-cell-world: cell::plugin
pub trait CellPlugin: Send + Sync + 'static {
    /// Stable name for logs and startup errors ("pets").
    fn name(&self) -> &'static str;
    /// Register handlers and hooks. Called once, at startup, in table order.
    fn build(&self, plugin: &mut CellPluginBuilder<'_>);
}
```

`CellPluginBuilder` collects registrations; `CellPlugins::build(&[&dyn CellPlugin])` validates them (§3.3) and freezes them into an `Arc`-shared registry. The registry is installed on the `SpaceManager` before the cell loop starts, because every call site that fires a hook already holds `&mut SpaceManager`. A hook call clones the `Arc` first (`let plugins = space_mgr.plugins().clone();`), so the registry and the `&mut SpaceManager` it passes to the handler never alias.

Handlers are plain `fn` pointers returning a boxed `Send` future, not closures or trait objects with state. A plugin's state lives in the world (the extension map or a `SpaceManager` resource), never in the plugin value, so a handler cannot hold state the world does not see.

```rust
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub struct CellMethodCall<'a> {
    pub entity_id: u32,
    pub method_index: u16,
    pub args: &'a [u8],
    pub tx: &'a mpsc::Sender<CellToBaseMsg>,
    pub space_mgr: &'a mut SpaceManager,
    pub engine: &'a ChainEngine,
}
pub type CellMethodHandler = for<'a> fn(CellMethodCall<'a>) -> BoxFuture<'a, ()>;
pub type TickHook =
    for<'a> fn(&'a mpsc::Sender<CellToBaseMsg>, &'a mut SpaceManager) -> BoxFuture<'a, ()>;
pub type EntityHook =
    for<'a> fn(u32, &'a mpsc::Sender<CellToBaseMsg>, &'a mut SpaceManager) -> BoxFuture<'a, ()>;
```

The base track has the same shape (`BasePlugin`, `BasePluginBuilder`, `BasePlugins`, in `cimmeria-base-session`'s `base::plugin`; §4.5). A `BaseMethodCall` carries the flattened base-method index, the argument bytes and a `BaseCtx` (the transport, the session maps, the channel to the cell and the database pool, the same fields as crafting's `CraftCtx`). There is no base hub that every handler holds, as the cell has `SpaceManager`, so each session carries the registry it was admitted under (`ConnectedClientState::plugins`), and the cell-message loop gets it from the `BaseService`. Crafting (`CraftingPlugin`) is the first base plugin.

### 3.2 Registration order and determinism

- **One table, in code.** The composition root (`cimmeria-services`) lists the plugins in a fixed slice. There is no link-time discovery (`inventory`, `linkme`): link order is not a contract, and a plugin that silently fell out of the link would be a missing registration nobody sees.
- **Hooks fire where the inline call was.** A hook point is a named position in core (`TickStage::AfterRingTransport`, `EntityHookPoint::BeforeBaseDestroy`, …), placed at exactly the line the feature's inline call occupied. Moving a feature must not move its work to a different point in the tick (C6).
- **Subscribers run in table order.** Where more than one plugin subscribes to one hook point and the order reaches the wire, a test pins the order.
- **Method handlers have no order.** Each index has exactly one owner (§3.3), so the lookup order cannot matter.
- **The GM gate stays first.** `dispatch_cell_method` runs the server-authority GM gate (#475) before it looks up a plugin handler, so a plugin cannot open a path around it.

### 3.3 Method-index dispatch and the startup assertion

A plugin registers a cell method by its flattened index, with the constant from `cimmeria-wire`:

```rust
plugin.cell_method(PET_INVOKE_ABILITY, invoke);   // 88
plugin.cell_method(PET_ABILITY_TOGGLE, toggle);   // 89
plugin.cell_method(PET_CHANGE_STANCE, stance);    // 90
```

The router looks the index up in the plugin registry after the GM gate; an index no plugin owns falls through to the static per-interface routers, as today. While migration is under way, core keeps one list, `PLUGIN_OWNED_CELL_METHODS`, of the indices that have left the static routers. `CellPlugins::build` fails when:

- two registrations claim one index (`DuplicateCellMethod`);
- a registration names an index that is not a client cell method (`cell_method_name(index) == "unknown"`, `UnknownCellMethod`);
- a registration names an index that is not on the plugin-owned list, so the static router could still claim it (`NotPluginOwned`).

A second check, `CellPlugins::check_complete`, fails when an index on the plugin-owned list has no handler. The orchestrator refuses to start the server on either failure; `CellService::start` also logs the missing indices at WARN, so a test harness that starts a bare cell sees the gap too. When the last interface migrates, `PLUGIN_OWNED_CELL_METHODS` is every exposed index in the `.def`, the static routers are gone, and the assertion is exactly "every client method index has exactly one handler". Methods the server does not implement are then registered explicitly as stubs that log, rather than falling through.

A test beside the default plugin table checks the other direction: for every plugin-owned index, the static router returns `false`, so no stale arm shadows a plugin.

### 3.4 The central message enums stop growing

`CellToBaseMsg` and `BaseToCellMsg` are named in hundreds of files, and adding a variant rebuilds every one. The per-feature sub-enums already in `cimmeria-wire` (`bank_cell_to_base`, `org_cell_to_base`, …) do not help: the central enum still names them, so `wire` must depend on them.

New feature messages go through one envelope variant instead:

```rust
pub enum CellToBaseMsg {
    // ... the existing variants, unchanged ...
    Plugin(PluginMsg),
}
pub struct PluginMsg {
    type_name: &'static str,       // for logs
    payload: Box<dyn Any + Send>,  // a feature-owned type
}
```

- The feature crate defines the payload type. The receiving side registers a consumer for it (`base_plugin.on_cell_message::<VaultOpened>(handler)`) and downcasts.
- The envelope travels on the **same** channel as every other message, so it keeps its FIFO position (C6). Per-feature channels are forbidden for this reason.
- Messages never leave the process, so `Box<dyn Any + Send>` costs one allocation and no serialization.
- Exhaustiveness is replaced by a startup check (every payload type a plugin declares it sends has exactly one consumer) and a negative log (a `PluginMsg` with no consumer logs WARN with its `type_name` and is dropped).

The envelope landed with the `BasePlugin` core (§4.5): `CellToBaseMsg::Plugin(PluginMsg)` in `cimmeria-wire`, `BasePluginBuilder::on_cell_message::<T>(handler)`, and `PLUGIN_CELL_MESSAGES`, the declared payload types that `check_complete` requires one consumer each for. Existing variants stay until their feature migrates; a migrating feature moves its variants into the envelope at its close-out. Crafting was the first: its seven variants (`Crafting`, `CraftingStations`, `GmAllCraft`, `GmCraftGrant`, `RespecCraftOpen`, `GrantExpertise`, `GrantAppliedSciencePoints`) left the enum; the payload types stay in `cimmeria-wire::crafting`, because the cell code that sends them (`cell-methods`, `cell-console`, `cimmeria-cell`) sits below the plugin. The pets pilot needs no envelope: its three cell methods reply through the existing `EntityMethodCall` / `WitnessEntityMethod` variants. #962 estimated the envelope alone would let about 74% of recent `wire` and `entity` commits avoid a full rebuild, though those still rebuild 58-90%; it is worth doing, but it is not the main lever. Moving feature code out of the low crates is.

### 3.5 The extension map

```rust
// cimmeria-entity: cell_entity::extensions
pub struct EntityExtensions { /* Vec<(TypeId, &'static str, Box<dyn Any + Send + Sync>)> */ }

impl EntityExtensions {
    pub fn get<T: Any>(&self) -> Option<&T>;
    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T>;
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> Option<T>;
    pub fn remove<T: Any>(&mut self) -> Option<T>;
    pub fn contains<T: Any>(&self) -> bool;
}
// on CellEntity:
pub extensions: EntityExtensions,
```

- **One slot per type.** The key is the Rust type, so a feature owns its slot by owning its type (`PetState`). Use a newtype for anything generic; never store a bare `u32` or `Vec`.
- **Storage is a small `Vec`, scanned linearly.** An entity carries zero to a few extensions; an empty map allocates nothing and costs 24 bytes, against 8 for the `Option<Box<PetState>>` it replaces.
- **The type lives with its lowest reader.** A feature crate that is a leaf owns its types. While lower crates still read a type (combat reads `PetState` for kill credit and the pet AI), it stays where they can see it (`cimmeria-entity`), and only the storage becomes generic. It moves up when its last lower reader does.
- **Not replicated, not persisted.** Nothing in the map reaches the client or the database on its own; the feature's code sends and saves explicitly, as today (C4).
- **The same type serves the base and the space.** `ConnectedClientState::extensions` (`base::plugin::SessionExtensions`) holds per-session feature state since §4.5. `SpaceManager::resources` (a `SpaceResources`, the same storage) holds space-wide feature state; the duel registry moved there in step 2 (§4.2), and the squad registry and the pending organization creations in step 3 (§4.3). A feature reads its resource through an extension trait on the map, called on the field (`mgr.resources.duels()`, `mgr.resources.squads()`), so the borrow stays on that one field. It stores the value on first write, so a manager with no feature installed reads an empty one. The pet registry follows when its feature moves.

### 3.6 Hook seams and missing registrations

The test rule from #962: a missing registration must log a warning or fail the startup assertion, never silently no-op.

| Seam | A missing registration … | Test |
|---|---|---|
| Plugin-owned cell method | fails startup (`check_complete`); a bare `CellService::start` logs WARN with the missing indices | Unit test on `CellPlugins`; startup test in the facade |
| Duplicate, unknown or not-plugin-owned method index | fails `CellPlugins::build` | Unit tests per error |
| Cell method dispatched with no handler at runtime | falls through to the router's existing WARN, `Unhandled cell method call` (#311) | Router test with an empty registry and a pet index |
| Tick, entity and death hook points | are covered by the plugin's method registrations: a plugin is installed whole or not at all, and a missing plugin fails `check_complete` | The facade test that the default table passes `check_complete` |
| Plugin-owned base method, or a declared `PluginMsg` payload type | fails startup (`BasePlugins::check_complete`); a bare `BaseService::start` logs WARN (`base.plugin`, `plugin_table_incomplete`) | Unit tests on `BasePlugins` |
| Duplicate, unknown or not-plugin-owned base-method index; a second or undeclared consumer | fails `BasePlugins::build` | Unit tests per error |
| Base method dispatched with no handler at runtime | falls through to the static router's WARN, `Unhandled SGWPlayer base method` (#311) | `cimmeria-base`'s `dispatch::tests::plugin_routing` |
| `PluginMsg` with no consumer at runtime | WARN at `base.plugin` with `reason = "no_consumer"` and `type_name`, message dropped | Unit test on the registry; the `Plugin` arm's dispatch test |

The WARN rows follow [negative-logging-convention.md](negative-logging-convention.md): a `reason` field, the method index or type name, and a message that says what the player loses.

### 3.7 Crate layout and rebuild fan-out

```text
core     cimmeria-entity        EntityExtensions on CellEntity (and, as SpaceResources, on SpaceManager)
         cimmeria-cell-world    cell::plugin (trait, builder, registry, hook points); SpaceManager holds the registry
         cimmeria-cell          the router consults the registry; the cell loop fires the tick hooks
         cimmeria-base-session  base::plugin (BasePlugin, builder, registry, session hook points); ConnectedClientState::{extensions, plugins}
         cimmeria-base          the base-method router consults the session's registry; the service holds the table
systems  cell-combat, cell-content, cell-interactions, cell-methods, cell-console   (unchanged)
leaves   cimmeria-cell-pets     PetsPlugin: the pet cell methods and the pet hooks
         cimmeria-cell-duel     DuelPlugin: the duel cell methods, the duel tick and the leave hooks
         cimmeria-cell-org      OrgPlugin: the organization cell methods, the disconnect and world-entry hooks
         cimmeria-cell-effect-scripts   every EffectScript and the EFFECT_SCRIPTS table (a registry, not a plugin)
         cimmeria-base-crafting CraftingPlugin: the crafting envelope consumers, the session and seam hooks, base::crafting
root     cimmeria-services      the plugin tables; installs them on the CellService and the BaseService
```

A leaf depends on core and on the systems it calls; only the composition root (and test code) depends on a leaf. A leaf edit rebuilds the leaf, `cimmeria-services` and what depends on the facade (the server, the admin API, the lab endpoint, the wire client). It no longer rebuilds `cell-methods`, `cell-console` or `cell`.

The estimates in #962 for the full migration: a crafting edit rebuilds about 53k lines instead of about 135k; mail about 45k, org about 42k, bank or chat about 33k. The envelope and the `entity-types` split add a 15-20% cut on the `wire` and `entity` long tail.

**Pilot measurement.** A pets edit went from 8 rebuilt crates to 7 in the workspace build, and from about 49k to about 14k production lines in the server build; see §4.1. A duels edit went from 13 to 9 crates (three of them test binaries only) and from about 113k to about 10k production lines; see §4.2. A squad edit went from 8 to 7 crates (one of them a test binary only) and from about 48k to about 14k production lines; see §4.3. An effect-script edit went from 15 to 9 crates (three of them test binaries only) and from about 103k to about 10k production lines; see §4.4. A crafting edit went from 8 crates to 5 in the server build, and from about 83k to about 21k production lines; see §4.5.

### 3.8 Migration order

Features move at their campaign's close-out, never while a campaign coordinator has packets in flight on that code.

1. **Pets (pilot).** The three pet cell methods (88-90), the pet tick hooks and the base-destroy hook become `PetsPlugin` in `cimmeria-cell-pets`; `CellEntity::pet` becomes an extension. The pet world half (the registry, spawn, teardown, owner hooks) stays in `cell-world`, and the pet AI and owner abilities stay in `cell-combat`, because combat, content, interactions and the console call them. They follow when those call sites go behind hooks (death credit, owner-path hooks).
2. **Duels** (`cell-duel`; Social Systems closed 2026-09-27). **Done** (§4.2). The duel cell methods, the duel tick and the disconnect, travel and death paths became `DuelPlugin`, and the registry became a `SpaceManager` resource. The challenge, the end paths, the non-lethal clamp, the harm-gate inputs and the GM commands stay in `cell-world` until combat, the AoI enter path and the base-message handler get seams for them.
3. **Squads and org creation** (`cell-org`; Organizations closed 2026-09-27). **Done** (§4.3). The OrganizationMember cell methods (8-19) and `onOrganizationCreation` (94), the squad disconnect, the registrar offer's end and the squad world-entry replay became `OrgPlugin`; the squad registry and the pending creations became `SpaceManager` resources, and `CellEntity::squad_id` an extension. The console-to-methods edge (`console/squad.rs`) is gone: the half the base-message handler and the console call (the base-forwarded squad invite and kick, the GM squad commands, the registrar reply and create result, and the squad fanout) moved from `cell-methods` down to `cell-interactions`, below both.
4. **Effect scripts** (`cell-effect-scripts`). **Done** (§4.4). A registry move rather than a plugin: every `EffectScript` moved to the leaf with its tests, the static `match` in `effects/registry.rs` became the leaf's `EFFECT_SCRIPTS` table, and `cell-world` keeps the trait, dispatch and an `EffectScripts` registry type the composition root builds at startup and the cell installs on its `SpaceManager`. The passive pass, the pet-script name predicates, the stat-buff ledger and the ammo shot helpers stay in `cell-world`, because layers below the leaf call them. Changed [abilities-and-effects-system.md](abilities-and-effects-system.md) (decision 33) and the CLAUDE.md line on where scripts go.
5. **The base features** (crafting after its close-out CR-13, then bank, mail, chat, vendor with trade, inventory, progression). **Core and crafting done** (§4.5): `BasePlugin`, `ConnectedClientState::extensions`, the `CellToBaseMsg` envelope and the session hook points landed first with no feature moved (#1077); then crafting became `CraftingPlugin` in `cimmeria-base-crafting`. Its verbs (95-100) are cell methods, not base methods (`SGWPlayer.def:916-948` is inside `<CellMethods>`), so it registers no base method: it consumes seven envelope payloads, owns its session state and lifecycle hooks, and takes three seams from the inventory and progression code (item use, the tool refresh, the ASP push). Its cell half (parsing 95-100) stays in `cell-methods`. The session teardown moved up to `base` and the unified `SessionCtx` come later: `BaseCtx` covers what the plugins need so far.
6. **The console** as a plugin registered by the facade ([services-crate-split.md §6](services-crate-split.md#6-expected-build-impact), "later options").
7. **NPC AI** last: the `npc_ai` / combat cycle has to break first.

## 4. Consequences

### 4.1 The pilot

The pilot PR implements only what pets needs: the `CellPlugin` trait, the builder and registry, cell-method registration with both startup checks, two tick hook points, one entity hook point, and `EntityExtensions`. The other hook points (AoI enter and leave, death, disconnect, space teardown) and the base side are specified here and built when a feature needs them; building them earlier would add code with no caller.

What moved, and what did not:

- **Moved to `cimmeria-cell-pets`:** the pet cell methods 88-90 (`cell::cell_methods::player::pet`, about 1.1k production lines and 1.9k test lines, from `cimmeria-cell-methods`), and `PetsPlugin`, which registers them, the owner sweep (`TickStage::AfterRingTransport`), the arrival VFX (`TickStage::AfterStatBuffs`) and the base-destroy despawn (`EntityHookPoint::BeforeBaseDestroy`).
- **Now generic in core:** `CellEntity::pet` is gone; a pet's `PetState` lives in `CellEntity::extensions`. The cell loop and `flush_and_destroy` fire hook points and no longer name pets. The router asks the plugin registry after the GM gate.
- **Stayed:** the pet world half in `cell-world` (registry, spawn, teardown, arrival queue, owner hooks), the pet AI and owner abilities in `cell-combat`, and the GM `.pet` console in `cell-console`. Lower crates call them. They move when those call sites go behind hooks.

**Rebuild measurement** (2026-09-28, warm dev build through the build lane on the Dev Drive, two runs each). The edit adds one `pub const` to the pet stance handler (`player/pet/stance.rs`), before in `cell-methods` and after in `cell-pets`:

| Command | Before: crates rebuilt | Before: time | After: crates rebuilt | After: time |
|---|---|---|---|---|
| `cargo build --workspace --all-targets` (CI exclusions) | 8: cell-methods, cell-console, cell, services, admin-api, wireclient, lab-mcp, server | 7.7 s, 7.9 s | 7: cell-pets, services, cell (test binary only), admin-api, wireclient, lab-mcp, server | 5.0 s, 5.1 s |
| `cargo build -p cimmeria-server` | 7: cell-methods, cell-console, cell, services, admin-api, lab-mcp, server | 6.0 s, 5.3 s (comment-only edit) | 5: cell-pets, services, admin-api, lab-mcp, server | 4.7 s |

In production lines, the server build rebuilds about 49k lines before (cell-methods 11.2k, cell-console 15.5k, cell 9.9k, services 2.1k, admin-api 4.5k, lab-mcp 1.2k, server 4.5k) and about 14k after (cell-pets 1.3k plus the same facade and binaries), a 72% cut. Wall time moves less (about -35% for the workspace build), because incremental compilation already makes a warm rebuild of an unchanged crate cheap. The line count is what grows with the codebase. `cimmeria-cell`'s test binary still rebuilds, because two of its tests (the base-destroy path and the router's plugin routing) install `PetsPlugin` as a dev-dependency. Moving those tests to the facade would remove it from the list.

**Tests changed beyond import paths.** Each is listed so a reviewer can check that none weakens a guard:

- About 65 test lines rewrote `entity.pet` to `entity.extensions.get::<PetState>()` (and `get_mut`, `contains`, `insert`, `remove`). Mechanical, with no assertion changed.
- `cimmeria-cell-methods`: `pet_methods_route_to_pet_not_world` pinned the static arm that no longer exists. It became `plugin_owned_methods_are_not_routed_here`, which asserts the opposite for every plugin-owned index. The pet row left `each_outer_range_routes_to_a_handler`. The positive routing proof moved to `cimmeria-cell-pets` (`each_pet_cell_method_reaches_the_pet_command_parser`) and to `cimmeria-cell` (`plugin_routing_tests`, through `dispatch_cell_method`).
- `cimmeria-cell`'s `destroy_entity_for_an_owner_despawns_the_pet` installs `PetsPlugin` on its fixture manager (one line), because the despawn is now the plugin's hook.

New tests: the registry's startup checks and hook order (`cell-world` `cell::plugin::tests`), the extension map (`cimmeria-entity`), the plugin's registrations (`cell-pets` `plugin_tests`), the router with and without the plugin (the missing-registration negative log), and the facade's default table (`services::plugins`).

### 4.2 Step 2: duels

Duels moved at the Social Systems close-out, with no packets in flight. The step added only what duels needed: four hook points (one of them of a new kind) and the `SpaceManager` resource map.

**New hook points** (`cell::plugin::hook_points`). Each sits on the line the duel's inline call occupied:

- `TickStage::AfterGateCrossing`: in the cell loop, after `gate_travel::crossing_tick` and before the auto-cycle tick. The duel tick runs here.
- `EntityHookPoint::BeforeDisconnectTeardown`: in `SpaceManager::disconnect_entity`, after the vault, gate-dial and crossing-hold scrubs and before the pet and AoI teardown.
- `EntityHookPoint::BeforeTravelSend`: at each of the 13 cell paths that send `TeleportPlayer` or `GateTravel` for a player (the GM travel commands, placement, the content teleport and gate actions, the ring transport, gate travel, respawn and the space transfer). The source scan that checked for `duel::on_travel(` at each site now checks for the hook (`every_travel_site_fires_the_travel_hook`).
- `DeathHookPoint::AfterPlayerThreatPurge`: a new hook kind, `DeathHook`, which gets the victim and the killer (the duel's end row logs the killer). It fires in the death resolver for a player target, after the NPC threat purge and before the owner's pets leave.

The travel and death call sites are in `cell-combat`, `cell-content`, `cell-interactions` and `cell-console`, below the plugin. They fire through two `SpaceManager` helpers, `fire_entity_hook` and `fire_death_hook`, which clone the registry first, as the cell loop does.

What moved, and what did not:

- **Moved to `cimmeria-cell-duel`:** the answer (`response`, cell method 102), the forfeit (`forfeit`, 103), the tick and the engage it runs (about 520 production lines, from `cimmeria-cell-world`), and `DuelPlugin`, which registers them and the three leave hooks. The duel tests moved with them (about 2.1k lines). That includes the tests of the challenge, the GM commands and the end paths, which drive the whole flow through the moved handlers; only the registry's own tests stay in `cell-world`.
- **Now generic in core:** `SpaceManager::duels` is gone. The registry is a `SpaceManager` resource, read through `cell::duel::DuelResources` (`mgr.resources.duels()`, `duels_mut()`). The cell loop, `disconnect_entity`, the death resolver and the travel sites fire hook points and no longer name duels. Duels keep no per-entity state, so nothing went into `CellEntity::extensions`.
- **Stayed in `cell-world`, and why:**
  - `DuelRegistry` itself. The harm gate (`combat::player_may_attack` and `may_hit_in_area`, called by every hostility gate in `cell-combat` and by `cimmeria-cell`'s auto-cycle), `engaged_opponent_entity` (the area candidates and the pet's defend) and the AoI enter path's PvP-flag replay (`pvp_flag_on_enter`) read it. Moving them needs a query seam that returns a value, which the hook model does not have.
  - The non-lethal clamp (`paths::clamp_partner_lethal`, `finish_clamped`), called in the middle of damage resolution in `cell-combat`. Moving it needs a combat hook seam with a return value.
  - The challenge (`challenge`), because `cimmeria-cell`'s base-message handler calls it for `BaseToCellMsg::Duel`. There is no base-message seam until the §3.4 envelope lands (step 5).
  - The GM `.duel_status` / `.duel_end` backends (`gm`), called by `cell-console`, and `send_player_line`, called by the auto-cycle.
  - `end` and the leave paths (`paths::on_disconnect`, `on_travel`, `on_death`), which the plugin's hooks call, and the shared pieces the moved code builds on (`outbound`, `combat`, `limits`, `connected_player`, `find_player`), now `pub` so the plugin crate can reach them. `cimmeria-cell-duel`'s `cell::duel` re-exports the whole world module, so the moved code keeps its `super::…` paths.

**Rebuild measurement** (2026-09-28, warm dev build through the build lane on the Dev Drive, two runs each). The edit adds one `pub const` to the duel answer (`duel/response.rs`), before in `cell-world` and after in `cell-duel`. Other agents kept the lane busy, so an `--exclusive` slot never came free; the runs used one slot (8 jobs), which makes the times noisier than §4.1's:

| Command | Before: crates rebuilt | Before: time | After: crates rebuilt | After: time |
|---|---|---|---|---|
| `cargo build --workspace --all-targets` (CI exclusions) | 13: cell-world, cell-combat, cell-content, cell-interactions, cell-pets, cell-methods, cell-console, cell, services, admin-api, wireclient, lab-mcp, server | 15.4 s, 18.1 s | 9: cell-duel, services, cell-combat, cell-console and cell (test binaries only), admin-api, wireclient, lab-mcp, server | 5.8 s, 6.2 s |
| `cargo build -p cimmeria-server` | 12: the same without wireclient | 13.8 s, 19.4 s | 5: cell-duel, services, admin-api, lab-mcp, server | 4.8 s, 4.6 s |

In production lines, the server build rebuilds about 113k lines before (cell-world 25.1k, cell-combat 22.9k, cell-content 13.2k, cell-interactions 7.3k, cell-pets 1.3k, cell-methods 10.1k, cell-console 15.5k, cell 8.2k, services 1.4k, admin-api 4.5k, lab-mcp 1.2k, server 2.0k; `.rs` files under `src/`, test files excluded) and about 10k after (cell-duel 0.7k plus the facade and binaries), a 91% cut. The gain is larger than the pilot's because the moved code came from `cell-world`, the bottom of the cell track, not from `cell-methods`. Three test binaries still rebuild in the workspace build: `cell-combat`, `cell-console` and `cell` each have tests that install `DuelPlugin` or call the moved tick.

An edit to the half that stayed (the registry, the end paths, the clamp) still rebuilds the cell track from `cell-world` up. That is the next lever: query and return-value seams for the harm gate and the clamp, and the base-message envelope for the challenge.

**Tests changed beyond import paths.** Each is listed so a reviewer can check that none weakens a guard:

- About 150 lines, in tests and in production code, rewrote `mgr.duels` to `mgr.resources.duels()` or `duels_mut()`, and added the `DuelResources` import. Mechanical, with no assertion changed.
- The duel test fixture (`make_mgr` in `cell-duel`'s `cell::duel::tests`) installs `DuelPlugin`, because the disconnect end (`disconnect_ends_duel_and_clears_both_flags`, the end table's `Disconnect` row, `leaving_withdraws_a_challenge_and_a_countdown`) now runs through the plugin's hook.
- `every_travel_site_ends_the_duel` became `every_travel_site_fires_the_travel_hook`: the scan counts `EntityHookPoint::BeforeTravelSend` instead of `duel::on_travel(` at each travel send, with the same sites, exemption and minimum.
- `cimmeria-cell-combat`'s `third_party_kill_is_normal_death` installs `DuelPlugin` on its fixture manager (one statement), because the death end is now the plugin's hook.
- `cimmeria-cell-methods`: `send_duel_response_routes_to_the_duel_handler` and `duel_forfeit_routes_to_the_duel_handler` pinned the static arms, which no longer exist. They moved to `cell-duel` as `send_duel_response_reaches_the_duel_handler` and `duel_forfeit_reaches_the_duel_handler`, through the registered handlers, with the same assertions. `plugin_owned_methods_are_not_routed_here` already covers every plugin-owned index; it now also asserts that 102 and 103 are on the list.
- `cimmeria-cell`'s `plugin_owned_methods_route_to_the_installed_plugin` installs both plugins and also asserts that 102 and 103 reach the duel handlers; `a_missing_plugin_logs_unhandled_for_each_plugin_owned_method` also asserts that no duel handler ran.
- `cell-world`'s plugin tests: the plugin-owned list is now 88-90 and 102-103, the complete registration registers both sets, and the missing-method error names the duel methods too.
- `cimmeria-cell-console`'s SS-U2 test calls the moved tick at its new path (`cimmeria_cell_duel::cell::duel::tick::run_at`).
- `cimmeria-cell-pets`' `pets_plugin_covers_every_plugin_owned_cell_method` and `each_pet_cell_method_reaches_the_pet_command_parser` assumed the plugin-owned list was the pet commands. They now name the three pet indices: pets alone leaves exactly the other plugins' methods (102-103) missing, and the routing loop drives 88-90 only.

New tests: the facade's `a_table_without_the_duel_plugin_fails_the_startup_check` (the missing-registration test: a table without `DuelPlugin` fails `check_complete` naming 102 and 103, so the orchestrator refuses to start); `DuelPlugin`'s registrations (`cell-duel` `plugin_tests`); the tick stage and the travel and death hooks fired the way core fires them, with and without the plugin (`cell::duel::tests::hooks`); and the hook helpers and death hooks in core (`cell-world` `cell::plugin::tests`).

### 4.3 Step 3: squads and org creation

Squads and organization creation moved at the Organizations close-out (2026-09-27), with the coordinators reassigned on 2026-09-28 and no packets in flight. The step added two hook points, one of them of a new kind, and reused `SpaceManager::resources` and `fire_entity_hook` from step 2.

**New hook points** (`cell::plugin::hook_points`). Each sits on the line the organization's inline call occupied:

- `EntityHookPoint::AfterDisconnectTradeCancel`: in `cimmeria-cell`'s `handle_disconnect_entity` (the base's `DisconnectEntity`), after the open trade is cancelled and before the Black Market session end, the last-position save and `SpaceManager::disconnect_entity`. The squad leave (`Logout`) and then the registrar offer's end run here, in that order, as the two inline calls did. It is not step 2's `BeforeDisconnectTeardown`, which fires later, inside `disconnect_entity`, after the vault and gate scrubs; moving the squad leave there would reorder its `onMemberLeftOrganization` sends against the trade cancel and the Black Market row.
- `PlayerHookPoint::AfterInitPlayerState`: a new hook kind, `PlayerHook`, which gets the entity id and the character id. It fires in the base-message handler's `InitPlayerState` arm, after `handle_init_player_state`. The squad replay needs the character id the base sent, not the one on the entity: an `onOrganizationLeft` owed to a player in gate transit is keyed by character, and the inline call delivered it even when the cell entity was missing (the ConnectEntity ordering-bug path). An `EntityHook` that read `player_id` off the entity would have dropped it there.

The org plugin needs no tick hook: invites and offers expire lazily when they are next touched, and nothing about a squad runs per tick. It needs no travel, death or AoI hook either: a squad survives a gate trip by design (the registry is keyed by character), the pending creation checks the space lazily, and squad membership is never sent on AoI enter (the frames follow entity presence, ORG-E1 Q6).

What moved, and what did not:

- **Moved to `cimmeria-cell-org`:** the organization router (`cell::organization::dispatch`, cell methods 8-19), the Team and Command forward to the base (`forward`), the squad invite answer (`respond`, CM 8), leave (CM 9), the minimap ping (CM 10), the loot mode (CM 18), the squad disconnect and the world-entry replay, and the creation name check (`on_organization_creation`, CM 94) with the offer's disconnect end: about 1.3k production lines, from `cimmeria-cell-methods`. The organization tests moved with them (about 2.2k lines), including the tests of the base-forwarded invite and kick and the GM backends, which drive the whole flow through the moved handlers. `OrgPlugin` registers the thirteen methods and the two hooks.
- **Now generic in core:** `SpaceManager::squads` and `SpaceManager::org_creations` are gone. The registries are `SpaceManager` resources, read through `cell::squad::SquadResources` (`mgr.resources.squads()`, `squads_mut()`) and `cell::org_creation::OrgCreationResources` (`org_creations()`, `org_creations_mut()`). `CellEntity::squad_id` is gone: the entity's squad is an `EntitySquad` in `CellEntity::extensions`, read and written through `cell::squad::{entity_squad_id, set_entity_squad_id}`. The router no longer has an organization arm; `handle_disconnect_entity` and the `InitPlayerState` arm fire hook points and no longer name squads. `cimmeria-cell-methods` has no organization module, and `social.rs` no CM 94 arm.
- **Moved down to `cimmeria-cell-interactions`, and why** (`cell::organization`, re-exported whole by the plugin's modules of the same name, so the moved code keeps its `super::…` paths):
  - the base-forwarded squad invite and kick (`handle_invite`, `handle_kick`) and the registrar reply and create result (`on_registrar_eligible`, `on_create_result`), because `cimmeria-cell`'s base-message handler calls them for `BaseToCellMsg::Org`. There is no base-message seam until the §3.4 envelope lands (step 5).
  - the GM `.squad_invite` / `.squad_join` backends (`gm_invite`, `gm_join`), because `cimmeria-cell-console` calls them, and the console is not a plugin until step 6.
  - the shared pieces the plugin builds on: the squad fanout, feedback lines, telemetry and the actor and reject helpers, the creation replies and telemetry, and `feedback_line`, now `pub`.

  They went to `cell-interactions`, not back to `cell-methods`, because the console must reach the GM backends and sits beside `cell-methods`, above `cell-interactions`. Keeping them in `cell-methods` would have kept the ORG-04 console-to-methods edge; putting them in `cell-world` would have put squad code at the bottom of the cell track. `cell-interactions` already held the registrar click (`interactions::org_registrar`), where the creation flow starts.
- **Stayed in `cell-world`:** `SquadRegistry` and `PendingCreations` (pure state, no I/O). The console's squad chat relay (`console::chat::squad`) and `.squad_info` read the registry, and the registrar click opens offers in it.

**Rebuild measurement** (2026-09-28, warm dev build through the build lane on the Dev Drive, two runs each, one slot of 8 jobs as in §4.2). The edit adds one `pub const` to the squad loot handler (`squad/loot.rs`), before in `cell-methods` and after in `cell-org`:

| Command | Before: crates rebuilt | Before: time | After: crates rebuilt | After: time |
|---|---|---|---|---|
| `cargo build --workspace --all-targets` (CI exclusions) | 8: cell-methods, cell-console, cell, services, admin-api, wireclient, lab-mcp, server | 7.7 s, 7.5 s | 7: cell-org, services, cell (test binary only), admin-api, wireclient, lab-mcp, server | 5.5 s, 4.4 s |
| `cargo build -p cimmeria-server` | 7: the same without wireclient | 8.8 s, 8.2 s | 5: cell-org, services, admin-api, lab-mcp, server | 4.0 s, 3.9 s |

In production lines, the server build rebuilds about 48k lines before (cell-methods 10.0k, cell-console 15.9k, cell 9.9k, services 2.2k, admin-api 4.5k, lab-mcp 1.2k, server 4.6k; `.rs` files under `src/`, test files excluded) and about 14k after (cell-org 1.3k plus the facade and binaries), a 71% cut. `cimmeria-cell`'s test binary still rebuilds in the workspace build, because its base-message and router tests install `OrgPlugin`.

Removing the console-to-methods edge also shortens every other `cell-methods` edit: one now rebuilds cell-methods, cell, services, admin-api, lab-mcp and server in the server build, and no longer `cell-console` (15.9k lines). The half that moved down to `cell-interactions` pays for that: an edit to the base-forwarded invite, the GM backends or the fanout now rebuilds from `cell-interactions` up (about 9k more lines than from `cell-methods`), until the base-message envelope and the console plugin let it move up into the leaf.

**Tests changed beyond import paths.** Each is listed so a reviewer can check that none weakens a guard:

- About 125 lines, in tests and in production code, rewrote `mgr.squads` and `mgr.org_creations` to `mgr.resources.squads()` / `squads_mut()` and `org_creations()` / `org_creations_mut()`, and `entity.squad_id` to `entity_squad_id(entity)` / `set_entity_squad_id(entity, …)`, with the trait imports. Mechanical, with no assertion changed.
- `cimmeria-cell`'s base-message organization tests (`base_messages::tests::org`) build their manager with `org_world()`, which installs `OrgPlugin`, because the CM 8 accept, the disconnect and the world-entry replay now run through the plugin.
- `cimmeria-cell-methods`: `social.rs`' `org_creation_routes_to_the_creation_handler` and `org_creation_rejects_a_forged_length` pinned the static CM 94 arm, which no longer exists. They moved to `cell-org` `plugin_tests` with the same assertions, through the registered handler. `plugin_owned_methods_are_not_routed_here` now also asserts that 8-19 and 94 are plugin-owned, and checks every per-interface router in the crate (being, ability manager, combatant, minigame, gate travel, inventory, mail, missionary, contact list, Black Market), not only the SGWPlayer one, for every plugin-owned index.
- `cimmeria-cell`'s `plugin_owned_methods_route_to_the_installed_plugin` installs all three plugins and asserts that each of 8-19 and 94 reaches the organization decoder; `a_missing_plugin_logs_unhandled_for_each_plugin_owned_method` also asserts that no `org` or `squad` row was written.
- `cell-world`'s plugin tests: the plugin-owned list is now 8-19, 88-90, 94 and 102-103 (and must stay ascending), the complete registration registers all three sets, and the missing-method error names the organization methods too.
- `cimmeria-cell-duel`'s `duel_plugin_registers_exactly_the_duel_methods` assumed the other plugins' methods were the pet commands; it now derives them from the plugin-owned list.
- `cimmeria-cell-methods`' `social::dispatch` keeps its signature, with `_tx` and `_space_mgr`: CM 94 was the last arm that used them.

New tests: the facade's `a_table_without_the_org_plugin_fails_the_startup_check` (the missing-registration test: a table without `OrgPlugin` fails `check_complete` naming 8-19 and 94, so the orchestrator refuses to start); `OrgPlugin`'s registrations, both hooks with and without the plugin, and CM 18 through its handler (`cell-org` `plugin_tests`); the player hook kind (`cell-world` `cell::plugin::tests::player_hooks_get_the_entity_and_the_character`); and the `cimmeria_cell_org=debug` log row (`cell_org_crate_events_keep_their_index`).

### 4.4 Step 4: effect scripts

The effect scripts moved on 2026-09-28. The ammo campaign, the last to add scripts, had merged its script packets (AM-11d, #1069), and no open PR touched `cell/effects/`. This step is a registry move, not a plugin: a script is not a client method or a hook, it is a value the effect runtime looks up by the `script_name` an effect row carries. So it adds no hook point and does not touch `CellPlugins`.

**The seam.** `cimmeria-cell-world` keeps the `EffectScript` trait, `EffectContext`, `dispatch_by_name` / `dispatch_on_remove` and a registry type, `cell::effects::registry::EffectScripts`: a frozen, `Arc`-shared map from `script_name` to a `&'static dyn EffectScript`.

- **The composition root fills it at startup, in fixed order.** `cimmeria-cell-effect-scripts` exports one table, `EFFECT_SCRIPTS: &[(&str, &dyn EffectScript)]`. The facade builds the registry from it (`services::plugins::effect_scripts`, beside `cell_plugins`), the orchestrator hands it to the `CellService` (`set_effect_scripts`), and `CellService::start` installs it on its `SpaceManager` (`install_effect_scripts`) right after the plugin table and before the space definitions load, so the spawn-time cover hold finds Cover Stance. There is no link-time discovery, for the reason in §3.2.
- **Dispatch reads it off the manager.** Every dispatch site already holds an `EffectContext`, which carries `&mut SpaceManager`, so `dispatch_by_name` looks the script up in `ctx.space_mgr.effect_scripts()`. The signatures of the dispatch functions and of `EffectScript` did not change, so no call site in combat, content or the world changed. A script is `&'static`, so the lookup hands back a reference that does not borrow the manager the script then mutates.
- **Lookup is deterministic.** A `HashMap` keyed by the exact, case-sensitive name; table order reaches only `EffectScripts::names`, for the startup log and the tests.
- **A duplicate or empty name fails startup.** `EffectScripts::build` returns `DuplicateScript` or `EmptyName`; the orchestrator logs `effect_scripts_invalid`, leaves the registry empty, and refuses to start the cell on an empty registry (`effect_scripts_empty`), as it refuses an incomplete plugin table. A bare `CellService` (a test harness) starts anyway and logs `effect_scripts_empty` at WARN.
- **A missing registration warns.** "Missing" is defined by the data, not by a list in core: a list of expected names in `cell-world` would make adding a script edit core again. Once the effect definitions load, the cell logs one `effect_script_unregistered` WARN with every `script_name` the rows carry that no script answers, and their effect ids. Dispatch still logs `effect_script_unknown` per call and falls back to the legacy NVP path, exactly as before. The shipped seed has two such rows, which no script ever answered (effect 658 names `Reload`, which the reload pipeline handles; effect 2907, a `test` row, has an empty name); the live-DB guard pins exactly those two, so a dropped or misspelled table row fails it.

What moved, and what did not:

- **Moved to `cimmeria-cell-effect-scripts`:** every `EffectScript` implementation, about 1.9k production lines and 3k test lines, from `cimmeria-cell-world`'s `cell::effects`: `scripts` (the damage, shield, stun and suppression scripts), `heal`, `cover_stance`, the pet scripts, the stimpack `StatBuff`, and the special-ammo families `ammo_dart_cc`, `ammo_dart_support`, `ammo_dart_tech`, `ammo_emp` and `ammo_incendiary` (the burn, which has no script of its own but runs `RangedEnergyDamage`, and whose only other readers were combat's tests). The static `match` in `effects/registry.rs` became the leaf's `EFFECT_SCRIPTS`, in the same order. The leaf's `cell::effects` re-exports the world's effect layer, so the moved code keeps its `super::…` and `crate::cell::effects::…` paths.
- **Stayed in `cell-world`, and why.** Each is called from below the leaf:
  - the passive pass (`effects/passives.rs`), which `cimmeria-cell`'s base-message handlers call at world entry, on a grant and on a respec;
  - the pet-script name predicates `acts_on_owner_pet` and `is_passive_script` (`effects/pet_scripts.rs`), which the owner-pet cast redirect in `cell-combat` and the passive pass ask;
  - the timed effect ledger (`effects/stat_buff/`: `SpaceManager::apply_timed_effect`, `remove_timed_effects`, `StatBuffRemoval`), which `cell-combat`'s stat-buff tick calls, and which is an inherent `impl SpaceManager` that cannot leave the crate defining `SpaceManager` anyway;
  - the special-ammo shot helpers the damage path reads (`effects/ammo_damage.rs`, `effects/ammo_explosive.rs`), which hold no script.

  No script needed a value-returning combat hook: every one is a synchronous `SpaceManager` mutator, so none was left behind for that reason. Three leaf modules share a name with the world module they extend (`registry`, `pet_scripts`, `stat_buff`) and re-export it.

**Rebuild measurement** (2026-09-28, warm dev build through the build lane on the Dev Drive, one slot of 8 jobs as in §4.2, two runs each). The edit adds one `pub const` to `effects/scripts.rs`, before in `cell-world` and after in `cell-effect-scripts`:

| Command | Before: crates rebuilt | Before: time | After: crates rebuilt | After: time |
|---|---|---|---|---|
| `cargo build --workspace --all-targets` (CI exclusions) | 15: cell-world, cell-combat, cell-duel, cell-content, cell-interactions, cell-pets, cell-methods, cell-console, cell-org, cell, services, admin-api, wireclient, lab-mcp, server | 11.7 s, 12.9 s | 9: cell-effect-scripts, services, cell-combat, cell-content and cell (test binaries only), admin-api, wireclient, lab-mcp, server | 6.3 s, 5.9 s |
| `cargo build -p cimmeria-server` | 14: the same without wireclient | 11.4 s, 10.8 s | 5: cell-effect-scripts, services, admin-api, lab-mcp, server | 4.8 s, 3.8 s |

In production lines (`.rs` files under `src/`, test files, test directories and inline `#[cfg(test)]` modules excluded), the server build rebuilds about 103k lines before (cell-world 22.9k, cell-combat 22.1k, cell-content 10.4k, cell-interactions 8.4k, cell-console 15.6k, cell-methods 5.3k, cell 6.9k, the three plugins 3.3k, services 1.0k, admin-api 4.1k, lab-mcp 1.1k, server 1.9k) and about 10k after (cell-effect-scripts 2.2k plus the facade and binaries), a 90% cut. Like §4.2, the gain is large because the moved code came from the bottom of the cell track. Three test binaries still rebuild in the workspace build: `cell-combat`, `cell-content` and `cell` install the registry in the tests that dispatch a script, and combat's ammo tests read the ammo families' constants.

An edit to what stayed (dispatch, the registry type, the passive pass, the ledger, the shot helpers) still rebuilds the cell track from `cell-world` up; those are runtime, not scripts, and change far less often.

**Tests changed beyond import paths.** Each is listed so a reviewer can check that none weakens a guard:

- A test that builds its own `SpaceManager` and dispatches a script now installs the registry first (one statement), because a bare manager has no scripts. Behind the move every manager saw the static table, so this restores what those tests ran against. The statement went into the shared fixtures, not the tests: `cell-combat`'s `damage_apply::tests::make_mgr_player_vs_npc`, `deployable::tests::deploy_mgr`, `use_ability::owner_pet::tests::world`, `use_ability::tests::support_shot::support_mgr` and the `ammo_dart_support` integration test's `world`; `cell-content`'s `consumable_use_tests::mgr`; `cimmeria-cell`'s `passive_abilities::fixture` and `npc_ai_cover_behaviour::seed_cover_stance_effects`; and `cimmeria-services`' `consumable_round_trip_tests::stage` (live-DB). One test installs it in its own body, `cimmeria-cell`'s `zero_health_guard::npc_killed_by_an_effect_bleed_does_not_shoot_back`, whose fixture (`make_ai_fixture`) many script-free tests share. Combat, content and `cimmeria-cell` re-export the installer as `test_support::install_effect_scripts`.
- The script unit tests moved with their files, unchanged. Their fixture `make_mgr_with_target` now installs the registry; its world half moved to `cimmeria_cell_world::test_fixtures` (`make_mgr_with_target`, `effect_with_nvp`), where `cell-world`'s `ammo_damage` tests also read it.
- `cell-world`'s `pets::tests::owner_buffs`: the six tests that dispatch a pet script or run the passive pass (`pet_stat_buff_toggles_for_a_toggled_ability`, `pet_stat_buff_is_timed_by_its_pulse_duration`, `pet_scripts_leave_a_non_pet_alone`, `pet_death_timer_dooms_the_pet`, `a_learned_passive_raises_speed_pet_and_unlearning_restores_it`, `the_passive_pass_runs_only_flagged_passive_scripts`) moved to the leaf as `cell::effects::pet_scripts::tests`, with the same assertions, on the shared pet world plus the registry. A test in `cell-world` cannot install the leaf's scripts (the leaf depends on it; §4.7). The owner-pet resolution and buff-ledger tests stayed.
- `cell-world`'s `live_db_use_cover::cover_stance_effect_rows_name_their_scripts` moved to the leaf as `cover_stance::live_db_tests`, unchanged.
- The old registry's `known_scripts_resolve` and `unknown_script_returns_none` moved to the leaf's `registry::tests`, reading the table (`registry::lookup`) instead of the `match`.
- `cimmeria-server`'s `cell_track_lower` parity test used `cimmeria_cell_world::cell::effects::scripts` as a sample world target; the module is gone, so it now samples `cell::effects::passives`, with the same assertions.

New tests: the registry type (`cell-world` `cell::effects::registry::tests`: lookup by exact name, order independence, the duplicate-id and empty-name build failures, the empty registry, and `unregistered`); the table (`cell-effect-scripts` `registry::tests`: every row builds and resolves to itself, and `install` makes dispatch find a script a bare manager misses); the missing-registration guard on the shipped data (`registry::live_db_tests::every_seeded_script_name_is_registered`); the facade's `the_effect_script_table_builds_and_names_every_script` and `a_duplicated_effect_script_row_fails_the_startup_build`; and the `cimmeria_cell_effect_scripts=debug` log row (`effect_scripts_crate_events_keep_their_index`).

### 4.5 Step 5: the `BasePlugin` core and crafting

Step 5 went in as two PRs: the base-track plugin core with no feature moved (#1077), then crafting as its first plugin. The core write-up is first, as it landed; the crafting move follows.

#### The core

The first PR added the core with no feature moved, so the registry existed, was empty and had every startup check tested. The split follows from what crafting turned out to be: its verbs 95-100 are SGWPlayer **cell** methods (`entities/defs/SGWPlayer.def:916-948`, inside `<CellMethods>`; [cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md)). The client sends them as `0x80 | index` to the cell, which parses them and forwards `CellToBaseMsg::Crafting` to the base. So crafting registers no base method; what its base half needs from the core is the envelope, per-session state and lifecycle hooks, and three upward seams from `cimmeria-base-methods` (item use, the tool refresh, the ASP push) that the crafting PR adds at their call sites.

**Where the core lives.** `cimmeria-base-session` (`base::plugin`), the bottom of the base track. Every site that fires a base hook is in it or above it: the disconnect teardown (`helpers::destroy_client_entities`) is in base-session itself, gate travel and `onClientReady` are in `base-world-entry`, `logOff` and the base-method router are in `base`, and the item-use, tool-refresh and ASP seams crafting needs are in `base-methods`. It also defines `ConnectedClientState`, which carries the extension map and the registry.

**What it registers.**

- `base_method(index, handler)`: an SGWPlayer exposed base method by flattened index (the message id minus `0xC0`). `dispatch_sgw_player_base_method` asks the session's registry before its static arms. The entity-type gate (Account vs SGWPlayer) runs before the call, in `connect_loop`, so a plugin handler sits behind it like any arm. The base has no GM-gated base method today.
- `on_cell_message::<T>(handler)`: the consumer of `CellToBaseMsg::Plugin` envelopes whose payload is a `T` (§3.4). The `Plugin` arm of the cell-message dispatcher (`route_cell_message`) routes it; the envelope keeps its FIFO position on the one channel (C6).
- Session hooks, each at a crafting inline call's exact line:
  - `SessionHookPoint::LogOffAfterEntityUnmapped` (`logOff`), `DisconnectAfterEntityUnmapped` (the teardown) and `GateTravelBeforeActiveCharacterCheck` (gate travel, before the fail-closed check): synchronous `fn(SessionEvent { entity_id, cause })`, where crafting drops the induction queue.
  - `SessionStateHookPoint::GateTravelBeforeCreateEntity`: synchronous `fn(&mut ConnectedClientState)` under the connected-map lock, where crafting forgets the origin world's stations.
  - `WorldEntryHookPoint::ClientReadyAfterOrgRestore`: async, with the entity, the character and a `BaseCtx`, where crafting's login sync runs.

**The startup checks** mirror the cell's. `PLUGIN_OWNED_BASE_METHODS` (empty) lists the indices that left the static router and `PLUGIN_CELL_MESSAGES` (empty in the core PR; crafting's seven since) the envelope's payload types. `BasePlugins::build` fails on `DuplicateBaseMethod`, `UnknownBaseMethod` (no name in `cimmeria-wire`'s new `base::names::base_method_name` table, which a def-conformance test checks against the flattened `.def`), `NotPluginOwned`, `DuplicateCellMessage` and `UndeclaredCellMessage`; `check_complete` fails on `MissingBaseMethods` and `MissingCellMessageConsumers`. The composition root builds the table (`services::plugins::base_plugins`, empty for now), the orchestrator hands it to the `BaseService` and refuses to start the base when it is incomplete (`plugin_table_incomplete`), and a bare `BaseService::start` logs the gap at WARN. While the production lists are empty, the tests validate against their own lists through `BasePlugins::build_with`.

**How the registry reaches a hook site.** The `BaseService` holds the table, stamps it on every session at login (`ConnectedClientState::plugins`) and passes it to the cell-message loop. A site with the connected map and an address reads it with `session_plugins`, which clones the `Arc` and releases the lock before a hook runs. `route_cell_message` takes it as an argument; `handle_cell_message`, now test-only, runs with an empty table, so the fifty-odd dispatch tests kept their calls.

**Behaviour.** Neutral: every registry was empty, so each new call was a no-op beside the inline call it would replace. The wire is unchanged; the `Plugin` variant is in-process only.

**Tests changed beyond import paths.** None changed in behaviour. Mechanical edits: the six test-side `ConnectedClientState` literals gained `extensions` and `plugins`; the login tests pass `&BasePlugins::empty()` to `handle_login`, and the handshake test to `run_connect_loop`.

New tests: the registry's checks, envelope routing and hook order (`cimmeria-base-session` `base::plugin::tests`, 16); the base-method router with and without a plugin, the static-router inversion guard and the `logOff` and teardown hooks (`cimmeria-base` `dispatch::tests::plugin_routing`); the two gate-travel hooks, including a refused transfer (`gate_travel::tests::plugin_hooks`), the world-entry hook (`client_ready::plugin_hook_tests`) and the `Plugin` arm (`tests_dispatch_arms::plugin_envelope`) in `cimmeria-base-world-entry`; the envelope type (`cimmeria-wire` `plugin_msg`); the name table and its def-conformance scan (`base_method_names_are_the_flattened_exposed_base_methods`); and the facade's `the_default_base_table_builds_and_is_complete`.

#### Crafting

Crafting moved at its close-out (CR-13, #983), with only the owner's UAT (CR-14) left and the coordinators reassigned on 2026-09-28.

**One crate or two.** The coordinator asked for one feature crate implementing both plugin kinds if that kept the import cycle inside the leaf. It became one *base* crate, `cimmeria-base-crafting`, and the cell half stayed where it was:

- The base half is about 10k production lines (25k with tests) and depends only on `cimmeria-base-session`. A crate that also implemented the `CellPlugin` would depend on `cimmeria-cell-world`: every cell-world edit would rebuild the 10k lines of crafting, and the base-track tests that install the plugin (`base-methods`, `base-world-entry`, `base`) would have to compile the cell track.
- The cell half is the parse-and-forward of 95-100 (`cell-methods`' `player::crafting`, about 200 production lines) and the senders in `cell-console` and `cimmeria-cell`. None of it is in the import cycle, which is entirely base-side (every verb, `feedback`, `sync` and `options` reach each other through `request::CraftCtx`). Moving the parse to a `cimmeria-cell-crafting` leaf, with 95-100 plugin-owned, is a later option with little to gain.

**What moved.** `base-session`'s `base::crafting`, whole, with its tests: the verbs, the induction engine (`session`), persistence, the options and tools, the GM grants, respec, feedback and telemetry. The crate's `base` module re-exports the session-layer modules the code names (`helpers`, `outbox`, `gm_feedback`, `session_identity`, `ConnectedClientState`), so the moved code keeps its `crate::base::…` paths. `CraftingPlugin` registers:

- consumers for the seven payloads (`CraftRequest`, `CraftingStations`, `GmAllCraft`, `GmCraftGrant`, `RespecCraftOpen`, and two new structs for the former struct variants, `GmGrantExpertise` and `GmGrantAppliedSciencePoints`), each calling the handler the central arm called;
- the induction drop at `LogOffAfterEntityUnmapped`, `DisconnectAfterEntityUnmapped` (`DropReason::Logout`) and `GateTravelBeforeActiveCharacterCheck` (`WorldChange`);
- the options reset at a new `SessionStateHookPoint::PlayCharacterAfterEntryLatch` (in `playCharacter`, where `crafting_options` was reset; the core PR had missed that site) and the world-entry reset at `GateTravelBeforeCreateEntity`;
- the login sync at `ClientReadyAfterOrgRestore`;
- the three seams below.

**The seams from `cimmeria-base-methods`.** The inventory and progression code called crafting directly, and those calls would have kept crafting below them. Each became a hook point at the call's exact line:

- `ItemUseHookPoint::CraftingItem` and `InstanceNotFound` in `useItem`: a new hook kind that **returns a value**, `ItemUseOutcome::{NotHandled, Refused, Consumed(ItemConsumed)}`. The first plugin in table order that does not answer `NotHandled` decides. On `Consumed` the core sends `onRemoveItem` and the inventory update and dispatches the outbox row, as before; the crafting-item SQL flag (`resources.crafting_item_effects`) stays in the core's instance query. With no plugin, a crafting item logs WARN `reason = "no_plugin"` at `base.plugin` and is not consumed.
- `InventoryHookPoint::AfterFullInventoryUpdate`, with the resync's rows: the Field Crafting Tool refresh.
- `ProgressionHookPoint::AfterAppliedScienceEarned`, with the new total: the ASP push. The earning itself (D-CR01, one ASP per level) stays in `grant_xp`, which persists it.

These sites hold an entity id, not an address, so they read the registry through `entity_plugins` (the entity's session's registry).

**Per-session state.** `ConnectedClientState::crafting_options` is gone. The same `CraftingSessionOptions` lives in `extensions`, read with `options::session_options` (`None` reads as the defaults) and written with `session_options_mut` (stores the defaults on first use). `CraftingOptionsExt` gives `ConnectedClientState` `crafting_options()` / `crafting_options_mut()` methods, so tests read and set one input as they did the field.

**What stayed, and why.**

- `inventory_locks`, which vendor, trade, mail, the Black Market and the ammo reserve take as well, moved to `base-session`'s `base::inventory_locks` (the leaf re-exports it at its old path).
- The payload types in `cimmeria-wire::crafting` and the client-method payloads: the cell senders below the plugin build them.
- The `is_crafting_item` flag in `useItem`'s instance query, and D-CR01's ASP earning in `grant_xp`: both are part of core queries and writes the hooks sit after.
- The cell half (parsing 95-100, the station tick's report, the console commands), as above.

**Behaviour.** The wire and the send order are unchanged: each hook fires at the line its inline call had, and the envelope travels on the one channel. One edge differs: a hook site reaches the plugins through the player's session, so crafting work for an entity whose session is already gone (a `useItem` arriving after the base dropped the session) no longer runs; before, the item use still ran its transaction for a client that was no longer there. With a session, which every production path has, nothing changes.

**Rebuild measurement** (2026-09-28, warm dev build through the build lane on the Dev Drive, `--exclusive`, two runs each). The edit adds one `pub const` to `crafting/craft/rules.rs`, before in `base-session` and after in `base-crafting`:

| Command | Before: crates rebuilt | Before: time | After: crates rebuilt | After: time |
|---|---|---|---|---|
| `cargo build --workspace --all-targets` (CI exclusions) | 9: base-session, base-methods, base-world-entry, base, services, admin-api, wireclient, lab-mcp, server | 12.8 s, 13.5 s | 9: base-crafting, services, base-methods, base-world-entry and base (test binaries only), admin-api, wireclient, lab-mcp, server | 9.9 s, 10.1 s |
| `cargo build -p cimmeria-server` | 8: the same without wireclient | 10.3 s, 9.9 s | 5: base-crafting, services, admin-api, lab-mcp, server | 4.6 s, 4.8 s |

In production lines (`.rs` under `src/`, test files, test directories and inline `#[cfg(test)]` modules excluded), the server build rebuilds about 83k lines before (base-session 29.4k with crafting, base-methods 25.8k, base-world-entry 11.6k, base 5.8k, services 1.6k, admin-api 3.8k, lab-mcp 1.0k, server 4.3k) and about 21k after (base-crafting 10.4k plus the facade and binaries), a 75% cut. #962 estimated 53k for crafting; the gain is larger because crafting left the bottom of the base track, not a middle layer. Three test binaries still rebuild in the workspace build: `base-methods`, `base-world-entry` and `base` install the plugin in the tests that drive their hook sites.

**Tests changed beyond import paths.** Each is listed so a reviewer can check that none weakens a guard:

- The tests that drive a crafting path through a core site install `CraftingPlugin` on their session (one statement): `base`'s `crafting_teardown`; `base-world-entry`'s gate-travel `crafting_queue` and `crafting_options`, `crafting_options_world_entry_tests` and the client-ready crafting live-DB test; `base-methods`' `use_crafting_item_tests`, `crafting_tools_tests` and `asp_earning_tests`; `base-crafting`'s `destroying_the_client_drops_its_crafting_queue`. `base-methods`' `crafting_supplies_tests` gives its session-less fixture a session with the plugin before `useItem`, since the item-use seam reaches the plugin through the session.
- The crafting dispatch-arm tests (`crafting_arm`, `crafting_gate`, `crafting_gm_grant`, `crafting_respec`, and the two crafting rows of `gm_grant_arms`) send the payload in the envelope and route through `route_cell_message` with the crafting table (`tests_dispatch_arms::crafting_plugins`), instead of the removed variants through `handle_cell_message`. Same assertions.
- The cell-side tests that matched a removed variant match the envelope's payload instead (`CellToBaseMsg::plugin_payload::<T>()`, or `Plugin(msg) if msg.is::<T>()` and a downcast): `cell-console`'s GM give, `.allcraft`, craft-grant, respeccraft and `.learndiscipline` / `.forgetdiscipline` tests, `cell-methods`' crafting forward and 95-routing tests, and `cimmeria-cell`'s station-tick test. Same assertions.
- Field access `state.crafting_options.x` became `state.crafting_options_mut().x` / `crafting_options().x` (`CraftingOptionsExt`). Mechanical.
- `base-session`'s `the_production_lists_are_empty_and_an_empty_table_is_complete` became `the_production_lists_name_crafting_and_an_empty_table_is_incomplete`: the envelope list now names crafting's seven payloads, so an empty table must fail `check_complete` naming them.
- `cimmeria-server`'s `base_session_events_keep_their_file_and_index` sampled `cimmeria_base_session::base::crafting::persistence`; the module moved, so it samples `base::inventory_locks`, with the same assertions.

New tests: the facade's `a_base_table_without_the_crafting_plugin_fails_the_startup_check` (the missing-registration test: an empty table fails `check_complete` naming the seven payload types, so the orchestrator refuses to start the base); `CraftingPlugin`'s registrations, a station report routed through the registry, and the `playCharacter` and gate-travel state hooks (`base-crafting` `plugin_tests`); the seam hook kinds, including first-decides item use (`base-session` `base::plugin::seam_tests`); the item-use seam's missing-registration guard (`use_crafting_item_tests::live_db_a_crafting_item_with_no_plugin_warns_and_is_not_consumed`); `CellToBaseMsg::plugin_payload` (`cimmeria-wire`); and the `cimmeria_base_crafting=debug` log row (`base_crafting_crate_events_keep_their_index`).

### 4.6 What gets better

- A feature edit rebuilds the feature, the facade and the binaries, not the cell track above the lowest crate it touches.
- `CellEntity`, `SpaceManager`, the routers and the cell loop stop changing when a feature is added.
- A missing registration is a startup failure, not a player ticket.

### 4.7 What gets worse, and the mitigations

- **Compile-time exhaustiveness.** The static `match` on method indices goes away. The startup assertion (§3.3) and the router's WARN replace it.
- **Indirection.** A method call is a registry lookup and an `Arc` clone before the handler. That is noise against a `SpaceManager` lookup and an `mpsc` send.
- **Tests below a leaf cannot install its plugin.** A test in `cell-world` that exercises a hook through a bare `SpaceManager` cannot see `cimmeria-cell-pets`. Tests move with the code they test; a test that drives a hook point from a lower crate installs the plugin from a crate at or above the leaf. The pilot records every test it had to move or change beyond an import path.
- **Borrowing.** A hook receives `&mut SpaceManager`, not a narrower view. That keeps handler code as it is today; narrowing it is a later refactor, not part of this decision.

## 5. Alternatives considered

| Option | Why not |
|---|---|
| **More layer crates** (the services split, continued) | Features cut across layers, so a feature edit still touches a low crate. #962's split analysis caps the gain from splitting `wire` and `entity` alone at 15-20%. |
| **Feature crates with hand-written hook traits** (`SpaceHooks`, `CraftingHooks`, #962 workstream C) | Needs one bespoke trait per seam and per feature, installed by hand. The plugin builder is the same idea with one registration surface and one startup check. The trait inversion used by `ContentEvents` stays the tool for single calls from a lower system up into a higher one. |
| **Plugins on `bevy_ecs`** | A rewrite of the cell core (about 250k lines), which must re-prove the ordering fixes (AoI deferral, cinematic hold, reanchor replay). ECS systems are synchronous while persistence is async on tokio. It puts property-sync ordering (C4, C6) at risk. We have no current need for its runtime benefits. Revisit if the extension map turns into a hand-rolled ECS: queries over "every entity with X" in hot ticks, or many plugins scanning all entities per tick. |
| **Separate processes per feature** | Adds the distributed cost [scaling-analysis.md](scaling-analysis.md) already says we pay for nothing, and new channels break the FIFO guarantee C6 relies on. |

## 6. Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| A migrated feature fires its work at a different point in the tick, reordering wire messages | Medium | Named hook points at the inline call's exact position (§3.2); wire-level replay and chain-replay suites unchanged; one hook point per inline call |
| A plugin drops out of the table | Low | `check_complete` fails startup; facade test pins the default table |
| A static router arm shadows a plugin index | Low | `NotPluginOwned` at build; the shadow test in §3.3 |
| The GM gate is bypassed by a plugin handler | Low | The gate runs before the registry lookup; the existing GM dispatch tests cover plugin indices through `dispatch_cell_method` |
| The extension map hides state that should be persisted | Medium | Nothing persists implicitly; the feature's save path is explicit and reviewed like today |
| Mid-campaign moves collide with coordinator packets | Medium | Features move only at close-out; the coordinator for the campaign signs off |
| Hook points multiply until the loop is unreadable | Low | A hook point is added only when a feature moves; each is documented at its enum variant |

## 7. References

- [#962](https://github.com/SandboxServers/Cimmeria/issues/962): the research record, the measurements and the owner decision (2026-09-28).
- [services-crate-split.md](services-crate-split.md): the layer split, amended by this ADR.
- [build-system.md](build-system.md): the build lane, and how rebuilds are measured.
- [../protocol/cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md): the cell method indices (88-90 for the pet commands).
- [../protocol/message-catalog.md](../protocol/message-catalog.md): the base and cell method paths.
- [negative-logging-convention.md](negative-logging-convention.md): the hook-miss log fields.
