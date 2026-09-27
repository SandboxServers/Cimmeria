# CR-E1 Worknotes

> Type: reference. Audience: crafting campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md).

## Contract

- **Packet:** CR-E1 — Client evidence for the crafting UI.
- **Depends on:** none.
- **Decisions read:** D-CR16 (respec two-step), D-CR20 (induction bar clock fallback).
- **Source revision/base:** `origin/main` @ `95366c59` on branch `craft/cre1-client-evidence`, the
  `cre1` worktree.
- **Owned paths (this packet):**
  - `docs/reverse-engineering/findings/crafting-client-ui.md` (new)
  - `docs/reverse-engineering/findings/README.md` (index row + count)
  - `docs/reverse-engineering/README.md` (doc-count prose)
  - `docs/reverse-engineering/findings/crafting-restoration.md` (C-61 correction)
  - `docs/reverse-engineering/findings/crafting-wire-formats.md` (C-62 correction)
  - `docs/analysis/crafting/worknotes/cr-e1.md` (this file)
  - Ghidra: `SGW.exe`'s `PRE_COMMENT` at `0x00e465d0` / `0x00e465d6` (`Crafting_isCraftTypeAllowed`)
- **Read set:** `docs/analysis/crafting/work-packets.md` §CR-E1, `audit.md` §3 (C-30..C-40) and §5
  (C-61, C-62), `README.md` (D-CR16, D-CR20); existing findings
  `crafting-restoration.md`/`crafting-state-machine.md`/`crafting-wire-formats.md`,
  `ability-trainer-ui.md` (AT-E1, the shared `onErrorCode` open question),
  `ability-resolution-pipeline.md` (the `FUN_00c6e220` clock reference);
  `.claude/agent-memory/combat-systems-advisor/ontimerupdate-wire-and-clock.md`;
  `entities/defs/SGWPlayer.def`, `entities/defs/enumerations.xml`; client Lua under
  `Content/UI/Core/Crafting/`, `Content/UI/Core/DisciplineTrainer/`; legacy
  `deprecated/python/{base,cell}/SGWPlayer.py`, `deprecated/python/cell/Crafter.py`,
  `deprecated/python/cell/commands/Crafting.py`; `crates/wire/src/cell/chat.rs`,
  `crates/cell-methods/src/cell/cell_methods/player/vendor/wire.rs` (existing feedback-channel
  precedent).

## What this packet is and is not

Documentation plus two Ghidra comment fixes only. No Rust was touched; no tests were run (nothing
compiles differently). Ghidra MCP against the already-open `SGW.exe` program and static reads of
the client Lua tree; no live client was running, so no x64dbg was used, per the rules file
("x64dbg only if a live client is running AND only non-freezing log breakpoints").

## Answers (full evidence and citations in `crafting-client-ui.md`)

1. **Respec (D-CR16).** The shipped Lua sends `respecCrafting()` (100, no args) exactly **once**,
   only from the Yes button of the cost-confirmation dialog (`Crafting.lua:187-191`), which itself
   only appears in response to receiving server method 112. There is no in-game UI trigger for the
   *first* call. The binary retains a separate `Event_SlashCmd_RespecCraft` console-command
   scaffold with no located handler body (same class of dead end as AT-E1's `onErrorCode`
   question). **Conclusion: this is evidence FOR D-CR16 as drafted**, not a correction — the
   client's single zero-arg opcode forces the server to distinguish query-vs-confirm by
   server-side pending-state, exactly as D-CR16 proposes. Confidence: HIGH for the send path,
   MEDIUM for why/how the first call would ever be triggered in a real client session.
2. **`CraftingOptions` (140).** Confirmed the full unpacker chain
   `0x00e49180` → `0x00e48de0` → `0x00e47250`, the section order (crafting/alloying/research/
   reverseEngineering at offsets `+0x38/+0x40/+0x48/+0x50`), and that the "last array element wins"
   behavior applies to both `items` and `entities`. Confirmed the client does **no** distance or
   entity-validity check — `isCraftingAllowed`/`onCraftingAllowedUpdate` is a nonzero check only,
   and the "machine" alias registration (`HVSystemOptionPolicyEnum::...@0x00c67bd0`) never
   validates the id. An arbitrary id (including the player's own, as `.allcraft` used) passes
   every client-side gate. Confidence: HIGH.
3. **Feedback.** `onErrorCode`'s native-side rendering remains **unresolved**, identically to
   AT-E1 — no Lua consumer, no embedded `CONDITION_FEEDBACK` string, and `EErrorCodeSystem` has
   only `ERRORCODE_SYSTEM_Ability = 0` (no crafting-specific system), so a code-213/214 send under
   system 0 would have the client read `InstanceID` as an ability id if anything reads it at all.
   `Mercury__unknown_00ceae50` is confirmed to be an **internal log write** (severity-coded record
   through the generic CME emit dispatcher), not a chat/screen path — its caller list spans Mercury
   protocol internals and `ZipFileSystem` async loading as well as every crafting client-check this
   packet traced. The path that **does** reach the player's chat is the legacy
   `SGWPlayer.feedback(msg)` → `onPlayerCommunication('', 0, CHAN_feedback, msg)`, already wired on
   the Rust side (`crates/wire/src/cell/chat.rs`) and already used by the vendor-rejection path.
   Confidence: HIGH that `Mercury__unknown_00ceae50` is not chat; unresolved (inherited from AT-E1)
   on `onErrorCode` rendering.
4. **Client clock `FUN_00c6e220` — CLOSED same day.** Traced to `FUN_00dd6c60`, an
   integer-tick-plus-interpolation clock read from a `ServerConnection`-owned singleton
   (`DAT_01ef2264`). Originally left open (the three `ClientMessageHandler` bodies had no direct
   code cross-references — vtable-dispatched only, and a dispatch-table walk was out of this
   packet's budget). **CR-02 live-traced all three handlers and shared the result**; CR-E1
   independently confirmed via `disassemble_bytes` at `0x00dd62a0`
   (`updateFrequencyNotification` — writes the `DAT_01e51cb8` tick-rate divisor from a `UINT8`),
   `0x00dd6820` (`setGameTime` — writes `+0x32c`/`+0x330` as a low16/high split of `gameTime`,
   `+0x334`/`+0x33c` the interpolation baseline), and `0x00dd6d00` (`tickSync` — writes
   `+0x32c` = full 32-bit `gameTime`, `+0x334`/`+0x33c`/`+0x34c` the baseline/pointer, `+0x344` =
   `tickRate * 0.001`, confirmed by reading `DAT_01848ab8`'s raw bytes). All three write the exact
   struct `FUN_00dd6c60` reads. Created + named + commented all three in Ghidra
   (`ClientMessageHandler_updateFrequencyNotification`/`_setGameTime`/`_tickSync`). Confidence:
   HIGH. **D-CR20's open item is resolved**: the client clock is server-message-driven; CR-02 can
   build `game_time_secs()` against it directly. See `crafting-client-ui.md` §4 for the full
   per-handler table.
5. **Alloy.** Confirmed the four counts (Normal 2000→10, Good 3000→5, Great 4000→2,
   Fantastic 5000→1, no Poor) and stack-quantity decrementing exactly as C-38 states. **Refines**
   C-38: the client rejects locally not just when zero quality buckets are satisfied but also when
   **two or more** are satisfied simultaneously ("Multiple categories... were met") — a rule audit
   C-38 didn't call out. Confirmed `aCurrentTierItemId` = first element of the current-tier UI slot
   and `aLowerTierItems` = the post-validation accumulator, sent regardless of local validation
   result (C-40 pattern). Confidence: HIGH.
6. **Research kickers.** Confirmed the one-per-science / not-own-science rule and the
   tech-competency-blind chance display, but **located the mechanism precisely**: it lives entirely
   in `ResearchPage.lua` (`addKickerToWindow`, an explicit science-equality check plus one
   dedicated UI window per science, so "one per science" is structural, not counted). The native
   sender's own re-validation (`Mercury__unknown_00e483f0`) does a **different, weaker** check — a
   generic `std::set<int>` dedup keyed on raw kicker item ids, not applied-science ids, so it
   cannot actually enforce the science rule despite reusing the same error string. No "+1
   Expertise" string or number exists anywhere in the crafting UI; kickers only move the displayed
   chance percentage. Confidence: HIGH.

## Corrections applied

- **C-61**: `Crafting_isCraftTypeAllowed` (`0x00e465d0`)'s `PRE_COMMENT` claimed its four returned
  offsets (`+0x38/+0x40/+0x48/+0x50`) were "blueprint known-crafts lists". They are the (tool,
  machine) `CraftingInfo` pairs message 140 populates. Fixed in Ghidra (comment now on the function
  entry address `0x00e465d0`, with a short pointer comment left at `0x00e465d6` where the old text
  had also been visible) and in `crafting-restoration.md`'s craft-type table.
- **C-62**: `crafting-wire-formats.md` typed `craftingEntityFlags` as `INT32`; `SGWPlayer.def:325-329`
  declares it `PYTHON` (matching `craftingOptions`). Fixed.

## Known gaps / open items (see `crafting-client-ui.md` "Open questions" for the full list)

1. `Event_SlashCmd_RespecCraft`'s console-command string binding and handler body — not traced
   (would need a `SGWTextCommandMgr` registration-table walk or a `vfunc_5` invoke-dispatch trace).
2. `onErrorCode` native rendering — unresolved, inherited from AT-E1, applies identically here.

Q4 (the client clock's message-driven setter) is no longer open — closed 2026-09-26 jointly with
CR-02 (see item 4 above and `crafting-client-ui.md` §4).

## Ghidra editing note

`mcp__ghidra__set_decompiler_comment` **appended** rather than replaced when first called at a
different address (`0x00e465d6`) than where the original mislabeled comment actually lived
(`0x00e465d0`'s `PRE_COMMENT`, distinct from its `PLATE` comment, which `get_plate_comment` does
not surface). Re-setting the comment at the correct address (`0x00e465d0`) produced the expected
replacement. Left a short pointer comment at `0x00e465d6` rather than leaving the old wrong text
or a duplicate wall of text. Anyone editing this function's comments again should check both
addresses.

**Follow-up (2026-09-26, same day):** after CR-02 shared its live-traced Q4 finding, verified all
three `ClientMessageHandler` bodies via `disassemble_bytes` (Ghidra had not auto-created function
boundaries for any of them — each is reached only via a data-referenced dispatch-table slot, not a
direct `CALL`) and used `create_function` + `set_plate_comment` to land
`ClientMessageHandler_updateFrequencyNotification` (`0x00dd62a0`),
`ClientMessageHandler_setGameTime` (`0x00dd6820`), `ClientMessageHandler_tickSync`
(`0x00dd6d00`) permanently in Ghidra. `create_function`'s naming-convention linter warned these
names aren't verb-first PascalCase (its suggested rewrite, `ClientmessagehandlerTicksync` etc., is
worse for readability and inconsistent with the rest of this binary's existing
`ClassName_methodName` naming, e.g. `Crafting_isCraftTypeAllowed`) — kept the descriptive names,
warnings are non-blocking.

## Commands run

- Read-only: `grep`/`sed`/`file` over the repo and the client Lua tree; no cargo commands (no Rust
  changed).
- `file` on every doc touched, to confirm CRLF line endings before and after edits.

## Integration notes for the coordinator

- No contended files touched. `crafting-restoration.md`/`crafting-wire-formats.md` edits are
  additive corrections (new paragraphs/notes), not restructuring — should merge cleanly regardless
  of CR-01's parallel edits to Rust files in the same area.
- CR-02's clock question is now answered (finding §4, updated 2026-09-26): the client's game clock
  is driven by `setGameTime`/`tickSync` (tick counter + interpolation baseline) and
  `updateFrequencyNotification` (the seconds divisor) — CR-02 can build `game_time_secs()` directly
  against the confirmed field layout rather than treating D-CR20's fallback as necessary.
- CR-04/CR-08's feedback wiring (D-CR14) should route through the `CHAN_feedback` /
  `onPlayerCommunication` path (already used by the vendor rejection code), not rely on
  `onErrorCode` alone, per finding §3.
