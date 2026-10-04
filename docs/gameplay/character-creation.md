---
title: "Character Creation"
type: reference
audience: engineers
last_updated: 2026-10-04
---

# Character Creation

## Overview

Character creation is handled by `deprecated/python/base/Account.py` and is one of the more complete systems in the emulator. It validates input, creates database records, assigns starting equipment and abilities, and manages the full flow from the character list screen through entering the world.

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

### Starter kit (Rust server)

The Rust handler is `crates/base/src/base/character_create/` (`mod.rs` parses and validates, `starter_kit.rs` grants). Every class starts with:

| What | Source | Notes |
|---|---|---|
| Abilities 592 Pistol Shot, 594 Strike, 597 Heal Focus, 1218 Medical Attention: Recuperation, 1646 Health Heal | `resources.char_creation_abilities` | Written to `sgw_player.abilities` in ability-id order. 597 restores Focus; 1646 and 1218 restore Health. With nothing selected they land on the caster (AB-01). |
| Item 55, SI 3 9mm Pistol | `resources.char_creation_items` | Placed by the same bag fill order as the clothing, so it lands in bandolier (container 3) slot 0, the active slot. Its magazine is loaded: `sgw_inventory.ammo` = `items.clip_size` (15 Bullet_Default rounds). Reloads of default ammo are free. |
| Clothing and accessories | `char_creation_choices.item_id` of the chosen (or forced) visual choices | Praxis characters get the prison set (3440 jacket, 3437 legs, 3438 boots); glasses and accessories follow the choices. |

Item 55 is the starter pistol because it is the lowest-grade pistol in the seed (tier 1, tech_comp 1), it is the pistol both tutorials hand out (Castle Cellblock chain 1005, SGC chain 3008), and buy list 1 sells it. With it drawn, Pistol Shot (592) fires the pistol's RANGED binding, 579 Pistol Auto Attack (`use_ability/weapon_redirect.rs`). Without a loaded weapon, 592 (`required_ammo = 1`) is refused with NoAmmo, which is what new characters got before 2026-10-04.

Every class gets the 9mm pistol, the Jaffa, Goa'uld and Asgard char_defs included: `items.discipline_ids` does not gate equipping. A per-class weapon is one `char_creation_items` row per char_def.

Visual groups resolve in group-id order, so the components array and the item placement are the same on every run. Two item choices can compete for one bag: the first choice's glasses (3497) and accessory (4343) both want the Face slot (5), so the accessory overflows to the backpack (container 1).

**Deviation from the tutorial design.** Castle Cellblock mission 622 ("arm yourself") is built around the prisoner finding a pistol: chain 1005 gives a second item 55 to the backpack when the Guard's body is searched, and chain 1004 completes the mission and opens the stasis-room door when an item 55 arrives in the bandolier (`item_equipped`). Both still work. The player ends up with two pistols, and the equip step is met by dragging either pistol into the bandolier from another container, the looted one or the starter one moved out and back. The narrative of an unarmed prisoner no longer holds; the maintainer asked for every class to spawn able to fire.

One INFO line per creation, `event = "character_created"`, names everything the character starts with: `abilities` ("592 Pistol Shot, 594 Strike, ...") and `items` ("3440 Prison Jacket @7/0, ..., 55 SI 3 9mm Pistol @3/0 ammo 15", container/slot after the `@`), plus `armed` (true when a loaded weapon is in the bandolier). A dropped item logs `starter_item_dropped` with its reason.

### Seeded playtest characters

`db/sgw/Players/Seed/sgw_player.sql` seeds one character per dev account (player ids 62-70: Test Soldier, cady, jorsh, cake, lomiada1, nonwo1984, ishido972, Friendly, Annoying). The colo rebuilds its database from this seed on every deploy. Each is the Praxis Commando (char_def 3, male human) that `createCharacter` makes for an account at access level 2 that takes the first choice in every optional visual group and skin tint 0: the same five abilities, the same inventory (`db/sgw/Inventory/Seed/sgw_inventory.sql`, starter pistol loaded), level 1, the Praxis start in Castle_CellBlock, `first_login = 1` and every other column at the table default. The seed's column list is the handler's INSERT plus `player_id`, so a column the handler leaves to its default stays at the default.

The live-DB guard `character_create::seed_parity_live_db_tests::seeded_characters_match_a_fresh_praxis_commando_live_db` creates a fresh char_def-3 character through the real handler and fails when a seeded row (ids, names and account aside) or its inventory differs. A change to character creation therefore fails it until the seed is updated to match.

Before 2026-10-04 the seeded characters were hand-written SGU Soldiers in SGC_W1 with only 592, 594 and 597, no weapon and access level 0, so the playtesters' characters had no health regen and could not fire Pistol Shot.

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
- Starter pistol, loaded, in the active bandolier slot for every class (`char_creation_items`)
- Seeded playtest characters identical to a created Praxis Commando, guarded by a live-DB parity test
- Character list display with lazy-loaded equipment visuals for preview
- Character deletion with proper foreign key cascade handling
- GM player entity creation for elevated-access accounts
- Resource version sync for client cache across eight resource categories

### In-client verification

The core create-and-enter flow is verified with a real client. The 2026-09-18 colo playtest ([report](../analysis/playtests/2026-09-18-colo-castle/README.md), §3 rows 6:58 PM and 8:04 PM) listed characters (count 1 → 2 → 3), created player 71 (Human Soldier, archetype 1) and player 72 (Jaffa, archetype 8), showed the creation-time boot item in the character-select preview, and entered Castle_CellBlock with both characters. Not yet verified in-client: deletion (the playtest's delete request never reached the server) and what the client displays when a name is rejected (two rejects for a surname with trailing whitespace were logged, but not what the client showed).

### What is Missing

- No character name filtering for profanity or reserved names
- No per-account character slot limit
- No server-side faction/archetype combination validation beyond the `charDefId` lookup
- No character rename functionality
- No character transfer between accounts
- The `extraName` field (Asgard compound naming) has an unresolved TODO about whether it should be removed
