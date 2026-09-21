# Mission PAK Overrides

> **Type**: explanation
> **Audience**: engineers
> **Last updated**: 2026-09-21
> **Companion docs**: [docs/engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md), [docs/protocol/message-catalog.md](../protocol/message-catalog.md), [docs/content/mission-chains.md](../content/mission-chains.md), [docs/content/equip-from-inventory-pattern.md](../content/equip-from-inventory-pattern.md), [TESTING.md](../../TESTING.md)

This document explains how Cimmeria adds **new mission steps** that the client renders in its quest log without reshipping `CookedDataMissions.pak` to every player. If you only need the operator runbook ("I want to add an Equip-the-X step to mission N"), skip to [Adding a new override](#adding-a-new-override).

The same in-memory-override mechanism carries Cimmeria's item and dialog changes. The mission case is the worked example throughout; [Dialog overrides](#dialog-overrides) covers what is different about dialogs, which is the only category with two distinct override kinds.

## The problem

The client's mission catalogue — step IDs, step display text, objective IDs, objective display text — lives in `CookedDataMissions.pak` on disk. The server's `db/resources/Missions/Seed/mission_steps.sql` is its **parallel** representation: both must agree on step IDs and display text, because the wire messages the server sends (`onMissionAdvance`, `onObjectiveUpdate`, etc.) only carry IDs and statuses, never the display strings.

So when you want to introduce a new client-visible step — for instance, "Equip the pistol" between mission 622's existing step 2113 ("Search the nearby corpses") and its terminal completion — you have two options:

1. **Rebuild `CookedDataMissions.pak`** with the new XML and ship it to every player. Operationally awful: every existing install needs the new artifact, and our PAKs are on the QA-build path documented in [docs/engine/cooked-data-pak-format.md](../engine/cooked-data-pak-format.md), which we don't want to fork.
2. **Patch the entries in-memory on the server** and lean on the existing cooked-data wire path (`versionInfoRequest` → `onVersionInfo` → `resourceFragment`) to push the patched XML to the client at handshake time. This is what Cimmeria does.

Option 2 works because the protocol has always supported per-key cache invalidation; we just hadn't been using it.

## How the handshake works

The client owns two caches for cooked data:

- **The bundled PAK** (`SourceCache.en-us/CookedDataMissions.pak`) — read-only, shipped with the client install.
- **The runtime cache** (`Documents/My Games/Firesky/SGWGame/Cache.en-US/`) — writable, populated from server pushes and consulted before falling back to the bundled PAK.

On every connection, the client asks the server for each category's current version. The server replies with an `onVersionInfo` message that carries an `InvalidKeys` array; for each named key the client drops its runtime-cache entry and waits for the server to push a `resourceFragment` replacement. Verified by Ghidra decomp of `ServerConnection::onVersionInfo` (`FUN_00449460`).

```mermaid
sequenceDiagram
    participant Client
    participant Server
    Note over Server: PAK loaded at startup<br/>mission_overrides applied<br/>metadata bumped by content hash
    Client->>Server: versionInfoRequest(category=3, client_version)
    alt client_version == server_version
        Server-->>Client: onVersionInfo(version, invalidate_all=false, InvalidKeys=[], RequiredUpdates=0)
        Note over Client: Cache hit; no further traffic
    else client_version != server_version<br/>and category has scoped overrides
        Server-->>Client: onVersionInfo(version, invalidate_all=false,<br/>InvalidKeys=[622, 641], RequiredUpdates=N)
        Note over Client: Drop _622, _641 from runtime cache;<br/>do NOT issue elementDataRequest
        Server-->>Client: resourceFragment(_622, patched XML)
        Server-->>Client: resourceFragment(_641, patched XML)
        Note over Client: Runtime cache repopulated;<br/>mission UI reads patched steps
    else client_version != server_version<br/>and no scoped overrides
        Server-->>Client: onVersionInfo(version, invalidate_all=true, InvalidKeys=[], RequiredUpdates=0)
        Note over Client: Drop entire category;<br/>lazy-fetch via elementDataRequest
    end
```

The three-way reply is what makes per-mission patching work without nuking the rest of the client's cooked-data cache. See `crates/services/src/base/cooked_data.rs:53-71` for the response-shape decision in code and `crates/services/src/mercury/protocol/resources.rs:80-113` for the `build_version_info` encoder.

### Why the server pushes proactively

The first version of the fix only invalidated; it didn't push. Symptom in dev: the runtime cache had `MetaData` advanced and `_622` / `_641` deleted, but the client never issued `elementDataRequest` for them — it was waiting for the server to push, because that's how the BigWorld client cache reload path works for `InvalidKeys`. Result: missions stopped being granted on subsequent logins because the catalog row was gone.

`push_overridden_elements` (`crates/services/src/base/cooked_data.rs:133-199`) ships one `resourceFragment` per InvalidKey immediately after the `onVersionInfo` reply. `RequiredUpdates` on the version-info packet is set to the InvalidKeys count so the client knows how many fragments to expect.

The fix is **self-healing**: a client left in a previously-broken state (entries deleted, MetaData advanced past the patched value) will mismatch on its next handshake, get the same `InvalidKeys` set, and receive the proactive push. No manual cache delete required on the client side.

## Where each piece lives

| Concern | File | Symbol |
|---|---|---|
| Per-mission XML patch + insertion-point spec | `crates/services/src/base/mission_overrides.rs` | `MissionOverride`, `MISSION_OVERRIDES`, `apply_override` |
| Apply patches at PAK load + bump metadata | `crates/services/src/base/resources/mod.rs:162-236` | `ResourceCache::apply_mission_overrides` |
| Track which element IDs were patched | `crates/services/src/base/resources/mod.rs:74-81` | `ResourceCache.overridden_elements` |
| Three-way `onVersionInfo` reply | `crates/services/src/base/cooked_data.rs:21-123` | `handle_version_info_request` |
| Push patched XML after the reply | `crates/services/src/base/cooked_data.rs:133-199` | `push_overridden_elements` |
| Wire encoder for `onVersionInfo` with `InvalidKeys` | `crates/services/src/mercury/protocol/resources.rs:80-113` | `build_version_info` |
| Wire-format guard | `crates/services/src/mercury/protocol/tests.rs:159-171` | `version_info_per_key_invalidation_round_trips_through_encoder` |
| Dialog full regeneration | `crates/services/src/base/dialog_overrides/mod.rs` | `DialogOverride`, `DialogScreen`, `DialogButton`, `DIALOG_OVERRIDES`, `generate_dialog_xml` |
| Dialog patch of a shipped entry | `crates/services/src/base/dialog_overrides/patch.rs` | `DialogPatch`, `ButtonPlan`, `apply_dialog_patch`, `apply_dialog_patches` |
| Per-zone dialog patch tables | `crates/services/src/base/dialog_overrides/patches_cellblock.rs`, `patches_castle.rs` | `CELLBLOCK_DIALOG_PATCHES`, `CASTLE_DIALOG_PATCHES` |
| Shared Server-Build dialog emitter | `crates/services/src/base/dialog_overrides/emit.rs` | `emit_cooked_dialog`, `escape_xml_attr`, `CookedDialog` |
| Cooked-dialog reader | `crates/services/src/base/dialog_overrides/parse.rs` | `parse_cooked_dialog` |
| Apply both dialog kinds + bump | `crates/services/src/base/resources/mod.rs` | `ResourceCache::apply_dialog_overrides` |

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

The same gotcha applies to mission 622, which now injects **two** steps for its sequenced loot split: `2113` ("Search the nearby corpses") is index 0, `80623` ("Search the NID Guard's body") must land at index 1, and `80622` ("Equip the pistol") at index 2. This is why mission 622 has two `MissionOverride` entries that must stay in registry order — the first is `insert_after_step_id: 2113` (injects 80623), the second is `insert_after_step_id: 80623` (injects 80622, anchoring on the just-injected step). Each advance is a single-step delta (2113→80623→80622), which the guard accepts. The regression test that pins the ordering is `override_622_injects_guard_then_equip_in_order` in `crates/services/src/base/mission_overrides.rs` (and `override_641_lands_between_2121_and_3563` pins the same discipline for mission 641).

## Metadata bump policy

The category's `MetaData` value is what the client compares against to decide whether to refresh anything at all. We need a fresh value when the override content changes — otherwise the client never refetches — but we also need it to be **stable across server starts**, because otherwise every reconnect re-invalidates the same entries even when nothing changed (and incidentally racks up unnecessary `resourceFragment` traffic on every connection).

The bump is content-derived (`crates/services/src/base/resources/mod.rs:204-223`):

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

Edit either an override's `mission_id` or `injected_steps_xml` and the hash changes, the bump changes, the client mismatches, and the per-key handshake fires. Same content across two starts → same bump → same MetaData → no churn.

## Dialog overrides

Dialogs ride the same handshake, the same `overridden_elements` bookkeeping and the same bump policy, but they are the one category where a single mechanism was not enough. The catalogue is `CookedDataDialogs.pak` (category 5, 5,405 entries).

What makes dialogs different from missions: the server's `displayDialog` path carries **only the dialog id**. The client draws the window type, every screen's text and speaker, and every button from its own cooked entry. Editing `db/resources/Dialogs/Seed/dialog_screens.sql` changes nothing a player can see. An in-memory override is the only delivery route.

### Two override kinds

**Full regeneration** — `DialogOverride`, in `crates/services/src/base/dialog_overrides/mod.rs`. Emits a complete `<COOKED_DIALOG>` from Rust-authored text. Use it for a dialog Cimmeria invented, where there is no canonical entry worth preserving. The brand-new case (the NID Guard corpse's 3996, which the PAK never shipped) and the corrected case (Frost's 3995) are both just an `elements.insert`, and generation is infallible.

**Patch** — `DialogPatch`, in `crates/services/src/base/dialog_overrides/patch.rs`. Parses the entry the client already shipped, edits only what the plan names, and re-emits. Use it for one of the 5,405 dialogs the game shipped. Restating tens of screens of voiced dialogue in a Rust source file to move one button is a transcription error waiting to happen; a patch cannot make that mistake, because it never retypes the text.

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
3. Check the two hard rules: a dialog keying a `dialog_choice` chain ends with zero buttons or a button on its final screen; and never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581, 5003, 5004 or 5009.

Both zone tables ship empty. A patch plan participates in the metadata bump, so editing one re-invalidates that entry on the next handshake; an empty table writes nothing to the hasher, so shipping the engine with no rows leaves the dialogs metadata exactly where it was and no client refetches for a change it cannot see.

## Adding a new override

When you want a new client-visible step to appear in the quest log:

1. **Add the server-side step row** in `db/resources/Missions/Seed/mission_steps.sql`. The chain engine reads this for `advance_step` / `step_status` evaluation. Pick a step ID well above the canonical PAK's range (the override modules use `80<mission_id>` — e.g., `80622` for a mission-622 step — so collisions are obvious).
2. **Add the matching objective row** in `db/resources/Missions/Seed/mission_objectives.sql`. Use a single space (`" "`) for `display_log_text` — see [Why a single-space objective display text](#why-a-single-space-objective-display-text) below for the rationale.
3. **Add a `MissionOverride` entry** to the `MISSION_OVERRIDES` slice in `crates/services/src/base/mission_overrides.rs`. The `injected_steps_xml` must use the same step ID and objective ID as the SQL rows; `insert_after_step_id` must be the step the chain is advancing **from** (not the one it's advancing to).
4. **Reference the new step ID in the relevant content chain action.** Example: chain 1003's `advance_step` with `target_id=622, target_key='80623'` advances mission 622 from step 2113 to the new Guard-search step 80623; chain 1005 then advances 80623 → 80622 (`db/resources/Content/Seed/castle_cellblock_chains.sql`, chains 1001–1007).

Cross-check: the server-side seed (`mission_steps.sql`, `mission_objectives.sql`) and the client-side override (`MISSION_OVERRIDES`) must agree on `StepID`, `ObjectiveID`, and the `IsHidden` / `IsOptional` flags. The wire message `onObjectiveUpdate` only carries the ID and status, so any drift surfaces as a missing UI line on the player's screen even though the chain engine thinks it's making progress.

### Why a single-space objective display text

The original game's mission XML uses the step's `<StepDisplayLogText>` for the player-visible objective string and leaves the per-objective `<DisplayLogText>` as a single space. See `_622` step 2113 / objective 2452 and `_641` step 2121 / objective 4116 in the canonical PAK. Putting the real text on both produces a visibly duplicated line in the live mission log — a regression observed on the Frost-step UI before this convention was adopted. The regression guard for this is `objective_display_text_is_blank_to_avoid_double_render` in `crates/services/src/base/mission_overrides.rs:258-271`.

## Testing the override path

Three layers of regression coverage:

- **Unit tests on the patcher** (`crates/services/src/base/mission_overrides.rs:146-271`, 5 tests) — XML insertion-point arithmetic, malformed-input refusal, the index-pinning guard for mission 641, and the duplicate-render guard for objective display text.
- **Wire-format guard** on the encoder (`crates/services/src/mercury/protocol/tests.rs:159-171`) — pins that `build_version_info` accepts `&[u32]` and that empty vs populated keys produce different output sizes. Catches a future signature change that drops the slice or makes it optional.
- **Dialog emitter, parser and patch tests** (`crates/services/src/base/dialog_overrides/`) — byte-exact emitter pins for a screen with zero, one and two buttons; parse/emit round trips that prove escaped text and `&#xA;` survive verbatim; patch tests on inline QA-shape fixtures proving `OnlyOn` leaves exactly one button on the named screen and `StripAll` leaves none while every speaker, screen id and body is unchanged; and `LogCapture` guards on all three skip warns.
- **Chain-replay tests** for the two missions that use this mechanism (`crates/services/src/cell/content/chain_replay_tests/mission_622.rs` — the sequenced Frost → 80623 → Guard → 80622 → equip flow, the per-step re-loot guards, and the login-restore chains 1006/1007; `crates/services/src/cell/content/chain_replay_tests/mission_641.rs` for chains 1055/1066). These exercise the full `chain_id → trigger → condition → action` round-trip against the seeded `resources.content_*` tables, including the equip-step gating.

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
- [docs/architecture/negative-logging-convention.md](negative-logging-convention.md) — the `reason`-field convention the dialog patch skips follow.
- [docs/content/equip-from-inventory-pattern.md](../content/equip-from-inventory-pattern.md) — the chain-author-facing companion: when and how to wire an equip step using `MissionOverride` plus an `item_equipped` trigger.
- [TESTING.md](../../TESTING.md) — picker for which test type fits which bug shape; the override path uses unit + wire-format + chain-replay.
