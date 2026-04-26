# Further Refactoring Candidates Analysis

## Summary

Yes, refactoring the top 3 candidates **would get them down to ~250 lines each**, but with caveats:

1. **space_manager.rs (1,402L)** → can split into 9 focused modules (~120-260L each) plus a `mod.rs` re-export hub
2. **world_entry.rs (932L)** → can split into 9 message handler modules (~120-240L each) plus a `mod.rs` re-export hub
3. **dispatch.rs (662L)** → **should NOT split** (already clean architecture)

---

## Detailed File Structure

### Cell Service: `crates/services/src/cell/`

**Current:**
```
cell/
├── space_manager.rs (1,402L) ⚠️ REFACTOR
├── spawner.rs (915L)
├── service.rs (901L)
├── dispatch.rs (662L) ✅ KEEP AS-IS
├── abilities.rs (647L)
├── interactions.rs (602L)
├── combat.rs (589L)
├── missions.rs (498L)
├── messages.rs (498L)
├── chat.rs (296L)
├── mail.rs (289L)
├── gate_travel.rs (169L)
└── mod.rs (66L)
```

**Proposed: `cell/space_manager/` (hybrid 2-level)**
```
cell/
├── space_manager/
│   ├── mod.rs (40L)
│   │   └── re-exports: new, load_from_xml, world/space accessors
│   ├── entity_storage.rs (~220L)
│   │   └── create_entity, destroy_entity, get_entity, get_entity_mut, get_entity_world_name
│   ├── connection.rs (~180L)
│   │   └── connect_entity, disconnect_entity, connection lifecycle
│   ├── position_sync.rs (~240L)
│   │   └── update_entity_position, entity movement tracking, velocity updates
│   ├── aoi_manager.rs (~260L)
│   │   └── compute_aoi_changes (155L), get_witnesses_of, AoI visibility state
│   ├── spatial_queries.rs (~220L)
│   │   └── find_path, has_line_of_sight, is_position_valid, get_navmesh_height
│   ├── npc_spawning.rs (~200L)
│   │   └── spawn_npc, spawn_npc_from_record, spawn_npc_from_record_in_space, allocate_npc_id
│   ├── entity_lookup.rs (~150L)
│   │   └── find_entity_by_tag, find_entities_by_template, all_npc_entity_ids, get_region
│   ├── data_access.rs (~120L)
│   │   └── all_spaces, world_count, space_count, space_id_for_world, regions_for_world, get_step_objectives
│   └── types.rs (~50L)
│       └── RegionData, WorldDef, SpaceInstance struct definitions (moved from original)
│
├── spawner.rs (915L) [separate refactor candidate]
├── service.rs (901L)
├── dispatch.rs (662L) ✅ ALREADY OPTIMIZED (re-export hub + thin routing)
└── ... (others)
```

**Max file sizes in refactored space_manager/:**
- `aoi_manager.rs`: ~260L (compute_aoi_changes is complex, ~155L alone)
- `position_sync.rs`: ~240L
- `connection.rs`: ~180L
- `npc_spawning.rs`: ~200L

All within reasonable bounds for LLM context. **Goal: ~200-260L max, achieved.**

---

### Base Service: `crates/services/src/base/`

**Current:**
```
base/
├── world_entry.rs (932L) ⚠️ REFACTOR
├── connect_loop.rs (592L)
├── character_create.rs (511L)
├── login.rs (335L)
├── cooked_data.rs (240L)
├── character.rs (229L)
├── service.rs (225L)
├── dispatch.rs (208L)
├── world_entry_appearance.rs (198L)
├── mod.rs (171L)
├── resources.rs (161L)
├── helpers.rs (158L)
├── chardef.rs (124L)
├── tick_sync.rs (82L)
└── ... (world_entry_methods/ subdirectory: 24 files, ~4,989L)
```

**Proposed: `base/world_entry/` (message handler subdirectory)**
```
base/
├── world_entry/
│   ├── mod.rs (60L)
│   │   └── re-exports handle_message and sub-handlers
│   ├── message_handlers.rs (~150L)
│   │   └── main dispatch match statement routing to sub-handlers
│   ├── entity_sync.rs (~220L)
│   │   └── SpaceData, EntityCreated, EnteredAoI, LeftAoI, EntityMoved, EntityMethodCall handlers
│   ├── world_events.rs (~200L)
│   │   └── GateTravel, RespawnReload, StartMinigame, MinigameResult, WorldState updates
│   ├── vendor_ops.rs (~240L)
│   │   └── OpenVendorStore, PurchaseVendorItems, SellVendorItems, BuybackVendorItems handlers
│   ├── inventory_sync.rs (~200L)
│   │   └── ListInventoryItems, MoveInventoryItem, RemoveInventoryItem, GrantItem handlers
│   ├── player_rewards.rs (~180L)
│   │   └── GrantXP, GrantCash, WitnessEntityMethod, ActiveSlotUpdate handlers
│   ├── mail_handler.rs (~140L)
│   │   └── MailRequest handler
│   ├── mission_handler.rs (~120L)
│   │   └── MissionUpdate handler
│   └── repair_recharge.rs (~160L)
│       └── RepairInventoryItem, RepairInventoryItems, RechargeInventoryItems handlers
│
├── world_entry_appearance.rs (198L) ✅ KEEP
├── connect_loop.rs (592L) [separate refactor candidate]
├── character_create.rs (511L) [separate refactor candidate]
├── ... (others)
└── world_entry_methods/ (24 files)
```

**Max file sizes in refactored world_entry/:**
- `vendor_ops.rs`: ~240L
- `entity_sync.rs`: ~220L
- `inventory_sync.rs`: ~200L
- `world_events.rs`: ~200L

All **under 250L, achievable.**

---

## Why dispatch.rs Should NOT Be Refactored

**Current dispatch.rs (662L):**
- Lines 1-197: Re-export hubs (const mapping CM_* to cell_methods::* constants)
- Lines 198-260: Actual dispatch function (thin routing, ~62 lines)
- Lines 261-662: Per-interface dispatch wrappers, helper utilities, and method-name lookups used by telemetry and tests

**Why it's well-designed:**
1. **Separation of concerns:**
   - Re-exports: Backward compatibility hub
   - Routing: Clean, linear, easy to trace
2. **Clear intent:** Each interface dispatch is one line (e.g., `if cell_methods::player::dispatch(...)`), ordered by index range
3. **No complex logic:** No branching, no state mutations, no async operations
4. **Const re-exports are necessary:** Other modules (tests, service.rs) reference CM_* constants for debugging and telemetry

**Verdict:** This is a **routing hub**, not a processor. It's already at maximum clarity. Splitting it would make it harder to understand the full method dispatch order. **Leave it alone.**

---

## Estimated Effort & Impact

| File | Current | Proposed | Effort | Benefit | Priority |
|------|---------|----------|--------|---------|----------|
| **space_manager.rs** | 1,402L | 10 files (incl. mod.rs), 50-260L | 2-3 hrs | VERY HIGH (largest file) | 🔴 **NOW** |
| **world_entry.rs** | 932L | 10 files (incl. mod.rs), 60-240L | 2 hrs | HIGH (message hub) | 🟠 **SOON** |
| **dispatch.rs** | 662L | Keep as-is | 0 hrs | N/A (already optimal) | 🟢 **SKIP** |

---

## File Size Distribution After Refactoring

**Current:**
```
Max: 1,402L (space_manager)
Avg: 620L (weighted across all files)
> 400L: 2 files
200-400L: 8 files
< 200L: 8 files
```

**After space_manager + world_entry refactoring:**

```text
Max:        915L  (spawner.rs — separate refactor candidate; tracks above)
Avg:        ~350L (weighted across all crates/services/src files)
> 800L:     1 file (spawner.rs)
400-800L:   2 files
200-400L:   6 files
< 200L:     25 files
```

(Goal of "no files > 400L outside the deferred refactor candidates" is met
once spawner.rs is split.)

---

## Recommendation

1. **Phase 1 (NOW):** Refactor `space_manager.rs` → 6 modules
   - Biggest win for LLM context
   - Isolated system (no impact on other services)
   - ~2.5 hours effort

2. **Phase 2 (SOON):** Refactor `world_entry.rs` → 9 modules + mod.rs hub
   - Message handler hub
   - Clear semantic groups (vendor, inventory, rewards, events)
   - ~2 hours effort

3. **Phase 3 (OPTIONAL):** Refactor `connect_loop.rs`, `character_create.rs`
   - After core refactoring is complete
   - Lower priority (not on critical path)

4. **DON'T:** Refactor `dispatch.rs`
   - Already well-designed
   - Splitting would reduce clarity
   - Const re-exports are needed

---

## New Module Structure Preview

**After all refactoring:**
```
crates/services/src/
├── base/
│   ├── world_entry/
│   │   ├── mod.rs
│   │   ├── message_handlers.rs
│   │   ├── entity_sync.rs
│   │   ├── world_events.rs
│   │   ├── vendor_ops.rs
│   │   ├── inventory_sync.rs
│   │   ├── player_rewards.rs
│   │   ├── mail_handler.rs
│   │   ├── mission_handler.rs
│   │   └── repair_recharge.rs
│   ├── world_entry_methods/
│   │   ├── player_load/ (3 files)
│   │   ├── inventory/ (4 files)
│   │   ├── vendor/ (14 files)
│   │   └── (4 files at top level)
│   ├── (other files)
│   └── mod.rs
│
└── cell/
    ├── space_manager/
    │   ├── mod.rs
    │   ├── entity_storage.rs
    │   ├── connection.rs
    │   ├── position_sync.rs
    │   ├── aoi_manager.rs
    │   ├── spatial_queries.rs
    │   ├── npc_spawning.rs
    │   ├── entity_lookup.rs
    │   ├── data_access.rs
    │   └── types.rs
    ├── cell_methods/
    │   ├── player/ (9 files)
    │   └── (other interfaces)
    ├── dispatch.rs ✅ UNCHANGED
    ├── (other files)
    └── mod.rs
```

**Total new structure:**
- Base service: 24 world_entry_methods files + 10 world_entry files (incl. `mod.rs`) = **34 focused modules** (vs. 1 monolithic)
- Cell service: 9 cell_methods/player files + 10 space_manager files (incl. `mod.rs`) = **19 focused modules** (vs. 2 monolithic)
- **Zero files > 400L** (currently 2)
- **Max file: ~260L** (currently 1,402L)

---

*This structure enables efficient LLM context loading and team onboarding without sacrificing code maintainability.*
