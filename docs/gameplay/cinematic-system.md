# Cinematic System (Kismet/Matinee Sequences)

> **Last updated**: 2026-03-02
> **Status**: ~60% implemented
> **Sources**: `python/cell/SGWSpawnableEntity.py`, `python/cell/AbilityManager.py`, `python/cell/RingTransporter.py`, `python/cell/SGWPlayer.py`, `python/cell/commands/Net.py`, `python/common/defs/Sequence.py`, `python/common/defs/EventSet.py`, `python/common/defs/RingTransporterRegion.py`, `entities/defs/SGWSpawnableEntity.def`, `db/resources.sql`

## Overview

Stargate Worlds uses Unreal Engine 3's **Kismet/Matinee** system for all cinematic presentation — camera movements, ring transport animations, stargate vortex effects, door opening sequences, console activations, holographic displays, ability VFX, and death animations. The client contains all the camera paths, particle effects, and audio cues baked into `.umap` tile files and `.upk` packages. The server's only job is to tell the client *when* to play *which* sequence, on *whom*, and *how the camera should behave*.

This is accomplished through a single RPC: `onSequence`. The server fires it; the client does all the visual work.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Sequence resource loading from DB | DONE | `Sequence.loadAll()`, `EventSet.loadAll()` |
| Cooked data export (PAK files) | DONE | `CookedDataKismetSeqEvent`, `CookedDataKismetSetEvent` |
| Entity event set assignment | DONE | `setEventSet()`, `onKismetEventSetUpdate` RPC |
| `playSequence()` base method | DONE | `SGWSpawnableEntity.playSequence()` — all entity types |
| Ability begin/end/interrupt sequences | DONE | `AbilityInstance` lifecycle triggers |
| Effect hit/pulse/init/remove sequences | DONE | `EffectInstance.doAction()`, `playPulseSequence()` |
| Stargate dialing sequences | DONE | `SGWPlayer.gateDialTimerExpired()`, `stargatePassed()` |
| Ring transport sequences | DONE | `RingTransporter.__beginTransport()`, `__allPlayersLoaded()` |
| Atrea script `Act_Sequence` node | DONE | Visual scripting compiles to `playSequence()` calls |
| Debug console commands | DONE | `net_seq`, `net_seqto`, `net_seqfrom` |
| Visibility safety sequence | DONE | `SEQUENCE_VisibilitySafety = 512` workaround |
| NVP parameter overrides | DONE | `sequences_nvp` table, mostly sound banks |
| Multi-player ring matinee | FIXME | Only first player gets the animation (see Known Issues) |
| DHD chevron lock animations | NOT IMPL | DHD events 6106-6112 exist in DB but aren't triggered during dialing |
| Stargate witness sequences | NOT IMPL | Gate sequences sent only to dialing player, not witnesses |
| Item equip/unequip sequences | NOT IMPL | `items_event_sets` has 2,767 entries but no code triggers them |
| Entity spawn/death/despawn sequences | PARTIAL | Death exists via Mob event set, spawn/despawn not triggered |
| Designer slot sequences (6000-6004) | NOT IMPL | 5 designer event types for custom triggers, unused |
| Channeled ability sequences | NOT IMPL | No channeled ability support yet |

## Architecture

The cinematic system follows a three-tier architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│ DATABASE                                                        │
│  sequences (1,973 rows) ──→ event_sets_sequences ──→ event_sets │
│  sequences_nvp (2,042 rows)                       (675 sets)    │
│  items_event_sets (2,767 rows)                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │ DB query at startup
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ SERVER (Python)                                                  │
│  Sequence / EventSet resource classes                            │
│  SGWSpawnableEntity.playSequence() ──→ onSequence RPC            │
│  AbilityManager: ability begin/end, effect pulse/hit             │
│  SGWPlayer: stargate dialing, gate crossing                      │
│  RingTransporter: teleport out/in                                │
│  Atrea scripts: Act_Sequence nodes ──→ playSequence() calls      │
└──────────────────────────────┬──────────────────────────────────┘
                               │ Mercury UDP / onSequence RPC
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ CLIENT (Unreal Engine 3)                                         │
│  Receives onSequence(seqId, source, target, viewType, ...)       │
│  Looks up KismetScriptName from cooked PAK data                  │
│  Triggers Kismet/Matinee sequence in the .umap level             │
│  Camera, particles, audio, animations all client-side            │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. At server startup, `Sequence.loadAll()` queries `resources.sequences` and `resources.sequences_nvp`, creating `Sequence` objects keyed by `sequence_id`.
2. `EventSet.loadAll()` queries `resources.event_sets` and `resources.event_sets_sequences`, grouping sequences into event sets keyed by `event_id`.
3. Both are serialized to cooked XML and packed into `.pak` files for client download (`CookedDataKismetSeqEvent` and `CookedDataKismetSetEvent`).
4. Each spawnable entity has a `kismetEventSetId` property (CELL_PUBLIC) that tells the client which event set to use for that entity.
5. When the server calls `playSequence()`, it sends `onSequence` to the entity's client and all witnesses (other players in AoI).
6. The client receives the `KismetEventSetSeqID`, looks up the `KismetScriptName` from cooked data, and fires the corresponding Kismet/Matinee node in the loaded `.umap` level.

---

## The `onSequence` RPC

The core client RPC that drives all cinematic playback. Defined in `SGWSpawnableEntity.def`:

```xml
<onSequence>
    <Arg> INT32 <ArgName>KismetEventSetSeqID</ArgName></Arg>
    <Arg> INT32 <ArgName>SourceID</ArgName></Arg>
    <Arg> INT32 <ArgName>TargetID</ArgName></Arg>
    <Arg> INT8  <ArgName>PrimaryTarget</ArgName></Arg>
    <Arg> FLOAT <ArgName>ImpactTime</ArgName></Arg>
    <Arg> ARRAY <of> NameValuePair </of> <ArgName>NameValuePairs</ArgName></Arg>
    <Arg> INT8  <ArgName>ViewType</ArgName></Arg>
    <Arg> INT32 <ArgName>InstanceId</ArgName></Arg>
</onSequence>
```

| Argument | Type | Purpose |
|----------|------|---------|
| `KismetEventSetSeqID` | INT32 | Sequence ID from `sequences` table — maps to a Kismet script name |
| `SourceID` | INT32 | Entity performing the action (ability caster, ring activator, etc.) |
| `TargetID` | INT32 | Entity being acted upon |
| `PrimaryTarget` | INT8 | 1 = this is the primary target, 0 = secondary (for multi-target effects) |
| `ImpactTime` | FLOAT | Timing offset for projectile impact synchronization |
| `NameValuePairs` | ARRAY | Runtime parameter overrides injected into the Kismet sequence |
| `ViewType` | INT8 | Camera control mode (`EKismetViewType`) |
| `InstanceId` | INT32 | Ability/effect instance ID for correlating visual with gameplay |

### Supporting RPCs

| RPC | Direction | Purpose |
|-----|-----------|---------|
| `onKismetEventSetUpdate(eventSetId)` | Server → Client | Update an entity's default event set (e.g., when equipping a weapon changes the animation set) |
| `PlayAllClientKismetSeq(eventType, eventSetId, sourceId, targetId)` | Cell method | Original BigWorld method that resolves event type to sequence ID and broadcasts. The emulator bypasses this by resolving in Python and calling `onSequence` directly. |

---

## Entity Definitions

### SGWSpawnableEntity.def Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `kismetEventSetId` | INT32 | CELL_PUBLIC | Default event set for this entity (abilities, death, spawn visuals) |
| `shouldSendKismet` | INT8 | CELL_PRIVATE | Whether this entity sends kismet sequences (default: 1) |

### SGWAbilityManager.def Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `effectSequenceId` | INT32 | CELL_PRIVATE | Currently playing effect sequence ID |

---

## EKismetViewType Enum

Controls how the client camera behaves during a sequence.

| Value | Name | Behavior |
|-------|------|----------|
| 0 | `KISMET_VIEW_Witness` | Camera locks for participants (default for combat sequences) |
| 1 | `KISMET_VIEW_NonWitness` | Camera locks for nearby non-participants |
| 2 | `KISMET_VIEW_Finish` | Finish/release a running camera lock |
| 3 | `KISMET_VIEW_EventInvoker` | Camera follows the entity that triggered the event (default for most non-combat) |
| 4 | `KISMET_VIEW_EventWitness` | Camera follows witnesses to the event |

**Usage patterns in the codebase:**
- `KISMET_VIEW_Witness` (0): Used by `AbilityManager.playSequence()` for all combat ability/effect sequences
- `KISMET_VIEW_EventInvoker` (3): Used by `SGWSpawnableEntity.playSequence()` as default, and explicitly by stargate and ring transport sequences
- The default in `playSequence()` is `KISMET_VIEW_EventInvoker` if no `viewType` is specified

---

## ESequenceEventType Enum

Every sequence has an `event_id` that categorizes what kind of game event triggers it. 66 defined types across 7 categories:

### Ability Events (1000-1007)

| Value | Name | Trigger |
|-------|------|---------|
| 1000 | `Ability_Begin` | Ability warmup starts — `AbilityInstance.launch()` |
| 1001 | `Ability_End` | Ability warmup completes — `AbilityInstance.afterWarmup()` |
| 1002 | `Ability_Interrupt` | Ability interrupted during warmup — `AbilityInstance.interrupt()` |
| 1003 | `Ability_Failed` | Ability failed to activate |
| 1004 | `Ability_ChannelBegin` | Channeled ability starts (NOT IMPLEMENTED) |
| 1006 | `Ability_ChannelFail` | Channeled ability fails (NOT IMPLEMENTED) |
| 1007 | `Ability_ChannelEnd` | Channeled ability completes (NOT IMPLEMENTED) |

Note: Value 1005 is skipped (no `Ability_ChannelInterrupt`).

### Effect Events (2000-2008)

| Value | Name | Trigger |
|-------|------|---------|
| 2000 | `Effect_Init` | Effect first applied — `EffectInstance.init()` |
| 2001 | `Effect_Removed` | Effect removed — `EffectInstance.remove()` |
| 2002 | `Effect_Hit_Normal` | Normal hit (QR result) |
| 2003 | `Effect_Hit_Crit` | Critical hit |
| 2004 | `Effect_Hit_Double_Crit` | Double critical hit |
| 2005 | `Effect_Hit_Glancing` | Glancing blow |
| 2006 | `Effect_Hit_Miss` | Miss |
| 2007 | `Effect_Pulse_Begin` | Effect pulse begins — `EffectInstance.pulse()` |
| 2008 | `Effect_Pulse_End` | Effect pulse ends / no result |

The `EffectInstance.resultEffects` dictionary maps QR result codes to the appropriate Effect_Hit_* sequence:

```python
# python/cell/AbilityManager.py:254-262
resultEffects = {
    RC_None:           Effect_Pulse_End,
    RC_Hit:            Effect_Hit_Normal,
    RC_Miss:           Effect_Hit_Miss,
    RC_Critical:       Effect_Hit_Crit,
    RC_DoubleCritical: Effect_Hit_Double_Crit,
    RC_Glancing:       Effect_Hit_Glancing
}
```

### Item Events (4000-4003)

| Value | Name | Trigger |
|-------|------|---------|
| 4000 | `Item_Equip` | Item equipped (NOT IMPLEMENTED — data exists in `items_event_sets`) |
| 4001 | `Item_Unequip` | Item unequipped (NOT IMPLEMENTED) |
| 4002 | `Item_Reload` | Weapon reloaded (NOT IMPLEMENTED) |
| 4003 | `Item_Use` | Item used (NOT IMPLEMENTED) |

The `items_event_sets` table (2,767 rows) maps item → ability → event set for item-specific animations but no server code reads this table.

### Entity Events (5000-5105)

| Value | Name | Trigger |
|-------|------|---------|
| 5000 | `Entity_Spawn` | Entity spawned |
| 5001 | `Entity_Death` | Entity died |
| 5002 | `Entity_Despawn` | Entity despawned |
| 5003 | `Entity_Alert` | Entity alerted (combat awareness) |
| 5005 | `Entity_CombatStateChanged` | Entered/exited combat |
| 5006 | `Entity_HealthMonitor` | Health threshold crossed |
| 5007-5011 | `Entity_Mission_*` | Mission lifecycle (new, step advance, objective unlock, complete) |
| 5012 | `Entity_Enemy_Aggro` | Enemy gained aggro on this entity |
| 5013 | `Entity_Enemy_Leash` | Enemy leashed (gave up chase) |
| 5014 | `Entity_Enemy_Death` | Enemy killed by this entity |
| 5015 | `Entity_Player_Level` | Player leveled up |
| 5016 | `Entity_FocusMonitor` | Focus threshold crossed |
| 5017-5018 | `Entity_Ally_Join/Leave` | Group member joined/left |
| 5100-5105 | `Entity_*_Enter/Exit` | Stance changes (Defensive, Conservative, Aggressive) |

Note: Value 5004 is skipped.

### Designer Events (6000-6004)

| Value | Name | Purpose |
|-------|------|---------|
| 6000-6004 | `Designer_1` through `Designer_5` | General-purpose designer slots for custom triggers |

These are used for zone-specific cinematics — door animations, console activations, holographic displays, mission-specific sequences. The Atrea visual scripting system references these via `Act_Sequence` nodes with specific sequence IDs.

### Stargate Events (6100-6113)

| Value | Name | Purpose |
|-------|------|---------|
| 6100 | `Stargate_MakeGate` | Gate vortex forms after dialing — triggered by `gateDialTimerExpired()` |
| 6101 | `Stargate_MakeGateFull` | Gate forms instantly (full animation) |
| 6102 | `Stargate_MakeGateNow` | Gate forms instantly (no animation) |
| 6103 | `Stargate_DestroyGate` | Gate vortex dissipates |
| 6104 | `Stargate_DestroyGateNow` | Gate destroyed instantly |
| 6105 | `Stargate_DialFailure` | Dialing failed |
| 6106-6112 | `Stargate_DHD1` through `DHD7` | Individual chevron lock animations |
| 6113 | `Stargate_CrossGate` | Player walks through the event horizon — triggered by `stargatePassed()` |

Each stargate event set contains exactly **14 sequences** (one per event type above). 14 zones have stargate event sets.

### Region Transport Events (8000-8001)

| Value | Name | Purpose |
|-------|------|---------|
| 8000 | `Region_Teleport_Out` | Ring transport departure animation — triggered by `RingTransporter.__beginTransport()` |
| 8001 | `Region_Teleport_In` | Ring transport arrival animation — triggered by `RingTransporter.__allPlayersLoaded()` |

### Undocumented Dialog VO Events (7000-7207)

The database contains sequences with event IDs in the 7xxx range that are NOT in the `ESequenceEventType` enum:

| Value | Purpose |
|-------|---------|
| 7000 | Volume duck down (audio) |
| 7001 | Volume duck up (audio) |
| 7100-7107 | Dialog voiceover play (primary channel, 8 variants) |
| 7200-7207 | Dialog voiceover play (secondary channel, 8 variants) |

These appear to be internal audio system events for dialog voiceover sequences.

---

## Database Schema

### `sequences` Table (1,973 rows)

```sql
CREATE TABLE sequences (
    sequence_id integer NOT NULL,  -- Primary key. This is KismetEventSetSeqID in the RPC.
    event_id    integer NOT NULL,  -- ESequenceEventType value (1000-8001+)
    kismet_script_name character varying(255) NOT NULL  -- Unreal Kismet node path
);
```

The `kismet_script_name` is a full Unreal Kismet node path within the map, e.g.:
- `SGC_W1.Main_Sequence.Prefabs.GLB-Stargate_Prefab_Seq` — SGC stargate
- `Harset.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq` — Harset ring transporter
- `KIS-SA_Beam_Source` — Generic ability beam VFX (source end)
- `KIS-SA_Melee_Source` — Generic melee attack VFX
- `Death` — Death animation

**169 unique Kismet script names** across all 1,973 sequences.

### `sequences_nvp` Table (2,042 rows)

```sql
CREATE TABLE sequences_nvp (
    sequence_id integer NOT NULL,    -- FK to sequences
    name        character varying(255) NOT NULL,  -- Parameter name
    value       character varying(255) NOT NULL   -- Parameter value
);
```

Name-value pairs injected into Kismet sequences at runtime. Almost all entries are `SoundBankName` values mapping to dialog voiceover audio files, e.g. `d_tollana/coppleman/MsM/ApartmentHunting_1_Vcaptcopplemann`.

### `event_sets` Table (675 rows)

```sql
CREATE TABLE event_sets (
    event_set_id integer NOT NULL,  -- Primary key
    name         character varying(200)  -- Human-readable name (often empty)
);
```

Groups sequences into logical sets. Each entity, stargate, or ring transporter references an event set. Named examples:
- `1025` — "Mob event set" (16 sequences — the default NPC animation set)
- `10002` — "Agnos" (14 stargate sequences)
- `10004` — "SGC_W1 Stargate" (14 stargate sequences)
- `10012` — "Tollana Stargate" (14 stargate sequences)
- `10000` — "Cellblock Ring fix" (2 ring transport sequences)

Many event sets (IDs 1060-1393) have empty names — these are ability/effect event sets assigned via `items_event_sets`.

### `event_sets_sequences` Table (1,958 rows)

```sql
CREATE TABLE event_sets_sequences (
    event_set_id integer NOT NULL,  -- FK to event_sets
    sequence_id  integer NOT NULL   -- FK to sequences
);
```

Many-to-many join table. An event set groups multiple sequences, each keyed internally by `event_id` for lookup.

### `items_event_sets` Table (2,767 rows)

```sql
CREATE TABLE items_event_sets (
    item_event_id integer NOT NULL,  -- PK
    item_id       integer NOT NULL,  -- FK to items
    ability_id    integer NOT NULL,  -- FK to abilities
    event_id      integer NOT NULL   -- FK to event_sets
);
```

Maps items to ability-specific event sets for item visual overrides (weapon-specific attack animations). Currently unused by server code.

---

## Python Implementation

### Sequence Resource (`python/common/defs/Sequence.py`)

```python
class Sequence(Resource):
    versioning = True
    def __init__(self, row, defMgr):
        self.seqId = row['sequence_id']
        self.eventId = row['event_id']
        self.kismetScriptName = row['kismet_script_name']
        self.nvps = {}

    def toXml(self):
        # Serializes to COOKED_KISMET_EVENT_SEQUENCE XML for client PAK delivery

    @staticmethod
    def loadAll(defs, defMgr):
        # SELECT * FROM resources.sequences
        # SELECT * FROM resources.sequences_nvp
```

Loaded as resource category `1`, cooked to `CookedDataKismetSeqEvent` PAK files.

### EventSet Resource (`python/common/defs/EventSet.py`)

```python
class EventSet(Resource):
    versioning = True
    def __init__(self, row, defMgr):
        self.id = row['event_set_id']
        self.instances = {}  # eventId -> Sequence

    def getSequence(self, eventId):
        return self.instances.get(eventId)

    @staticmethod
    def loadAll(defs, defMgr):
        # SELECT * FROM resources.event_sets
        # SELECT * FROM resources.event_sets_sequences
        # Groups: defs[event_set_id].instances[seq.eventId] = seq
```

Loaded as resource category `6`, cooked to `CookedDataKismetSetEvent` PAK files. Depends on `sequence` being loaded first.

### SGWSpawnableEntity.playSequence() (`python/cell/SGWSpawnableEntity.py:231-251`)

The primary entry point for all sequence playback:

```python
def playSequence(self, seqId, targetId=None, primaryTarget=1, time=None, nvps=None,
        viewType=None, instanceId=None, playOnSelf=True, playOnWitnesses=True):
    type = viewType or Atrea.enums.KISMET_VIEW_EventInvoker
    if self.client is not None and playOnSelf:
        self.client.onSequence(seqId, self.entityId, targetId or 0, primaryTarget,
            time or 0, nvps or [], type, instanceId or 0)
    if playOnWitnesses:
        self.witnesses.onSequence(seqId, self.entityId, targetId or 0, primaryTarget,
            time or 0, nvps or [], type, instanceId or 0)
```

Sends `onSequence` to both the entity's own client and all witnesses (other players within AoI distance).

### Entity Event Set Methods (`python/cell/SGWSpawnableEntity.py:209-375`)

```python
def setEventSet(self, eventSetId):       # Updates entity's event set + notifies client/witnesses
def getEventSet(self) -> EventSet:       # Returns current EventSet resource object
def getSequence(self, eventId) -> Sequence:  # Shortcut: entity.getSequence(Ability_Begin)
```

### Visibility Safety Workaround (`python/cell/SGWSpawnableEntity.py:158-167`)

```python
def setVisible(self, visible):
    if visible:
        self.client.onVisible(1)
        self.playSequence(Constants.SEQUENCE_VisibilitySafety, self.entityId, playOnWitnesses=False)
```

Sequence ID 512 (`SEQUENCE_VisibilitySafety`) is played when restoring entity visibility — a workaround for an SGW client bug where entities would remain invisible without this sequence nudge.

---

## Sequence Triggers

### Ability Lifecycle (`python/cell/AbilityManager.py`)

Abilities trigger sequences at three points:

1. **Begin** (line 621): When warmup starts
   ```python
   beginSeq = self.ability.getSequence(Atrea.enums.Ability_Begin)
   self.manager.playSequence(beginSeq.seqId, ent.entityId, 0, self.targetId)
   ```

2. **Interrupt** (line 653): If cancelled during warmup
   ```python
   interruptSeq = self.ability.getSequence(Atrea.enums.Ability_Interrupt)
   self.manager.playSequence(interruptSeq.seqId, invokerId, 0, self.targetId)
   ```

3. **End** (line 671): When warmup completes and effects are applied
   ```python
   endSeq = self.ability.getSequence(Atrea.enums.Ability_End)
   self.manager.playSequence(endSeq.seqId, self.entity.entityId, 0, self.targetId)
   ```

The `AbilityManager.playSequence()` method (line 879) uses `KISMET_VIEW_Witness` for all combat sequences.

### Effect Lifecycle (`python/cell/AbilityManager.py`)

Effects trigger sequences at four points via `doAction()` (line 445):

1. **Init** (line 300): `doAction("onEffectInit", Effect_Init)` — when effect first applies
2. **Pulse Begin** (line 330): `doAction("onPulseBegin", Effect_Pulse_Begin)` — each tick
3. **Pulse End** (line 332): `doAction("onPulseEnd", Effect_Pulse_End)` — each tick ends
4. **Remove** (line 345): `doAction("onEffectRemoved", Effect_Removed)` — when effect expires or is dispelled

Additionally, `playPulseSequence()` (line 304) maps the QR result code to the appropriate `Effect_Hit_*` event type and plays the corresponding sequence (e.g., critical hits get a different visual than misses).

The `doAction()` method (line 445) also plays the effect's own sequence after executing the script action:
```python
sequence = self.effect.getSequence(eventId)
if sequence:
    self.manager.playSequence(sequence.seqId, self.invokerId, 0)
```

### Stargate Sequences (`python/cell/SGWPlayer.py`)

Two points in the gate travel flow:

1. **Gate opens** (line 2112): After 4-second dial timer expires
   ```python
   self.client.onSequence(
       self.dialingStargate.eventSet.getSequence(Atrea.enums.Stargate_MakeGate).seqId,
       self.entityId, self.entityId, 1, Atrea.getGameTime(), [],
       Atrea.enums.KISMET_VIEW_EventInvoker, 0)
   ```

2. **Player crosses** (line 2124): When player walks into the event horizon
   ```python
   self.client.onSequence(
       self.dialingStargate.eventSet.getSequence(Atrea.enums.Stargate_CrossGate).seqId,
       self.entityId, self.entityId, 1, Atrea.getGameTime(), [],
       Atrea.enums.KISMET_VIEW_EventInvoker, 0)
   ```

Note: Both send only to `self.client`, not witnesses. Other players don't see the stargate animation — this is a known gap.

### Ring Transport Sequences (`python/cell/RingTransporter.py`)

Full state machine with 8 states and timed transitions:

```
STATE_IDLE → STATE_SEND_WAIT → STATE_SEND_WARMUP → STATE_REMOTE_LOAD_WAIT →
    ... (remote side) → STATE_RECV_WAIT → STATE_RECV_WARMUP →
    STATE_REMOTE_LOAD_WAIT → STATE_REMOTE_WARMUP → STATE_COOLDOWN → STATE_IDLE
```

**Teleport Out** (line 192-197):
```python
sequence = self.region.eventSet.getSequence(Atrea.enums.Region_Teleport_Out)
player.playSequence(sequence.seqId, player.entityId,
    viewType=Atrea.enums.KISMET_VIEW_EventInvoker)
```

**Teleport In** (line 229-234):
```python
sequence = self.region.eventSet.getSequence(Atrea.enums.Region_Teleport_In)
player.playSequence(sequence.seqId, player.entityId,
    viewType=Atrea.enums.KISMET_VIEW_EventInvoker)
```

**Timing details** (all times relative to destination selection):

| Time | Event | Code |
|------|-------|------|
| T+0.0s | `Region_Teleport_Out` sequence plays, movement locked | `__beginTransport()` |
| T+3.5s | Player becomes invisible | `__hideTimerExpired()` |
| T+4.0s | Teleport to destination, load destination map | `__warmupTimerExpired()` |
| T+load | `Region_Teleport_In` sequence plays at destination | `__allPlayersLoaded()` |
| T+load+3.0s | Player becomes visible | `__remoteWarmupTimerExpired()` |
| T+load+5.5s | Movement unlocked, transport complete | `__cooldownTimerExpired()` |

### Atrea Visual Scripts (`data/scripts/`)

The Atrea visual scripting system compiles XML node graphs to Python. The `Act_Sequence` node plays sequences:

```xml
<!-- data/scripts/spaces/SGC_W1.script -->
<Node Id="26" Ref="Act_Sequence" X="-771" Y="129">
    <Property Name="Comment" Value="Open Hammond's door"/>
    <Property Name="Sequence Id" Value="10005"/>
    <Property Name="View Type" Value="0"/>
    <Property Name="Visible to Source" Value="true"/>
    <Property Name="Visible to Target" Value="true"/>
    <Property Name="Visible to Witnesses" Value="true"/>
    <Port Name="Source" Flags="2"/>
    <Port Name="Target" Flags="0"/>
</Node>
```

This compiles to a `playSequence(10005, ...)` call. Script files with sequence references include:
- `spaces/SGC_W1.script` — Hammond's office door, stargates, briefing room doors
- `spaces/CellBlock.script` — Ring transporter fixes
- `missions/Castle_CellBlock/ArmYourself.script` — Ring transport in tutorial
- `missions/Castle_CellBlock/329.script` — Prisoner cell door
- `missions/Castle_CellBlock/FindAmbernol.script` — Omega Site beam teleporter
- `missions/SGC_W1/SecurityOffice.script` — Beta Site stargate

---

## Content Inventory

### Stargate Sequences (14 zones)

Each zone with a stargate has a full 14-sequence event set:

| Event Set | Zone | Named |
|-----------|------|-------|
| 10002 | Agnos | "Agnos" |
| 10003 | Lucia | "Lucia" |
| 10004 | SGC_W1 | "SGC_W1 Stargate" |
| 10005 | Ihpet_Crater (SGU) | "Ihpet Crater (SGU) Stargate" |
| 10006 | Ihpet_Crater (Praxis) | "Ihpet Crater (Praxis) Stargate" |
| 10007 | Menfa (SGU) | "Men'fa (SGU) Stargate" |
| 10008 | Menfa (Praxis) | "Men'fa (Praxis) Stargate" |
| 10009 | Beta_Site_E1 | "Beta Site E1 Stargate" |
| 10010 | Omega_Site | "Omega Site Stargate" |
| 10011 | The Castle | "The Castle Stargate" |
| 10012 | Tollana | "Tollana Stargate" |
| 10013 | Dakara_E1 | "Dakara E1 Stargate" |

All 14 sequences per gate reference the same Kismet script path (`GLB-Stargate_Prefab_Seq`) prefixed by zone name, so each gate plays its own zone-specific Matinee camera path.

### Ring Transport Sequences (6 zones)

| Zone | Ring Count | Kismet Script Pattern |
|------|------------|----------------------|
| Castle_CellBlock | 1 | `GLB-RingTransporterBase_TC00_Pf0_Seq` |
| Lucia | 11 | `GLB-RingTransporter00_Pf0_Seq` through `_10` |
| Harset | 5 | `GLB-RingTransporterBase_TC00_Pf0_Seq` through `_TC04` |
| Tollana | 3 | `GLB-RingTransporterBase_TC00_Pf0_Seq` through `_TC02` |
| Beta_Site | 1+ | `GLB-RingTransporterBase_TC00_Pf0_Seq` |
| Omega_Site | 1+ | `GLB-RingTransporterBase_TC00_Pf0_Seq` |

### Asgard Beam Teleporter Sequences (3 zones)

| Zone | Beam Points | Kismet Script |
|------|-------------|---------------|
| Omega_Site | 5 | `ASG-TeleportBase00` through `04` |
| Beta_Site | 1+ | `ASG-TeleportBase00` |
| Pertho_Genetics_Lab | 1+ | `ASG-TeleportBase00` |

### Zone-Specific Cinematics

| Zone | Sequences | Purpose |
|------|-----------|---------|
| Castle_CellBlock | `Prisoner329CellDoor` | Prison cell door opens during mission 329 |
| Castle_CellBlock | `StraegisAttack` | Straegis combat cinematic |
| Castle_CellBlock | `StasisBlockDoubleDoors` | Stasis block double doors |
| Castle_CellBlock | `TakeCoverIndicator` | Cover system tutorial indicator |
| SGC | `BriefingRoomBlastDoors` | SGC briefing room blast doors opening |
| SGC | `GenHammondOfficeDoor` | General Hammond's office door |
| SGC | `CartersLabDoors` | Carter's lab doors |
| SGC | `Incoming_Traveller` | Incoming wormhole alarm sequence |
| SGC | `Code9` | Code 9 base alert |
| SGC | `General'sPhone` | Phone ringing in General's office |
| SGC | `SGC_Door03` | Generic SGC door |
| Pertho_Genetics_Lab | `Databank_01`, `_02`, `_03` | Data console activation (holographic display) |
| Pertho_Genetics_Lab | `Containment_Field` | Containment field toggle |
| Pertho_Genetics_Lab | `Security_Field` | Security field activation |
| Pertho_Genetics_Lab | `Matter_Generator` | Matter generator console |
| Pertho_Genetics_Lab | `World_Projection` | Holographic world projection display |
| Temple | `GA-Door00` sequence family | Ancient door animations |
| Temple | `TankExplosion` | Tank explosion cinematic |
| Egypt | `AnatSymbiote` | Anat symbiote cinematic |
| Dakara | `GA-Hatak00` | Ha'tak ship sequences |

### Client Cinematic Packages (16 `.upk` files)

Pre-rendered or scripted cinematics stored as Unreal packages:

| Package | Content |
|---------|---------|
| `Cine-Intro_Commando.upk` | Commando faction intro cinematic |
| `Cine-Intro_Jaffa.upk` | Jaffa faction intro |
| `Cine-Intro_Soldier.upk` | Soldier faction intro |
| `Cine-Intro_Scientist.upk` | Scientist faction intro |
| `Cine-Intro_Asgard.upk` | Asgard faction intro |
| `Cine-Intro_Goauld.upk` | Goa'uld faction intro |
| `Cine-CellBlock.upk` | Castle CellBlock story cinematic |
| `Cine-SGC.upk`, `Cine-SGC_01.upk` | SGC story cinematics |
| `KismetAudioAssets.upk` | Audio assets tied to Kismet sequences |
| + 6 others | Additional faction/zone cinematics |

### Default Mob Event Set

Event set **1025** ("Mob event set") contains 16 sequences and is the default animation set for all NPCs. It covers entity spawn, death, despawn, and the base combat ability animations.

---

## Debug Console Commands

Three debug commands for testing sequences in-game (`python/cell/commands/Net.py`):

| Command | Arguments | Description |
|---------|-----------|-------------|
| `net_seq` | `<sequenceId> [viewType]` | Play sequence on targeted entity (source = target = entity) |
| `net_seqto` | `<sequenceId> [viewType]` | Play from player to targeted entity |
| `net_seqfrom` | `<sequenceId> [viewType]` | Play from targeted entity to player |

Default `viewType` is `KISMET_VIEW_EventInvoker` (3) for all three commands.

---

## Configuration

| Constant | Value | Location | Purpose |
|----------|-------|----------|---------|
| `MAX_KISMET_VIEW_DISTANCE` | 300.0 | `python/common/Config.py:12` | Maximum distance for sequence visibility |
| `SEQUENCE_VisibilitySafety` | 512 | `python/common/Constants.py:37` | Special sequence ID for visibility bug workaround |

---

## Known Issues

1. **Ring transport multi-player bug**: Only the first player in the ring region gets the Matinee animation. Playing it for each player messes up the ring model animation because the Matinee sequence controls a shared world object (the ring prop). Marked as `FIXME` in `RingTransporter.py:193-194`.

2. **Stargate sequences not sent to witnesses**: `SGWPlayer.gateDialTimerExpired()` and `stargatePassed()` send `onSequence` only to `self.client`, not to `self.witnesses`. Other players in AoI don't see the gate open or the player walk through.

3. **No DHD chevron animations**: The 7 chevron lock events (`DHD1`-`DHD7`, values 6106-6112) have sequences defined in the DB for every stargate but are never triggered. The dialing flow goes directly from `beginDialing()` → 4-second timer → `Stargate_MakeGate`, skipping the per-chevron animation.

4. **Item event sets unused**: 2,767 `items_event_sets` entries mapping items to weapon-specific ability animations exist in the DB but no server code reads this table or triggers item-specific sequences.

5. **No gate destroy sequence on cancel**: When canceling a dial, the code fires a `stargate::destroy` event but doesn't play the `Stargate_DestroyGate` sequence.

---

## Missing Features

| Feature | Difficulty | Notes |
|---------|------------|-------|
| DHD chevron lock animations | LOW | Fire `DHD1`-`DHD7` sequences during dial with ~0.5s delays |
| Stargate witness visibility | LOW | Change `self.client.onSequence` to `self.playSequence()` in gate code |
| Gate destroy sequence | LOW | Add `playSequence(Stargate_DestroyGate)` in `cancelDialing()` |
| Item equip/unequip VFX | MEDIUM | Read `items_event_sets`, trigger Item_Equip/Unequip on equipment change |
| Multi-player ring animations | MEDIUM | Need to instance the Matinee sequence or sync to a single play |
| Entity spawn/despawn sequences | LOW | Fire Entity_Spawn/Entity_Despawn from entity lifecycle |
| Entity stance change sequences | LOW | Fire Entity_*_Enter/Exit when combat stance changes |
| Mission milestone sequences | LOW | Fire Entity_Mission_* from mission advancement code |
| Channeled ability sequences | MEDIUM | Requires channeled ability system first |
| Designer slot triggers | VARIES | Zone-specific — wire up per content requirement |

---

## Related Documents

- [Effect System](effect-system.md) — Effects that trigger sequences via `EF_SequenceOn*` flags
- [Ability System](ability-system.md) — Ability lifecycle that drives sequence playback
- [Gate Travel](gate-travel.md) — Stargate dialing and zone transitions
- [Combat System](combat-system.md) — Combat sequences for abilities and effects
- [Data-Driven Content Engine](../architecture/data-driven-content-engine.md) — How sequences integrate with the proposed content engine
- [Gap Analysis](../gap-analysis.md) — System-level feature completeness tracking
