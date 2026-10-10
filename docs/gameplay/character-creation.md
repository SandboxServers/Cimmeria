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

### Refusal codes

`onCharacterCreateFailed` (0x83) carries one `INT32`, and the client shows the `error_texts` row with that id, so every refusal must send a real `ERROR_*` row. The Rust handler (`crates/base/src/base/character_create/fail_code.rs`) sends python's codes:

| Code | `error_texts` moniker | When |
|---|---|---|
| 10000 | `ERROR_CharacterCreationNotEnoughInformation` | The payload is short or malformed, or a required (`VIS_Optional`) visual group has no choice |
| 10001 | `ERROR_CharacterCreationInvalidCharacterType` | An unknown char_def, or a start profile that cannot be used (lock L3, below) |
| 10002 | `ERROR_CharacterCreationInvalidSkinColor` | A skin tint outside 0-15 |
| 10003 | `ERROR_CharacterCreationUnspecifiedError` | An invalid visual group or choice, no database, or a database error |
| 20001 | `ERROR_InvalidCharacterName` | The name or extra name breaks the format rules (3-20 characters; letters, digits, spaces, hyphens, apostrophes), or the name is taken |

Before Class Start v6 CS-08 the handler sent 1 (name taken), 2 (bad payload, name, tint or char_def) and 3 (no database or a database error). Those ids are `CONDITION_FEEDBACK_*` rows, so a rejected name showed `CONDITION_FEEDBACK_PositionCheckNotBelow`. The payload, names, tint and char_def are checked before the database, so a malformed request gets its own code even with no database attached.

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

### Start profiles (Rust server)

The Rust handler is `crates/base/src/base/character_create/` (`mod.rs` parses, validates and writes; `start_profile.rs` loads and checks the profile; `starter_kit.rs` grants). Since Class Start v6 CS-02 ([ledger](../analysis/class-start-v6/README.md)) everything a new character starts with comes from one data-driven **start profile** per char_def: a row of `resources.char_creation` plus its `char_creation_abilities` and `char_creation_items` rows. Rust reads only this; `crates/resources/src/base/chardef.rs` keeps just the char_def's identity (alignment, archetype, gender, bodyset), which a live-DB test checks against the table. The universal spawn kit is gone.

| Profile | char_defs | Start world | Level | Abilities (provenance kind) | Items | State |
|---|---|---|---|---|---|---|
| `PRA_OPCORE_SOLDIER` / `_COMMANDO` / `_SCIENTIST` / `_ARCHAEOLOGIST` | 1, 11 / 3, 13 / 20, 22 / 5, 15 | Castle_CellBlock (-334.231, 73.472, -228.026) | 1 | none | none | `CANONICAL` |
| `PRA_LOYALIST_JAFFA` | 7, 17 | Castle_CellBlock | 1 | none | none | `CANONICAL` |
| `SGU_HUMAN_SOLDIER` / `_COMMANDO` / `_SCIENTIST` / `_ARCHAEOLOGIST` | 2, 12 / 4, 14 / 21, 23 / 6, 16 | SGC_W1 (201.5, 1.31, 49.724) | 1 | none | none | `CANONICAL` |
| `SGU_FREE_JAFFA` | 8, 18 | Dakara_E1 (100, -17.4, 230), the gate plaza | 1 | 597 Heal Focus, 1218 Recuperation (`racial_core`); 1984 Staff Swing (`signature`) | 2797 Serpent Staff (4342 Standard Chestplate is the forced Torso choice, worn from creation) | `CANONICAL` |
| `PRA_GOAULD` | 10, 19 | Castle_CellBlock | 1 | 592, 594, 597, 1218, 1646 (`legacy_kit`) | 55 SI 3 9mm Pistol | `NON_CANONICAL_BLOCKED_LEGACY` (OD-CS08) |
| `SGU_ASGARD` | 9 | SGC_W1 | 1 | 592, 594, 597, 1218, 1646 (`legacy_kit`) | 55 | `NON_CANONICAL_BLOCKED_LEGACY` (OD-CS09) |

The visual-choice clothing (the Praxis prison set, glasses, accessories) is placed as before, first, then the profile's items, then the debug kit's.

Pistol 55 (the debug kit's and the holding states' weapon) carries `ITEM_Pistol`, so with it drawn and loaded Pistol Shot (592) fires as itself. Since CS-07 there is no redirect to the weapon's RANGED binding (579 Pistol Auto Attack is the pistol's own weapon-granted attack, which right-click resolves); 592 with a non-pistol weapon drawn is refused with `WrongWeaponType` (`use_ability/weapon_requirement.rs`), and with an empty magazine (`required_ammo = 1`) it is refused with NoAmmo until the free reload.

- **Level.** `start_level` (1 everywhere), with one training point and one Applied Science Point per level. It never comes from a mission's seeded level (the Dakara missions are level 3; the Free Jaffa start is level 1).
- **Provenance.** Every profile ability with a kind other than `legacy_kit` gets an `sgw_player_ability_grants` row in the creation transaction, so it survives respec and the GM / Debug NPC reset and counts as branch credit ([grant provenance](../analysis/class-start-v6/README.md#grant-provenance-contract-cs-01a)). `legacy_kit` abilities get no row and no credit, as every starter did before.
- **Guns start empty.** Every gun placed at creation has 0 rounds (OD-CS13 amendment, 2026-10-05); default reload is free, so the player reloads once. The holding states' pistol 55 is empty too: the one intended change to their otherwise literal behaviour.
- **Debug kit (lock L2).** `char_creation.debug_kit = true` adds the debug kit, `resources.char_creation_debug_kit_abilities` / `_items` (592, 594, 597, 1218, 1646 and an empty pistol 55), with no provenance rows, and sets `sgw_player.debug_kit` so the GM / Debug NPC reset gives the kit back. It is never derived from access level. No profile sets it today; the seeded playtest characters below are debug-kit characters.
- **Holding states.** `NON_CANONICAL_BLOCKED_LEGACY` rows keep today's runtime literally while their real start is blocked (Goa'uld B4, Asgard B1-B3) and are removed as one unit. A canonical profile carrying a `legacy_kit` ability is refused.
- **One chestplate.** The Free Jaffa's 4342 Standard Chestplate comes from its forced Torso visual choice (`char_creation_choices` 540 and 1217), which places it in the Chest slot. Until CS-08 the profile listed it too, so a second one landed in the backpack.
- **Fail closed (lock L3).** Creation refuses, with `onCharacterCreateFailed` code 10001 (`ERROR_CharacterCreationInvalidCharacterType`) and an ERROR `event = "character_create_failed"`, a char_def with no profile (`no_start_profile`), a profile with a problem (`empty_world`, `origin_position`, `start_level_out_of_range`, `legacy_kit_on_canonical_profile`, ...), a world missing from `resources.worlds` (`start_world_unknown`), or a world the cell announced no space for (`start_world_not_loaded`; the cell sends `EnterableWorlds` at startup). The cell also audits every profile at boot (`event = "start_profile_invalid"`).

The other start-world readers use the same profiles: the GM-only world redirect sends a refused player to their own profile's home (a Free Jaffa to Dakara_E1), `.gotolocation <world>` lands on a start world's profile point, and a death in a start world with no respawner respawns at its profile point. Dakara_E1 also has respawner 610 at the plaza point. The base's space fallback no longer lands an unknown world in Castle_CellBlock: it fails closed with an ERROR (`unknown_world_no_space`).

One INFO line per creation, `event = "character_created"`, names everything the character starts with: `profile_id`, `start_state`, `debug_kit`, `level`, `abilities` ("597 Heal Focus, 1218 ...") and `items` ("3440 Prison Jacket @7/0, ..., 2797 Serpent Staff @3/0", container/slot after the `@`), plus `armed` (true when a weapon is in the bandolier).

The `sgw_player` row, the starter inventory and the provenance rows are written in one transaction. If an item can't be placed or written, nothing is kept and the client gets the DB-error code (3); the ERROR line is `starter_item_failed` or `starter_grant_failed` with a `reason` (`db_error`, `unknown_item`, `no_valid_container`, `all_valid_containers_full`).

### Seeded playtest characters

`db/sgw/Players/Seed/sgw_player.sql` seeds one character per dev account (player ids 62-70: Test Soldier, cady, jorsh, cake, lomiada1, nonwo1984, ishido972, Friendly, Annoying). The colo rebuilds its database from this seed on every deploy. Each is a **debug-kit** Praxis Commando (char_def 3, male human): what `createCharacter` makes with the debug kit applied for an account at access level 2 that takes the first choice in every optional visual group and skin tint 0. They are tester GMs, and OD-CS01 lets debug profiles keep the pistol: the legacy kit abilities, pistol 55 with 0 rounds in bandolier slot 0 (`db/sgw/Inventory/Seed/sgw_inventory.sql`), `debug_kit = true`, level 1, the Praxis start in Castle_CellBlock, `first_login = 1` and every other column at the table default.

The live-DB guard `character_create::seed_parity_live_db_tests::seeded_characters_match_a_fresh_praxis_commando_live_db` creates a fresh char_def-3 character through the real handler with the debug kit forced on (a test-only parameter of `create_character`, never access level) and fails when a seeded row (ids, names and account aside) or its inventory differs. Columns with a clock default (`now()` and the like) are left out of the comparison. `profile_live_db_tests` creates every char_def and checks it against its matrix row, and checks that a start world with no cell space refuses and rolls back.

What testers notice: the colo rebuilds its database on every start, so after each deploy the seeded characters are new level-1 characters again. They wake in the Castle_CellBlock stasis room with the intro movie (`first_login = 1`), know no stargates, and must reload the pistol once before Pistol Shot fires. The UAT guide lists the rebuild as known issue K24.

### Completion

7. On success, calls `sendCharacterList()` to refresh the client's roster display with the new character included.

---

## Archetypes

The archetype ID is the 0-based position in the `resources."EArchetype"` enum ([EArchetype.sql](../../db/resources/Archetypes/Types/EArchetype.sql)). It is the value stored in `sgw_player.archetype`, sent on the wire, and compared by the content engine's `archetype` condition. ID 0 is a placeholder, not a playable class. Which alignments may pick an archetype comes from the char defs in [chardef.rs](../../crates/resources/src/base/chardef.rs) (where each starts is the start profile above): the four human classes exist for both Praxis and the SGU, the other four for one side only.

| ID | Enum value | Name | Alignments |
|----|------------|------|------------|
| 0 | `ARCHETYPE_Any` | Any | none (placeholder) |
| 1 | `ARCHETYPE_Soldier` | Soldier | Praxis, SGU |
| 2 | `ARCHETYPE_Commando` | Commando | Praxis, SGU |
| 3 | `ARCHETYPE_Scientist` | Scientist | Praxis, SGU |
| 4 | `ARCHETYPE_Archeologist` | Archeologist | Praxis, SGU |
| 5 | `ARCHETYPE_Asgard` | Asgard | SGU |
| 6 | `ARCHETYPE_Goauld` | Goa'uld | Praxis |
| 7 | `ARCHETYPE_Sholva` | Shol'va (Free Jaffa) | SGU |
| 8 | `ARCHETYPE_Jaffa` | Jaffa | Praxis |

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
- Data-driven start profiles (world, point, level, kit, provenance, debug kit, holding states), fail-closed on an unloadable world (Class Start v6 CS-02)
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
