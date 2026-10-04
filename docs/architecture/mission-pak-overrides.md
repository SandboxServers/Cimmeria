# Mission PAK Overrides

> **Type**: explanation
> **Audience**: engineers
> **Last updated**: 2026-09-28
> **Companion docs**: [docs/engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md), [docs/protocol/message-catalog.md](../protocol/message-catalog.md), [docs/content/mission-chains.md](../content/mission-chains.md), [docs/content/equip-from-inventory-pattern.md](../content/equip-from-inventory-pattern.md), [TESTING.md](../../TESTING.md)

This document explains how Cimmeria adds **new mission steps** that the client renders in its quest log without reshipping `CookedDataMissions.pak` to every player. If you only need the operator runbook ("I want to add an Equip-the-X step to mission N"), skip to [Adding a new override](#adding-a-new-override).

The same in-memory-override mechanism carries Cimmeria's item, dialog, Kismet sequence and world-info changes. The mission case is the worked example throughout; [Dialog overrides](#dialog-overrides) covers what is different about dialogs, which is the only category with two distinct override kinds, and [World info overrides](#world-info-overrides-category-12) covers new worlds.

Items (category 4) also have two kinds. The Slappack `ItemOverride` rows patch attributes of an entry the PAK already ships (the icon and the stack cap). Since the ammo campaign's AM-07 (#1044), `ITEM_ADDITIONS` in [`crates/resources/src/base/item_overrides/`](../../crates/resources/src/base/item_overrides/) adds **whole new entries** for ids the PAK does not ship: `new_items.rs` generates the `COOKED_ITEM` bytes and `ammo_items.rs` lists the 15 special-ammo items, 9000-9014. An addition that collides with a shipped id is skipped (`reason = "id_ships_in_pak"`), and the additions feed the items metadata bump, so every client resyncs the category once. That a client renders a wholly new id is unproven in game; the ammo UAT's first step checks it.

## The problem

The client's mission catalogue — step IDs, step display text, objective IDs, objective display text — lives in `CookedDataMissions.pak` on disk. The server's `db/resources/Missions/Seed/mission_steps.sql` is its **parallel** representation: both must agree on step IDs and display text, because the wire messages the server sends (`onMissionAdvance`, `onObjectiveUpdate`, etc.) only carry IDs and statuses, never the display strings.

So when you want to introduce a new client-visible step — for instance, "Equip the pistol" between mission 622's existing step 2113 ("Search the nearby corpses") and its terminal completion — you have two options:

1. **Rebuild `CookedDataMissions.pak`** with the new XML and ship it to every player. Operationally awful: every existing install needs the new artifact, and our PAKs are on the QA-build path documented in [docs/engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md), which we don't want to fork.
2. **Patch the entries in-memory on the server** and lean on the existing cooked-data wire path (`versionInfoRequest` → `onVersionInfo` → `resourceFragment`) to push the patched XML to the client at handshake time. This is what Cimmeria does.

Option 2 works because the server can bump a category's version and push entries the client never asked for; since #840 a bumped category is resynced in full.

## How the handshake works

The client owns two caches for cooked data:

- **The bundled PAK** (`SourceCache.en-us/CookedDataMissions.pak`) — read-only, shipped with the client install.
- **The runtime cache** (`Documents/My Games/Firesky/SGWGame/Cache.en-US/`) — writable, populated from server pushes and consulted before falling back to the bundled PAK.

On every connection, the client asks the server for each category's current version (at character select). Because the patched categories' versions are bumped by a content hash, a client holding the shipped category sees a mismatch, and the server resyncs the whole category: every entry, patched ones included, then the served version. The client does not fetch anything itself; it waits for the pushes.

```mermaid
sequenceDiagram
    participant Client
    participant Server
    Note over Server: PAK loaded at startup<br/>overrides applied<br/>metadata bumped by content hash
    Client->>Server: versionInfoRequest(category, client_version)
    alt client_version == server_version
        Server-->>Client: onVersionInfo(server_version, invalidate_all=false, RequiredUpdates=0)
        Note over Client: Cache hit; no further traffic
    else client_version != server_version
        Server-->>Client: onVersionInfo(!server_version, invalidate_all=true, RequiredUpdates=0)
        Note over Client: Delete every entry of the category;<br/>stamp the placeholder version
        Server-->>Client: resourceFragment x N (every entry, paced)
        Note over Client: Each entry written as it arrives;<br/>a lookup of one not yet here sends elementDataRequest
        Client->>Server: elementDataRequest(category, key)
        Server-->>Client: resourceFragment (that entry, next, ahead of the stream)
        Server-->>Client: onVersionInfo(server_version, invalidate_all=false, RequiredUpdates=0)
        Note over Client: Holds exactly the server's category;<br/>next login matches
    end
```

`VersionReply::decide` (`crates/base-session/src/base/cooked_sync/decision.rs`) makes the decision and the per-session resync task (`crates/base-session/src/base/cooked_sync/task.rs`) sends the resync; `build_version_info` in `crates/wire/src/mercury/protocol/resources.rs` is the encoder.

### Why every mismatch is a full resync

`versionInfoRequest` carries only a version, so the server cannot tell which entries the client holds. The client's `onVersionInfo` handler (`0x00441630`, re-decompiled 2026-09-28) deletes every entry of the category from its writable cache PAK when `InvalidateAll` is set, stamps `Version` into `MetaData` before any entry arrives, and never fetches what it dropped. So the only way to make the client hold exactly the server's category (nothing extra, nothing missing, nothing stale) is to delete everything and push everything (#840).

Before #840 a category with an override list got a per-key reply (just the overridden ids) and every other category got `invalidate_all = true` with nothing pushed. The second branch emptied every client's Kismet sequence table on 2026-09-20 (#754), and it emptied category 16 on every login, because in-world `chatJoin` (`0xC0`) was being read as a `versionInfoRequest` (see [Routing](#0xc00xc1-are-cache-messages-only-at-character-select)). The per-key reply also left anything else the client held from another build in place.

Five details keep the resync safe:

- **Placeholder version first, real version last.** The opening reply stamps the bitwise NOT of the served version, and the closing reply, queued behind every entry on the same reliable channel, stamps the real one. A client that disconnects part-way keeps the placeholder, sees a mismatch at its next login, and resyncs again.
- **`RequiredUpdates = 0`, so misses are asked for.** Each per-category request function in the client sends `elementDataRequest` only while the category's `RequiredUpdates` (`ServerSource+0x48`) is 0 (`0x00cfe060`, `0x00d20150`, … one per category). The lookup does not block: it returns nothing and asks, and the entry is written when it arrives, so the next lookup hits.
- **Misses jump the stream.** `elementDataRequest` (`0xC1` at character select, SGWPlayer `0xD5` in-world) is served from the server's category as the very next transfer on the session's task. Unknown categories and keys are refused, and a session is rate-limited: a bucket of 100, 50 a second, at most 256 waiting. Refusals log a throttled WARN.
- **Paced through the reliable window.** The resync task never lets more than 24 reliable packets be outstanding on the session (`SYNC_IN_FLIGHT_BUDGET`, the 32-slot TX window minus 8 slots left for game traffic), waiting for acks before it sends more. It runs on its own task per session, so the receive loop and other players never wait on it.
- **World entry waits only for the categories with no miss path.** The client has a request function for categories 1-11, 13, 14, 15 and 19 (the `Event_NetOut_elementDataRequest` constructor `0x00cfdeb0` has one caller per category) and none for 12, 16, 17, 18, 20 or 21. Those six are held (`HELD_CATEGORIES`): 12 because `onClientMapLoad` needs the world table at map load, and the other five because an entry they look up early could never be recovered. They total about 230 entries, under a second. The held categories stream first, then missions, dialogs and items, and TextStrings last; everything but the held set keeps streaming after world entry. The stock client shows nothing while Play waits, which is why the wait is kept this short.

How long a resync takes. The client acks about every 100 ms while it receives (colo SigNoz, 2026-09-28), so a resync moves about 24 packets per 100 ms, roughly 240 packets or 330 KB a second:

| Category | Entries | Packets | Streams in | Holds Play? |
|---|---:|---:|---:|---|
| Held set (12, 16, 17, 18, 20, 21) | ~225 | ~230 | < 1 s | **yes** |
| Kismet sequences (1) | 1,975 | 1,977 | 8 s | no |
| Missions (3) | 1,040 | 2,272 | 9 s | no |
| Dialogs (5) | 5,405 | 5,938 | 25 s | no |
| Items (4) | 6,059 | 6,679 | 28 s | no |
| Text strings (10) | 29,126 | 29,128 | 2 min | no |
| All 21 categories | 55,000+ | 57,700+ | 4 min | held set only |

So the longest Play wait is the held set, under a second. An entry the player needs before its category has streamed is asked for and arrives next, one round trip plus at most a window's worth of packets already in flight (about 100-200 ms at the colo's ack cadence).

A mismatch happens when a client first meets a build whose served version it does not hold: a changed override bumps its category's version, so every client resyncs that one category once. Moving between builds resyncs each differing category on every switch.

### `0xC0`/`0xC1` are cache messages only at character select

`versionInfoRequest` and `elementDataRequest` are `0xC0` and `0xC1` in the Account entity's method space. In-world the same ids are `SGWPlayer.chatJoin` and `chatLeave` (Communicator indices 0 and 1), and the ClientCache methods are `0xD4`/`0xD5`; in-world misses arrive as `0xD5`. The encrypted receive loop sends `0xC0`/`0xC1` to the cache handlers only while the session has no player entity. Before #840 it sent them there in both phases, so the client's login rejoin of its default user channels (`channel-chat`, `channel-roleplay`, `channel-alliance`) was read as a request for category 12 or 16 at version `0x00680063` (the UTF-16 "ch" after the string length), and category 16 got an `InvalidateAll` with nothing pushed on every login (colo SigNoz, 2026-09-28). That reply also went out as `0x80` addressed to the player entity, which in-world is SGWPlayer client method 0, not `onVersionInfo`.

## Where each piece lives

| Concern | File | Symbol |
|---|---|---|
| Per-mission XML patch + insertion-point spec | `crates/resources/src/base/mission_overrides.rs` | `MissionOverride`, `MISSION_OVERRIDES`, `apply_override` |
| Apply patches at PAK load + bump metadata | `crates/resources/src/base/resources/apply_overrides.rs` | `ResourceCache::apply_mission_overrides` |
| Content-derived `MetaData` bumps, one per category | `crates/resources/src/base/resources/metadata_bump.rs` | `compute_metadata_bump`, `compute_world_info_metadata_bump`, … |
| Track which element IDs were patched | `crates/resources/src/base/resources/mod.rs:74-81` | `ResourceCache.overridden_elements` |
| `onVersionInfo` decision | `crates/base-session/src/base/cooked_sync/decision.rs`, `crates/base-session/src/base/cooked_data.rs` | `VersionReply::decide`, `handle_version_info_request` |
| Full-category resync, misses, held set, stream order | `crates/base-session/src/base/cooked_sync/` | `start_resync`, `serve_miss`, `HELD_CATEGORIES`, `rank`, `defer_until_synced` |
| Wire encoder for `onVersionInfo` | `crates/wire/src/mercury/protocol/resources.rs` | `build_version_info` |
| Wire-format guard | `crates/wire/src/mercury/protocol/tests.rs` | `version_info_invalid_keys_payload_layout_is_byte_exact` |
| Resync guards (convergence, pacing, disconnect, relog, telemetry) | `crates/base-session/src/base/cooked_sync/tests/` | `no_category_is_ever_invalidated_without_being_repopulated`, … |
| Dialog full regeneration | `crates/resources/src/base/dialog_overrides/mod.rs` | `DialogOverride`, `DialogScreen`, `DialogButton`, `DIALOG_OVERRIDES`, `generate_dialog_xml` |
| Dialog patch of a shipped entry | `crates/resources/src/base/dialog_overrides/patch.rs` | `DialogPatch`, `ButtonPlan`, `apply_dialog_patch`, `apply_dialog_patches` |
| Per-zone dialog patch tables | `crates/resources/src/base/dialog_overrides/patches_cellblock.rs`, `patches_castle.rs` | `CELLBLOCK_DIALOG_PATCHES`, `CASTLE_DIALOG_PATCHES` |
| Shared Server-Build dialog emitter | `crates/resources/src/base/dialog_overrides/emit.rs` | `emit_cooked_dialog`, `escape_xml_attr`, `CookedDialog` |
| Cooked-dialog reader | `crates/resources/src/base/dialog_overrides/parse.rs` | `parse_cooked_dialog` |
| Apply both dialog kinds + bump | `crates/resources/src/base/resources/apply_overrides.rs` | `ResourceCache::apply_dialog_overrides` |
| New Kismet sequences (category 1) | `crates/resources/src/base/sequence_overrides.rs` | `SequenceOverride`, `SEQUENCE_OVERRIDES`, `generate_sequence_xml` |
| New worlds (category 12) | `crates/resources/src/base/world_info_overrides.rs` | `WorldInfoOverride`, `WORLD_INFO_OVERRIDES`, `generate_world_info_xml` |
| Apply the world-info entries + bump | `crates/resources/src/base/resources/apply_overrides.rs` | `ResourceCache::apply_world_info_overrides` |

## The XML-index gotcha

The client uses **XML declaration order** — the order in which `<Steps>` blocks appear in the patched mission XML — as the step *index*, and its mission state machine enforces sequential progression. An `advance_step` from a low-index step to a much higher-index step is read as a multi-step skip, and the sequential-progression guard snaps the displayed step to the next sequential index instead of honouring the targeted advance.

That's why `MissionOverride` carries `insert_after_step_id` rather than appending blindly to the tail of the XML.

### Worked example: mission 641

Mission 641 ("Preparation") in the canonical PAK has three steps in this XML order:

| XML index | StepID | Text |
|---|---|---|
| 0 | 2121 | Prepare yourself for the escape |
| 1 | 3563 | Speak to Col. Marsh |
| 2 | 3564 | Use the terminal |

We want to introduce a new step 80641 ("Equip the P90") between 2121 and 3563. The pickup chain (1055) advances the mission from step 2121 → 80641 when the player loots the P90 from the locker, and the equip chain (1066) advances from 80641 → 3563 when the player drops the P90 into the bandolier.

**If we appended `<Steps StepID="80641" …>` at the end of the XML:**

| XML index | StepID |
|---|---|
| 0 | 2121 |
| 1 | 3563 |
| 2 | 3564 |
| 3 | 80641 ← new |

The chain advances from step 2121 (index 0) to step 80641 (index 3). The client's sequential-progression guard sees a three-step jump, refuses, and snaps the displayed step to the next sequential index (3563, index 1). The player never sees "Equip the P90"; they see "Speak to Col. Marsh" with no P90 in the bandolier — which is the bug we shipped before adding `insert_after_step_id`.

**With `insert_after_step_id: 2121`:**

| XML index | StepID |
|---|---|
| 0 | 2121 |
| 1 | 80641 ← new |
| 2 | 3563 |
| 3 | 3564 |

The advance from index 0 → index 1 is a single-step delta, the guard accepts, and the player sees "Equip the P90".

The same gotcha applies to mission 622, which now injects **two** steps for its sequenced loot split: `2113` ("Search the nearby corpses") is index 0, `80623` ("Search the NID Guard's body") must land at index 1, and `80622` ("Equip the pistol") at index 2. This is why mission 622 has two `MissionOverride` entries that must stay in registry order — the first is `insert_after_step_id: 2113` (injects 80623), the second is `insert_after_step_id: 80623` (injects 80622, anchoring on the just-injected step). Each advance is a single-step delta (2113→80623→80622), which the guard accepts. The regression test that pins the ordering is `override_622_injects_guard_then_equip_in_order` in `crates/resources/src/base/mission_overrides.rs` (and `override_641_lands_between_2121_and_3563` pins the same discipline for mission 641).

## Metadata bump policy

The category's `MetaData` value is what the client compares against to decide whether to refresh anything at all. We need a fresh value when the override content changes — otherwise the client never refetches — but we also need it to be **stable across server starts**, because otherwise every reconnect re-invalidates the same entries even when nothing changed (and incidentally racks up unnecessary `resourceFragment` traffic on every connection).

The bump is content-derived (`crates/resources/src/base/resources/metadata_bump.rs`; the mission one is shown):

```rust
let mut hasher = std::collections::hash_map::DefaultHasher::new();
for ov in MISSION_OVERRIDES {
    ov.mission_id.hash(&mut hasher);
    ov.injected_steps_xml.hash(&mut hasher);
}
let bump = ((hasher.finish() as u32) & 0xFFFF) | 0x1;
missions.metadata = missions.metadata.wrapping_add(bump);
```

Two design points worth calling out:

- **`& 0xFFFF`** keeps the bump small. The QA-build `CookedDataMissions` MetaData is `7538`; bumping by up to 65535 still leaves the value far below the next category's range and well within `u32`.
- **`| 0x1`** guarantees the bump is non-zero. A zero bump would leave `MetaData` unchanged across server starts — the client would never see a mismatch and the patched XML would never reach it. Belt-and-braces against a hash that happens to land on a multiple of 65536.

Edit either an override's `mission_id` or `injected_steps_xml` and the hash changes, the bump changes, the client mismatches, and the category is resynced. Same content across two starts → same bump → same MetaData → no churn.

## Dialog overrides

Dialogs ride the same handshake, the same `overridden_elements` bookkeeping and the same bump policy, but they are the one category where a single mechanism was not enough. The catalogue is `CookedDataDialogs.pak` (category 5, 5,405 entries).

What makes dialogs different from missions: the server's `displayDialog` path carries **only the dialog id**. The client draws the window type, every screen's text and speaker, and every button from its own cooked entry. Editing `db/resources/Dialogs/Seed/dialog_screens.sql` changes nothing a player can see. An in-memory override is the only delivery route.

### Two override kinds

**Full regeneration** — `DialogOverride`, in `crates/resources/src/base/dialog_overrides/mod.rs`. Emits a complete `<COOKED_DIALOG>` from Rust-authored text. Use it for a dialog Cimmeria invented, where there is no canonical entry worth preserving. The brand-new case (the NID Guard corpse's 3996, which the PAK never shipped) and the corrected case (Frost's 3995) are both just an `elements.insert`, and generation is infallible. A new dialog id must be at most 65535 (`MAX_COOKED_ELEMENT_ID`): overrides of 100100 and 100101 crashed the client on map load, so Cimmeria-authored dialogs use 60100-60199. `every_cooked_override_element_id_fits_in_16_bits` enforces the bound for every category; see `docs/reverse-engineering/findings/cooked-dialog-override-crash.md`.

**Patch** — `DialogPatch`, in `crates/resources/src/base/dialog_overrides/patch.rs`. Parses the entry the client already shipped, edits only what the plan names, and re-emits. Use it for one of the 5,405 dialogs the game shipped. Restating tens of screens of voiced dialogue in a Rust source file to move one button is a transcription error waiting to happen; a patch cannot make that mistake, because it never retypes the text.

A patch declares a dialog id, an optional replacement `ui_screen_type`, and one of three button plans:

| Plan | Effect |
|---|---|
| `ButtonPlan::Keep` | Every button stays where the cook put it, in document order. Pair with a `ui_screen_type` change. |
| `ButtonPlan::StripAll` | Every button is removed from every screen. |
| `ButtonPlan::OnlyOn { screen_id, button_type, button_id, text }` | Every button is removed, then exactly one is placed on the named screen. |

Both kinds serialise through one emitter (`emit_cooked_dialog`), so they cannot drift into two different on-the-wire shapes.

### Why buttons are a gameplay decision

Closing a dialog that has **zero** buttons makes the client send `dialogButtonChoice(dialogId, -1)`. Closing one that has **any** button sends nothing. Clicking a button sends that button's cooked `ButtonID` and closes.

So whether a dialog carries a button decides whether a `dialog_choice` content chain ever fires. A dialog that keys such a chain must have either no buttons at all, or a button on its **final** screen — a button that stops before the final screen soft-locks a player who reads to the end and presses Done. That is what `StripAll` and `OnlyOn` exist to fix.

Button **order** within a screen is load-bearing for the same reason: the client turns a click into a position in the screen's button array and sends whichever `ButtonID` sits at that position, so the parser and emitter both preserve document order.

The full client contract these rules come from — fourteen facts read out of the client rather than inferred — is in [docs/analysis/dialog-ui-redesign/work-packets.md](../analysis/dialog-ui-redesign/work-packets.md).

### Text is kept in its escaped form

A patch carries every screen's `Text` attribute through **exactly as the cook wrote it**, entity references and all. It is never decoded and re-encoded.

That is deliberate. The QA entries write a newline as `&#xA;`, leave apostrophes raw rather than as `&apos;`, and carry `&lt;&lt;playername>>` template markers. Decoding and re-encoding would turn `&#xA;` into a literal newline, which an XML reader then normalises to a space — silently reflowing a line the patch promised not to touch. Keeping the raw form makes "change the buttons, keep the text" a byte-exact promise.

Rust-authored strings go the other way: an authored screen body or button label is written plainly in the source and escaped by `escape_xml_attr` on the way out.

### Attribute order in the emitted XML

Root attributes are alphabetised (`DialogFlags`, `DialogID`, `KismetEventSetID`, `UIScreenType`), which is the Server-Build convention.

Children are not alphabetised, because the Server Build did not alphabetise them either: [docs/engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md) shows `<Screens SpeakerID ScreenID Text>`, where alphabetical would be `ScreenID SpeakerID Text`. Comparing the two builds, the cooker's child transform was "move `Text` last, leave everything else in its cooked order".

Applying that same transform to `<Buttons>` is a no-op. All 4,349 `<Buttons>` elements in the committed QA PAK are already `ButtonType ButtonID Text`, with an explicit `</Buttons>` close and none self-closing (census 2026-09-21). So the emitter writes `<Buttons ButtonType ButtonID Text></Buttons>`. Order is a readability choice rather than a functional one — the client's cooked parser looks attributes up by name — but emitting the shipped order keeps a diff between a patched entry and its original legible.

### When a patch cannot apply

A patch transforms an entry that must already exist, so unlike a full regeneration it can fail. Three cases each emit a `warn!` with a stable `reason` field and leave the canonical bytes untouched; none aborts the pass or server startup:

| `reason` | Cause |
|---|---|
| `dialog_entry_absent` | The dialog id is not in the loaded catalogue. |
| `screen_absent` | `OnlyOn` named a `screen_id` the entry does not have. The whole patch is refused, checked before any mutation, so the entry is never left stripped-but-not-repopulated. |
| `unparsable_entry` | The cooked XML shape no longer parses. |

Field naming follows [docs/architecture/negative-logging-convention.md](negative-logging-convention.md), and each warn has a `LogCapture` guard.

### Adding a dialog patch

1. Add the `DialogPatch` to the zone table — `patches_cellblock.rs` for Castle_CellBlock, `patches_castle.rs` for Castle. One file per zone so two packets editing different zones never collide.
2. Make `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` agree in the same commit. The client renders from the patch; the seed is the committed record and what the dialog button linter reads. They are kept in sync by hand, exactly as the full-regeneration overrides already require.
3. Check the two hard rules: a dialog keying a `dialog_choice` chain ends with zero buttons or a button on its final screen; and never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581, 5003, 5004, 5008 or 5009.

4. Add the row to the zone's `patch_seed_agreement_<zone>.rs` guards, which check the plan against the seed and run it against the committed `data/cache/CookedDataDialogs.pak`. The patcher keeps the original entry when a plan cannot apply, so without that guard a typo'd dialog or screen id leaves no failing test.

The Castle_CellBlock table carries twelve `StripAll` rows (DU-02a: navigation-only Accept / Receive Item buttons, including the 3999 read-to-end soft-lock). The Castle table carries three `OnlyOn` rows (DU-02b: 2573, 5861 and 2576 keep one button, on their final screen, so the mission 701 briefings fire their chains when read to the end). A patch plan participates in the metadata bump, so editing one resyncs the dialogs category on the next handshake; an empty table writes nothing to the hasher, so shipping the engine with no rows leaves the dialogs metadata exactly where it was and no client refetches for a change it cannot see.

## World info overrides (category 12)

`CookedWorldInfo.pak` (category 12, 91 worlds, shipped `MetaData` 5959) is the client's world table: one `COOKED_WORLD_INFO` entry per world id, naming the world, its client map and its day length. `onClientMapLoad` sends a `WorldID`, `areaName` and `mapPath`, and a world id the table has never seen is new to the client.

The historical CellBlock worlds (1201–1207, [Historical CellBlocks](../analysis/historical-cellblocks/README.md)) were the first new worlds; the Debug Area (1300, [Debug Area](../analysis/debug-area/README.md)) is the second kind, a new world id on a map the client already ships. `WORLD_INFO_OVERRIDES` builds one entry per world in the wire crate's `ADDED_WORLDS` table (`cimmeria_wire::mercury::world_data::added_worlds`), so the ids, world names and client maps cannot drift from what `onClientMapLoad` sends. Each entry's `Flags` is the shipped entry's of the map it plays on (1 for the CellBlock, 0 for Ihpet_Crater_Light); a test pins the Debug Area entry as the shipped `_73` entry with only `World` and `WorldID` changed. Every entry is a full regeneration, like a new Kismet sequence, so nothing can fail to apply.

A client holding the shipped table (`MetaData` 5959) is resynced: it receives all 99 world entries, the eight added worlds among them, then the served version. Every shipped world is served untouched. `world_info_resync_converges_on_the_server_table` in `cimmeria-base-session` (`base::cooked_sync::tests::resync`) pins that exchange on the wire against the committed PAKs.

The generator reproduces the shipped QA-build shape byte for byte: the five SOAP namespace declarations; attributes in the order `Flags`, `MinPerDay`, `MinToRealMin`, `ClientMap`, `World`, `WorldID`; and an explicit end tag. `generated_world_info_xml_matches_shipped_entries` checks it against the real `_12` (stock CellBlock) and `_1` (CombatSim, whose `ClientMap` differs from its `World`) entries.

### A bumped category must keep its override list everywhere

Once a client takes a server's bumped version, it holds that version. If it then connects to a server whose category has **no** override list (an older build, or the colo before the change deploys), that server sees a version it does not hold. A server with #840 resyncs the whole category, so the client ends up with that server's table. A build from before #840 answers `invalidate_all = true` with nothing pushed, and the client empties the whole category and persists the empty table, exactly as it did for Kismet sequences on 2026-09-20. No change to the newer server can prevent that: the older build is the one doing the wiping.

So:

- Never remove an override list from a category once it has shipped while servers older than #840 are still in use. Changing its content is fine: the new bump resyncs the category.
- Expect this when one client moves between servers on different builds and one of them predates #840. For category 12, a GM who tested the historical worlds against a newer local server and then logs in to an older server loses the world table. The fix is to copy the client's pristine `SourceCache.en-us\CookedWorldInfo.pak` over `Documents\My Games\Firesky\SGWGame\Cache.en-US\CookedWorldInfo.pak` with SGW.exe closed; the next login to a server with the overrides pushes the added worlds (1201–1207, 1300) again. A login to any server from #840 on repairs it without the copy.

The server-side fix shipped in #840: every mismatch is now a full resync (see [Why every mismatch is a full resync](#why-every-mismatch-is-a-full-resync)). It protects every build from then on; builds that predate it still wipe. The repair above is also in [Troubleshooting](../troubleshooting.md#a-cooked-data-category-went-empty-after-logging-in-to-another-server).

## Adding a new override

When you want a new client-visible step to appear in the quest log:

1. **Add the server-side step row** in `db/resources/Missions/Seed/mission_steps.sql`. The chain engine reads this for `advance_step` / `step_status` evaluation. Pick a step ID well above the canonical PAK's range (the override modules use `80<mission_id>` — e.g., `80622` for a mission-622 step — so collisions are obvious).
2. **Add the matching objective row** in `db/resources/Missions/Seed/mission_objectives.sql`. Use a single space (`" "`) for `display_log_text` — see [Why a single-space objective display text](#why-a-single-space-objective-display-text) below for the rationale.
3. **Add a `MissionOverride` entry** to the `MISSION_OVERRIDES` slice in `crates/resources/src/base/mission_overrides.rs`. The `injected_steps_xml` must use the same step ID and objective ID as the SQL rows; `insert_after_step_id` must be the step the chain is advancing **from** (not the one it's advancing to).
4. **Reference the new step ID in the relevant content chain action.** Example: chain 1003's `advance_step` with `target_id=622, target_key='80623'` advances mission 622 from step 2113 to the new Guard-search step 80623; chain 1005 then advances 80623 → 80622 (`db/resources/Content/Seed/castle_cellblock_chains.sql`, chains 1001–1007).

Cross-check: the server-side seed (`mission_steps.sql`, `mission_objectives.sql`) and the client-side override (`MISSION_OVERRIDES`) must agree on `StepID`, `ObjectiveID`, and the `IsHidden` / `IsOptional` flags. The wire message `onObjectiveUpdate` only carries the ID and status, so any drift surfaces as a missing UI line on the player's screen even though the chain engine thinks it's making progress.

### Why a single-space objective display text

The original game's mission XML uses the step's `<StepDisplayLogText>` for the player-visible objective string and leaves the per-objective `<DisplayLogText>` as a single space. See `_622` step 2113 / objective 2452 and `_641` step 2121 / objective 4116 in the canonical PAK. Putting the real text on both produces a visibly duplicated line in the live mission log — a regression observed on the Frost-step UI before this convention was adopted. The regression guard for this is `objective_display_text_is_blank_to_avoid_double_render` in `crates/resources/src/base/mission_overrides.rs:258-271`.

## Testing the override path

Three layers of regression coverage:

- **Unit tests on the patcher** (`crates/resources/src/base/mission_overrides.rs:146-271`, 5 tests) — XML insertion-point arithmetic, malformed-input refusal, the index-pinning guard for mission 641, and the duplicate-render guard for objective display text.
- **Wire-format guard** on the encoder (`crates/wire/src/mercury/protocol/tests.rs:159-171`) — pins that `build_version_info` accepts `&[u32]` and that empty vs populated keys produce different output sizes. Catches a future signature change that drops the slice or makes it optional.
- **Dialog emitter, parser and patch tests** (`crates/resources/src/base/dialog_overrides/`) — byte-exact emitter pins for a screen with zero, one and two buttons; parse/emit round trips that prove escaped text and `&#xA;` survive verbatim; patch tests on inline QA-shape fixtures proving `OnlyOn` leaves exactly one button on the named screen and `StripAll` leaves none while every speaker, screen id and body is unchanged; and `LogCapture` guards on all three skip warns.
- **Chain-replay tests** for the two missions that use this mechanism (`crates/cell-content/src/cell/content/chain_replay_tests/mission_622.rs` — the sequenced Frost → 80623 → Guard → 80622 → equip flow, the per-step re-loot guards, and the login-restore chains 1006/1007; `crates/cell-content/src/cell/content/chain_replay_tests/mission_641.rs` for chains 1055/1066). These exercise the full `chain_id → trigger → condition → action` round-trip against the seeded `resources.content_*` tables, including the equip-step gating.

See [TESTING.md](../../TESTING.md) for the picker that maps these test types to bug shapes.

## Out-of-scope notes / documentation debt

- **Multi-language support.** All current overrides are English-only (`StepDisplayLogText` is hard-coded in the source). If the project ever ships localized PAKs, `MISSION_OVERRIDES` will need a per-language story (probably a locale → text map keyed off the same `mission_id` / `step_id` shape). TODO; flag this if it lands.
- **Hot reload.** Overrides apply at PAK load, which happens once at server startup. Editing `MISSION_OVERRIDES` requires a restart. The DB seed (`mission_steps.sql`) and chains are also load-once today; both are listed as future work in [docs/content/proposed-extensions.md](../content/proposed-extensions.md).
- **Larger structural patches.** `apply_override` only inserts `<Steps>` blocks. Modifying or removing existing steps, or patching `<Objectives>` inside a kept step, would need a different patcher shape. Not blocked, just not built — flag if the use case appears.

## Related documents

- [docs/engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md) — the on-disk PAK format, three-way QA / Server / Discord build comparison, why we serve QA-build PAKs.
- [docs/protocol/message-catalog.md](../protocol/message-catalog.md) — `onVersionInfo` (`Event_NetIn_onVersionInfo`) and the protocol-internal `versionInfoRequest` / `elementDataRequest` events.
- [docs/content/mission-chains.md](../content/mission-chains.md) — the full mission catalogue; chains 1003/1004 (mission 622) and 1055/1066 (mission 641) use this mechanism.
- [docs/analysis/dialog-ui-redesign/work-packets.md](../analysis/dialog-ui-redesign/work-packets.md) — the client contract behind the dialog button rules, read out of the client rather than inferred.
- [docs/analysis/historical-cellblocks/README.md](../analysis/historical-cellblocks/README.md) — the seven historical CellBlock worlds the category-12 overrides exist for.
- [docs/architecture/negative-logging-convention.md](negative-logging-convention.md) — the `reason`-field convention the dialog patch skips follow.
- [docs/content/equip-from-inventory-pattern.md](../content/equip-from-inventory-pattern.md) — the chain-author-facing companion: when and how to wire an equip step using `MissionOverride` plus an `item_equipped` trigger.
- [TESTING.md](../../TESTING.md) — picker for which test type fits which bug shape; the override path uses unit + wire-format + chain-replay.
