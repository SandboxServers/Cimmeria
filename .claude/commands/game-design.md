# Game Design Principles

> Game design thinking for Stargate Worlds content authoring. Apply when designing missions, combat systems, progression, or game mechanics.

---

## 1. Core Loop Design

Every game needs a fun 30-second loop:

```
ACTION → Player does something
FEEDBACK → Game responds
REWARD → Player feels good
REPEAT
```

### SGW Core Loop

```
Explore Zone → Accept Mission → Complete Objectives → Earn Rewards → Unlock New Content
```

### Loop by System

| System | Core Loop |
|--------|-----------|
| Combat | Engage → Use Abilities → Defeat → Loot |
| Missions | Accept → Complete Steps → Report → Reward |
| Crafting | Gather → Learn Blueprint → Craft → Equip/Sell |
| Exploration | Discover → Interact → Unlock → Progress |

## 2. Player Psychology

### Motivation Types (Bartle's Taxonomy)

| Type | Driven By | SGW Feature |
|------|-----------|-------------|
| **Achiever** | Goals, completion | Missions, leveling, crafting mastery |
| **Explorer** | Discovery, secrets | Zone exploration, lore, hidden areas |
| **Socializer** | Interaction, community | Organizations/guilds, chat, co-op missions |
| **Killer** | Competition, dominance | PvP, leaderboards, rare gear |

### Reward Schedules

| Schedule | Effect | SGW Use |
|----------|--------|---------|
| **Fixed** | Predictable | Mission completion rewards |
| **Variable** | Engaging | Loot drops, crafting outcomes |
| **Ratio** | Effort-based | XP per kill, gathering yields |

## 3. Difficulty Balancing

```
Too Hard → Frustration → Quit
Too Easy → Boredom → Quit
Just Right → Flow → Engagement
```

### SGW Quality Rating System

The game uses a Quality Rating (QR) system for combat balance:
- Player QR = function of level, gear, abilities
- Enemy QR = defined per NPC type
- Damage/defense scaled by QR differential

### Balancing Strategies

| Strategy | How |
|----------|-----|
| **Level gating** | Zone level requirements |
| **Scaling difficulty** | Enemy QR per zone |
| **Group content** | Higher difficulty, requires cooperation |
| **Accessibility** | Multiple approaches to objectives |

## 4. Progression Design

### Progression Types in SGW

| Type | Implementation |
|------|---------------|
| **Skill** | Player mastery of combat system |
| **Power** | Level ups, gear upgrades, new abilities |
| **Content** | New zones, mission chains unlock |
| **Story** | Stargate narrative progression |

### Pacing Principles

- Early wins (hook in Castle Cellblock tutorial)
- Gradually increase challenge per zone
- Rest beats between intense combat
- Meaningful choices (alignment: SGC vs Goa'uld)

## 5. Mission Design

### Mission Structure

```
Hook → Objectives → Challenges → Resolution → Reward
```

### Objective Types

| Type | Example |
|------|---------|
| Kill | Defeat 5 Jaffa guards |
| Collect | Gather 3 Naquadah samples |
| Interact | Activate the DHD |
| Escort | Protect the scientist to the gate |
| Discover | Find the hidden Goa'uld lab |

### Chain Design (for Chain Editor)

Chains link missions into narrative arcs:
- **Trigger** → What starts the chain (level, zone entry, item pickup)
- **Condition** → What gates progression (mission complete, faction rep)
- **Action** → What happens (spawn NPC, grant reward, unlock zone)

## 6. Anti-Patterns

| Don't | Do |
|-------|-----|
| Design in isolation | Playtest constantly |
| Polish before fun | Prototype mechanics first |
| Force one way to play | Allow player expression |
| Punish excessively | Reward progress |
| Wall of text quests | Show, don't tell |
| Tedious backtracking | Respect player time |

## Project Context

- **1,041 missions** in the database
- **1,887 abilities** across archetypes (Soldier, Scientist, Commando, etc.)
- **6,060 items** (weapons, armor, consumables, quest items)
- **3,217 effects** (buffs, debuffs, damage-over-time)
- **6 of 24 zones** partially playable
- Mission system uses step-based progression with `sgw_mission` table
- Chain editor enables visual authoring of mission chains
