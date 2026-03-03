# Developer Guide

How to extend the Cimmeria server: add entity types, properties, methods, commands, interactions, and missions. Assumes you can already build and run the server (see [Operator Guide](operator-guide.md)).


## Architecture Overview

Cimmeria uses a three-service architecture inherited from BigWorld Technology:

| Service | Role | Key Data |
|---------|------|----------|
| **AuthenticationServer** | Login, account validation, shard registry | Accounts, sessions |
| **BaseApp** | Persistent entity state, client routing, cell coordination | Base properties, client connections |
| **CellApp** | Spatial simulation, movement, combat, interactions | Cell properties, positions, AoI |

Every game entity is split across Base and Cell. The **Base** part lives on BaseApp and handles persistence and client communication. The **Cell** part lives on CellApp and handles spatial simulation — position, movement, combat, interactions. Some entities also have a **Client** part (properties and methods visible to the game client).

Python scripts define entity behavior. C++ handles networking, entity lifecycle, and the scripting runtime. You will rarely need to touch C++ to add game features.

For full service topology details, see [service-architecture.md](architecture/service-architecture.md). For C++ internals, see [server-internals.md](architecture/server-internals.md).


## Adding a New Entity Type

### Step 1: Create the .def File

Create `entities/defs/MyEntity.def`:

```xml
<root>
  <Parent> SGWSpawnableEntity </Parent>

  <Implements>
    <Interface> Lootable </Interface>
  </Implements>

  <Properties>
    <displayName>
      <Type> WSTRING </Type>
      <Flags> CELL_PUBLIC </Flags>
      <Default> Unnamed </Default>
    </displayName>

    <hitPoints>
      <Type> INT32 </Type>
      <Flags> OWN_CLIENT </Flags>
      <Default> 100 </Default>
      <Persistent> true </Persistent>
    </hitPoints>

    <isActive>
      <Type> INT8 </Type>
      <Flags> CELL_PRIVATE </Flags>
      <Default> 1 </Default>
    </isActive>
  </Properties>

  <CellMethods>
    <activate>
      <Exposed/>
    </activate>

    <deactivate>
      <Exposed/>
    </deactivate>

    <onDamaged>
      <Arg> INT32 </Arg>  <!-- damage amount -->
      <Arg> INT32 </Arg>  <!-- attacker entity ID -->
    </onDamaged>
  </CellMethods>

  <ClientMethods>
    <onStateChanged>
      <Arg> INT8 </Arg>  <!-- new state -->
    </onStateChanged>
  </ClientMethods>

  <BaseMethods>
    <requestSave>
    </requestSave>
  </BaseMethods>
</root>
```

See [entity-def-guide.md](guides/entity-def-guide.md) for the full .def XML syntax reference.

### Step 2: Register in entities.xml

Add the entity type to `entities/entities.xml`:

```xml
<root>
  <!-- ... existing entities ... -->
  <MyEntity/>
</root>
```

The position in the file determines the entity's type index in the wire protocol. Add new types at the end to avoid breaking existing indices.

### Step 3: Create Cell Python Script

Create `python/cell/MyEntity.py`:

```python
import Atrea
import Atrea.enums as enums

class MyEntity(Atrea.CellEntity):

    def __init__(self):
        Atrea.CellEntity.__init__(self)

    def enteredSpace(self):
        """Called when the entity enters a space/world."""
        pass

    def load(self):
        """Called after entity is created from database."""
        pass

    def persist(self):
        """Called when entity state should be saved to database."""
        pass

    def destroyed(self):
        """Called before entity is removed from the world."""
        pass

    # Exposed methods (callable by client)
    def activate(self):
        self.isActive = 1
        self.allClients.onStateChanged(1)

    def deactivate(self):
        self.isActive = 0
        self.allClients.onStateChanged(0)

    # Internal methods
    def onDamaged(self, amount, attackerId):
        self.hitPoints -= amount
        if self.hitPoints <= 0:
            self.hitPoints = 0
            self.deactivate()
```

### Step 4: Create Base Python Script

Create `python/base/MyEntity.py`:

```python
import Atrea

class MyEntity(Atrea.BaseEntity):

    def __init__(self):
        Atrea.BaseEntity.__init__(self)

    def cellCreated(self):
        """Called when the cell part of this entity has been created."""
        pass

    def requestSave(self):
        """Called by cell or other base entities to trigger persistence."""
        self.cell.persist()
```

### Key Entity Callbacks

| Callback | When Called | Use For |
|----------|------------|---------|
| `__init__` | Entity object created | Initialize instance variables (not entity properties) |
| `enteredSpace` | Entity placed in a world | Start timers, register with systems |
| `load` | Entity restored from database | Rebuild runtime state from persisted properties |
| `persist` | Periodic backup or explicit save | Write state to database |
| `destroyed` | Entity removed from world | Clean up timers, references, registrations |
| `cellCreated` | (Base) Cell part instantiated | Forward data to cell, start cell-side logic |


## Adding Properties

Properties are declared in `.def` files and automatically synchronized to clients based on their flags.

### Property Flags

| Flag | Visibility | Persistence | Typical Use |
|------|-----------|-------------|-------------|
| `CELL_PRIVATE` | Cell only | No | Internal state, timers, AI data |
| `CELL_PUBLIC` | Cell + all clients in AoI | No | Name, appearance, visible state |
| `OWN_CLIENT` | Cell + owning client only | No | Health, inventory, quest status |
| `ALL_CLIENTS` | All clients everywhere | No | Global state (rare) |
| `BASE` | Base only | No | Account references, session data |
| `BASE_AND_CLIENT` | Base + owning client | No | Account-level client data |

### Persistence

Add `<Persistent> true </Persistent>` to save a property to the database. The property name becomes the column name.

For string properties, also set `<DatabaseLength>` to control the column size:

```xml
<displayName>
  <Type> WSTRING </Type>
  <Flags> CELL_PUBLIC </Flags>
  <Persistent> true </Persistent>
  <DatabaseLength> 64 </DatabaseLength>
</displayName>
```

### Property Types

| Type | Python Type | Size | Notes |
|------|-------------|------|-------|
| `INT8` / `UINT8` | int | 1 byte | Often used as boolean (0/1) |
| `INT16` / `UINT16` | int | 2 bytes | |
| `INT32` / `UINT32` | int | 4 bytes | Entity IDs, counters |
| `INT64` / `UINT64` | long | 8 bytes | |
| `FLOAT` / `FLOAT32` | float | 4 bytes | |
| `FLOAT64` | float | 8 bytes | |
| `STRING` | str | variable | ASCII string |
| `WSTRING` | str | variable | Unicode string |
| `PYTHON` | any | variable | Arbitrary pickled Python object |
| `MAILBOX` | Mailbox | N/A | Reference to another entity |
| `ARRAY` | list | variable | Typed array (define element type in alias) |
| `FIXED_DICT` | dict | variable | Typed dictionary (define fields in alias) |

### How Changes Sync

When you assign a value to a `CELL_PUBLIC` or `OWN_CLIENT` property in Python, the framework automatically serializes the change and sends it to relevant clients. No manual network code needed:

```python
# This automatically notifies all clients in AoI:
self.displayName = "New Name"

# This automatically notifies only the owning client:
self.hitPoints = 50
```


## Adding Methods

Methods enable RPC (Remote Procedure Call) between entity parts and between server and client.

### Method Types

| Section | Callable From | Executes On |
|---------|---------------|-------------|
| `CellMethods` | Base, Cell, or Client (if `<Exposed/>`) | CellApp |
| `BaseMethods` | Cell, Base, or Client (if `<Exposed/>`) | BaseApp |
| `ClientMethods` | Cell or Base | Game client |

### Exposed Methods

The `<Exposed/>` tag allows the game client to call a CellMethod or BaseMethod directly. **This is a security boundary** — exposed methods must validate all inputs because clients can send arbitrary data:

```xml
<CellMethods>
  <useItem>
    <Exposed/>
    <Arg> INT32 </Arg>  <!-- item ID -->
  </CellMethods>
```

```python
def useItem(self, itemId):
    # ALWAYS validate client input
    item = self.inventory.getItem(itemId)
    if item is None:
        return  # Client sent invalid ID
    if not item.isUsable():
        return  # Client trying to use unusable item
    # ... actual logic
```

### Method Arguments

Arguments are positional, defined by `<Arg>` tags in order. The Python method signature must match:

```xml
<onDamaged>
  <Arg> INT32 </Arg>   <!-- damage -->
  <Arg> INT32 </Arg>   <!-- attackerId -->
  <Arg> UINT8 </Arg>   <!-- damageType -->
</onDamaged>
```

```python
def onDamaged(self, damage, attackerId, damageType):
    pass
```


## Adding Interfaces

Interfaces are reusable bundles of properties and methods that can be mixed into any entity type.

### Creating an Interface

Create `entities/defs/interfaces/MyInterface.def`:

```xml
<root>
  <Properties>
    <isInteractable>
      <Type> INT8 </Type>
      <Flags> CELL_PUBLIC </Flags>
      <Default> 1 </Default>
    </isInteractable>
  </Properties>

  <CellMethods>
    <onInteract>
      <Exposed/>
      <Arg> INT32 </Arg>  <!-- interacting entity ID -->
    </onInteract>
  </CellMethods>

  <ClientMethods>
    <onInteractionResult>
      <Arg> UINT8 </Arg>  <!-- result code -->
    </onInteractionResult>
  </ClientMethods>
</root>
```

### Using an Interface

Add `<Implements>` to the entity .def:

```xml
<Implements>
  <Interface> MyInterface </Interface>
  <Interface> Lootable </Interface>
</Implements>
```

The entity's Python class must implement all methods declared by the interface. Interface methods can be defined directly in the entity class or in a separate mixin — the framework doesn't enforce a specific pattern.

### Existing Interfaces

| Interface | Purpose | Used By |
|-----------|---------|---------|
| `SGWBeing` | Core living entity (name, level, stats, movement) | SGWPlayer, SGWMob, SGWPet |
| `SGWCombatant` | Combat stats, damage, threat, stealth | SGWPlayer, SGWMob, SGWPet |
| `SGWAbilityManager` | Ability activation and cooldowns | SGWPlayer, SGWMob, SGWPet |
| `Lootable` | Loot drops and looting UI | SGWMob, SGWPet |
| `Communicator` | Chat channels, whispers, emotes | SGWPlayer |
| `OrganizationMember` | Guild membership | SGWPlayer |
| `MinigamePlayer` | Minigame participation | SGWPlayer |
| `GateTravel` | Stargate/ring transport | SGWPlayer |
| `SGWInventoryManager` | Inventory and equipment | SGWPlayer |
| `SGWMailManager` | In-game mail | SGWPlayer |
| `Missionary` | Missions and quests | SGWPlayer |
| `SGWPoller` | Poll/survey participation | SGWPlayer |
| `ContactListManager` | Friends/ignore lists | SGWPlayer |
| `SGWBlackMarketManager` | Auction house | SGWPlayer |
| `ClientCache` | Client-side caching hints | SGWPlayer |
| `DistributionGroupMember` | Distribution group membership | SGWEntity |
| `EventParticipant` | Kismet event system | SGWEntity |
| `GroupAuthority` | Squad/team management | SGWPlayerGroupAuthority |


## Python Scripting Patterns

### Entity Access

```python
# Get entity by ID (within same cell/space)
entity = Atrea.findEntity(entityId)

# Access entity's space
space = self.space

# Entity properties
pos = self.position  # Vec3
eid = self.entityId  # uint32
did = self.databaseId  # int32 (persistent ID)
```

### Mailbox RPC

Call methods on other entity parts or remote entities:

```python
# Cell → Client (call a ClientMethod on the owning client)
self.client.onStateChanged(newState)

# Cell → All Clients in AoI (broadcast to all nearby clients)
self.allClients.onStateChanged(newState)

# Cell → Base (call a BaseMethod on this entity's base part)
self.base.requestSave()

# Base → Cell (call a CellMethod on this entity's cell part)
self.cell.onDamaged(50, attackerId, 1)

# Base → Client (call a ClientMethod)
self.client.onEffectResults(data)

# Cross-entity RPC via mailbox
otherEntity.cell.someMethod(arg1, arg2)
```

### Database Access

```python
# Query (returns list of dict-like row objects)
rows = Atrea.dbQuery(
    "SELECT * FROM sgw_player WHERE player_id = %d" % self.databaseId
)
if rows:
    row = rows[0]
    self.playerName = row['player_name']
    self.level = row['level']

# Non-query (INSERT, UPDATE, DELETE)
Atrea.dbQuery(
    "UPDATE sgw_player SET level = %d WHERE player_id = %d"
    % (self.level, self.databaseId)
)
```

### Timers

```python
# Add a timer (returns timer ID)
timerId = Atrea.addTimer(
    delaySeconds,      # float: seconds until first fire
    repeatSeconds,     # float: repeat interval (0 = one-shot)
    callbackName       # string: method name to call
)

# Cancel a timer
Atrea.cancelTimer(timerId)

# Timer callback receives timer ID
def onMyTimer(self, timerId):
    pass
```

### Event Firing

```python
# Fire an event that missions and scripts can listen for
self.fire("mission.accepted::" + str(missionId))
self.fire("mission_objective.started::" + str(objectiveId))
```

### Hot-Reload

During development, reload Python scripts without restarting:

```python
Atrea.reloadClasses()
```

This reloads all entity Python classes. Existing entity instances will use the new class definitions on next method call.

### Definition Manager

Access game data loaded from the database:

```python
from common.defs.Def import DefMgr

# Look up an item definition
itemDef = DefMgr.get('item', itemId)

# Look up an ability definition
abilityDef = DefMgr.get('ability', abilityId)

# Look up a mission definition
missionDef = DefMgr.get('mission', missionId)
```


## Adding GM Commands

GM commands are Python functions registered in the console command system. They can be invoked from the Python console or the game client (via `/` chat commands for authorized players).

### Command Function Signature

```python
def myCommand(player, target):
    """Short description of what the command does.

    @param target: the targeted entity
    @type target: CellEntity
    """
    # player = the player who issued the command
    # target = the player's current target (may be None)
    target.getStat(Atrea.enums.health).setCurrent(100)
    target.sendDirtyStats()
```

### Registering Commands

Add command functions to a module in `python/cell/commands/`. The command framework discovers them automatically.

### Existing Command Examples

From `python/cell/commands/Player.py`:

```python
def killPlayer(player, target):
    """Kill the targeted entity"""
    target.getStat(Atrea.enums.health).setCurrent(0)
    target.sendDirtyStats()

def revivePlayer(player, target):
    """Revive the targeted entity"""
    health = target.getStat(Atrea.enums.health)
    health.setCurrent(health.max)
    target.sendDirtyStats()
    target.unsetCombatantStateFlag(Atrea.enums.PLAYER_STATE_Dead)
```

From `python/cell/commands/Entity.py` — commands for entity inspection and debugging.


## Adding Interactions

Interactions define how players interact with world objects (vendors, loot containers, ability trainers, etc.).

### Interaction Base Class

All interactions extend the base `Interaction` class in `python/cell/interactions/Interaction.py`.

### Example: Lootable Interaction

From `python/cell/interactions/Lootable.py`:

```python
class Lootable(Interaction):
    def __init__(self):
        self.loot = []
        self.nextLootIndex = 1
        self.looters = {}

    def interactionSetMapId(self):
        """Return the dialog set map ID for this interaction, or None."""
        return None

    def generateLoot(self):
        """Generate random loot from the entity's loot table."""
        table = DefMgr.get('lootTable', self.entity.LootTableID)
        self.randomizeLoot(table)

    def onInteract(self, player, mapId):
        """Called when a player interacts with this entity."""
        self.sendLootList(player, True)

    def onLootItem(self, player, index):
        """Called when a player picks up a loot item."""
        item = self.getLootByIndex(index)
        if item and item.isPlayerEligible(player):
            player.inventory.addItem(item.design, item.quantity)
            self.removeLoot(index)
```

### Existing Interactions

| Interaction | File | Purpose |
|-------------|------|---------|
| `Lootable` | `interactions/Lootable.py` | Loot container UI |
| `Vendor` | `interactions/Vendor.py` | NPC vendor buy/sell |
| `AbilityTrainer` | `interactions/AbilityTrainer.py` | Learn new abilities |
| `DHD` | `interactions/DHD.py` | Stargate Dial Home Device |

### Registering Interactions

Interactions are registered during cell startup in `python/cell/__init__.py` via `registerInteractions()`. Add your interaction class to the registration function.


## Adding Missions

Missions use a database-driven schema with optional Python scripts for complex behavior.

### Database Schema

Missions are defined across several tables:

```
missions                    # Top-level mission definition
├─ mission_steps           # Steps within a mission (sequential)
│  └─ mission_objectives   # Objectives within a step (may be parallel)
│     └─ mission_tasks     # Tasks within an objective
└─ mission_reward_groups   # Reward options
   └─ mission_rewards      # Individual rewards in each group
```

### MissionManager

The `MissionManager` class (`python/cell/MissionManager.py`) tracks all missions for a player:

```python
class MissionManager:
    def __init__(self, player):
        self.player = weakref.ref(player)
        self.instances = {}  # missionId -> MissionInstance

    def offerMission(self, missionId, dialogId):
        """Offer a mission to the player."""

    def abandonMission(self, missionId):
        """Player abandons an active mission."""

    def completeObjective(self, objectiveId):
        """Mark an objective as complete, advance mission state."""

    def completeMission(self, missionId, rewardIndex):
        """Complete mission and grant selected reward."""

    def persist(self):
        """Save all mission state to database."""
```

### MissionInstance

Each active mission is tracked as a `MissionInstance`:

```python
class MissionInstance:
    missionId           # Database mission ID
    status              # MISSION_Not_Active, MISSION_Active, MISSION_Completed, MISSION_Failed
    currentStepId       # Current step being worked on
    completedSteps      # List of completed steps
    activeObjectives    # Currently active objectives
    completedObjectives # Completed objectives
    script              # Optional Python script instance
```

### Mission Scripts

For complex mission behavior, create a Python script in `python/cell/missions/`:

```
python/cell/missions/
├─ MissionBase.py       # Base mission script class
├─ General/             # General/system missions
├─ SGC_W1/              # SGC World 1 missions
├─ Harset/              # Harset zone missions
└─ Castle_CellBlock/    # Castle cellblock missions
```

Mission scripts extend `MissionBase` and override lifecycle callbacks:

```python
from cell.missions.MissionBase import MissionBase

class MyMission(MissionBase):
    def onAccepted(self, player):
        """Called when player accepts the mission."""
        pass

    def onObjectiveComplete(self, player, objectiveId):
        """Called when an objective is completed."""
        pass

    def onCompleted(self, player):
        """Called when the mission is turned in."""
        pass

    def onAbandoned(self, player):
        """Called when the player abandons the mission."""
        pass
```

### Events and Missions

Missions listen for events fired by game systems:

```python
# Mission system listens for these events:
player.fire("mission.accepted::" + str(missionId))
player.fire("mission_step.started::" + str(stepId))
player.fire("mission_objective.started::" + str(objectiveId))
```


## Python Module Layout

```
python/
├─ Atrea/                       # Engine bindings (imported as Atrea module)
│
├─ base/                        # BaseApp entity scripts
│  ├─ __init__.py               #   baseStarted(), createResource()
│  ├─ SGWEntity.py              #   Base entity root class
│  ├─ SGWBeing.py               #   Base being class
│  ├─ SGWPlayer.py              #   Account/session management, chat, minigame callbacks
│  ├─ SGWSpawnableEntity.py     #   Base spawnable entity
│  ├─ Chat.py                   #   Channel manager implementation
│  └─ minigame/                 #   Minigame server integration
│
├─ cell/                        # CellApp entity scripts (primary game logic)
│  ├─ __init__.py               #   cellStarting(), cellStarted(), spaceCreated()
│  ├─ SGWEntity.py              #   Cell entity root class
│  ├─ SGWSpawnableEntity.py     #   Cell spawnable entity
│  ├─ SGWBeing.py               #   Stat system, combat base class
│  ├─ SGWPlayer.py              #   Primary player gameplay (~400+ lines)
│  ├─ SGWMob.py                 #   NPC AI, threat, combat
│  ├─ SGWPet.py                 #   Pet behavior
│  ├─ SGWGmPlayer.py            #   GM command implementations
│  ├─ Item.py                   #   Item entity logic
│  ├─ Space.py                  #   Space/world management
│  ├─ SpaceManager.py           #   Space registry and lifecycle
│  ├─ Inventory.py              #   Inventory system
│  ├─ Bag.py                    #   Container/bag logic
│  ├─ MissionManager.py         #   Mission tracking and state
│  ├─ Crafter.py                #   Crafting system
│  ├─ Trade.py                  #   Player-to-player trading
│  ├─ AbilityManager.py         #   Ability execution
│  ├─ ConsoleCommands.py        #   Command framework
│  ├─ Global.py                 #   Global singletons (PlayersByName, etc.)
│  ├─ actions/                  #   Action handlers (6 files)
│  ├─ commands/                 #   GM/player commands
│  │  ├─ Player.py              #     kill, revive, teleport, etc.
│  │  ├─ Entity.py              #     Entity info/debug
│  │  └─ Stargate.py            #     Gate commands
│  ├─ effects/                  #   Effect/buff/debuff handlers
│  ├─ interactions/             #   Interaction handlers
│  │  ├─ Interaction.py         #     Base interaction class
│  │  ├─ Lootable.py            #     Loot system
│  │  ├─ Vendor.py              #     NPC vendor
│  │  ├─ AbilityTrainer.py      #     Ability trainer
│  │  └─ DHD.py                 #     Dialing device
│  ├─ missions/                 #   Mission script implementations
│  │  ├─ MissionBase.py         #     Base mission script class
│  │  ├─ General/               #     General missions
│  │  ├─ SGC_W1/                #     SGC World 1 missions
│  │  ├─ Harset/                #     Harset zone missions
│  │  └─ Castle_CellBlock/      #     Castle cellblock missions
│  ├─ profiles/                 #   Profile data
│  └─ spaces/                   #   Space-specific logic
│
├─ common/                      # Shared code
│  ├─ __init__.py
│  ├─ Constants.py              #   Game constants
│  ├─ Config.py                 #   Configuration access
│  ├─ Logger.py                 #   Logging functions
│  ├─ Event.py                  #   Event system
│  └─ defs/                     #   Definition/resource loaders
│     ├─ Def.py                 #     DefMgr — central resource manager
│     ├─ Ability.py             #     Ability definitions
│     ├─ Item.py                #     Item definitions
│     ├─ Mission.py             #     Mission definitions
│     ├─ Sequence.py            #     Kismet sequence handling
│     └─ EventSet.py            #     Event set definitions
│
└─ profilehooks.py              # Performance profiling utility
```


## Common Pitfalls

### Python 3.4 Limitations

The embedded Python is 3.4.1. Many modern Python features are **not available**:

| Feature | Minimum Version | Status |
|---------|----------------|--------|
| f-strings | 3.6 | **Not available** — use `%` or `.format()` |
| `async`/`await` | 3.5 | **Not available** |
| Type hints | 3.5 | **Not available** |
| Dataclasses | 3.7 | **Not available** |
| Walrus operator (`:=`) | 3.8 | **Not available** |
| `match`/`case` | 3.10 | **Not available** |

```python
# DO:
msg = "Player %s has %d HP" % (self.playerName, self.hitPoints)
msg = "Player {} has {} HP".format(self.playerName, self.hitPoints)

# DON'T:
msg = f"Player {self.playerName} has {self.hitPoints} HP"  # SyntaxError!
```

### GIL Considerations

All Python code runs on the main service thread under the GIL. Long-running Python operations will block the entire service tick. Keep Python logic fast and use database async operations for heavy queries.

### Exposed Method Validation

Any method marked `<Exposed/>` can be called by the game client with arbitrary arguments. **Always validate inputs:**

```python
def invokeAbility(self, abilityId, targetId):
    # Validate ability exists and player has it
    ability = DefMgr.get('ability', abilityId)
    if ability is None:
        return

    # Validate target exists and is in range
    target = Atrea.findEntity(targetId)
    if target is None:
        return

    # Now safe to proceed
    self.abilityManager.activate(ability, target)
```

### Property Flag Mistakes

- Setting `CELL_PUBLIC` on a property with sensitive data (health modifiers, internal state) exposes it to all nearby clients
- Forgetting `<Persistent> true </Persistent>` means the property resets on server restart
- Using `ALL_CLIENTS` is expensive — it broadcasts to every connected client, not just nearby ones
- `PYTHON` type properties cannot be inspected by the client; use typed properties for anything the client needs to read

### Weak References

Use `weakref.ref()` when storing references to entities to avoid circular references that prevent garbage collection:

```python
import weakref

# Store a weak reference
self.owner = weakref.ref(player)

# Check if entity still exists
owner = self.owner()
if owner is not None:
    owner.client.onSomething()
```
