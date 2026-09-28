---
title: "ADR: feature plugins over CellEntity and SpaceManager (no ECS)"
type: explanation
audience: engineers
last_updated: 2026-09-28
---

# ADR: feature plugins over `CellEntity` and `SpaceManager` (no ECS)

> **Status:** Accepted (owner decision, 2026-09-28, [#962](https://github.com/SandboxServers/Cimmeria/issues/962)). Pilot: pets, in PR "refactor(962): pets plugin pilot". Amends [services-crate-split.md](services-crate-split.md).
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

The base track gets the same shape later (`BasePlugin`, a `BaseMethodCall` carrying the session context), when the first base feature moves (§3.7).

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

Existing variants stay until their feature migrates; a migrating feature moves its variants into the envelope at its close-out. The pets pilot needs no envelope: its three cell methods reply through the existing `EntityMethodCall` / `WitnessEntityMethod` variants. #962 estimated the envelope alone would let about 74% of recent `wire` and `entity` commits avoid a full rebuild, though those still rebuild 58-90%; it is worth doing, but it is not the main lever. Moving feature code out of the low crates is.

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
- **The same type serves the base and the space.** `ConnectedClientState` gets an `extensions` field when the first base feature migrates, and `SpaceManager` gets a `resources` map for space-wide feature state (the pet, duel and squad registries) when their lower readers move.

### 3.6 Hook seams and missing registrations

The test rule from #962: a missing registration must log a warning or fail the startup assertion, never silently no-op.

| Seam | A missing registration … | Test |
|---|---|---|
| Plugin-owned cell method | fails startup (`check_complete`); a bare `CellService::start` logs WARN with the missing indices | Unit test on `CellPlugins`; startup test in the facade |
| Duplicate, unknown or not-plugin-owned method index | fails `CellPlugins::build` | Unit tests per error |
| Cell method dispatched with no handler at runtime | falls through to the router's existing WARN, `Unhandled cell method call` (#311) | Router test with an empty registry and a pet index |
| Tick and entity hook points | are covered by the plugin's method registrations: a plugin is installed whole or not at all, and a missing plugin fails `check_complete` | The facade test that the default table passes `check_complete` |
| `PluginMsg` with no consumer (when the envelope lands) | WARN with `type_name`, message dropped | Unit test on the consumer registry |

The WARN rows follow [negative-logging-convention.md](negative-logging-convention.md): a `reason` field, the method index or type name, and a message that says what the player loses.

### 3.7 Crate layout and rebuild fan-out

```text
core     cimmeria-entity        EntityExtensions on CellEntity
         cimmeria-cell-world    cell::plugin (trait, builder, registry, hook points); SpaceManager holds the registry
         cimmeria-cell          the router consults the registry; the cell loop fires the tick hooks
systems  cell-combat, cell-content, cell-interactions, cell-methods, cell-console   (unchanged)
leaves   cimmeria-cell-pets     PetsPlugin: the pet cell methods and the pet hooks
root     cimmeria-services      the plugin table; installs it on the CellService
```

A leaf depends on core and on the systems it calls; only the composition root (and test code) depends on a leaf. A leaf edit rebuilds the leaf, `cimmeria-services` and what depends on the facade (the server, the admin API, the lab endpoint, the wire client). It no longer rebuilds `cell-methods`, `cell-console` or `cell`.

The estimates in #962 for the full migration: a crafting edit rebuilds about 53k lines instead of about 135k; mail about 45k, org about 42k, bank or chat about 33k. The envelope and the `entity-types` split add a 15-20% cut on the `wire` and `entity` long tail.

**Pilot measurement.** Filled in by the pilot PR (§4.1).

### 3.8 Migration order

Features move at their campaign's close-out, never while a campaign coordinator has packets in flight on that code.

1. **Pets (pilot).** The three pet cell methods (88-90), the pet tick hooks and the base-destroy hook become `PetsPlugin` in `cimmeria-cell-pets`; `CellEntity::pet` becomes an extension. The pet world half (the registry, spawn, teardown, owner hooks) stays in `cell-world`, and the pet AI and owner abilities stay in `cell-combat`, because combat, content, interactions and the console call them. They follow when those call sites go behind hooks (death credit, owner-path hooks).
2. **Duels** (`cell-duel`; Social Systems closed 2026-09-27). The duel tick and `DuelRegistry` move; the registry becomes a `SpaceManager` resource.
3. **Squads and org creation** (`cell-org`; Organizations closed 2026-09-27). Also removes the console-to-methods edge (`console/squad.rs`).
4. **Effect scripts** (`cell-effect-scripts`). A registry move rather than a plugin (the static table at `effects/registry.rs`); changes [abilities-and-effects-system.md](abilities-and-effects-system.md) and the CLAUDE.md line on where scripts go.
5. **The base features** (`base-crafting` after the crafting close-out CR-13, then bank, mail, chat, vendor with trade, inventory, progression). Brings in `BasePlugin`, `ConnectedClientState::extensions`, the `CellToBaseMsg` envelope, the session teardown moved up to `base`, and the unified `SessionCtx`.
6. **The console** as a plugin registered by the facade ([services-crate-split.md §6](services-crate-split.md#6-expected-build-impact), "later options").
7. **NPC AI** last: the `npc_ai` / combat cycle has to break first.

## 4. Consequences

### 4.1 The pilot

The pilot PR implements only what pets needs: the `CellPlugin` trait, the builder and registry, cell-method registration with both startup checks, two tick hook points, one entity hook point, and `EntityExtensions`. The other hook points (AoI enter and leave, death, disconnect, space teardown) and the base side are specified here and built when a feature needs them; building them earlier would add code with no caller.

The pilot measures the rebuild of a pets-only edit before and after, and records it in this section.

### 4.2 What gets better

- A feature edit rebuilds the feature, the facade and the binaries, not the cell track above the lowest crate it touches.
- `CellEntity`, `SpaceManager`, the routers and the cell loop stop changing when a feature is added.
- A missing registration is a startup failure, not a player ticket.

### 4.3 What gets worse, and the mitigations

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
