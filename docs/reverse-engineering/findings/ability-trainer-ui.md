---
title: "Finding: Ability Trainer UI Client Evidence (AT-E1)"
type: reference
audience: contributors doing RE, ability-trees campaign workers
last_updated: 2026-10-04
---

# Finding: Ability Trainer UI Client Evidence (AT-E1)

> **Status**: Partial — four of five questions answered with direct evidence (client Lua source plus Ghidra decompiles); question 2 (`onErrorCode` rendering) is **UNRESOLVED**, marked explicitly below. **Update 2026-09-27:** a native (non-Lua) subscriber to `Event_NetIn_onErrorCode` does exist, found from RTTI; whether it renders anything is still open. See §2. Written under a tightened budget; no further Ghidra sweeps were run once the open items were identified — see "Open questions" for what would close them.
>
> **Binary**: `SGW.exe` (32-bit x86 PE). **Client Lua**: `Content/UI/Core/Ability/Ability.lua`, `Content/UI/Core/ExpBar/ExpBar.lua`, `Content/UI/Core/Trainer/Trainer.toc`.
> **Companion campaign docs**: `docs/analysis/ability-trees/work-packets.md` (AT-E1), `docs/analysis/ability-trees/audit.md` (A-21..A-25), `docs/analysis/ability-trees/README.md` (D-AT08, D-AT10).

---

## 1. How `getTrainableList`/`getTrainableInfo` combine the tree and the trainer list (Q1)

**Confidence: HIGH — confirmed by decompile of both native functions.**

The client exposes five Lua-bound natives for the trainer/ability system (a sixth, `getAbilityList`, was added to the table on 2026-10-04). All are registered via a `#ferror in function '<name>'` string next to the Lua-argument-check shim; the shim calls into a small inner function that does the real work:

| Lua-visible name | Shim (arg-check) address | Inner function address | Reads from |
|---|---|---|---|
| `getTrainingTreeCount()` | `0x00aa2ac0` | `0x00ad8700` | `GameEntityManager::instance()+0x8c → +0x50` |
| `getTrainableList(tab)` | `0x00aa2ba0` | `0x00add0a0` | same `+0x8c → +0x50` field, indexed by `tab-1` |
| `getTrainableInfo(id)` | `0x00aa2c20` | `0x00add1b0` | `GameEntityManager::instance()+0x8c → +0x50` (the entry) and `+0x3c` (`haveIt`); corrected 2026-10-04, see the note below |
| `getAbilityList(group)` | `0x00aa2740` | `0x00adb810` | `+0x8c → +0x3c → +0xc` (the known-ability ids; added 2026-10-04) |
| `buyTrainable(id)` | `0x00aa2ca0` | `0x00ad8720` (sender) | — |
| `respecAbilities()` | `0x00aa2d80` | `0x00aeacd0` (sender) | — |

`GameEntityManager::instance()` is `FUN_00c66ad0`, which asserts against `.\Src\GameEntityManager.cpp` and returns the singleton at `0x01ef244c` — already catalogued in `docs/reverse-engineering/address-map.md` as `g_EntityManager`. The two sub-fields at `+0x50` and `+0x3c` under the player entity pointer at `instance()+0x8c` are **two independently populated containers**:

- **`+0x50` is the ability-tree cache** — an array-of-arrays. `getTrainingTreeCount()` (`0x00ad8700`) returns its outer size; `getTrainableList(tab)` (`0x00add0a0`) bounds-checks `1 <= tab <= treeCount`, indexes the outer array at `tab-1`, then walks the inner array **in the order it was stored** (a plain begin→end iterator loop calling `FUN_00ada620` per element to build the returned Lua table). This container is populated from `onAbilityTreeInfo` (client method 141, `ARRAY<ARRAY<INT32>> AbilityLists`).
- **`+0x3c` is the trainer's offered-ability map** — a lookup keyed by ability id, populated from `onTrainerOpen` (client method 113, `ARRAY<TrainerAbility>` where `TrainerAbility = FIXED_DICT{INT32 abilityID, UINT8 trainable}`, `entities/defs/alias.xml:417-422`).

**`getTrainableInfo(id)` (`0x00add1b0`) is the join point**, and it is asymmetric:

1. It looks the id up in the **trainer map** (`+0x3c`). If the id is **not present**, the function returns *without ever writing any field* onto the Lua result table — no `id`, no `name`, nothing. Back in `Ability.lua:75`, `trainableInfo.id == nil` is exactly the guard that hides the button (`AbilityMod.refreshButton(nil, i, AbilityUnavailable)` → `buttonWin:hide()`). **This confirms A-23: a tree node absent from the trainer's offered list is hidden, not greyed** — down to the exact byte-level mechanism, not just the Lua-side symptom.
2. If the id **is** present, the function reads `id`/`name`/`description`/`icon`/`trainingCost` off the matched entry, then computes two fields the wire never carries directly:
   - `haveIt`: a **separate** lookup (via `FUN_00d2a000`) against the player's known-abilities set — i.e. "do I already know this ability" is **client-computed from the known-abilities cache** (populated by `onKnownAbilitiesUpdate`), not read off the trainer entry.
   - `trainable`: `NOT haveIt AND (wire trainable byte != 0)` — the exact byte the server sent in `onTrainerOpen`'s per-entry array (offset `+4` inside the 5-byte `TrainerAbility` entry), gated client-side by "don't offer to train something I already have."

**Order**: the tree window's button order comes from **`onAbilityTreeInfo`'s array order** (the tree), not the trainer's offered-list order — `getTrainableList` never touches the trainer map at all; it only walks the `+0x50` tree cache.

**Cap beyond `MAX_BUTTONS = 30`**: none found. The native loop that builds the Lua table for a tab (`FUN_00add0a0`) walks the *entire* inner array with no length ceiling; the only limit is `Ability.lua`'s own `for i=1,AbilityMod.MAX_BUTTONS do ... trainableList[i]` — ids beyond index 30 in a branch are simply never read. Confirms A-22's finding is a Lua-side cap only.

> [!NOTE]
> **Correction, 2026-10-04 (headless Ghidra decompile of `0x00add1b0`).** The offsets above are swapped for `getTrainableInfo`. It looks the entry up through `FUN_00e19890(player->+0x50)`, the same object `getTrainableList` reads, and computes `haveIt` with `FUN_00d2a000(player->+0x3c)`. So `+0x3c` is the player's **known-abilities** object: `FUN_00d2a000` looks the id up in its map at `+0x28`, and its id array sits at `+0xc`. The trainer map lives under `+0x50`, and the two bullets above that describe `+0x3c` as the trainer map are wrong on that point; the table is corrected.

### `getAbilityList`: the known-ability list (2026-10-04)

The stock UI Lua never calls it, but `getAbilityList` is a registered native (registered at `0x00ad4c84` inside `FUN_00acbb10`). Its shim `0x00aa2740` requires one argument, then calls `FUN_00adb810`, which ignores that argument. `FUN_00adb810` walks the id array at `player->+0x3c->+0xc` (`FUN_00d29d90` returns `+0xc`) and builds a Lua array with `FUN_00ada620`, the per-id pusher `getTrainableList` uses. The result is the client's known-ability ids, as `onKnownAbilitiesUpdate` filled them.

The `UIAbilityGroup` enum, registered next to it, is `None = 0`, `Training = 1`, `KnownAbility = 2`, `Inventory = 3` (getter constants: `FLDZ`, `FLD1`, double `2.0` at `0x01866fe0`, double `3.0` at `0x0183db60`). Client patch `009-starter-hotbar` calls `getAbilityList(UIAbilityGroup.KnownAbility)` to find the abilities to put on a new character's bar.

---

## 2. `onErrorCode` rendering — UNRESOLVED

**Confidence: HIGH that a native subscriber type is compiled in (RTTI, 2026-09-27); none established for what it shows — treat rendering as an open question, not a negative finding.**

What was checked, all with negative results:

- A case-insensitive search for `ErrorCode` across the **entire** client tree (`Content/UI` and the whole `SGWGame` working copy) returns **zero matches in any `.lua` file**. No UI module subscribes to any `Events.*` signal whose name mentions error codes.
- `writeLocalFeedback` — the native chat/system-feedback print function used elsewhere for user-facing text (`GateMail.lua:286`, `Social.lua:101-109`, `Trade.lua:308`, etc.) — has no call site anywhere near an error-code path.
- Ghidra function-name search for `.*ErrorCode.*`, `.*ConditionFeedback.*`, `.*ConditionHandler.*`, `.*ErrorFeedback.*` returns only the registration/RTTI stubs already known (`register_NetIn_onErrorCode` at `0x00d77f00`, `CME_EventSignal_...vfunc_0` at `0x00d77fe0`) — no behavioral handler with a suggestive name exists. **Superseded in part (2026-09-27):** the name search was right that no *named* handler exists, but a native subscriber does; see ["A native subscriber exists"](#a-native-subscriber-exists-rtti-2026-09-27) below.
- No `CONDITION_FEEDBACK` string is embedded in the binary at all (`search_strings` returns zero hits) — the enum names in `entities/defs/enumerations.xml` are a documentation reconstruction, not literal client-side text; there is no evidence the client ever had per-code localized strings for this system.

This is exactly the open question flagged in `docs/analysis/harset-rebuild/worknotes/H06.md`: *"nobody has traced `onErrorCode`'s handler in the binary."* This finding does not close it. What was ruled out: **no Lua-scripted consumer exists.** What remains open: whether a purely native (non-Lua) C++ listener is subscribed to `Event_NetIn_onErrorCode` and does something silent-but-real (e.g. a floating combat-text style flash, or a debug-only log), versus the event having zero listeners and being dropped entirely. Distinguishing those requires tracing the CME event's subscriber list at runtime or via the generic `vfunc_5` invoke-dispatch mechanism (see `docs/reverse-engineering/findings/cme-event-signal.md`), which was not attempted here under the tightened budget.

**Wire format** (already documented, restated for the mapping table below): `onErrorCode` = `UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID` (client method 121). Under `SystemID = 0` (`ERRORCODE_SYSTEM_Ability`, the only token `EErrorCodeSystem` defines), `InstanceID` is read by the client as an ability id (confirmed in `docs/gameplay/gate-travel.md`).

> [!NOTE]
> **Superseded in part (2026-09-27).** The "zero listeners, dropped entirely" branch of the "This is exactly the open question" paragraph above is ruled out. A native subscriber exists; what it does is still open. The evidence is below.

### A native subscriber exists (RTTI, 2026-09-27)

A headless-Ghidra pass over `SGW.exe` (read-only, `-noanalysis`, using the script in [tools/re/ghidra-headless/](../../../tools/re/ghidra-headless/README.md)) found this MSVC RTTI type-name string at `0x01e20da8`:

```text
.?AV?$FreeCallback@UNoSubject@EventSignal@CME@@PAVCommunicator@@P6AXPBVEvent_NetIn_onErrorCode@@PAXPAV4@@ZV5@@EventSignal@CME@@
```

Demangled by hand (Ghidra's demangler does not accept raw RTTI type-name strings):

```cpp
CME::EventSignal::FreeCallback<
    NoSubject,
    Communicator*,
    void (__cdecl*)(Event_NetIn_onErrorCode const*, void*, Communicator*)>
```

A `FreeCallback` is the free-function counterpart of the `MemberCallback` described in [cme-event-signal.md](cme-event-signal.md#cmemembercallback-struct-layout): instead of calling a method on a bound object, it calls a plain `__cdecl` function and passes the bound context, here a `Communicator*`, as the last argument. So some native function receives every `onErrorCode` together with a `Communicator`. `Communicator` has no named methods in the Ghidra project; from its name it is plausibly the chat/communication delivery class, which would make a visible line possible, but that is inference, not evidence.

**What this settles:** `Event_NetIn_onErrorCode` has at least one listener, so the event is not dropped on the floor. Strictly, RTTI proves the callback type is compiled into the client, not that it is subscribed at runtime; since the type exists only to be subscribed, a live subscription is near-certain, and the live trace in Open questions would confirm it. **What it does not settle:** whether that listener renders a line, a flash, or nothing a player sees (a log, a counter). The function's address and body were not resolved. The trade result has the same shape of open item, a native `Trade` member subscriber with unknown behaviour; see [trade-result-client-handling.md](trade-result-client-handling.md#a-native-trade-subscriber-exists-rtti-2026-09-27).

### D-AT08 error-code mapping recommendation

`EConditionHandlerFeedback` (`entities/defs/enumerations.xml:1207-1460`) has **no tokens dedicated to the trainer's own gates** (training points, branch-spend). The three gates that predate the ability-tree system have a clean or near-clean match; the two gates the pack introduces do not.

| `TrainReject` reason | Recommended code | Fit |
|---|---|---|
| Wrong archetype | `CONDITION_FEEDBACK_NotSpecifiedArchetype` (6) | **Exact.** Paired opposite `SpecifiedArchetype` (21); this is literally what the token means. |
| Missing prerequisite ability | `CONDITION_FEEDBACK_EntityDoesNotHaveAbility` (167) | **Exact.** Named precisely for "you don't have ability X." |
| Not at / too far from a trainer | `CONDITION_FEEDBACK_OutsideDistanceCheck` (43) | **Close.** Generic proximity-check-failed token (system 1012); not trainer-specific, but semantically correct and already the shape used elsewhere for interaction-range rejections (contrast `OutsideWeaponRange` (42), which is combat-specific and the wrong domain here). |
| Level too low | `CONDITION_FEEDBACK_LevelGreaterThanOrEqual` (9) | **Close.** The unlock check is "level >= N"; this token names that comparator directly (tied to system 1003, the generic level-check family). |
| Not enough training points (`AT-03`'s `NotEnoughPoints`) | **No token fits.** Fallback: `CONDITION_FEEDBACK_StatValueLessThan` (35) | The enum's only "not enough of a counted resource" family is `AppliedSciencePoints` (213/214, crafting-specific) and the generic `StatValue*` comparators (30-35, system 1008). `StatValueLessThan` is the least-wrong generic fallback — treat training points as a numeric player stat for feedback purposes. Flag this explicitly as a reused code, not a literal match, if it ships. |
| Branch-spend gate (`AT-03`'s `SpendGate`) | **No token fits.** Same fallback: `CONDITION_FEEDBACK_StatValueLessThan` (35) | Same reasoning — branch points spent is a derived numeric threshold with no dedicated 2009 token. |
| Duplicate purchase of an already-known ability | **No error code — stays silent** (per D-AT08 as already drafted) | Correct as designed: Q1's evidence shows an already-known ability's button is rendered `AbilityKnown`, not clickable via `buyTrainable`, so a genuine double-click race is the only way to hit this path, and a nuisance toast on it would be worse than silence. |

The last two rows are the load-bearing caveat for AT-04/AT-08: **any code chosen for the two new gates is a reuse, not a recovery**, because the 2009 enum never anticipated skill-point economies. Record that provenance distinction in the implementation PR, not just here.

---

## 3. `Events.PropertyUpdated` refresh (Q3)

**Confidence: HIGH — confirmed directly from `Ability.lua` source, no Ghidra needed.**

```lua
function AbilityMod.onPropertyUpdated( this, unitId, propType, propValue )
    if propType == Property.TrainingPoints and unitsEqual( Unit.Player, unitId ) then
        AbilityMod.refreshTrainingPoints()
    end
end
...
AbilityWin:subscribe(Events.PropertyUpdated, 'AbilityMod.onPropertyUpdated')
```

Yes: a server `onEntityProperty(GENERICPROPERTY_TrainingPoints, value)` push fires the generic `Events.PropertyUpdated(unitId, propType, propValue)` signal, and `AbilityMod.onPropertyUpdated` filters for `propType == Property.TrainingPoints` on the local player, then calls `refreshTrainingPoints()`, which re-reads `getUnitProperty(Unit.Player, Property.TrainingPoints)` and updates `Ability_TrainingText`. The subscription is registered once at script load on the window object (not gated on visibility), so this fires whether or not the Ability window is currently open — it is safe (and correct) for the server to push this property update on every grant, independent of whether the trainer window happens to be showing.

---

## 4. Level 50 cap: no client-side table (Q4)

**Confidence: HIGH — confirmed by absence across every level/XP display path checked.**

`ExpBar.lua` is a pure passthrough:

```lua
function ExpBarMod.onExpBarUpdated( this )
    local curExp = getExperience()
    local maxExp = getMaxExperience()
    ExpBar_ProgressBar:setProperty( "CurrentProgress", tostring( curExp / (1.0 * maxExp) ) )
end
ExpBarWin:subscribe(Events.ExperienceUpdate, 'ExpBarMod.onExpBarUpdated')
```

Both `getExperience()` and `getMaxExperience()` are cached property reads with **no client-side XP table and no hardcoded cap**. They are kept current by two independent server pushes: `onExpUpdate` (client method 131, `INT32 Exp`) and `onMaxExpUpdate` (client method 132, `INT32 MaxExp`) — confirmed in `docs/protocol/client-method-dispatch-table.md:278-279`. Every other level display checked (`Character.lua:396`, `UnitFrames.lua:238`, `Greet.lua:96`) calls the plain native `unitLevel(unit)` and prints the integer with no formatting like "/20" and no clamp.

**Conclusion for the server**: the level-50 cap is **entirely a server-side concern**; no client patch is needed. The one thing that matters at the cap is `onMaxExpUpdate`'s payload: `ExpBarMod.onExpBarUpdated` divides `curExp / (1.0 * maxExp)` with **no zero-guard**. At level 50 the server must send a **non-zero** `MaxExp` (recommended: `MaxExp == Exp`, i.e. the bar reads 100% and never moves again) rather than `0`, which would produce a NaN/inf progress-bar value client-side.

---

## 5. Respec: what `respecAbilities()` sends and what the client expects back (Q5)

**Confidence: HIGH on the send side (Ghidra-confirmed); MEDIUM-HIGH on the receive side (Lua-confirmed absence, not exhaustively traced).**

**Send side.** `respecAbilities()`'s shim (`0x00aa2d80`) takes **zero Lua arguments** beyond the implicit self (matches `SGWPlayer.def:602`'s `<resetMyAbilities><Exposed/></resetMyAbilities>`, no `<Arg>` elements, and `docs/protocol/cell-method-dispatch-table.md:280`'s row `72 | resetMyAbilities | YES | (none)`). Its inner function (`0x00aeacd0`) allocates a 12-byte request object (`FUN_00aea010`), then hands it to a generic sender (`FUN_00ae0100`) reached through `thunk_FUN_0054c900` — this is architecturally the same "call a cell method with a small allocated arg-record" shape used elsewhere in the client, and it is entirely consistent with a bare, zero-payload `resetMyAbilities` call. **This confirms the campaign's assumption at face value**: `respecAbilities()` → cell method 72 `resetMyAbilities`, no arguments.

**Receive side — what the client is confirmed to do:**

- `onKnownAbilitiesUpdate` (client method 101, `ARRAY<INT32> AbilityData`) is the only wire push that updates the client's known-abilities cache. `Ability.lua` subscribes to a **derived** signal, `Events.AbilityUpdate(this, groupId, abilityId)`, and on `groupId == UIAbilityGroup.KnownAbility` (while the Ability window is visible) re-runs `refreshAbilityTree`. This is the mechanism that would pick up a post-respec known-list change, provided the native bridge that turns a full `onKnownAbilitiesUpdate` array into per-id `Events.AbilityUpdate` events treats *removed* ids the same way it treats *added* ones (this diffing logic was not traced further — see open questions).
- `Events.PropertyUpdated` (section 3) is the mechanism for the points counter.
- `onTrainerOpen` re-send (already documented server-side behavior, A-25/AT-08) is what refreshes the trainable/greyed state of the tree window.

**Does the client remove abilities from the hotbar itself? Confirmed: NO — not that any Lua evidence shows.** A full-tree grep of `Content/UI` for `Events.AbilityUpdate` returns **exactly one subscriber: `Ability.lua`**. `ActionButtons.lua` (the action-bar renderer) never subscribes to `Events.AbilityUpdate`, `Events.PropertyUpdated`, or any ability-removal signal. Its per-button refresh (`ActionButtons.lua:272-286`) calls `getAbilityInfo(actionInfo.subId)` to derive the button's *shape*, but there is no code path that clears `actionInfo.id`/`subId` (the persisted binding) when the underlying ability becomes unknown. **This directly contradicts the assumption in `work-packets.md` AT-08 ("Remove refunded abilities from the hotbar the way AT-E1 question 5 says the client expects")** — the client does not expect anything of the kind; it has no client-side hotbar-cleanup mechanism at all. A respec that removes abilities server-side will leave stale, now-dead action-bar bindings in place client-side until the player manually clears them or attempts to use one (which will presumably fail server-side, silently, per the existing silent-rejection pattern). AT-08 should either accept this as a known, harmless-but-inconsistent UX gap (pressing a dead button does nothing, same failure mode as every other silent rejection today) or treat "clear the hotbar" as new server+content work with no existing client hook to lean on.

---

## Evidence trail

| Claim | Address / file:line | Method |
|---|---|---|
| `GameEntityManager::instance()` singleton | `0x00c66ad0` → asserts against `GameEntityManager.cpp`; singleton at `0x01ef244c` (`g_EntityManager` in `address-map.md`) | Decompile |
| `getTrainingTreeCount` shim / inner | `0x00aa2ac0` / `0x00ad8700` | Decompile |
| `getTrainableList` shim / inner | `0x00aa2ba0` / `0x00add0a0` | Decompile |
| `getTrainableInfo` shim / inner | `0x00aa2c20` / `0x00add1b0` | Decompile |
| `buyTrainable` shim / inner | `0x00aa2ca0` / `0x00ad8720` | Decompile |
| `respecAbilities` shim / inner | `0x00aa2d80` / `0x00aeacd0` | Decompile |
| `TrainerAbility` wire struct | `entities/defs/alias.xml:417-422` | Existing doc (audit-trainer-runtime.md B4) |
| `onTrainerOpen` def | `entities/defs/SGWPlayer.def:1194-1198` | Existing doc |
| `resetMyAbilities` def | `entities/defs/SGWPlayer.def:602` | Existing doc |
| `onErrorCode` client method 121 dispatch table row | `docs/protocol/client-method-dispatch-table.md:268` | Existing doc |
| `onAbilityTreeInfo` client method 141 dispatch table row | `docs/protocol/client-method-dispatch-table.md:288` | Existing doc |
| `onExpUpdate`/`onMaxExpUpdate` rows 131/132 | `docs/protocol/client-method-dispatch-table.md:278-279` | Existing doc |
| No `.lua` file in the client mentions `ErrorCode` | Full-tree grep, `Content/UI` and `SGWGame` root | Grep (2026-09-25) |
| No Ghidra function name matches `ErrorCode`/`ConditionFeedback`/`ConditionHandler` beyond registration stubs | `search_functions_enhanced` regex sweep | Ghidra (2026-09-25) |
| `register_NetIn_onErrorCode` / CME emit-info stub | `0x00d77f00` / `0x00d77fe0` | Decompile |
| Native `FreeCallback<NoSubject, Communicator*, void(__cdecl*)(Event_NetIn_onErrorCode const*, void*, Communicator*)>` subscriber | RTTI type-name string `0x01e20da8` | Headless Ghidra string search (2026-09-27) |
| `EConditionHandlerFeedback` enum, no trainer-specific tokens | `entities/defs/enumerations.xml:1207-1460` | Direct read |
| `Ability.lua` full source (Q1/Q3/Q5 evidence) | `Content/UI/Core/Ability/Ability.lua` | Direct read |
| `ExpBar.lua` full source (Q4 evidence) | `Content/UI/Core/ExpBar/ExpBar.lua` | Direct read |
| `Trainer.toc` disabled, A-21 re-confirmed | `Content/UI/Core/Trainer/Trainer.toc` | Direct read |
| `ActionButtons.lua` has no ability-removal subscriber | `Content/UI/Core/ActionButtons/ActionButtons.lua:240-299` | Direct read + grep |

## Open questions

1. **`onErrorCode` client rendering (Q2) — UNRESOLVED, re-scoped 2026-09-27.** A native listener *does* exist (a `FreeCallback` bound to a `Communicator*`, see §2); the open part is what it does. Resolving it needs either a live-client trace of the `Event_NetIn_onErrorCode` dispatch (a non-freezing breakpoint, condition `0` plus a log, on `CmeEventSignal_Subscribe` at `0x00a5c150` filtered to this event, or on the callback's invoke; see [sgw-live-debugging.md](../../guides/sgw-live-debugging.md)) or a full GUI RTTI re-analysis in Ghidra that resolves the callback's vtable and function pointer. Until resolved, assume the server-side `onErrorCode` send is **correct-but-unverified presentation** — it satisfies the "every button press gets feedback" rule at the wire level, but whether the player actually sees anything is unknown.
2. **`onKnownAbilitiesUpdate` add/remove diffing** — not traced. The native bridge that turns the flat `ARRAY<INT32>` into per-id `Events.AbilityUpdate(groupId, abilityId)` calls was not located; whether it fires once per newly-known id, once per newly-*unknown* id (post-respec), or requires the array to represent a delta rather than the full set, is unconfirmed. This bears directly on whether AT-08's respec response (which must reduce the known set) will visually refresh the Ability window correctly.
3. **`getAbilityInfo` behavior for a no-longer-known ability id** — not checked. If it still resolves ability metadata regardless of ownership (likely, since ability defs are static content), the stale action-bar button described in section 5 will render normally right up until a doomed `useAbility` call.
