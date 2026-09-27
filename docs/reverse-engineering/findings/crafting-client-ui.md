---
title: "Finding: Crafting Client UI Evidence (CR-E1)"
type: reference
audience: contributors doing RE, crafting campaign workers
last_updated: 2026-09-27
---

# Finding: Crafting Client UI Evidence (CR-E1)

> **Status**: All six packet questions answered with direct evidence (Ghidra decompiles of
> `SGW.exe` plus the client Lua source under `Content/UI/Core/Crafting/` and
> `Content/UI/Core/DisciplineTrainer/`). Q4's client-clock domain, originally left open pending a
> `ClientInterface` dispatch-table walk or a live x64dbg watch, was **closed the same day** by
> CR-02's live trace plus this finding's independent static confirmation (§4) — see the update
> note below.
>
> **Binary**: `SGW.exe` (32-bit x86 PE). **Client Lua**: `Content/UI/Core/Crafting/*.lua`,
> `Content/UI/Core/DisciplineTrainer/DisciplineTrainer.lua`.
> **Companion campaign docs**: `docs/analysis/crafting/work-packets.md` (CR-E1),
> `docs/analysis/crafting/audit.md` (C-30..C-40, C-61, C-62), `docs/analysis/crafting/README.md`
> (D-CR16, D-CR20).
> **Prior findings this builds on**: `docs/reverse-engineering/findings/crafting-restoration.md`,
> `crafting-state-machine.md`, `crafting-wire-formats.md`, `ability-trainer-ui.md` (AT-E1, the
> `onErrorCode` open question this reuses), `ability-resolution-pipeline.md` (the `FUN_00c6e220`
> clock).
>
> **Update (2026-09-26, same day):** CR-02 (bigworld-engine-advisor) live-traced the three
> `ClientMessageHandler` bodies §4 originally could not locate and shared the result; CR-E1
> independently confirmed all three via static `disassemble_bytes` and landed the corresponding
> Ghidra function creation + naming + comments (`ClientMessageHandler_updateFrequencyNotification`
> @ `0x00dd62a0`, `ClientMessageHandler_setGameTime` @ `0x00dd6820`,
> `ClientMessageHandler_tickSync` @ `0x00dd6d00`). §4 below is the closed version; no contradiction
> was found.

---

## 1. Respec flow — confirmed single send, no in-game trigger for the query (Q1, feeds D-CR16)

**Confidence: HIGH for the shipped send path; MEDIUM for the open "how does the query ever
fire" question.**

The client Lua ships exactly **one** call site for `respecCrafting()` (cell method 100, no
args) in the entire UI tree: `Crafting.lua:187-191`.

```lua
function CraftingMod.onCraftingRespecPrompt(this)
    local message = localize("Crafting","RespecConfirmation", {NaqAmount=getCraftingCostToRespec()})
    PromptMod.showPrompt( localize("Crafting","Respec"), message, localize("Global","Yes"), localize("Global","No"), CraftingMod.onRespecAccepted )
end

function CraftingMod.onRespecAccepted( promptType, index )
    if index == PromptMod.BUTTON_YES then
        respecCrafting()
    end
end
```

`onCraftingRespecPrompt` is itself the **handler for server method 112** (`onCraftingRespecPrompt`,
`INT32 CostToRespec`) — confirmed by decompiling its native counterpart at `0x00e476f0`, which
stores the cost at `this+0x64` (`*(undefined4 *)((int)this + 100) = uStack_38;`) before firing
the UI event that reaches this Lua handler. So the only way the confirm dialog appears is in
**response to the server already having sent 112** — there is no Lua code path that requests 112
in the first place. `DisciplineTrainer.lua` (the discipline-tree window, analogous to the ability
trainer) has **no** respec button or `respecCrafting` reference at all (grepped clean).

Tracing the native sender confirms the full chain and that it fires exactly once per click:

- Lua `respecCrafting()` → arg-check shim `FUN_00aaafe0` (guards against wrong arg count, prints
  `"#ferror in function 'respecCrafting'."` on failure) → `FUN_00aeaf10`, which allocates and
  constructs an `Event_NetOut_RespecCraft` object (`FUN_00aea370`) and emits it exactly once via
  the CME dispatcher (`thunk_FUN_0054c900` → `FUN_00ae04e0`).
- `SGWNetworkManager`'s subscriber for that event (`SGWNetworkManager_VEvent_NetOut_RespecCraft___EventHandler__vfunc_0`
  @ `0x00d68450`) is the wire serializer; it carries no payload (matches the `.def`'s zero-argument
  `respecCrafting`).

**What the binary retains but the shipped UI never calls**: a second, distinct CME event class,
`Event_SlashCmd_RespecCraft` (RTTI `0x01dbbbb0`, string `Event_SlashCmd_RespecCraft` at
`0x018417f0`), with a full `SGWTextCommandMgr` `MemberCallback` subscription
(`.?AV?$MemberCallback@UNoSubject@EventSignal@CME@@VSGWTextCommandMgr@@...@Event_SlashCmd_RespecCraft@@...`
at `0x01e01540`). This is the same architecture as the ~256 other `/`-console slash commands
(see `reference_slash_command_registry` memory) — a dev/QA console command, presumably
`/respecCraft` or similar, that would let a tester trigger the cost-prompt query without a UI
button. Its two vtable slots (`Event_SlashCmd_RespecCraft__vfunc_2/3` @ `0x0059ab10`/`0x0059a9f0`)
have **no direct code cross-references** beyond their own vtable data entries — meaning the
actual command-string-to-handler binding lives inside `SGWTextCommandMgr`'s registration table,
which this pass did not walk (same class of open item as AT-E1's unresolved `onErrorCode`
listener; would need a `vfunc_5` invoke-dispatch trace per `cme-event-signal.md`).

**Conclusion for D-CR16**: the client evidence **supports** the two-step design as already
drafted, rather than correcting it. Because the only wire message is a zero-argument `respecCrafting`
(100) and the shipped UI can only *send* it from the Yes button of a dialog the server itself must
have already prompted, the server has no way to distinguish a "query" call from a "confirm" call
by payload — it **must** be state-tracked server-side (a pending-respec flag with a timeout), exactly
as D-CR16 proposes. Nothing in the client contradicts "first call is treated as a query, a second
call while pending executes it."

**Legacy Python**: `grep -rn -i respec deprecated/python/cell/Crafter.py deprecated/python/cell/commands/Crafting.py`
returns nothing — the legacy server never implemented crafting respec at all, consistent with
`feedback_legacy_server_no_gate_travel`-style gaps elsewhere in this codebase.

---

## 2. `CraftingOptions` (140) unpacker chain and the C-61 correction (Q2)

**Confidence: HIGH — full decompile of the three-function chain.**

`onUpdateCraftingOptions` (client method 140, payload `CraftingOptions` FIXED_DICT) is consumed
by `FUN_00e49180(this, param_1)`, decompiled in full. It extracts the top-level `"aOptions"`
property into a `CME::BasicPropertyTree`, then calls `FUN_00e48de0` **once per craft-type
section**, writing the result into four fixed offsets on `this` (the `VCrafting` instance):

| Section name (wire) | Target offset | Unit-alias code passed | Craft type (from §below) |
|---|---|---|---|
| `"crafting"` | `this+0x38` | `0x16` (22) | `CraftBlueprint` (1) |
| `"alloying"` | `this+0x40` | `0x15` (21) | `CraftAlloy` (8) |
| `"research"` | `this+0x48` | `0x18` (24) | `CraftResearch` (2) |
| `"reverseEngineering"` | `this+0x50` | `0x17` (23) | `CraftReverseEng` (4) |

`FUN_00e48de0(param_1, tree, sectionName, unitAliasCode)` extracts that section's `"items"` sub-array
into `*param_1` (offset 0, the *tool*) and its `"entities"` sub-array into `param_1[1]` (offset +4,
the *machine*), each via `FUN_00e47250`.

`FUN_00e47250(param_1, tree, fieldName, unitAliasCode)` is the function that produces C-35's
"keeps only the last id" behavior, now precisely characterized: it walks the `ARRAY<INT32>`
property node as a linked list and on **every** iteration overwrites `*param_1` with the current
element's value (`*param_1 = *(int *)(iVar2 + 8);`) — there is no accumulation, so after the loop
only the *last* array element survives. When `unitAliasCode != 0` (i.e. only for the `"entities"`
field, never `"items"`), it also calls
`HVSystemOptionPolicyEnum__unknown_00c67bd0(GameEntityManager::instance(), unitAliasCode, lastEntityId)`,
which registers that raw entity id as the alias for a `Unit.*` enum slot (`Unit.CraftBlueprint`,
`Unit.CraftAlloy`, etc., per `Crafting.lua:203-206`'s `TypeUnitMap`) and fires
`Event_UI_UnitMappingChanged`. **This registration performs no existence or range check on the
id** — it is a raw int→alias bind through a generic map insert (`BW__unknown_00c6c500`).

### C-61 correction (applied to Ghidra and this doc)

`Crafting_isCraftTypeAllowed` (`0x00e465d0`) — the native behind Lua's `isCraftingAllowed(craftType)`
— returns exactly these same four offsets by the same craft-type enum (`case 1` → `+0x38`, `case 8`
→ `+0x40`, `case 2` → `+0x48`, `case 4` → `+0x50`). The pre-existing Ghidra `PRE_COMMENT` at
`0x00e465d0` (recovered 2026-06-20) labelled these offsets **"blueprint known-crafts list"**. That
is wrong: they are the **(tool, machine) `CraftingInfo` pairs populated over the wire by message
140**, not a client-side cache of known blueprint ids. The comment has been corrected in Ghidra
(see the `CORRECTION (2026-09-26...)` block now on the function's PRE_COMMENT) and in
`crafting-restoration.md`'s craft-type table (see that file's changelog). The **actual**
known-blueprints accessor is the separate `Crafting_getKnownBlueprints` @ `0x00e46830`, which this
pass did not need to re-decompile (its switch structure and static-placeholder sharing were
already correctly documented).

### Client-side distance/validity checks: none found

`Crafting.lua`'s `onCraftingAllowedUpdate` (the only consumer of `isCraftingAllowed`) does:

```lua
local tool, machine = isCraftingAllowed(craftType)
if 0 ~= machine then ... -- "Machine: <name>", green
elseif 0 ~= tool then ... -- "Tool: <name>", green
else ... -- "Disabled", red
end
```

This is a **nonzero check only** — no distance math, no verification that the id resolves to a
live, in-range entity. Both the Lua consumer and the native `isCraftingAllowed`/unpacker chain
just return whatever the server last sent. Combined with the alias registration having no
existence check either, **an entity id that is not a real nearby machine — including the
player's own entity id, as the legacy `.allcraft` GM command sent (`D-CR17`) — passes every
client-side gate as long as it is nonzero.** All range/legitimacy enforcement is necessarily
server-side, consistent with C-40.

---

## 3. Feedback visibility — `onErrorCode` unresolved (as in AT-E1); `Mercury__unknown_00ceae50` is an internal log, not chat (Q3)

**Confidence: HIGH that `Mercury__unknown_00ceae50` is not a chat/screen path — it is the same
open question as AT-E1 for whether `onErrorCode` renders anything.**

> [!NOTE]
> **Superseded in part (2026-09-27).** The "no behavioral handler exists" half of the AT-E1 result
> restated below no longer holds. A native `FreeCallback` subscriber bound to a `Communicator*` is
> compiled into the client (RTTI type-name string at `0x01e20da8`), so `Event_NetIn_onErrorCode`
> has a native listener. Whether it renders codes 213/214 is **still unresolved**. Full evidence:
> [ability-trainer-ui.md §2, "A native subscriber exists"](ability-trainer-ui.md#a-native-subscriber-exists-rtti-2026-09-27);
> the mechanism is in [cme-event-signal.md](cme-event-signal.md).

### `onErrorCode` (client method 121) with codes 213/214

`entities/defs/enumerations.xml` confirms `CONDITION_FEEDBACK_EnoughAppliedSciencePoints = 213`
and `CONDITION_FEEDBACK_NotEnoughAppliedSciencePoints = 214`. But `EErrorCodeSystem`
(`enumerations.xml:1203`) has **exactly one token**, `ERRORCODE_SYSTEM_Ability = 0` — the same
fact AT-E1 already established for the ability trainer. There is no crafting-specific
`ERRORCODE_SYSTEM` value. This finding does not re-derive AT-E1's negative result (no Lua consumer
for `Event_NetIn_onErrorCode`, no `CONDITION_FEEDBACK` string embedded anywhere in the binary,
only the registration/RTTI stubs `register_NetIn_onErrorCode` @ `0x00d77f00` and
`CME_EventSignal_...vfunc_0` @ `0x00d77fe0`) — it still holds, and crafting's use of `onErrorCode`
inherits the same open question: **it is unresolved whether any native (non-Lua) listener renders
codes 213/214 at all.** (Since 2026-09-27: a native listener exists; what it renders is the open
part.) What this finding adds: under `SystemID = 0` (the only token that exists),
AT-E1 confirmed the client reads `InstanceID` as an **ability id**. If a crafting `onErrorCode`
send reuses `SystemID = 0`, the client will interpret whatever `InstanceID` the server sends as an
ability id — a domain mismatch worth flagging if `onErrorCode` is used for crafting rejections at
all, independent of whether it renders anything.

### `Mercury__unknown_00ceae50` — confirmed to be an internal log write, not a chat/screen print

Decompiling it and its callee `Mercury__unknown_00ceac90` shows it builds a formatted log record
(severity code `0x9`, an empty-format-args wide string) and posts it through the same generic CME
emit dispatcher used throughout the engine (`thunk_FUN_0054c900` → `FUN_00cf99a0`) — the same
plumbing pattern as the crash/exception paths elsewhere in this binary, not the
`onPlayerCommunication`/`writeLocalFeedback` chat path AT-E1 already identified as the client's
actual visible-text mechanism.

Its caller list (over 90 sites) is decisive: it is called from **every crafting client-check
function this packet inspects** (`FUN_00e47b10`, `FUN_00e46e80`, `FUN_00e47ec0` — alloy sender,
`Mercury__unknown_00e46990` — alloy counts, `FUN_00e48a90`, `Mercury__unknown_00e483f0` — research
kicker sender) **and** from dozens of unrelated Mercury protocol-internals and `ZipFileSystem`
async-loading functions. A log sink shared between crafting UI validation, low-level network
protocol code, and file-loading code is a generic internal warning/debug log, not a player-facing
message. This directly confirms audit C-40's "logs a warning ... and sends anyway" for every
crafting sender this packet traced.

### The server text path that *does* show in chat

The legacy Python confirms the one path that unambiguously reaches the player's chat window:
`SGWPlayer.feedback(msg)` (both `deprecated/python/base/SGWPlayer.py:64` and
`deprecated/python/cell/SGWPlayer.py:362`) sends
`client.onPlayerCommunication('', 0, Atrea.enums.CHAN_feedback, msg)`. This is the generic
system-feedback chat channel, already wired on the Rust side (`crates/wire/src/cell/chat.rs`,
consumed via `crates/wire/src/cell/messages/base_to_cell.rs` and `map_loaded.rs`) and already used
by at least the vendor-rejection path
(`crates/cell-methods/src/cell/cell_methods/player/vendor/wire.rs`). **This — not `onErrorCode` —
is the D-CR14 "readable text line" mechanism crafting rejections should reuse**; `onErrorCode`
should be treated as "correct-but-unverified presentation" exactly as AT-E1 already concluded for
the ability trainer, not as the primary feedback channel.

---

## 4. The client clock `FUN_00c6e220` — CLOSED: all three sync messages feed it (Q4, feeds CR-02 / D-CR20)

**Confidence: HIGH — the open item below was closed same-day by CR-02 (live trace) and confirmed
independently here via static disassembly of all three handler bodies.**

`FUN_00c6e220(int param_1) { FUN_00dd6c60(*(int *)(param_1 + 0x28)); }` is a thin wrapper.
`FUN_00dd6c60` reads four fields from the struct it's handed:

```text
tickCount   = *(int    *)(struct + 0x32c)   // wraps via +2^32 if negative (int32 rollover)
lastBase    = *(double *)(struct + 0x334)
interval    = *(double *)(struct + 0x344)
current_ptr = **(double**)(struct + 0x34c)  // dereferenced pointer to a live double
frac        = interval > 0 ? (current_ptr - lastBase) / interval : 0
result      = (tickCount_as_float + frac) / DAT_01e51cb8   // DAT_01e51cb8 = the tick-rate/"hertz" constant
```

This is the shape of a BigWorld-style engine clock: an integer tick counter plus a sub-tick
interpolation fraction, converted to seconds by dividing by the tick rate constant. The object
`FUN_00c6e220` reads from is reached through a lazily-constructed global singleton
(`DAT_01ef2264`, built by `Mercury__unknown_00c6f870` / `FUN_00c6f690`), and is a
`ServerConnection`-owned sub-struct.

`FUN_00c6e220`'s caller list spans systems that have nothing to do with server time sync —
`ZipFileSystem__unknown_00e09160` (async package/file loading), numerous unlabelled `Mercury*`
protocol-internals functions, and (relevantly) crafting's own `onTimerUpdate` handler
`FUN_00e47800` and multiple ability-cooldown computations (e.g. `FUN_00c6bc20`, which computes
`abilityCooldownTarget - FUN_00c6e220(...)`). This is exactly what a shared, general "current game
time in seconds" accessor looks like — it is not crafting-specific or ability-specific, it is the
one clock every timer-remaining computation in the client uses.

### The setters — confirmed (CR-02 live trace + CR-E1 static disassembly)

CR-02 (bigworld-engine-advisor) traced this live against a running client while investigating
CR-02's own game-clock packet, and reported the three `ClientMessageHandler` bodies this finding
originally could not locate (their RTTI descriptors have no direct code cross-references — they
are reached only through `ClientInterface`'s message dispatch table, which is why Ghidra had not
auto-created function boundaries for any of them). CR-E1 independently verified all three via
`disassemble_bytes` and created + named + commented the functions in Ghidra
(`ClientMessageHandler_updateFrequencyNotification` @ `0x00dd62a0`,
`ClientMessageHandler_setGameTime` @ `0x00dd6820`, `ClientMessageHandler_tickSync` @ `0x00dd6d00`).
All three write into the exact same `ServerConnection` sub-struct `FUN_00dd6c60` reads:

| Handler | Confirmed writes |
|---|---|
| `updateFrequencyNotification` (`0x00dd62a0`) | Reads one `UINT8` (ms-per-tick) from the arg, converts to float (`CVTSI2SS`), stores into `DAT_01e51cb8` — the "hertz"/tick-rate divisor `FUN_00dd6c60` uses. Confirmed byte-for-byte: `MOVZX ECX,[EAX]` → `CVTSI2SS XMM0,ECX` → `MOVSS [0x01e51cb8],XMM0`. |
| `setGameTime` (`0x00dd6820`) | `+0x32c` = **low 16 bits** of the arg's `gameTime` (`MOVZX EDX,AX; SUB EAX,EDX` splits it), `+0x330` = the high part — a deliberate word-split encoding, not a truncation bug. `+0x334` = the struct's previous `+0x33c` value (rolls the interpolation baseline forward). `+0x33c` = `*(this+0x174)` if that pointer is non-null, else `0.0`. |
| `tickSync` (`0x00dd6d00`) | Args: `UINT32 gameTime, UINT32 tickRate`. `+0x32c` = `gameTime` (full 32-bit this time, unlike `setGameTime`'s split). `+0x334` = previous `+0x33c` value. `+0x34c` = `*(this+0x174)` (the pointer itself, unconditionally). `+0x33c` = `*(that pointer)` (a fresh "now" double). `+0x344` = `tickRate` (with the standard negative-int32 `+2^32` wraparound fix) `* DAT_01848ab8` — confirmed ≈ `0.001` by reading the raw bytes (`3F5062...`, an IEEE-754 double in the `2^-10` magnitude band), i.e. `tickRate` is milliseconds and `+0x344` becomes seconds-per-tick, matching `FUN_00dd6c60`'s `interval` field exactly. |

**Conclusion**: the client's game clock **is** driven by the server's sync messages, closing the
open item this finding originally flagged. `setGameTime` and `tickSync` both feed the same
tick-counter/interpolation-baseline fields (`+0x32c`/`+0x334`/`+0x33c`/`+0x34c`) that
`FUN_00dd6c60` reads, and `updateFrequencyNotification` feeds the divisor
(`DAT_01e51cb8`) that same function uses to convert to seconds — the "general engine clock" read
in the caller graph above is general in the sense that *every* system reads the same clock, not in
the sense that it is locally driven. **CR-02 should build `game_time_secs()` to match this: an
absolute tick count (matching `tickSync`'s `gameTime`) plus a millisecond-resolution `tickRate`
(the `updateFrequencyNotification` divisor), with `setGameTime`'s odd low/high word split noted as
a client quirk to reproduce if `setGameTime` is the message the server sends at login** (per the
combat-advisor's `ontimerupdate-wire-and-clock` memory, `build_time_sync` currently hardcodes all
three messages to zero at login — this is the gap CR-02 closes).

---

## 5. Alloy — counts, stack-quantity counting, and the wire payload (Q5, C-38)

**Confidence: HIGH — full decompile of both the validation function and the sender.**

### Counts and the "exactly one quality" rule (refines C-38)

`Mercury__unknown_00e46990(container, currentTierMinusOne, lowerTierItemsArray)` builds a local
map keyed by quality id with required counts:

| Quality id | Name | Required count |
|---|---|---|
| 2000 | Normal | 10 |
| 3000 | Good | 5 |
| 4000 | Great | 2 |
| 5000 | Fantastic | 1 |

**No `1000` (Poor) entry exists** — confirms C-38's "no Poor" exactly. For each id in
`aLowerTierItems`, the client looks up the item, checks its tier equals `currentTier - 1`
("Elementary components must be one tier lower than the component" on mismatch), and
**decrements the matching quality bucket by the item's stack quantity** (`*(int *)(iVar2 + 0x10)`,
not by 1 per item) — confirms "counts stack quantity" precisely.

After processing all items, the client counts how many of the four buckets reached ≤ 0
(fully satisfied). This refines C-38's "requires one quality's count to be met":

- **Zero** buckets satisfied → error `"The quantity of elementary components per item quality was
  not met"`.
- **Two or more** buckets satisfied → error `"Multiple categories of elementary components were
  met"` — a rule audit.md did not previously call out: submitting *enough* of two different
  qualities is **also** rejected locally, not just submitting too few of any one.
- Exactly one bucket satisfied → no local error.

Both error paths log via `Mercury__unknown_00ceae50` (§3) and **do not stop the send** — the
build-and-submit code below runs regardless of the local validation result (C-40 pattern, confirmed
again here).

### `aCurrentTierItemId` / `aLowerTierItems` (from the sender `FUN_00e47ec0`)

- `aCraftId`: forwarded from a stored selection struct on `this` (`*(param_1 + 0xc)`), i.e. the
  alloy recipe currently selected in the UI.
- `aCurrentTierItemId`: the **first** element of whatever container backs the "current tier" UI
  slot (`if (container empty) id = 0; else id = *firstElement;`) — a single-item slot, matching
  the one-drop-target design implied by `AlloyPage.lua`.
- `aLowerTierItems`: **not** the raw pre-validation input — it is the accumulator array
  (`FUN_00e12c60`) that the same validation loop above builds by appending each looked-up item
  unconditionally (the append call is outside the two error-check branches), so it is
  functionally the same list of ids the caller supplied, just re-walked through the local checks.
  Per C-40, this is sent whether or not the local Normal/Good/Great/Fantastic check passed.

---

## 6. Research kickers — one-per-science lives entirely in Lua; the native sender's own recheck is a weaker item-id dedup (Q6, C-39, corrects a detail of C-39)

**Confidence: HIGH for the Lua-side rule (direct source read); HIGH for the native sender's
actual (different) check, from full decompile of `Mercury__unknown_00e483f0`, `FUN_00d352d0`
(a generic `std::set<int>`-style insert-or-find), and `FUN_00d20c90` (item → applied-science
accessor).**

### The real one-per-science / not-own-science rule: enforced in Lua, before the sender ever runs

`ResearchPage.lua` maintains `ResearchMod.scienceToWindow` (science id → UI drop-window) and
`ResearchMod.kickers` (window → item id). `addKickerToWindow` (`:196-219`) is the actual gate:

```lua
function ResearchMod.addKickerToWindow( destWindow, itemID )
    if true ~= getItemIsKicker(itemID) then
        return PromptMod.showPrompt(..., localize("Crafting","NotAKicker"), ...)
    end
    local kickerScience = getItemScience(itemID)
    if ResearchMod.pendingItem and getItemScience(ResearchMod.pendingItem) == kickerScience then
        return PromptMod.showPrompt(..., localize("Crafting","NoKickerFromSameScience"), ...)
    end
    if ResearchMod.scienceToWindow[kickerScience]:getID() ~= destWindow:getID() then
        return PromptMod.showPrompt(..., localize("Crafting","InvalidKickerMessage"), ...)
    end
    ResearchMod.kickers[destWindow] = itemID
    ...
end
```

This confirms C-39's Lua-side claim exactly: **the not-own-science check is explicit**
(`getItemScience(pendingItem) == kickerScience` → `NoKickerFromSameScience`), and the
**one-per-science cap is structural, not a counted check** — each science has exactly one
dedicated drop window (`scienceToWindow[scienceID]`), so a second kicker of the same science
can only ever replace the first in that same slot, never add a second entry. `Crafting.int`'s
`NoKickerFromSameScience = "Kickers cannot come from the same applied science as the item being
researched"` confirms the string.

`ResearchMod.calculateChance` (`:86-92`) also confirms the second half of C-39: `chance = 100 -
skill; for each kicker: chance = chance + 5` — **no reference to tech competency anywhere in the
displayed chance formula**, confirming the display ignores the tech-competency ceiling that
`D-CR15`'s server formula enforces.

### The native sender's own recheck is a *different*, weaker rule — worth flagging, not acting on

`Mercury__unknown_00e483f0(aItemId, aKickers)` does its own validation before building the wire
message:

1. Item lookup fails → throw (client-internal exception, not a player-visible path).
2. `FUN_00d20c50(item) == 0` → not researchable → log via `Mercury__unknown_00ceae50` (§3), text
   `"This item is not researchable"`.
3. **Before the kicker loop**, it computes the researched item's own applied-science id
   (`FUN_00d20c90(item)`, confirmed by decompile: reads item-def offset `+0x74`) and inserts it
   into a fresh `std::set<int>` (`local_60`, via `FUN_00d352d0` — confirmed by decompile to be a
   generic red-black-tree set insert/find, not a crafting-specific function).
4. For each kicker id: item lookup fails → throw; `FUN_00d20bf0(item) == 0` → not a kicker → log
   `"This item is not a kicker"`; otherwise it inserts the kicker's **raw item id** (not its
   science id — confirmed, `local_80 = *piVar8` reads directly from the `aKickers` array iterator,
   with no `FUN_00d20c90` call in between) into the *same* set, and if that insert finds the value
   already present, logs `"This applied science cannot have another kicker added"`.

Because step 3 seeds the set with a **science id** (a small enum value) and step 4 checks
**raw item ids** (large instance ids) against the same set, this native-side recheck cannot
realistically enforce "no kicker from the item's own science" — the two key domains do not
overlap. What it **does** enforce is a much narrower "the same kicker item id cannot appear twice
in one `aKickers` submission," reusing the same (misleadingly worded) error string. This is a
genuine mismatch between the error text and what the code checks, not a bug that affects gameplay
(the shipped Lua UI never lets a duplicate-science or duplicate-item submission reach this sender
in the first place, and D-CR15's server-side check must be authoritative regardless per C-40).
Recorded here so the crafting server implementation does not mistake this native function for
where the science-uniqueness rule "really" lives — it lives in `ResearchPage.lua`.

### The kicker's "+1 Expertise" text — does not exist client-side

A full grep of `Content/UI/Core/Crafting/` for `expertise`/`Expertise` finds only: the
`DisciplineExpertiseUpdate` event name, `getDisciplineExpertise()` calls, and the chance-formula
comment above. **There is no "+1 Expertise" string or number anywhere in the crafting UI.**
Kickers only ever affect the displayed **chance** percentage (`+5` per kicker); they have no
separate expertise-gain display. If `D-CR15`'s "+5 per successful research" expertise-gain rule
was the source of the packet's "+1 Expertise" phrasing, it does not correspond to any client-side
UI element — expertise gain on success is a server-only outcome the client never previews.

---

## Evidence trail

| Claim | Address / file:line | Method |
|---|---|---|
| `onCraftingRespecPrompt` (112) handler stores cost at `this+0x64` | `0x00e476f0` | Decompile |
| Sole `respecCrafting()` call site, gated behind Yes | `Crafting.lua:181-191` | Direct read |
| No respec reference in `DisciplineTrainer.lua` | grep, 2026-09-26 | Grep |
| `respecCrafting` Lua shim / sender | `0x00aaafe0` / `0x00aeaf10` | Decompile |
| `Event_NetOut_RespecCraft` wire handler (payload-free) | `0x00d68450` | Decompile |
| `Event_SlashCmd_RespecCraft` RTTI, no resolved handler body | `0x018417f0`, `0x01e01540`, `0x0059ab10`/`0x0059a9f0` | Search + xrefs |
| Legacy Python has no respec code | grep, 2026-09-26 | Grep |
| `CraftingOptions` unpacker chain | `0x00e49180` → `0x00e48de0` → `0x00e47250` | Decompile |
| Unit-alias registration, no existence check | `0x00c67bd0` (`HVSystemOptionPolicyEnum`) | Decompile |
| `Crafting_isCraftTypeAllowed` offsets = (tool,machine) pairs, not blueprint lists | `0x00e465d0` | Decompile + Ghidra comment fix |
| `onCraftingAllowedUpdate` — nonzero check only, no distance math | `Crafting.lua:31-48` | Direct read |
| `EErrorCodeSystem` has only `ERRORCODE_SYSTEM_Ability = 0` | `entities/defs/enumerations.xml:1203` | Direct read |
| `onErrorCode` has no Lua consumer, no embedded `CONDITION_FEEDBACK` string | `ability-trainer-ui.md` §2 | Existing finding (AT-E1) |
| `onErrorCode` has a native `FreeCallback` subscriber bound to `Communicator*` (behaviour unresolved) | RTTI type-name string `0x01e20da8`; `ability-trainer-ui.md` §2 | Headless Ghidra string search (2026-09-27) |
| `Mercury__unknown_00ceae50`/`...ac90` build an internal log record, not chat text | `0x00ceae50`, `0x00ceac90` | Decompile |
| `Mercury__unknown_00ceae50` caller list spans Mercury internals + ZipFileSystem + every crafting sender | xref sweep, 2026-09-26 | Xrefs |
| Legacy `feedback()` → `onPlayerCommunication(..., CHAN_feedback, msg)` | `deprecated/python/cell/SGWPlayer.py:362-368`, `base/SGWPlayer.py:64-67` | Direct read |
| `FUN_00c6e220` → `FUN_00dd6c60`: tick+fraction clock, callers span ZipFileSystem/Mercury/abilities/crafting | `0x00c6e220`, `0x00dd6c60` | Decompile + xrefs |
| `updateFrequencyNotification`/`setGameTime`/`tickSync` handlers confirmed writing the clock struct | `0x00dd62a0`, `0x00dd6820`, `0x00dd6d00` | Disassemble + Ghidra function creation (CR-02 live trace + CR-E1 static confirm, 2026-09-26) |
| `DAT_01848ab8` ≈ 0.001 (tickSync's ms→seconds tick-rate multiplier) | `0x01848ab8` | `read_memory` (IEEE-754 double bit pattern) |
| `DAT_01ef2264` singleton touched only by two accessor wrappers | `0x00c6f870`, `0x00c6f840` | Xrefs |
| Alloy quality counts (2000/3000/4000/5000 → 10/5/2/1, no Poor), stack-quantity decrement | `0x00e46990` | Decompile |
| Alloy "exactly one quality met" rule (0 or ≥2 satisfied buckets both reject) | `0x00e46990` | Decompile |
| Alloy sender: `aCurrentTierItemId` = first slot element, `aLowerTierItems` = post-loop accumulator | `0x00e47ec0` | Decompile |
| Research: not-own-science + one-per-science via dedicated windows | `ResearchPage.lua:196-219` | Direct read |
| Research chance formula ignores tech competency | `ResearchPage.lua:86-92` | Direct read |
| Native research sender's set-based recheck keys on raw item id, not science id (weaker than Lua's rule) | `0x00e483f0`, `0x00d352d0`, `0x00d20c90` | Decompile |
| No "+1 Expertise" string anywhere in crafting UI | grep, 2026-09-26 | Grep |

## Open questions

1. **`Event_SlashCmd_RespecCraft`'s command-string binding and handler body (Q1)** — not traced;
   would need a `SGWTextCommandMgr` registration-table walk or a `vfunc_5` invoke-dispatch trace
   per `cme-event-signal.md`.
2. **`onErrorCode` native rendering** — unresolved, same open item as AT-E1; applies identically
   to crafting's codes 213/214. Re-scoped 2026-09-27: the native listener exists (a `FreeCallback`
   bound to `Communicator*`); what it renders needs a live trace of the `Event_NetIn_onErrorCode`
   dispatch or a full GUI RTTI re-analysis. The feedback-channel line (§3) stays the recommended
   mechanism for crafting rejections either way.

Q4 (the client clock's message-driven setter) is **no longer open** — see §4's update note.
