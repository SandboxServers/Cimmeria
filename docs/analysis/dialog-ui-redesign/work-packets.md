# Dialog UI Redesign Work Packets (Castle_CellBlock + Castle)

> Type: how-to. Audience: Claude Code coordinator and packet workers.
> Created: 2026-09-21. Source brief: external handoff "Castle_CellBlock + Castle Dialog UI Reassignment" (not imported; its usable content is restated here).
> Companions: [Cellblock ledger](../castle-cellblock-rebuild/work-packets.md), [Castle ledger](../castle-rebuild/work-packets.md), [m706-708 worknote](../castle-rebuild/worknotes/m706-708.md) (engine fact 1), [TESTING.md](../../../TESTING.md).

This ledger reuses the dispatch, ownership, worknote and acceptance rules of the [legacy command parity ledger](../legacy-command-parity/work-packets.md#dispatch-rules). Status vocabulary is the Castle ledger's. Nothing here has been implemented, built or tested in the client.

## Client Contract

Every packet is bound by these facts. Each was read from the client, not inferred from the brief. Client UI root: `..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI\Core\`. Cooked catalogue: `data/cache/CookedDataDialogs.pak` (5,406 entries).

| # | Fact | Evidence |
|---|---|---|
| F1 | The server sends only a dialog id. The client draws window type, screen text, speakers and buttons from its own cooked entry. Seed edits under `db/resources/Dialogs/Seed/` change nothing in-game. | `crates/services/src/cell/interactions/dialog.rs:51-56`, `crates/services/src/base/dialog_overrides.rs:1-34` |
| F2 | The only delivery route for a changed dialog is a startup override of the cooked entry plus the per-key invalidation handshake. The current generator regenerates the whole entry from Rust text and cannot emit `<Buttons>`. | `dialog_overrides.rs:150-168`, `base/resources/mod.rs:364-408` |
| F3 | Cooked shape: `<COOKED_DIALOG DialogFlags KismetEventSetID UIScreenType DialogID>` with `<Screens SpeakerID Text ScreenID>` children, each holding zero or more `<Buttons ButtonType ButtonID Text>`. | PAK entries `_3999`, `_2576`, `_2572` |
| F4 | `Dialog.DialogType`, `Dialog.RadioType` and `Dialog.RealizationType` all register the same window (`DialogWin`) and the same init function. A type change from 2 to 4 or 5 has no visual effect on a displayed dialog. | `Dialog/Dialog.lua:71-73` |
| F5 | Type 0 (`DUIST_None`) is registered to the Blurb window under a "TEMP HACK" comment. It is a modal, single-screen Blurb. The Dialog module has no bark or subtitle path. | `Dialog/Blurb.lua:35-38` |
| F6 | Next, Previous and Done are client chrome on `DialogWin`. Next shows when more screens remain or Accept is visible. Decline shows automatically whenever Accept shows. | `Dialog/Dialog.lua:13-45`, `Dialog/Dialog.layout` |
| F7 | `DialogWin` renders only Accept (type 2) and Generic1-3 (type 4 and up). `BlurbWin` renders only More Info (type 1) and Accept (type 2), has no Next, and has a title-bar close control. | `Dialog/Dialog.lua:3-8`, `Dialog/Blurb.lua:3-6`, `Dialog/Blurb.layout:8` |
| F8 | Closing a dialog that has ZERO buttons sends `dialogButtonChoice(dialogId, -1)`. Closing a dialog that has ANY button sends nothing. Clicking a button sends its cooked `ButtonID` (F14) and closes. | `FUN_00d249c0` in SGW.exe (re-read 2026-09-21), [m706-708 worknote](../castle-rebuild/worknotes/m706-708.md) fact 1 |
| F9 | `dialog_choice` chains match on dialog id only. There is no authorable `button_id` condition, so Accept and More Info cannot be told apart today. | `crates/content-engine/src/triggers/matching.rs:139-140`, `castle_701_chains.sql:92-95` |
| F10 | `IsImmediate` alone decides display versus queue, for every screen type. `1` opens the window. `0` raises `DialogAvailable`, and Lua only has lure queues for Radio (radio icon), Realization (idea icon) and Tutorial. A Blurb or Dialog type sent with `IsImmediate=0` silently vanishes. Opening a lure is local only; the server is told nothing. Cimmeria always sends `1`, as the legacy Python did. | `FUN_00d25900`, `FUN_00d25200`, `FUN_00d25160` in SGW.exe; `Dialog/PushMission.lua:3-4`, `Dialog/DialogSetup.lua:93-151`; `deprecated/python/cell/SGWPlayer.py:718` |
| F11 | No shipped dialog uses type 4 or 5. Census: type 2 = 4,279, type 1 = 983, type 0 = 128, type 3 = 15. Types 4 and 5 are unexercised by original content. | PAK census 2026-09-21 |
| F12 | A working non-modal text route already exists: `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`, used by chat and GM feedback (owner-confirmed working). Channels include `CHAN_say` (0) and `CHAN_splash` (11). | `entities/defs/interfaces/Communicator.def:48-53`, `crates/services/src/cell/chat.rs:49,167`, `ChatWindow/ChatWindow.lua:93-118` |
| F13 | The client holds one non-tutorial and one tutorial dialog at a time. A second non-tutorial display EVICTS the first through the discard path, so an evicted zero-button dialog sends its `-1` AFTER the server has re-pinned to the new dialog. The server single-pins (`open_dialog_id`) and rejects that choice, so the evicted dialog's chain never fires. The legacy Python kept a dict of displayed dialogs. A queued dialog opened later hits the same rejection. | `FUN_00d24f10` in SGW.exe; `cell_methods/player/interaction/dialog.rs:36-55`; `deprecated/python/cell/SGWPlayer.py` `displayedDialogs` |
| F14 | A button click puts the cooked `ButtonID` on the wire, not the button's index: 8 for Accept, 9 for More Info, 70 for Receive Item, 71 for Take Missions. Colo telemetry also shows players clicking More Info on Blurb 2298, which does nothing today. | SigNoz `dialogButtonChoice` logs, 7 days to 2026-09-21; local `logs/server.log` shows `-1` for closes of 2982, 3995, 3996 |

Consequences that overrule the brief:

- The brief's rule "closing a window must never advance anything" is wrong for this client. F8 is the mechanism most progression chains rely on.
- **Hard rule:** a dialog that is the key of a `dialog_choice` chain must have either zero buttons, or a button on its FINAL screen. A button that stops before the final screen soft-locks a player who reads to the end and presses Done. Dialogs 3999, 5861 and 2576 ship in that broken shape today.
- **Hard rule (inherited):** never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581, 5003, 5004, 5008 or 5009.
- 5019 must never be displayed as a dialog of any type. 4003 stays BlockedEvidence (no recovered energy-field actor).

## Native Findings

Read-only Ghidra pass on SGW.exe, 2026-09-21. Addresses are static, image base `0x00400000`.

| Question | Answer | Anchor | Confidence |
|---|---|---|---|
| What does `IsImmediate` do? | `1` emits `Event_UI_DialogDisplay` once the cooked entry is cached. `0` emits `Event_UI_DialogAvailable`. Screen type never influences the choice. | handler `FUN_00d25900`, split in `FUN_00d25200`, display core `FUN_00d24f10` | High |
| `MissionFlags`, `aMissionId` | Stored, never read on the display path. `MissionFlags` is exposed to Lua through `getActiveDialogMissionFlags`; no caller found in the Dialog module. | `FUN_00aa5970` | High for "unused here" |
| `activateAvailableDialog` | Re-enters the same display core locally. Sends nothing to the server. | `FUN_00ad8670`, `FUN_00d25160` | High |
| Lua constants | Blurb 1, Dialog 2, Tutorial 3, Radio 4, Realization 5. Buttons: MoreInfo 1, Accept 2, Decline 3, Generic1-3 = 4, 5, 6. The type value is the raw cooked `UIScreenType`. There is no constant for 0. | static ints at `0x01b16120` | High |
| Button id on the wire | The Lua index becomes a position in the screen's button array; the cooked `ButtonID` at that position is sent. Button ORDER inside a screen is therefore significant. | `FUN_00ad8690`, `FUN_00d24e70`, `FUN_00d24860` | High, and matches colo telemetry |
| Active dialog slots | One non-tutorial, one tutorial. A different incoming dialog evicts the occupant through discard. Same id is an idempotent re-display. | `FUN_00d24f10`, `FUN_00d249c0` | High |
| `0x1c20 + type` on close | Goes through the generic entity-listener pin primitive also used for the portrait pins. No audio call found. Treat "per-type sound" as unsupported. | `FUN_00d22c90` | Medium |
| Parser tolerance of types 4 and 5 | Attribute lookup looks name-keyed. No validation found, but the parser itself was not traced. DU-00 settles it in the client. | `FUN_015e4d10` (untraced) | Low |
| What raises the splash text event | Not traced. `CHAN_splash` stays unverified and out of scope. | none | Low |

Correction owed to `docs/reverse-engineering/findings/dialog-portrait-lookup.md`: `FUN_00d25310` is the cooked-cache-ready handler, not the `Event_NetIn_DialogDisplay` handler, which is `FUN_00d25900`.

## Target Matrix

Only rows that change are listed. Every other dialog in the brief already matches its target (zero buttons, type 2) and must be left alone.

### Button changes (client-visible)

| Dialog | Shipped buttons | Target | Keyed by chain | Packet | Note |
|---:|---|---|---|---|---|
| 2299 | Accept on 5 of 5 | none | yes | DU-02a | close then emits -1 (F8) |
| 4001 | Accept on 5 of 5 | none | yes | DU-02a | |
| 5022 | Accept on 8 of 10 | none | yes | DU-02a | |
| 3999 | Receive Item on 7 of 9 | none | yes | DU-02a | fixes read-to-end soft-lock |
| 5023 | Receive Item on 9 of 11 | none | yes | DU-02a | |
| 2309 | Accept on 3 of 3 | none | no | DU-02a | cosmetic |
| 2516 | Accept | none | no | DU-02a | narration |
| 5859 | Accept (Blurb) | none | no | DU-02a | objective update, X-close only |
| 2305, 4000, 2308, 2518 | Accept, some with More Info (Blurb) | D-DU1 | no | DU-02a | shown AFTER accept by chains 1151-1154, so both buttons are dead |
| 2573 | Accept on 7 of 7 | Accept on final 113558 only | yes (1204) | DU-02b | Decline appears with it (F6) |
| 5861 | Accept on 5 of 8 | Accept on final 96789 only | yes (1205) | DU-02b | closes D-CA13 |
| 2576 | Take Missions on 3 of 5 | Take Missions on final 96825 only | yes (1237-1239) | DU-02b | closes 701 seed GAP 3 |
| 2572 | More Info + Accept (Blurb) | unchanged | not wired | DU-06 | needs F9 lifted first |

### Type labels (no visible change when displayed immediately, F4)

| Target type | Dialogs | Packet |
|---|---|---|
| 4 Radio | 5862, 4982, 4985, 4989, 4991, 2584 | DU-04 |
| 5 Realization | 2982, 3995, 3996, 2303, 2297, 3998, 2516, 2517, 2575, 522, 2580, 4984, 4990, 5004, 2586 | DU-05 |

None of the six Radio dialogs is the key of a `dialog_choice` chain, so a player ignoring a lure cannot stall progression. 2575 and 5004 ARE chain keys and must stay zero-button.

### Barks

| Source | Lines | Packet |
|---|---|---|
| 5019 screens 96351-96354 | "Let's move out!", "I'll draw their fire!...", "Crouch down when you're in cover!", "Flank their position while I draw their fire!" | DU-03, DU-07 |
| 5019 screens 96355-96357 | Future Self exposition | excluded (Cellblock ledger non-goal) |

## Waves

```text
Wave 0 (all parallel, no dependencies)
  DU-RE   native display path            DONE 2026-09-21; findings doc owed under DU-DOC
  DU-08   offered-dialog set (F13)       rust-gameserver-dev           worktree
  DU-00   in-client type probe           coordinator + owner UAT       throwaway branch
  DU-01   override patch engine          rust-gameserver-dev           worktree
  DU-03   bark action                    rust-gameserver-dev           worktree
  DU-L    dialog button linter           testing-validation-engineer   worktree
  DU-DOC  client contract doc (draft)    documentation-writer          worktree

Wave 1 (after DU-01; parallel with each other)
  DU-02a  Cellblock button patches       mission-systems-advisor review
  DU-02b  Castle button patches          mission-systems-advisor review
  DU-05   Realization labels             after DU-00 passes
  DU-06   button_id condition + 2572     after D-DU2

Wave 2
  DU-04   Radio lure                     after DU-00 + DU-01 + DU-08
  DU-07   Marsh bark chains              after DU-03, gated on the Cellblock escort phase

Wave 3
  DU-UAT  UAT guide rows + owner pass    after everything merged
  DU-DOC  finalise
```

Parallel implementation workers run with `isolation: "worktree"`. File ownership is arranged so no two Wave 0 or Wave 1 packets edit the same file, with one exception: the `executor/mod.rs` match arm for DU-03 is handed to the coordinator, as in the Castle ledger.

## Packets

### DU-RE

**Status:** Review (trace complete 2026-09-21; results in Native Findings; findings doc not yet written). **Scope title:** native dialog display path. **Agent:** game-archaeology-specialist, read-only Ghidra.
**Scope:** in `DialogController`'s `Event_NetIn_DialogDisplay` handler, what `IsImmediate`, `MissionFlags` and `aMissionId` do; whether screen type influences queue versus display; what `activateAvailableDialog` does natively and whether it tells the server; the numeric values of the Lua `Dialog.*Type` and `Dialog.Button*Type` constants; what the per-type id `0x1c20 + type` in `FUN_00d249c0` is; how many dialogs can be active at once; whether the cooked parser is attribute-order independent and accepts types 4 and 5; what raises `Event_UI_SplashMessageReceived`.
**Acceptance:** findings doc under `docs/reverse-engineering/findings/` with addresses and confidence, indexed from both READMEs; the Native Findings section above filled in.

### DU-00

**Status:** Ready. **Scope title:** in-client probe of types 4 and 5 with zero new code. **Owner time:** one client session.
**Scope:** on a throwaway branch, flip the two Cimmeria-authored overrides (3995 Frost, 3996 Guard) to `ui_screen_type` 4 and 5. The existing generator already supports this. The owner searches both corpses in the Cellblock.
**Acceptance (record screenshots):** window opens; text renders; Done closes; the search chains still fire (proves F8 holds for types 4 and 5); note any sound difference against type 2; note title and portrait behaviour. **Fail path:** if either type misbehaves, DU-04 and DU-05 drop to label-only seed comments and no override ships.

### DU-01

**Status:** Ready. **Scope title:** patch-mode dialog overrides with button emission. **Agent:** rust-gameserver-dev.
**Entries:** `crates/services/src/base/dialog_overrides.rs`, `base/resources/mod.rs:203-223,364-408`, `docs/engine/cooked-data-pak-format.md` (Server-Build shape), `docs/architecture/mission-pak-overrides.md`.
**Scope:** add a second override kind that transforms the canonical cooked entry instead of re-authoring its text. Shape: `DialogPatch { dialog_id, ui_screen_type: Option<u32>, buttons: ButtonPlan }` with `ButtonPlan::{Keep, StripAll, OnlyOn { screen_id, button_type, button_id, text }}`. The patcher parses the QA-build entry (SOAP namespaces present), keeps every `Screens` row byte-for-byte in text and speaker, preserves button order within a screen (the client resolves clicks by array position), applies the plan, and re-emits Server-Build XML through one shared emitter that now writes nested `<Buttons>`. Extend `DialogScreen` with an optional button list so full regenerations can carry buttons too. Fold patches into `compute_dialog_metadata_bump`. A patch whose dialog id or `screen_id` is absent from the loaded PAK must `warn!` and skip, never panic (negative-logging convention). Keep patch tables in one file per zone (`dialog_patches_cellblock.rs`, `dialog_patches_castle.rs`) so Wave 1 packets never touch the same file. Ship with both tables empty.
**Acceptance:** byte-exact emitter tests for a screen with zero, one and two buttons; patch tests on inline QA-shape fixtures of 2576 and 3999 proving `OnlyOn` leaves exactly one button on the named screen and `StripAll` leaves none while text is unchanged; a guard that fails if the missing-screen `warn!` is removed (`LogCapture`); bump changes when a plan changes and is stable across runs. The PAK is not in git, so no test may read `data/cache/`.
**Exclude:** any table rows; any seed edits.

### DU-L

**Status:** Ready. **Scope title:** dialog button linter. **Agent:** testing-validation-engineer.
**Entries:** `crates/content-engine/tests/interact_tag_linter.rs` (precedent), `db/resources/Dialogs/Seed/dialog_screen_buttons.sql`, `dialog_screens.sql`, the four `castle_*_chains.sql` files.
**Scope:** a seed-parsing test, no DB, enforcing the two hard rules in Client Contract for every dialog id that keys a `dialog_choice` chain in the Castle and Cellblock seed files. Carry an explicit allowlist for 3999, 5861 and 2576 that DU-02a and DU-02b must empty. Add a second check: a Blurb (`DUIST_DefaultBlurb`) may only carry button types 1 and 2 (F7).
**Acceptance:** linter fails when a button row is added to 5003; fails when 2576's final-screen button is removed after DU-02b; the allowlist is empty at the end of Wave 1.
**Note:** the linter reads the seed, the client reads the override. DU-02a/b keep the two in sync by hand, as `dialog_overrides.rs` step 1 already requires, and add one test per zone asserting every `DialogPatch` agrees with the seed rows.

### DU-02a

**Status:** BlockedDependency (DU-01). **Scope title:** Cellblock button patches. **Review:** mission-systems-advisor.
**Entries:** Target Matrix rows 2299 to 2518; `castle_cellblock_chains.sql` chains keyed on 2299, 4001, 5022, 3999, 5023; chains 1151-1154, 1161, 1172.
**Scope:** `StripAll` patches for 2299, 4001, 5022, 3999, 5023, 2309, 2516 and 5859; delete the matching `dialog_screen_buttons.sql` rows in the same commit. Apply D-DU1 to the four post-accept blurbs. Confirm from the chain seeds that the weapon behind "Receive Item" is granted by the 3999/5023 `dialog_choice` chain or earlier, and record which.
**Acceptance:** existing chain-replay tests for missions 638, 640 and 641 stay green unmodified (chains match on dialog id only, F9); new replay cases feed `button_id = -1` for each stripped keyed dialog and assert the same resolved actions; patch-versus-seed agreement test; DU-L allowlist entry for 3999 removed.
**Exclude:** type changes (DU-05); any change to 2300, 5021, 5020.

### DU-02b

**Status:** BlockedDependency (DU-01). **Scope title:** Castle button patches. **Review:** mission-systems-advisor.
**Entries:** Target Matrix rows 2573, 5861, 2576; `castle_701_chains.sql` chains 1204, 1205, 1237-1239 and its GAP 3 note; Castle audit D-CA13.
**Scope:** `OnlyOn` patches: 2573 Accept (type 2, id 8) on 113558; 5861 Accept on 96789; 2576 Take Missions (type 4, id 71) on 96825. Move the seed button rows to match. Update the GAP 3 and D-CA13 notes to resolved.
**Acceptance:** replay tests for 701 accept and the 2576 turn-in unchanged and green; patch-versus-seed agreement test; DU-L allowlist entries for 5861 and 2576 removed. UAT row: read 2576 to the last screen, press Take Missions, missions 702 and 703 arrive; close early with X, nothing is granted and Copplemann can be re-asked.
**Exclude:** 2572, 30, 2042, 2043, 2044 (not wired to any chain today; wiring new optional missions is out of scope).

### DU-03

**Status:** Ready. **Scope title:** non-modal NPC bark action. **Agent:** rust-gameserver-dev; coordinator lands the `executor/mod.rs` arm.
**Entries:** `crates/services/src/cell/chat.rs` (payload builder for method 28), `crates/content-engine/src/loader/action.rs`, `cell/content/executor/`, `db/resources/Dialogs/Seed/dialog_screens.sql` and `speakers.sql`, F12.
**Scope:** new action `npc_bark` with params `{ "screen_id": N, "speaker": "Col. Marsh", "channel": "say" }`. The executor resolves the line text server-side from the `dialog_screens` resource row and sends `onPlayerCommunication(speaker, 0, channel, text)` to the triggering player only. The speaker is an explicit param because 5019's screens carry `SpeakerID 0`. Do not route through the `SystemMessage` stub, whose wire format is still unknown.
**Acceptance:** byte-exact wire test of the method-28 payload; loader unit test; chain-replay test for a fixture chain; a guard that an unknown `screen_id` warns and sends nothing. UAT row: line appears in the chat window as Marsh while the player keeps moving and firing; no window opens.
**Not in scope:** the on-screen splash text. Its native trigger was not traced, so `"channel"` accepts only `say` until someone verifies `CHAN_splash` in the client.

### DU-08

**Status:** Ready. **Scope title:** offered-dialog set replaces the single open-dialog pin. **Agents:** rust-gameserver-dev, server-authority-enforcer review.
**Entries:** `cell/interactions/dialog.rs:42-49`, `cell_methods/player/interaction/dialog.rs:25-55`, security finding CAT-J-01 (#479), F13.
**Scope:** replace `open_dialog_id: Option<i32>` with a small bounded set of offered dialog ids. `send_dialog_display` inserts; a valid choice removes exactly that id (still one-shot); logout and world change clear it. This fixes a live defect independent of the lure: when the server displays B while zero-button A is open, the client evicts A and sends `(A, -1)`, which today is rejected, so A's chain is lost. Bound the set (eight is ample: the client holds two active dialogs plus lures) and evict the oldest with a `warn!`.
**Acceptance:** authority tests: forged id still rejected; replayed id rejected; display A, display B, then choice `(A, -1)` is accepted and fires A's chain (this case must FAIL on today's code); then the choice for B is accepted; set cleared on logout; overflow warns. Audit every chain in the Castle and Cellblock seeds that displays a dialog from a `dialog_choice` chain and record whether a now-accepted eviction changes behaviour.

### DU-04

**Status:** BlockedDependency (DU-00, DU-01, DU-08). **Scope title:** Radio lure delivery. **Agents:** rust-gameserver-dev, mission-systems-advisor for the chain edits.
**Scope:** type-4 patches for the six Radio dialogs. The type is REQUIRED here, not a label: a type-2 dialog sent non-immediate has no queue and vanishes (F10). `display_dialog` gains an optional `"immediate": false` param that sets the `IsImmediate` wire byte to 0; the loader must REJECT `immediate: false` for any dialog whose seed `ui_screen_type` is not Radio or Realization. The offered-dialog set from DU-08 already covers a lure opened minutes later, because opening it tells the server nothing. Decide the wire `EntityId` for a remote speaker using `docs/reverse-engineering/findings/dialog-portrait-lookup.md`, since 5862 is currently pinned to Gerschon only for transport. Convert chain displays of 5862, 4982, 4985, 4989, 4991 and 2584 to non-immediate.
**Acceptance:** wire test pinning byte 12 to 0 and to 1; authority tests that a forged choice is still rejected, a queued-then-opened choice is accepted, and an id is single-use; replay tests for each converted chain. UAT: radio icon flashes on Castle entry as a Jaffa, clicking it opens 5862, 2584 changes speakers correctly.
**Relog:** the client queue is memory only. None of the six is a progression key, so a transmission lost to a relog is acceptable; do not add restore chains.
**Fail path:** if the in-client lure misbehaves, ship the type labels only and close the packet.

### DU-05

**Status:** BlockedDependency (DU-01, DU-00). **Scope title:** Realization labels. **Scope:** type-5 patches for the fifteen dialogs in the matrix, `ui_screen_type = 5` on overrides 3995 and 3996, and the matching `dialogs.sql` seed values. `ButtonPlan::Keep` on all of them.
**Acceptance:** patch-versus-seed agreement test; DU-L still green for 2575 and 5004. **Value:** documentary only. The trace found no per-type audio and F4 shows no visual difference. It does make these dialogs eligible for the idea-icon lure later. First packet to cut.

### DU-06

**Status:** BlockedDecision (D-DU2). **Scope title:** `button_id` condition and the 2572 offer flow. **Agents:** rust-gameserver-dev, mission-systems-advisor.
**Scope:** add an authorable `button_id` condition (`matching.rs:139`, `conditions.rs`, loader), noting F8's `-1` for close. Then wire Gerschon's Human offer as Blurb 2572: Accept accepts 701, More Info displays 2573, whose final-screen Accept (DU-02b) accepts 701. Mission 701 must not be acceptable twice.
**Acceptance:** condition unit tests including `-1`; replay tests for Accept, More Info then Accept, More Info then Decline, and a second interaction after accept. **Note:** the condition compares against the cooked `ButtonID` (F14): 8 Accept, 9 More Info, -1 close.

### DU-07

**Status:** BlockedDependency (DU-03) and gated on the Cellblock Marsh escort phase. **Scope title:** Marsh bark chains. **Agents:** mission-systems-advisor, npc-ai-spawn-advisor.
**Scope:** fire the four usable 5019 lines from existing triggers: escort start ("Let's move out!"), Mess Hall region entry ("I'll draw their fire!..."), first cover use or the `player_flanked_npc` trigger from C06 ("Crouch down...", "Flank their position..."). Each line once per mission run. New chain ids from the Cellblock ledger's free block.
**Acceptance:** replay tests per chain plus the already-fired negative case; relog does not replay barks.

### DU-DOC

**Status:** Ready (draft), finalise in Wave 3. **Agent:** documentation-writer.
**Scope:** RE finding `docs/reverse-engineering/findings/dialog-controller-wire-flow.md` built from the Native Findings table, with the correction to `dialog-portrait-lookup.md` and both README index rows; new reference doc `docs/content/dialog-ui-client-contract.md` carrying the Client Contract table and hard rules; update `docs/architecture/mission-pak-overrides.md` for patch mode; add the two hard rules to `.github/instructions/content-chains.instructions.md`; document `npc_bark` and any new condition in `docs/content/content-engine.md` and `docs/guides/extend-the-content-engine.md`; index entries in `docs/readme.md` (including a row for this ledger) and `docs/content/README.md`. Docs are CRLF.

### DU-UAT

**Status:** BlockedDependency (all). **Scope:** add rows to the Cellblock and Castle UAT guides for: 3999 read-to-end then Done grants progress; 2576 final-screen Take Missions; 5861 Accept and Decline; 5859 closes with X and nothing else happens; Marsh barks never open a window; Radio lure if DU-04 shipped. Screenshots for 2572, 4001, 5862, 2584, 2580 and a bark line.

## Open Decisions

| Id | Question | Default if unanswered |
|---|---|---|
| D-DU1 | Blurbs 2305, 4000, 2308, 2518 appear after the mission is already accepted. Keep Accept as an acknowledgement button, or strip to X-close only? More Info on 4000 and 2308 is a dead button either way. | Keep Accept, strip More Info. |
| D-DU2 | Is the Blurb 2572 offer flow wanted now? It needs a new engine condition and changes how mission 701 is offered to Humans. | Defer; ship DU-02b without it. |
| D-DU3 | Radio lure means the player can ignore a transmission. Acceptable for all six, or should 2584 stay immediate because it carries the throne-room briefing? | Lure for all but 2584. |
