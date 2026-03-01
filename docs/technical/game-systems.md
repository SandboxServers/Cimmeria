# Game Systems

Overview of all major game systems identified in sgw.exe, organized by the network message patterns and string references that reveal them.

> **Note:** Many of these systems have been confirmed working in live gameplay testing. The combat system, mission system, inventory, spawning, dialog/interactions, entity aggression/threat, and client-hinted regions have all been verified end-to-end in the Castle Cellblock zone.

## Combat System

### Core Architecture
- **CombatQueue** class (source: `CombatQueue.cpp` at 0x019eac40)
  - Subscribes to `Event_NetIn_onEffectResults` — processes incoming combat results
  - Uses `Event_Cache_ElementReady<Ability>` / `Event_Cache_ElementError<Ability>` — async ability data loading
  - Effect confirmation via `Event_NetOut_ConfirmEffect`

### Combat Messages
| Direction | Message | Purpose |
|-----------|---------|---------|
| S→C | `onEffectResults` | Damage/heal/status results |
| S→C | `onSequence` | Combat animation sequence |
| S→C | `onLOSResult` | Line of sight check result |
| S→C | `onMeleeRangeUpdate` | Melee range notification |
| S→C | `onAggressionOverrideUpdate` | Forced aggression |
| S→C | `onThreatenedMobsUpdate` | Threat list |
| C→S | `UseAbility` | Fire ability |
| C→S | `useAbilityOnGroundTarget` | AoE targeting |
| C→S | `ConfirmEffect` | Confirm effect received |
| C→S | `SetCrouched` | Cover stance |
| C→S | `TestLOS` | Request LOS check |

### Cover System
- `ChangeCoverWeight` / `ChangeCoverStanceWeight` — Adjustable cover mechanics
- `RegenerateCoverLinks` — Rebuild cover graph
- Cover links visible via `/showcover` debug command

## Ability System

### Architecture
- Abilities loaded asynchronously from cooked data
- Ability trees with training points
- Racial paradigm abilities
- Auto-cycle abilities

### Messages
| Direction | Message | Purpose |
|-----------|---------|---------|
| S→C | `KnownAbilitiesUpdate` | Full ability list |
| S→C | `AbilityTreeInfo` | Ability tree structure |
| C→S | `TrainAbility` | Learn ability |
| C→S | `ResetAbilities` | Reset all |
| C→S | `RespecAbility` | Respec single |

## Pet System

Companion/pet abilities with stance management:
| Direction | Message | Purpose |
|-----------|---------|---------|
| S→C | `PetAbilities` | Pet ability list |
| S→C | `PetStances` | Available stances |
| S→C | `PetStanceUpdate` | Stance changed |
| C→S | `PetInvokeAbility` | Pet ability use |
| C→S | `PetChangeStance` | Change pet stance |
| C→S | `PetAbilityToggle` | Toggle pet ability |

## Stargate System

The signature feature — functional Stargates for zone travel.

### World Data
| String | Address | Purpose |
|--------|---------|---------|
| `worldStargateList` | 0x019d8cd8 | All gates in zone |
| `knownStargateList` | 0x019d8cec | Player's known addresses |
| `hiddenStargateList` | 0x019d8d00 | Undiscovered gates |

### Cooked Data
- `CookedDataStargates.pak` — Pre-processed gate definitions
- `CookedData::StargateType` — Gate type data structure (RTTI at 0x01e24ee0)
- `COOKED_STARGATE` flag at 0x019bb7b4

### Gate Travel Classes
| Class | RTTI Address | Purpose |
|-------|-------------|---------|
| `GateTravel` | (via callbacks) | Gate travel manager |
| `USeqEvent_Stargate` | 0x01dc5d58 | Kismet stargate event |
| `Event_World_DialStargateAddress` | 0x01db5b68 | Dial event |
| `Event_World_StargateEvent` | 0x01db5bec | General gate event |

### Network Messages
| Direction | Message | Purpose |
|-----------|---------|---------|
| S→C | `setupStargateInfo` | Initial gate data |
| S→C | `updateStargateAddress` | Gate address discovered |
| S→C | `StargateRotationOverride` | DHD chevron animation |
| S→C | `StargateTriggerFailed` | Gate activation failed |
| S→C | `onStargatePassage` | Player traveled through gate |
| S→C | `onDisplayDHD` | Show DHD interface |
| S→C | `onDHDReply` | DHD interaction result |
| S→C | `onRingTransporterList` | Ring transporter list |
| C→S | `GiveStargateAddress` | GM: give address |
| C→S | `RemoveStargateAddress` | GM: remove address |
| C→S | `onDialGate` | Player dials gate |
| C→S | `SetRingTransporterDestination` | Set ring destination |
| C→S | `DHD` | DHD interaction |

### Gate Travel Flow (Reconstructed from callbacks)
1. Server sends `setupStargateInfo` with world/known/hidden lists
2. Player approaches gate → `onDisplayDHD` shows DHD interface
3. Player dials address → `onDialGate` sent to server
4. Server validates → `StargateRotationOverride` for chevron animations
5. Success: `onStargatePassage` triggers map transition
6. Failure: `StargateTriggerFailed` with error

## Inventory System

### Containers
- Multiple container types (personal, vault, team vault, command vault, org vault)
- Item movement between containers
- Cash (currency) tracking

### Store System
- NPC stores with buy/sell/buyback
- Item repair and recharge services

### Messages (see [network-messages.md](network-messages.md) for complete list)
Key patterns: `onContainerInfo`, `onUpdateItem`, `onRemoveItem`, `CashChanged`, `LootDisplay`, `onStoreOpen/Close/Update`

## Mission System

### Architecture
- Mission assignment, tracking, and completion
- Step-based progression with objectives
- Mission sharing between team members
- Reward selection
- Mission history

### Messages
Key patterns: `onMissionUpdate`, `onStepUpdate`, `onObjectiveUpdate`, `MissionOffer`, `MissionRewards`

## Organization System

### Hierarchy
Organizations appear to support multiple tiers:
- **Squad** — Small group (5-6 players)
- **Team** — Mid-size group
- **Command** — Large organization (guild equivalent)

### Features
- Rank system with customizable rank names and permissions
- MOTD (Message of the Day)
- Member notes and officer notes
- Organization cash/bank
- Organization XP
- PvP organization support
- Invitation system with accept/decline

### Messages (15 Organization variants)
`onOrganizationInvite`, `onOrganizationJoined`, `onOrganizationLeft`, `onOrganizationRosterInfo`, `onOrganizationNameUpdate`, `onOrganizationMOTDUpdate`, `onOrganizationRankUpdate`, `onOrganizationRankNameUpdate`, `onOrganizationNoteUpdate`, `onOrganizationOfficerNoteUpdate`, `onOrganizationCashUpdate`, `onOrganizationExperienceUpdate`, `onMemberJoinedOrganization`, `onMemberLeftOrganization`, `onMemberRankChangedOrganization`

## Crafting & Disciplines

### Crafting Actions
| Command | Purpose |
|---------|---------|
| `Craft` | Create item from recipe |
| `Alloy` | Combine materials |
| `Research` | Learn new recipe |
| `ReverseEngineer` | Deconstruct for knowledge |
| `RespecCraft` | Respec crafting skills |
| `SpendAppliedSciencePoint` | Spend science point |

### Disciplines
- Discipline system (class specializations)
- Respec support
- Racial paradigm levels

### Resources
- **Naqahdah** — Primary crafting currency/resource (from Stargate lore)
- Applied Science Points — Crafting research currency
- Blueprints — Craftable item templates
- Training Points — Ability/skill training

## Black Market (Auction House)

Player-to-player auction system:
| Direction | Message | Purpose |
|-----------|---------|---------|
| S→C | `BMAuctions` | Auction listings |
| S→C | `BMAuctionUpdate` | Auction state change |
| S→C | `BMAuctionRemove` | Auction ended |
| S→C | `BMError` | Error message |
| S→C | `BMOpen` | Market opened |
| C→S | `BMCreateAuction` | List item |
| C→S | `BMCancelAuction` | Cancel listing |
| C→S | `BMPlaceBid` | Place bid |
| C→S | `BMSearch` | Search listings |

## Mail System

Full in-game mail with attachments and COD:
| Direction | Message | Purpose |
|-----------|---------|---------|
| S→C | `onMailHeaderInfo` | Mail list |
| S→C | `onMailHeaderRemove` | Mail deleted |
| S→C | `onMailRead` | Mail body |
| S→C | `onSendMailResult` | Send confirmation |
| C→S | `SendMailMessage` | Send mail |
| C→S | `RequestMailHeaders` | Fetch headers |
| C→S | `RequestMailBody` | Read mail |
| C→S | `DeleteMailMessage` | Delete |
| C→S | `ArchiveMailMessage` | Archive |
| C→S | `ReturnMailMessage` | Return to sender |
| C→S | `TakeItemFromMailMessage` | Take attachment |
| C→S | `TakeCashFromMailMessage` | Take currency |
| C→S | `PayCODForMailMessage` | Pay COD |

## Minigame System

Extensive minigame framework:
- Registration/matchmaking system
- Spectator support
- Helper system
- Contact-based minigame access
- Instance management
- Call/accept/decline flow

13 unique `Event_NetIn_` minigame messages, 15 `Event_NetOut_` variants, 4 debug commands.

## Dueling System

PvP duel system:
- Challenge/accept/decline flow
- Entity tracking (duel participants)
- Forfeit option
- Duel area management

## Trading System

Direct player-to-player trading:
- Request → Propose → Lock → Confirm flow
- State tracking per trade session

## Contact List System

Friend/ignore list management:
- Multiple named lists per player
- Add/remove members
- List flags (online notification, etc.)
- Create/delete/rename lists

## Space Queue System

Instanced space content queue:
- Queue → Ready → Enter flow
- Status polling
- Strike team integration

## Entity System

### BigWorld Integration
- `ABigWorldEntity` (RTTI at 0x01dc873c) — UE3 actor wrapping BigWorld entity
- `UBigWorldInfo` (RTTI at 0x01dcd6c4) — BigWorld configuration info
- Source: `BigWorldEntity.cpp` at 0x018cacb8
- Config: `game-ini:Engine.BigWorldInfo.DefaultBigWorld` at 0x018f1230

### Entity Lifecycle
1. `onRemoteEntityCreate` — Entity spawned in client view
2. `onEntityMove` / `onRemoteEntityMove` — Position updates
3. `onVisible` — Entity enters visibility range
4. `onRemoteEntityRemove` — Entity leaves client view

### Entity Properties
- Entity flags, properties, tints
- Being appearance (full visual data)
- Name/ID updates
- Static mesh name updates
- Archetype/faction/alignment

### Interaction Type Bitmask (`EInteractionNotificationType`)

Entity interaction types are controlled by a UINT64 bitmask that determines which visual indicators the client shows:

| Bits | Purpose |
|------|---------|
| 1-21 | NPC types (banker, vendor, trainer, minigames, etc.) |
| 22-25 | A-Story mission states (pending, available, active, turn-in) |
| 26-29 | Non-A-Story mission states |
| 30 | `INT_MissionWorldObject` — quest item outline glow |
| 31 | `INT_MissionWaypoint` |
| 32 | `INT_DrossPile` |

Scripts set these dynamically via `setInteractionType()`. If an entity template has `interaction_type=0`, the script must set the correct bits or the entity will lack its visual quest indicator.

## Event Signal System (CME Framework)

The client uses a publish-subscribe event system:
- **`CME::EventSignal`** — Central event bus
- **`CME::EventSignal::TypedEmitInfo<T>`** — Type-safe event emission
- **`CME::EventSignal::MemberCallback<...>`** — Object method subscriptions
- **`CME::EventSignal::CallbackImpl<T>`** — Callback implementations

All network messages, slash commands, and game events flow through this system.

## World Events

| Event Struct | RTTI Address | Purpose |
|-------------|-------------|---------|
| `Event_World_DialStargateAddress` | 0x01db5b68 | Stargate dial |
| `Event_World_StargateEvent` | 0x01db5bec | General stargate |
| `Event_Sys_FrameStart` | (via callback) | Per-frame tick |
| `Event_Cache_ElementReady<Ability>` | (via callback) | Ability data loaded |
| `Event_Cache_ElementError<Ability>` | (via callback) | Ability load error |
