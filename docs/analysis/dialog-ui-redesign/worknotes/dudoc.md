# DU-DOC Worknotes

> Type: reference. Audience: dialog-UI-redesign coordinator.
> Companions: [work-packets.md](../work-packets.md), [dialog-controller-wire-flow.md](../../../reverse-engineering/findings/dialog-controller-wire-flow.md), [dialog-ui-client-contract.md](../../../content/dialog-ui-client-contract.md).

## Contract

- **Packet:** DU-DOC — client contract documentation, draft pass. Docs only; no cargo, no crates, no seed.
- **Decisions in force:** the Client Contract facts F1-F14 and both hard rules in [work-packets.md](../work-packets.md) are binding evidence, re-verified here against the client (see [Discrepancies](#discrepancies-found)).
- **Depends on:** DU-RE (complete). Wave 3 finalisation is a separate pass.
- **Source revision / base:** branch `dialog-ui/dudoc-client-contract` in the `dudoc` worktree, based on `main` @ `192d4216`.
- **Owned paths:**
  - `docs/reverse-engineering/findings/dialog-controller-wire-flow.md` (new)
  - `docs/content/dialog-ui-client-contract.md` (new)
  - `docs/analysis/dialog-ui-redesign/worknotes/dudoc.md` (this file)
  - Index/correction edits, minimal and additive only: `docs/reverse-engineering/findings/dialog-portrait-lookup.md`, `docs/reverse-engineering/README.md`, `docs/reverse-engineering/findings/README.md`, `docs/reverse-engineering/address-map.md`, `docs/content/README.md`, `docs/readme.md`, `.github/instructions/content-chains.instructions.md`
- **Read set:** `docs/analysis/dialog-ui-redesign/work-packets.md`; the DU-RE raw report; the client UI module at `…\SGWGame\Content\UI\Core\Dialog\` (`DialogSetup.lua`, `Dialog.lua`, `Blurb.lua`, `Tutorial.lua`, `TutorialScreen.lua`, `PushMission.lua`, `Dialog.layout`, `Blurb.layout`, `Tutorial.layout`, `Dialog.toc`, `Dialog.int`); `data/cache/CookedDataDialogs.pak`; `entities/defs/SGWPlayer.def`, `entities/defs/interfaces/Communicator.def`; `docs/protocol/client-method-dispatch-table.md`, `docs/protocol/cell-method-dispatch-table.md`; `crates/services/src/cell/interactions/dialog.rs`, `crates/services/src/cell/cell_methods/player/interaction/dialog.rs`, `crates/services/src/cell/chat.rs`, `crates/services/src/base/dialog_overrides.rs`, `crates/content-engine/src/triggers/matching.rs`; `db/resources/Dialogs/Types/EDialogUIScreenType.sql`, `db/resources/Dialogs/Tables/*.sql`, `db/resources/Dialogs/Seed/dialogs.sql`; `docs/reverse-engineering/findings/dialog-portrait-lookup.md`, `docs/architecture/mission-pak-overrides.md`, `docs/analysis/castle-rebuild/worknotes/m706-708.md` (engine fact 1).

## What shipped

| File | Kind | Diátaxis type | Notes |
|---|---|---|---|
| `docs/reverse-engineering/findings/dialog-controller-wire-flow.md` | new, 251 lines | reference | The RE finding. Wire signature, the `IsImmediate` split with the decompile excerpt, `activateAvailableDialog` local-only, the `Dialog` constants table with addresses, two slots and eviction-through-discard, the close sentinel versus the cooked `ButtonID` with telemetry, what each window draws, address inventory, a fenced-off Unverified section, per-section confidence. |
| `docs/content/dialog-ui-client-contract.md` | new, 155 lines | reference | Author-facing. Window types, drawable button types per window, chrome, close semantics + both hard rules, immediate-versus-lure, one-at-a-time eviction, seed-versus-override, barks-through-chat, a "not built yet" table pointing at the ledger. |
| `docs/reverse-engineering/findings/dialog-portrait-lookup.md` | edit, +32 / -3 | — | Dated correction note plus two diagram labels. Track 2 marked disputed. Nothing else touched. |
| `docs/reverse-engineering/address-map.md` | edit, +26 | — | New `### Dialog System — DialogController Display Path (DU-RE, 2026-09-21)` table in the file's existing format, inserted before the CRT IAT section. |
| `docs/reverse-engineering/README.md` | edit, +3 / -1 | — | One paragraph in the findings prose; doc count `66 → 70` (was already stale). |
| `docs/reverse-engineering/findings/README.md` | edit, +2 / -1 | — | One table row; directory count `64 → 70` (was already stale at 69). |
| `docs/content/README.md` | edit, +1 | — | One row in the content-engine table. |
| `docs/readme.md` | edit, +2 | — | One row in the `content/` table, one in the `findings/` table. Only rows added. |
| `.github/instructions/content-chains.instructions.md` | edit, +26 | — | New `## Dialog buttons and the two hard rules` section plus one linked-reference line. |

No file under `crates/`, `db/`, `TESTING.md`, or `docs/analysis/dialog-ui-redesign/work-packets.md` was touched. `docs/architecture/mission-pak-overrides.md` patch mode (DU-01) and `npc_bark` (DU-03) were left alone as instructed.

## Citations verified

Every `file:line` repeated in either new doc was opened and checked. Verified as stated:

| Citation | What it shows |
|---|---|
| `Dialog.lua:71-73` | Types 2, 4, 5 all register `DialogWin` with `DialogMod.initDialog` |
| `Dialog.lua:4-8` | `dialogButtonMap`: Accept + Generic1-3; Decline commented out at `:5` |
| `Dialog.lua:18` | `Dialog_DeclineButton:setVisible( Dialog_AcceptButton:isVisible() )` |
| `Dialog.lua:22-33` | Next/Prev versus Prev/Done window selection, and Next disabled on the last screen when Accept shows |
| `Dialog.lua:49` | `selectActiveDialogChoice(dialogId, this:getID())` |
| `Dialog.lua:64-67, 83-85` | Done, Decline and the X all route to `onDialogDoneClicked` → `discardAvailableDialog` |
| `Blurb.lua:5-6` | `blurbButtonMap`: More Info + Accept only; Decline commented out at `:7` |
| `Blurb.lua:17` | Blurb Decline visibility tracks Accept |
| `Blurb.lua:35, 38` | `BlurbType` and type `0` (the `TEMP HACK`) both register `BlurbWin` |
| `Blurb.lua:45-46` | Blurb close handlers |
| `DialogSetup.lua:54` | Unregistered type draws nothing |
| `DialogSetup.lua:67-83` | `enableDialogButtons` hides all, shows only mapped types, assigns the 1-based id at `:78`, sets text at `:79` |
| `DialogSetup.lua:86-93` | `onDialogAvailable` returns immediately when no queue exists for the type |
| `PushMission.lua:3-4` | Queues registered for Radio and Realization only |
| `Tutorial.lua:104` | Tutorial queue registration |
| `TutorialScreen.lua:20-30` | `initTutorial` — never calls `enableDialogButtons` |
| `TutorialScreen.lua:81, 88-89` | Tutorial window registration and its close handlers |
| `Dialog.layout:8-9`, `Blurb.layout:7-8`, `Tutorial.layout:9-10` | `TitlebarEnabled=False`, `CloseButtonEnabled=True` on all three |
| `Dialog.layout:90` | `Dialog_DoneButton`, inside `Dialog_PrevOnlyWindow` |
| `Dialog.layout:111-123` | Generic1-3 are `TextButton_2` |
| `Dialog.layout:128,135`, `Blurb.layout:71,79,86` | Accept / Decline / More Info are `ImageButton_2` with fixed art |
| `entities/defs/SGWPlayer.def:1150-1156` | `onDialogDisplay` arg order: `EntityId, DialogID, MissionFlags, IsImmediate (UINT8), aMissionId` |
| `entities/defs/SGWPlayer.def:621-625` | `dialogButtonChoice` is `<Exposed/>`, `INT32 DialogId, INT32 ButtonId` |
| `docs/protocol/client-method-dispatch-table.md:252` | Client method 105 |
| `docs/protocol/cell-method-dispatch-table.md:283` | Cell method 75 |
| `crates/services/src/cell/interactions/dialog.rs:51-56` | The 17-byte payload; `IsImmediate` hardcoded to `1` at `:55` |
| `crates/services/src/cell/cell_methods/player/interaction/dialog.rs:36-45` | The single `open_dialog_id` pin rejecting a mismatched choice |
| `crates/services/src/base/dialog_overrides.rs:1-12` | Seed edits have zero in-game effect |
| `entities/defs/interfaces/Communicator.def:48-53` | `onPlayerCommunication(WSTRING Speaker, UINT8 SpeakerFlags, UINT8 Channel, WSTRING Text)` |
| `crates/services/src/cell/chat.rs:49, 167` | `ON_PLAYER_COMMUNICATION = 28`, and the send site |

Two independent re-derivations, both agreeing with the ledger:

1. **PAK census re-run** over `data/cache/CookedDataDialogs.pak` with `zipfile` + a regex on `UIScreenType`. Result: type 0 → 128, type 1 → 983, type 2 → 4,279, type 3 → 15, types 4 and 5 → **zero**. F11 confirmed exactly. (Precision note: the archive holds 5,406 *entries*, of which one is a 3-byte `MetaData` entry, so there are 5,405 dialogs. The ledger's "5,406 entries" is correct as written; do not restate it as a dialog count.)
2. **Wire offsets** derived from the `.def` rather than copied: `EntityId` 0, `DialogID` 4, `MissionFlags` 8, `IsImmediate` 12, `aMissionId` 13, total 17 bytes. This matches `Vec::with_capacity(17)` in the server and confirms DU-04's "pin byte 12" acceptance criterion.
3. **The seed enum matches the client constants.** `db/resources/Dialogs/Types/EDialogUIScreenType.sql:7-12` orders `DUIST_None, DefaultBlurb, DefaultDialog, DefaultTutorial, DefaultRadio, DefaultRealization` — 0 through 5, value-for-value with the static ints at `0x01b16120`. The ledger never claimed this; it is worth knowing that the Radio and Realization labels already exist in the schema even though no row uses them.

## Discrepancies found

Reported rather than papered over. None is fatal to a packet, but three change acceptance wording.

1. **F7 overstates the Blurb's close control.** F7 reads "`BlurbWin` … has no Next, and has a title-bar close control", citing `Blurb.layout:8`. That line is right but the implied exclusivity is wrong: `DialogWin` (`Dialog.layout:8-9`) and `TutorialWin` (`Tutorial.layout:9-10`) carry the identical `TitlebarEnabled=False` + `CloseButtonEnabled=True` pair, and `Dialog.lua:85` subscribes `DialogWin.EventCloseClicked`. **Every** dialog window has a working X. This matters to DU-02b's UAT row ("close early with X"), which is valid for `DialogWin` dialogs too, and to DU-02a's 5859 row, which calls X-close a Blurb property. Both new docs state it correctly.

2. **`dialog-portrait-lookup.md` Track 2 is contradicted by the now-recovered Lua.** That document infers the speaker name from a CookedData `speakers` lookup and admits in its open question 3 that the dialog Lua was never recovered. It has been: `Dialog.lua:42` and `Blurb.lua:19` both do `setText( unitName(Unit.Dialog) )` — the GameEntityManager `DialogSpeaker` slot `0x11` — with no CookedData lookup at display time. If that reading holds, the blank portrait and the player-name fallback are **one** bug (a slot pin that never landed), and that document's "Fix 2 — Col Marsh name" targets the wrong table. I made the minimal edit asked for plus a second numbered bullet marking Track 2 disputed; I did **not** rewrite Track 2, because confirming it needs someone to establish what `unitName` returns for an unpinned slot. **Coordinator action: this deserves an issue, or a DU-RE follow-up, before anyone acts on `dialog_screens.speaker_id` for 4001.**

3. **An undrawable button still suppresses the close sentinel.** The ledger treats "the window can't draw it" (F7) and "buttons suppress the `-1`" (F8) as separate facts. They compose badly and nobody stated the composition: `enableDialogButtons` skips a button whose type is absent from the window's map (`DialogSetup.lua:69-82`), so it renders nothing and is unclickable — but the native discard path counts cooked buttons off the screen records, not off the rendered window, so it still suppresses the close send. A Generic button on a Blurb is therefore a silent soft-lock with no visible symptom at all. Stated in both new docs and in the DU-L-facing instructions section; **DU-L should lint button-type-versus-window as a hard failure, not a style warning.**

4. **Tutorial dialogs never render cooked buttons.** `initTutorial` (`TutorialScreen.lua:20-30`) does not call `enableDialogButtons`; it pages text with `getActiveDialogPageText` / `getActiveDialogPageCount` and draws its own Done. The button rules do not apply to type 3 at all. Not wrong in the ledger, just absent. Worth a DU-L allowance so the linter does not demand a final-screen button on a tutorial.

5. **Decline is the close path, not a third choice.** F6 calls Decline "client chrome", which is right, but the consequence is unstated: Decline is wired to `onDialogDoneClicked` (`Dialog.lua:84`) / `onBlurbClosed` (`Blurb.lua:46`), never to `selectActiveDialogChoice`. So on a dialog that **has** buttons, Decline sends nothing — identical to Done and the X. **DU-06's acceptance lists a "More Info then Decline" replay case; on current client behaviour that case receives no wire event at all, so the test needs re-specifying as "no chain fires".**

6. **Blurb's Decline button looks broken in the original client.** `Blurb.lua:46` reads `BlurbWin:subscribe(Blurb_DeclineButton.EventClicked, …)` — it subscribes the *window*, where the sibling at `Dialog.lua:84` subscribes the *button*. This reads as an original defect leaving Blurb's Decline inert. Filed as LOW confidence in the finding's Unverified table; one click in a client session settles it. Relevant to D-DU1, which is about whether to keep Accept on four Blurbs — if Decline is inert, a Blurb with Accept shows a dead Decline beside it.

7. **Minor citation drift in the ledger** (no action needed, recorded so a future reader is not confused): F7's `Dialog.lua:3-8` is really `:4-8` and `Blurb.lua:3-6` is really `:5-6`; F9's `matching.rs:139-140` is `:141-142` on this base. F5's `Blurb.lua:35-38`, F10's Lua citations, F4's `Dialog.lua:71-73` and the DU-RE report's Lua line numbers all check out exactly.

8. **Only Generic 1-3 display authored button text.** Generic buttons are `TextButton_2`; Accept, Decline and More Info are `ImageButton_2` with fixed `NormalImage` art, though `setText` is still called on them (`DialogSetup.lua:79`). Marked MEDIUM in the finding because it is inferred from the widget type, not observed running. Authors writing label text on an Accept button are writing it for nobody.

9. **Lure queues survive a UI reload but not a relog; tutorials survive both.** `DialogMod.populateQueue` re-reads `getAvailableDialogs()` on module load (`DialogSetup.lua:193-197, 201`), and tutorial pending/viewed ids persist per character through `GDialogMod_SavedData` (`Dialog.toc` `CharacterVariable`, `Tutorial.lua:51-69`). The native `AllAvailableDialogs` container is process-lifetime, so DU-04's "the client queue is memory only … a transmission lost to a relog is acceptable" stands for Radio and Realization. Refinement, not a correction.

## Commands run

| Command | Exit | Result |
|---|---|---|
| `python … zipfile census of data/cache/CookedDataDialogs.pak` | 0 | 5,406 entries; type census as above; types 4/5 absent |
| `tools/lint-md.ps1` (repo-wide; the config's globs override per-file args) | 1 | Warn-only. **Zero** violations in either new doc after one fix. Pre-existing violations elsewhere untouched. |
| `file <each touched doc>` | 0 | All report "with CRLF line terminators" |
| `git diff --stat` | 0 | 93 insertions / 6 deletions across seven edited files, plus two new files — no whole-file rewrites |

One lint violation was mine and is fixed: `MD028/no-blanks-blockquote` where the two hard-rule callouts sat as adjacent blockquotes separated by a blank line. A connecting sentence now separates them.

Two lint violations in `docs/reverse-engineering/findings/README.md` (`MD056` table-column-count at the `combat-formulas-status.md` row, `MD047` missing trailing newline) pre-date this branch — confirmed against `git show HEAD:…`. Left alone; not mine to fix in this packet.

## Regression proof

Not applicable. This packet ships no code, no seed and no test. The equivalent discipline here is that every repeated `file:line` was opened rather than copied, and the two numeric claims most likely to be wrong — the PAK type census and the wire offsets — were re-derived from primary sources rather than restated from the ledger.

## Known gaps

- **Wave 3 finalisation is still owed.** Once DU-01, DU-03, DU-04, DU-06 and DU-08 land, `docs/content/dialog-ui-client-contract.md` § "What is not built yet" must be emptied into the body, and `docs/content/content-engine.md` plus `docs/guides/extend-the-content-engine.md` need the `npc_bark` action and any new condition documented. Those belong to DU-03 and DU-06 respectively, per this packet's exclusions.
- **`docs/architecture/mission-pak-overrides.md` still describes regenerate-only overrides.** The contract doc links to it for the delivery mechanism. When DU-01 lands patch mode, that link stays valid but the linked page needs DU-01's update, or an author reading the chain will conclude buttons cannot be delivered.
- **The splash-text route is still untraced.** `Event_UI_SplashMessageReceived` exists (RTTI `0x01e0d5e8`) but nothing was found that raises it. `CHAN_splash` (11) stays unverified, which is why the contract doc offers only the chat route for non-modal text.
- **No figures.** The display-versus-queue split would read well as a Mermaid sequence diagram, but figure sources under `docs/drafts/spec/figures/` are gated by two blocking CI jobs and this packet writes outside that tree. Flagged as documentation debt rather than attempted.

## Integration edits the coordinator must make

1. **Fill the ledger's Native Findings section** and flip DU-RE from Review to Done, citing `docs/reverse-engineering/findings/dialog-controller-wire-flow.md`. DU-RE's acceptance ("findings doc … indexed from both READMEs") is now met. I do not own `work-packets.md`.
2. **Correct F7 in the Client Contract table** — the title-bar close control is on all three windows, not only the Blurb. See discrepancy 1.
3. **Re-specify DU-06's "More Info then Decline" acceptance case** as "no wire event, no chain fires". See discrepancy 5.
4. **Tell DU-L two things**: an undrawable button still suppresses the close sentinel, so button-type-versus-window is a correctness rule and not a style rule (discrepancy 3); and type-3 tutorials carry no buttons at all, so they need an allowance (discrepancy 4).
5. **Decide what to do about `dialog-portrait-lookup.md` Track 2.** It is now marked disputed in place. Either open an issue, or hand it to a DU-RE follow-up. Nobody should act on its "Fix 2" until then. See discrepancy 2.
6. **Add a ledger row to `docs/readme.md`** if the ledger itself should be indexed. DU-DOC's ledger scope line asks for it, but `docs/analysis/dialog-ui-redesign/work-packets.md` is coordinator-owned and I only added rows for the two docs this packet wrote.

## Open questions

- Should the contract doc carry the per-dialog Target Matrix, or keep pointing at the ledger? It points today, on the grounds that the matrix is a work list with a finite life and the contract is permanent. Say so if you want it inlined at Wave 3.
- D-DU1 is still open and the contract doc does not take a side on whether the four post-accept Blurbs keep Accept. If discrepancy 6 turns out to be real (Blurb Decline inert), that is an argument for stripping to X-close only.
