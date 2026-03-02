# Progression System

Covers XP gain, leveling, stat growth, training points, and applied science points. These systems are partially implemented: leveling and ability training work end-to-end, while stat scaling and point granting on level-up are absent.

---

## XP and Leveling

### Constants

Defined in `python/common/Constants.py`:

```python
MAX_LEVEL = 20
LEVEL_EXP = [
    0,
    100, 200, 300, 600, 1000, 1600, 2500, 4000, 6000, 9000,
    14000, 18000, 25000, 40000, 60000, 90000, 120000, 180000, 250000, 400000
]
```

The list is indexed by level. `LEVEL_EXP[n]` is the total XP required to reach level `n`. Index 0 is unused (level 0 does not exist). A TODO comment in the source marks these as placeholder values pending a properly balanced curve.

### XP Thresholds

| Level | Total XP Required | XP to Next Level |
|-------|-------------------|------------------|
| 1 | 100 | 100 |
| 2 | 200 | 100 |
| 3 | 300 | 100 |
| 4 | 600 | 300 |
| 5 | 1,000 | 400 |
| 6 | 1,600 | 600 |
| 7 | 2,500 | 900 |
| 8 | 4,000 | 1,500 |
| 9 | 6,000 | 2,000 |
| 10 | 9,000 | 3,000 |
| 11 | 14,000 | 5,000 |
| 12 | 18,000 | 4,000 |
| 13 | 25,000 | 7,000 |
| 14 | 40,000 | 15,000 |
| 15 | 60,000 | 20,000 |
| 16 | 90,000 | 30,000 |
| 17 | 120,000 | 30,000 |
| 18 | 180,000 | 60,000 |
| 19 | 250,000 | 70,000 |
| 20 | 400,000 | 150,000 |

Note: The XP-to-next values are irregular. Level 12 requires less XP than level 11, which is almost certainly a placeholder artifact rather than intentional design.

### giveExperience()

Fully implemented on the player entity:

```python
def giveExperience(self, exp):
    self.exp += exp
    self.client.onExpUpdate(self.exp)
    while self.exp > Constants.LEVEL_EXP[self.level] and self.level <= Constants.MAX_LEVEL:
        self.level += 1
        self.client.giveXPForLevel(self.level)
        self.witnesses.giveXPForLevel(self.level)
        self.client.onMaxExpUpdate(Constants.LEVEL_EXP[self.level])
        self.setLevel(self.level)
        self.feedback('Congratulations! You are now level %d.' % self.level)
```

Behavior:
- Accumulates XP and immediately notifies the client of the new total via `onExpUpdate`.
- The while loop handles multi-level-up in a single call: if a single XP grant spans multiple thresholds, all intermediate levels are applied in sequence.
- Each level-up notifies the client (`giveXPForLevel`), nearby witnesses, and updates the displayed XP cap (`onMaxExpUpdate`).
- `setLevel()` applies level-dependent changes (currently limited; see stat scaling below).
- The level cap is MAX_LEVEL = 20. The condition `self.level <= Constants.MAX_LEVEL` prevents advancement past 20.

### XP Sources

| Source | Status | Notes |
|--------|--------|-------|
| Mission completion | Implemented | `self.giveExperience(mission.rewardXp)` called in mission handler |
| Mob kills | Not implemented | No `giveExperience` call in combat or death handlers |
| Minigames | Not implemented | No XP reward hook in minigame completion |

Mob XP is the most significant gap. In a complete implementation, `SGWMob.onDead()` would calculate XP based on mob level and an `isWorthXP` flag, then distribute it to the tapped player or squad.

### Database Schema

```sql
-- sgw_player table
level   INTEGER DEFAULT 1 NOT NULL,   -- CHECK: 0 <= level <= 20
exp     INTEGER DEFAULT 0 NOT NULL,
```

Both columns are persisted per character. The level constraint enforces the 0-20 range at the database layer.

---

## Stat System

### Primary Attributes

Six primary attributes govern combat and ability behavior:

| Index | Name | Description |
|-------|------|-------------|
| 0 | coordination | Ranged accuracy |
| 1 | engagement | Melee accuracy |
| 2 | fortitude | Resistance to health damage |
| 3 | morale | No current mechanical effect |
| 4 | perception | Defense against incoming accuracy checks |
| 5 | intelligence | Resistance to focus damage |

### Archetype Base Stats

Loaded from the `resources.archetypes` database table at character creation and applied by `setupPlayer()`:

| Archetype | coord | engage | fort | morale | percep | intel | health | focus | healthPerLevel | focusPerLevel |
|-----------|-------|--------|------|--------|--------|-------|--------|-------|----------------|---------------|
| Soldier | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1,570 | 10 | 70 |
| Commando | 4 | 4 | 2 | 3 | 5 | 3 | 760 | 1,570 | 10 | 70 |
| Scientist | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1,570 | 10 | 70 |
| (others) | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1,570 | 10 | 70 |

All values are placeholders. Only the Commando archetype differs from the default. The `healthPerLevel` and `focusPerLevel` columns are loaded into the archetype definition object but are never applied (see below).

### Stat Initialization

`setupPlayer()` applies archetype stats when the player entity first loads:

```python
archetype = DefMgr.get('archetype', self.archetype)
self.getStat(coordination).update(0, archetype.coordination, archetype.coordination)
self.getStat(engagement).update(0, archetype.engagement, archetype.engagement)
self.getStat(fortitude).update(0, archetype.fortitude, archetype.fortitude)
self.getStat(morale).update(0, archetype.morale, archetype.morale)
self.getStat(perception).update(0, archetype.perception, archetype.perception)
self.getStat(intelligence).update(0, archetype.intelligence, archetype.intelligence)
self.getStat(health).update(0, archetype.health, archetype.health)
self.getStat(focus).update(0, archetype.focus, archetype.focus)
# Hardcoded resistances:
self.getStat(kineticRes).update(0, 40, 2000)
self.getStat(mentalRes).update(0, 20, 2000)
self.getStat(healthRes).update(0, 30, 2000)
```

The `update(min, current, max)` signature sets all three bounds at once. Resistances are hardcoded rather than archetype-driven.

### Stat Class

Defined in `SGWBeing.py`:

```python
class Stat(object):
    __slots__ = ('min', 'cur', 'max', 'baseMin', 'baseCur', 'baseMax',
                 'dirty', 'baseDirty', 'callback')
```

Each stat tracks both base values and current (modified) values. Available methods:

- `setCurrent(value)` - Set current value directly
- `change(delta)` - Modify current value by a delta
- `changeByPercent(pct)` - Modify by percentage of max
- `changeByMaxPercent(pct)` - Modify by percentage of base max
- `changeByMinMaxPercent(pct)` - Modify proportionally between min and max
- `setMin(value)` / `setMax(value)` - Adjust bounds
- `update(min, cur, max)` - Set all three values at once
- `clone()` - Duplicate stat for temporary calculations

### Stat Roles in Combat

Primary attributes feed directly into the Quick Resolution (QR) combat system:

| Stat | Combat Role | Formula |
|------|-------------|---------|
| coordination | Ranged accuracy bonus | `coord * 0.05` added to QR roll |
| engagement | Melee accuracy bonus | `engage * 0.05` added to QR roll |
| perception | Defense against accuracy | `percep * 0.05` subtracted from attacker QR |
| fortitude | Reduces health damage taken | `fort * 0.01` as damage resistance multiplier |
| intelligence | Reduces focus damage taken | `intel * 0.01` as damage resistance multiplier |
| morale | None | No active formula; placeholder for future use |

### Stat Scaling on Level-Up: Not Implemented

`healthPerLevel` and `focusPerLevel` are present in the archetype definition and loaded from the database, but `setLevel()` does not apply them. When a player levels up:

- The level number increments.
- Client and witnesses are notified.
- No stat values change.

A player at level 20 has the same health and focus caps as a player at level 1. This is the most significant gap in the progression system.

### Derived Stats

No derivation formulas exist. Stats such as accuracy rating, defense rating, and armor factors are set directly rather than computed from primary attributes. There is no formula layer that converts, for example, high coordination into a bonus to hit chance beyond the direct QR multiplier.

---

## Training Points

Training points (TP) gate ability learning. A player spends one TP to permanently unlock an ability via an in-world AbilityTrainer NPC.

### Storage

```sql
-- sgw_player table
training_points INTEGER DEFAULT 0
```

### Earning: Not Implemented

No code grants training points on level-up. `giveTrainingPoints(points)` exists and is functional, but it is never called by the leveling path. Points must be manually inserted into the database for testing.

### Spending

Fully implemented via the AbilityTrainer interaction handler:

```python
def onTrainAbility(self, player, abilityId):
    if player.trainingPoints < 1:
        player.onError("Not enough training points")
        return
    player.consumeTrainingPoints(1)
    player.addAbility(abilityId)
```

Each ability costs exactly 1 TP. Before the point is consumed, the system validates:

- Player has at least 1 training point.
- The ability definition exists.
- Player level meets the ability's required level.
- All prerequisite abilities are already learned.

If any check fails, the client receives an error message and no points are spent.

### Respec

Not implemented. The respec handler returns `player.onError('Not implemented yet!')`. There is no mechanism to refund spent training points or unlearn abilities.

### Utility Methods

```python
def giveTrainingPoints(self, points):
    self.trainingPoints += points
    self.client.onTrainingPointsUpdate(self.trainingPoints)

def consumeTrainingPoints(self, points):
    assert self.trainingPoints >= points
    self.trainingPoints -= points
    self.client.onTrainingPointsUpdate(self.trainingPoints)
```

Both methods update the client immediately after modifying the stored value.

---

## Applied Science Points

Applied science points (ASP) gate discipline learning. Disciplines are crafting specializations organized in a skill tree.

### Storage

```sql
-- sgw_player table
applied_science_points INTEGER DEFAULT 0
```

### Earning: Not Implemented

No code grants applied science points on level-up. The grant methods exist and are functional but are never called by the leveling path. Points must be manually set in the database.

### Spending

Fully implemented:

```python
def spendAppliedSciencePoints(self, disciplineId):
    # Checks performed before spending:
    # - Player has >= 1 ASP
    # - Discipline definition exists
    # - Racial paradigm level requirement is met
    # - All prerequisite disciplines are known at >= 50 expertise
    learned = self.learnDiscipline(disciplineId)
    if learned:
        self.consumeAppliedSciencePoints(1)
```

Each discipline costs 1 ASP. The spend only occurs if `learnDiscipline()` succeeds.

### Discipline System

Disciplines represent crafting specializations within a branching skill tree:

- Each discipline starts at expertise level 1 when first learned.
- Expertise ranges from 0 to 100 and increases through crafting activity.
- Prerequisites require prior disciplines to be at expertise >= 50 before the next tier can be unlocked.
- Racial paradigm level acts as a category gate: certain disciplines require a minimum paradigm level for the player's race before they can be learned.

### Racial Paradigm

- All racial paradigms begin at level 1 on character creation.
- Paradigm level can be raised via `updateRacialParadigmLevel()`.
- Paradigm level gates entire categories of disciplines rather than individual ones.

### Utility Methods

```python
def giveAppliedSciencePoints(self, points):
    self.appliedSciencePoints += points
    self.client.onAppliedSciencePointsUpdate(self.appliedSciencePoints)

def consumeAppliedSciencePoints(self, points):
    assert self.appliedSciencePoints >= points
    self.appliedSciencePoints -= points
    self.client.onAppliedSciencePointsUpdate(self.appliedSciencePoints)
```

---

## Implementation Status Summary

| Feature | Status |
|---------|--------|
| XP accumulation | Implemented |
| Level-up loop (multi-level) | Implemented |
| Mission XP rewards | Implemented |
| Mob kill XP rewards | Not implemented |
| Minigame XP rewards | Not implemented |
| Archetype base stats on create | Implemented |
| Stat scaling per level (health, focus) | Not implemented |
| Training point spending (ability learn) | Implemented |
| Training point granting on level-up | Not implemented |
| Ability respec | Not implemented |
| Applied science point spending (disciplines) | Implemented |
| Applied science point granting on level-up | Not implemented |
| Discipline expertise progression | Implemented |
| Racial paradigm gating | Implemented |

---

## Recommended Implementation Work

The following items are needed to complete the progression loop. They are listed in priority order.

**1. Stat scaling on level-up**

In `setLevel()`, apply the archetype's `healthPerLevel` and `focusPerLevel` values for each level gained:

```python
def setLevel(self, level):
    archetype = DefMgr.get('archetype', self.archetype)
    bonus_health = archetype.healthPerLevel * (level - 1)
    bonus_focus = archetype.focusPerLevel * (level - 1)
    self.getStat(health).setMax(archetype.health + bonus_health)
    self.getStat(focus).setMax(archetype.focus + bonus_focus)
```

**2. Training points on level-up**

Call `giveTrainingPoints(n)` inside the level-up block in `giveExperience()`. The exact formula is unknown; 1 TP per level is a reasonable baseline. Some games grant additional points at milestone levels.

**3. Applied science points on level-up**

Grant ASP at defined level thresholds (e.g., every 5 levels). The original design intent is unknown.

**4. XP from mob kills**

In `SGWMob.onDead()`, calculate XP based on mob level and an `isWorthXP` flag, then call `giveExperience()` on the tapped entity. XP should be distributed to squad members if the tapping entity is in a group.

**5. Balanced XP curve**

Replace the placeholder `LEVEL_EXP` table with a mathematically consistent curve. The current values contain an anomaly at level 12 (lower delta than level 11) and the early levels are very compressed.

**6. Stat derivation formulas**

If derived stats (accuracy rating, defense rating) are intended to scale with primary attributes beyond the existing QR multipliers, a formula layer would need to be added to `setLevel()` or a dedicated `recalculateDerivedStats()` method.

**7. Respec implementation**

Ability respec requires refunding training points equal to the number of non-default abilities learned, then clearing the player's ability list back to archetype defaults. Applied science respec similarly requires refunding ASP and clearing learned disciplines (with expertise loss).

**8. Soft caps and diminishing returns**

No evidence of diminishing returns exists in the current code. Standard MMO design applies a soft cap so that stacking a single primary stat yields decreasing returns past a threshold. This would require formula changes to the QR combat system.
