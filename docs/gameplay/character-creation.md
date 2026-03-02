# Character Creation

## Overview

Character creation is handled by `python/base/Account.py` and is one of the more complete systems in the emulator. It validates input, creates database records, assigns starting equipment and abilities, and manages the full flow from the character list screen through entering the world.

The `Account` entity acts as the persistent session anchor between login and world entry. All character management operations are routed through it.

---

## Account Entity

Defined in `entities/defs/Account.def`.

### Properties

| Property | Scope | Description |
|---|---|---|
| `characterList` | BASE | Cached list of characters for this account |
| `activePlayerID` | BASE | Player ID currently in-world, or 0 if none |

### Client Methods (server -> client)

| Method | Description |
|---|---|
| `onCharacterList` | Sends the character roster to the client |
| `onCharacterCreateFailed` | Reports creation failure with error code |
| `onCharacterVisuals` | Sends equipment visual data for the character preview |
| `onCharacterLoadFailed` | Reports failure when entering the world |

### Exposed Base Methods (client -> server)

| Method | Description |
|---|---|
| `logOff` | Disconnects the client session |
| `createCharacter` | Creates a new character |
| `playCharacter` | Enters the world with a selected character |
| `deleteCharacter` | Deletes a character from the account |
| `requestCharacterVisuals` | Fetches item visuals for character preview |
| `onClientVersion` | Handles client resource version negotiation |

---

## Character List Flow

When the account entity attaches to a controller (client connects and authenticates), it immediately loads the character roster.

### Steps

1. `attachedToController()` runs a query against the database:
   ```sql
   SELECT * FROM sgw_player WHERE account_id = N
   ```

2. `sendCharacterList()` formats the result rows into a `CharacterInfoList` structure. Each entry includes:
   - `playerId`
   - `name`
   - `level`
   - `archetype`
   - `alignment`
   - `gender`
   - `bodyset`
   - `components`
   - `skinColorId`

   Before sending, it checks `ChannelManager.isPlayerOnline()` to detect duplicate logins. If the character is already active in-world, the entry is flagged accordingly.

3. `requestCharacterVisuals(playerId)` is called by the client when the player highlights a character in the UI. It lazy-loads item visuals by querying `sgw_inventory` for equipment slots and the active bandolier, then calls `onCharacterVisuals` to push the data to the client for the preview render.

---

## Character Creation Flow

Entry point: `createCharacter(name, extraName, charDefId, visualChoices, skinTintColorId)`

### Validation Steps

1. **Validate character definition** - Calls `DefMgr.get('character_creation', charDefId)`. If the `charDefId` does not exist in the resource data, creation fails immediately with `onCharacterCreateFailed`.

2. **Validate visual choices** - Calls `charDef.getAllChoices()` and checks that each submitted visual group choice is valid for the selected definition. Invalid choices cause an early failure.

3. **Validate name uniqueness** - Runs:
   ```sql
   SELECT player_id FROM sgw_player WHERE name = 'N'
   ```
   If any row is returned, creation fails with a name-taken error.

4. **Validate skin tint** - Checks the submitted `skinTintColorId` against the `Constants.SKIN_TINTS` list. An unrecognized tint ID causes failure.

### Database Writes

5. **Insert player row** - Inserts into `sgw_player` with values sourced from the character definition and the account:
   - `alignment` from `charDef`
   - `archetype` from `charDef`
   - `gender` from `charDef`
   - Starting position (hardcoded default or pulled from `charDef`)
   - Body component list from `visualChoices`
   - Starting ability list from `charDef`
   - `access_level` inherited from the parent account record

6. **Insert starting items** - Iterates the starting equipment list from `charDef` and inserts each item into `sgw_inventory`. Slot placement follows the `BagFillOrder` priority rules.

### Completion

7. On success, calls `sendCharacterList()` to refresh the client's roster display with the new character included.

---

## Archetypes

Eight archetypes are defined in `resources.archetypes`, split between two factions.

| ID | Name | Alignment |
|----|------|-----------|
| 0 | Soldier | SGC |
| 1 | Commando | SGC |
| 2 | Scientist | SGC |
| 3 | Archeologist | SGC |
| 4 | Asgard | System Lords |
| 5 | Goa'uld | System Lords |
| 6 | Sholva | System Lords |
| 7 | Jaffa | System Lords |

Each archetype definition includes:

- **Base stats:** coordination, engagement, fortitude, morale, perception, intelligence
- **Derived stats:** base health, base focus, health per level, focus per level
- **Ability trees:** three trees per archetype, referenced by ID

The `charDef` resource determines which archetype a given character definition uses. The archetype record is stored in `sgw_player.archetype` as an integer ID.

The `extraName` parameter supports Asgard-style compound naming but currently has an outstanding TODO regarding whether that field should be removed.

---

## Playing a Character

Entry point: `playCharacter(playerId)`

1. Validates that the requested `playerId` is owned by this account (queries `sgw_player` for `account_id` match).
2. Checks `ChannelManager.isPlayerOnline()` to prevent duplicate world entry.
3. Selects the entity class based on `access_level`:
   - Standard players create an `SGWPlayer` entity.
   - Accounts with elevated access create an `SGWGmPlayer` entity.
4. Calls `Atrea.createCellEntity()` to spawn the player entity into the game world at their stored position and space.

On failure, calls `onCharacterLoadFailed` with an error code.

---

## Deleting a Character

Entry point: `deleteCharacter(playerId)`

Runs a DELETE with an ownership check:

```sql
DELETE FROM sgw_player WHERE player_id = N AND account_id = M
```

Foreign key cascade rules handle cleanup of dependent records:

| Table | Cascade Behavior |
|---|---|
| `sgw_inventory` | DELETE (removes all items) |
| `sgw_mission` | DELETE (removes all mission state) |
| `sgw_gate_mail.sender_id` | SET NULL (preserves mail records) |

There is no soft-delete or recovery mechanism. Deletion is immediate and permanent.

---

## Client Version / Resource Sync

Entry point: `versionInfoRequest(categoryId, version)` (exposed via `onClientVersion`)

The client sends its locally cached version number for each resource category. The server compares against the current cooked data version and sends a diff if the client is out of date.

Resource categories handled:

| Category |
|---|
| `world_info` |
| `stargate` |
| `container` |
| `blueprint` |
| `applied_science` |
| `discipline` |
| `racial_paradigm` |
| `interaction` |

---

## Database Schema

```sql
CREATE TABLE sgw_player (
    player_id           SERIAL PRIMARY KEY,
    account_id          integer REFERENCES account(account_id),
    name                character varying(50) UNIQUE NOT NULL,
    level               integer DEFAULT 1 NOT NULL,         -- range: 0-20
    alignment           integer DEFAULT 0 NOT NULL,         -- range: 0-5
    archetype           integer DEFAULT 0 NOT NULL,         -- range: 0-8
    gender              integer DEFAULT 1 NOT NULL,         -- range: 1-3
    pos_x               real,
    pos_y               real,
    pos_z               real,
    world_location      character varying(30),
    bodyset             character varying(50),
    components          character varying(255)[],
    naquadah            integer DEFAULT 0,
    exp                 integer DEFAULT 0,
    abilities           integer[] DEFAULT '{}',
    known_stargates     integer[] DEFAULT '{}',
    interaction_maps    player_interaction_map[] DEFAULT '{}',
    training_points     integer DEFAULT 0,
    discipline_ids      integer[] DEFAULT '{}',
    racial_paradigm_levels integer[] DEFAULT '{}',
    applied_science_points integer DEFAULT 0,
    blueprint_ids       integer[] DEFAULT '{}',
    known_respawners    integer[] DEFAULT '{}',
    skin_color_id       integer DEFAULT 0,
    bandolier_slot      integer DEFAULT 0                   -- range: 0-3
);
```

---

## Implementation Status

### What is Implemented

- Full creation validation flow (definition, visuals, name, skin tint)
- Name uniqueness enforcement via database query
- Visual choice validation against character definition data
- Starting equipment assignment with `BagFillOrder` slot placement
- Starting ability assignment from character definition
- Character list display with lazy-loaded equipment visuals for preview
- Character deletion with proper foreign key cascade handling
- GM player entity creation for elevated-access accounts
- Resource version sync for client cache across eight resource categories

### What is Missing

- No character name filtering for profanity or reserved names
- No per-account character slot limit
- No server-side faction/archetype combination validation beyond the `charDefId` lookup
- No character rename functionality
- No character transfer between accounts
- The `extraName` field (Asgard compound naming) has an unresolved TODO about whether it should be removed
