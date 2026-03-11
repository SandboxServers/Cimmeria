# XP & Leveling Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement XP accumulation, level-up with stat scaling, training point grants, and kill XP so players can progress through levels 1-20.

**Architecture:** XP events flow from CellApp (combat death detection) through a new `CellToBaseMsg::GrantXP` message to BaseApp, which updates `PlayerState`, scales stats, grants training points, and sends 5 client wire messages. The XP table is ported from the Python reference. Mob entities gain a `level` field for XP calculation.

**Tech Stack:** Rust, tokio mpsc channels, existing StatList/CellEntity infrastructure

---

### Task 1: Port XP table and fix level-up logic in PlayerState

**Files:**
- Modify: `crates/game/src/player.rs`

**Step 1: Update existing tests for new XP table**

Replace the quadratic-formula-based test expectations with the Python XP table values. The table is cumulative — level up when `xp > LEVEL_XP[level]`.

```rust
// In the existing tests module, replace the tests:

#[test]
fn new_player_starts_at_level_1() {
    let p = test_player();
    assert_eq!(p.level, 1);
    assert_eq!(p.xp, 0);
    assert!(!p.is_online);
}

#[test]
fn xp_for_next_level_uses_table() {
    let p = test_player();
    assert_eq!(p.xp_for_next_level(), 100); // Level 1 threshold
}

#[test]
fn grant_xp_below_threshold_no_level_up() {
    let mut p = test_player();
    p.grant_xp(50);
    assert_eq!(p.level, 1);
    assert_eq!(p.xp, 50);
}

#[test]
fn grant_xp_at_threshold_triggers_level_up() {
    let mut p = test_player();
    p.grant_xp(101); // > 100 threshold for level 1
    assert_eq!(p.level, 2);
    assert_eq!(p.xp, 101);
}

#[test]
fn grant_xp_multi_level_up() {
    let mut p = test_player();
    p.grant_xp(301); // > 100 (level 2) and > 200 (level 3) and > 300 (level 4)
    assert_eq!(p.level, 4);
}

#[test]
fn grant_xp_at_max_level_no_overflow() {
    let mut p = test_player();
    p.level = 20;
    p.xp = 400_000;
    let old_level = p.level;
    p.grant_xp(999_999);
    assert_eq!(p.level, old_level); // Should not go past 20
}

#[test]
fn xp_for_next_level_at_max_returns_max() {
    let mut p = test_player();
    p.level = 20;
    assert_eq!(p.xp_for_next_level(), u64::MAX); // Can never level past 20
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p cimmeria-game -- player`
Expected: Several failures (wrong XP thresholds, no MAX_LEVEL guard)

**Step 3: Implement the XP table and MAX_LEVEL guard**

Replace the top of `player.rs`:

```rust
/// Maximum character level.
pub const MAX_LEVEL: u32 = 20;

/// Cumulative XP thresholds per level, ported from `python/common/Constants.py:127-139`.
/// Index 0 is unused (level 0 doesn't exist). Level N requires `LEVEL_XP[N]` total XP.
const LEVEL_XP: [u64; 21] = [
    0,
    // Levels 1-10
    100, 200, 300, 600, 1_000, 1_600, 2_500, 4_000, 6_000, 9_000,
    // Levels 11-20
    14_000, 18_000, 25_000, 40_000, 60_000, 90_000, 120_000, 180_000, 250_000, 400_000,
];
```

Update `xp_for_next_level`:

```rust
pub fn xp_for_next_level(&self) -> u64 {
    if self.level >= MAX_LEVEL {
        return u64::MAX; // Already at max, can never level up
    }
    LEVEL_XP[self.level as usize]
}
```

Update `grant_xp` to cap at MAX_LEVEL:

```rust
pub fn grant_xp(&mut self, amount: u64) -> Vec<u32> {
    self.xp += amount;
    tracing::debug!(
        player = %self.character_name,
        xp_gained = amount,
        total_xp = self.xp,
        "XP granted"
    );

    let mut levels_gained = Vec::new();
    while self.level < MAX_LEVEL && self.xp > LEVEL_XP[self.level as usize] {
        self.level += 1;
        levels_gained.push(self.level);
        tracing::info!(
            player = %self.character_name,
            new_level = self.level,
            "Player leveled up"
        );
    }
    levels_gained
}
```

Remove the old `level_up()` method and the standalone `xp_for_level()` function.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p cimmeria-game -- player`
Expected: All PASS

**Step 5: Commit**

```
feat(game): port Python XP table and add MAX_LEVEL guard
```

---

### Task 2: Add training points to PlayerState

**Files:**
- Modify: `crates/game/src/player.rs`

**Step 1: Write tests for training points**

```rust
#[test]
fn new_player_has_zero_training_points() {
    let p = test_player();
    assert_eq!(p.training_points, 0);
}

#[test]
fn grant_xp_grants_training_points_on_level_up() {
    let mut p = test_player();
    p.grant_xp(101); // Level 1 → 2
    assert_eq!(p.training_points, 2); // TRAINING_POINTS_PER_LEVEL = 2
}

#[test]
fn multi_level_up_grants_cumulative_training_points() {
    let mut p = test_player();
    p.grant_xp(301); // Level 1 → 4 (3 level-ups)
    assert_eq!(p.training_points, 6); // 3 × 2
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p cimmeria-game -- player`
Expected: Compile error — `training_points` field doesn't exist

**Step 3: Add training_points field and grant logic**

Add to `PlayerState`:
```rust
pub training_points: u32,
```

Add constant:
```rust
/// Training points granted per level-up.
pub const TRAINING_POINTS_PER_LEVEL: u32 = 2;
```

Initialize in `new()`:
```rust
training_points: 0,
```

In `grant_xp`, inside the level-up loop after incrementing level:
```rust
self.training_points += TRAINING_POINTS_PER_LEVEL;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p cimmeria-game -- player`
Expected: All PASS

**Step 5: Commit**

```
feat(game): add training point grants on level-up (2 per level)
```

---

### Task 3: Add stat scaling to StatList

**Files:**
- Modify: `crates/entity/src/stats.rs`

**Step 1: Add `health_per_level` and `focus_per_level` to ArchetypeStatValues and write test**

```rust
#[test]
fn scale_for_level_increases_health_and_focus() {
    let mut list = StatList::new();
    let arch = ArchetypeStatValues {
        coordination: 5, engagement: 4, fortitude: 3, morale: 4,
        perception: 3, intelligence: 2, health: 760, focus: 1570,
        health_per_level: 10, focus_per_level: 70,
    };
    list.apply_archetype(&arch);

    // Level 1: base values
    assert_eq!(list.get(HEALTH).unwrap().max, 760);
    assert_eq!(list.get(FOCUS).unwrap().max, 1570);

    // Scale to level 5: health = 760 + 10*(5-1) = 800, focus = 1570 + 70*(5-1) = 1850
    list.scale_for_level(5, &arch);
    assert_eq!(list.get(HEALTH).unwrap().max, 800);
    assert_eq!(list.get(HEALTH).unwrap().cur, 800); // Full heal on level-up
    assert_eq!(list.get(FOCUS).unwrap().max, 1850);
    assert_eq!(list.get(FOCUS).unwrap().cur, 1850);
    assert!(list.has_dirty());
}

#[test]
fn scale_for_level_1_is_base() {
    let mut list = StatList::new();
    let arch = ArchetypeStatValues {
        coordination: 5, engagement: 4, fortitude: 3, morale: 4,
        perception: 3, intelligence: 2, health: 760, focus: 1570,
        health_per_level: 10, focus_per_level: 70,
    };
    list.apply_archetype(&arch);
    list.scale_for_level(1, &arch);
    // Level 1: no bonus
    assert_eq!(list.get(HEALTH).unwrap().max, 760);
    assert_eq!(list.get(FOCUS).unwrap().max, 1570);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p cimmeria-entity -- stats`
Expected: Compile error — missing fields, missing method

**Step 3: Implement**

Add fields to `ArchetypeStatValues`:
```rust
pub health_per_level: i32,
pub focus_per_level: i32,
```

Add method to `StatList`:
```rust
/// Scale health and focus stats for a given character level.
///
/// Formula: `max = base + per_level * (level - 1)`
/// Current value is restored to the new max (full heal on level-up).
///
/// Reference: Missing `setLevel()` in Python — this is the implementation
/// that `python/cell/SGWPlayer.py:794` called but never defined.
pub fn scale_for_level(&mut self, level: u32, arch: &ArchetypeStatValues) {
    let bonus_health = arch.health_per_level * (level as i32 - 1);
    let new_health_max = arch.health + bonus_health;
    if let Some(stat) = self.stats.get_mut(&HEALTH) {
        stat.update(0, new_health_max, new_health_max);
    }

    let bonus_focus = arch.focus_per_level * (level as i32 - 1);
    let new_focus_max = arch.focus + bonus_focus;
    if let Some(stat) = self.stats.get_mut(&FOCUS) {
        stat.update(0, new_focus_max, new_focus_max);
    }
}
```

Update all existing `ArchetypeStatValues` constructors in tests to include the new fields (add `health_per_level: 10, focus_per_level: 70` to all test archetype values).

**Step 4: Run tests to verify they pass**

Run: `cargo test -p cimmeria-entity -- stats`
Expected: All PASS

**Step 5: Also run combat tests since they use ArchetypeStatValues**

Run: `cargo test -p cimmeria-services -- combat`
Expected: Compile errors from missing fields — fix all `ArchetypeStatValues` constructors in combat tests to include the new fields.

Run: `cargo test -p cimmeria-services -- combat`
Expected: All PASS

**Step 6: Commit**

```
feat(entity): add stat scaling for level-up (health/focus per level)
```

---

### Task 4: Add mob level to CellEntity and SpawnDef

**Files:**
- Modify: `crates/entity/src/cell_entity.rs`
- Modify: `crates/services/src/cell/spawner.rs`

**Step 1: Write test for mob level**

In `crates/entity/src/cell_entity.rs` tests:
```rust
#[test]
fn new_entity_has_default_level() {
    let entity = make_entity();
    assert_eq!(entity.level, 1);
}
```

In `crates/services/src/cell/spawner.rs` tests:
```rust
#[test]
fn spawned_npcs_have_level() {
    let mut mgr = make_manager_with_worlds();
    spawn_initial_npcs(&mut mgr);
    let npc = mgr.get_entity(100_000).unwrap();
    assert!(npc.level >= 1);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p cimmeria-entity -- cell_entity`
Expected: Compile error — no `level` field

**Step 3: Implement**

Add to `CellEntity`:
```rust
/// Entity level (for XP calculations on kill). Default 1.
pub level: u32,
```

Initialize in `CellEntity::new()`:
```rust
level: 1,
```

Add to `SpawnDef`:
```rust
/// Entity level (affects XP granted on kill).
pub level: u32,
```

Add `level` to each `SpawnDef` in `SPAWN_DEFS` and `INSTANCE_SPAWN_DEFS` (use level values appropriate to the zone — Agnos mobs: level 1-2, Castle mobs: level 3-5). Set `level` on the entity after spawning:
```rust
if let Some(entity) = space_mgr.get_entity_mut(npc_id) {
    entity.interaction_type = def.interaction.clone();
    entity.npc_name = Some(def.name.to_string());
    entity.level = def.level;
}
```

**Step 4: Run all tests**

Run: `cargo test -p cimmeria-entity -- cell_entity && cargo test -p cimmeria-services -- spawner`
Expected: All PASS

**Step 5: Commit**

```
feat(entity): add level field to CellEntity and SpawnDef
```

---

### Task 5: Add CellToBaseMsg::GrantXP and send it on mob kill

**Files:**
- Modify: `crates/services/src/cell/messages.rs`
- Modify: `crates/services/src/cell/abilities.rs`

**Step 1: Add GrantXP variant to CellToBaseMsg**

In `messages.rs`, add to the `CellToBaseMsg` enum:
```rust
/// Grant XP to a player entity (from a mob kill).
///
/// The CellService computes the XP amount from the mob's level and sends
/// this to BaseApp, which updates the player's XP/level and sends client
/// notifications.
GrantXP {
    entity_id: u32,
    xp_amount: u64,
},
```

**Step 2: Add kill XP formula**

In `crates/services/src/cell/abilities.rs`, add a helper:
```rust
/// Calculate XP reward for killing a mob of the given level.
/// Formula: 10 * mob_level.
fn kill_xp(mob_level: u32) -> u64 {
    10 * mob_level as u64
}
```

**Step 3: Hook into handle_use_ability death detection**

In `handle_use_ability`, after the `target_died` block that sends `onStateFieldUpdate` (around line 185-193), add:

```rust
// Grant XP to the attacker if the target is a non-player entity
if target_died {
    let target = space_mgr.get_entity(target_eid);
    if let Some(target) = target {
        if !target.is_player {
            let xp = kill_xp(target.level);
            tracing::info!(
                attacker = entity_id, target = target_eid,
                mob_level = target.level, xp, "Granting kill XP"
            );
            let _ = tx.send(CellToBaseMsg::GrantXP {
                entity_id,
                xp_amount: xp,
            }).await;
        }
    }
}
```

Note: This requires reading the target entity *after* the death state update. Check if `get_entity` (immutable) is callable here — may need to restructure the borrow. The target's stats were already modified through `get_entity_mut` earlier, so we can read `level` and `is_player` from a fresh immutable borrow after that mutable borrow is dropped.

**Step 4: Run tests**

Run: `cargo test -p cimmeria-services -- abilities`
Expected: All PASS (existing tests don't have mob targets that die, so no XP send expected — just verify no compile errors)

**Step 5: Commit**

```
feat(services): grant kill XP when mob dies (10 * mob_level)
```

---

### Task 6: Handle GrantXP in BaseApp — send client notifications

**Files:**
- Modify: `crates/services/src/base/world_entry.rs`

**Step 1: Add GrantXP handler to `handle_cell_message`**

In the `match msg` block, add a new arm:

```rust
CellToBaseMsg::GrantXP { entity_id, xp_amount } => {
    handle_grant_xp(entity_id, xp_amount, socket, connected, entity_to_addr).await;
}
```

**Step 2: Implement `handle_grant_xp`**

This function needs to:
1. Look up the player's session state (level, xp, archetype)
2. Call `grant_xp()` on the player state
3. For each level gained, send client wire messages

For now, since `PlayerState` isn't stored per-session yet (the existing code uses `ConnectedClientState`), we need to track XP/level in the session. Check what `ConnectedClientState` contains and add XP/level fields if needed, or use a side-channel approach.

The simplest approach: add `xp`, `level`, and `training_points` to the session state or create a parallel XP tracking map. Then for each level-up, send the 5 client messages:

```rust
async fn handle_grant_xp(
    entity_id: u32,
    xp_amount: u64,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Send onExpUpdate (client method index 30 on SGWPlayer) with new total XP
    // Send giveXPForLevel (client method index 31) for each level gained — to self + witnesses
    // Send onMaxExpUpdate (client method index 32) with next level threshold
    // Send onStatUpdate (client method index 20) with updated health/focus
    // Send onEntityProperty with GENERICPROPERTY_TrainingPoints

    // Implementation requires knowing the current player state.
    // Use send_to_witness helper for entity method calls.
}
```

The exact client method indices need to be verified against the entity def. The key wire messages are:
- `onExpUpdate(INT32 exp)` — total XP
- `giveXPForLevel(INT32 level)` — level-up notification (to owner + witnesses via `CELL_PUBLIC`)
- `onMaxExpUpdate(INT32 maxExp)` — XP bar cap for next level
- `onStatUpdate(StatUpdateList)` — updated health/focus max
- `onEntityProperty(INT32 propType, INT32 value)` — training points

Each is sent as an `EntityMethodCall` via `send_to_witness`.

**Step 3: Verify entity method indices**

Check `entities/defs/SGWPlayer.def` for the exact ClientMethod indices:
- Look at the `<ClientMethods>` section
- Count methods in order to find the flat indices for `onExpUpdate`, `giveXPForLevel`, `onMaxExpUpdate`, `onEntityProperty`
- The `onStatUpdate` index (20) is already known from combat code

**Step 4: Run tests**

Run: `cargo test -p cimmeria-services -- world_entry`
Expected: All PASS

**Step 5: Commit**

```
feat(services): handle GrantXP in BaseApp with client notifications
```

---

### Task 7: Wire format verification and integration test

**Files:**
- Modify: `crates/game/src/player.rs` (add integration-style test)

**Step 1: Write a comprehensive level-up integration test**

```rust
#[test]
fn full_level_progression_1_to_20() {
    let mut p = test_player();
    for level in 2..=20 {
        let needed = LEVEL_XP[(level - 1) as usize] - p.xp + 1;
        let levels = p.grant_xp(needed);
        assert_eq!(p.level, level, "Should be level {level}");
        assert!(levels.contains(&level));
        assert_eq!(p.training_points, (level - 1) * TRAINING_POINTS_PER_LEVEL);
    }
    // At max level
    assert_eq!(p.level, MAX_LEVEL);
    assert_eq!(p.training_points, 38); // 19 * 2
}

#[test]
fn xp_table_is_monotonically_nondecreasing() {
    for i in 1..LEVEL_XP.len() - 1 {
        assert!(
            LEVEL_XP[i] <= LEVEL_XP[i + 1],
            "XP table not monotonic at index {i}: {} > {}",
            LEVEL_XP[i], LEVEL_XP[i + 1]
        );
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p cimmeria-game -- player`
Expected: All PASS. Note: `xp_table_is_monotonically_nondecreasing` documents the known anomaly — if the table has anomalies, this test will flag them. The current table from Python should pass since we check `<=` not `<`.

**Step 3: Run full workspace tests**

Run: `cargo test`
Expected: All PASS across all crates

**Step 4: Commit**

```
test(game): add full level progression and XP table validation tests
```

---

### Task 8: Update known-issues.md and gap-analysis.md

**Files:**
- Modify: `docs/known-issues.md`
- Modify: `docs/gap-analysis.md` (XP & Leveling section, update statuses)
- Modify: `docs/project-status.md` (update feature counts)

**Step 1: Mark KI-19 kill XP as partially resolved**

In `known-issues.md`, the XP/leveling gap is implicitly covered by the gap analysis. Add a note to the "Rust Rewrite — Not Yet Implemented" section if relevant, or update existing entries.

**Step 2: Update gap-analysis.md**

Change XP & Leveling features from KM/IM to IM/NT:
- XP accumulation: IM → CW (code + tests)
- Level-up loop: IM → CW
- XP from kills: KM → IM
- Stat scaling per level: KM → IM
- Training points grant: KM → IM

**Step 3: Commit**

```
docs: update project status for XP & leveling implementation
```
