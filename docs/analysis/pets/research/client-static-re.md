# Pets — Client Static RE Findings (2026-09-26)

> Static-only pass: Ghidra (SGW.exe, image base 00400000) + direct read of uncooked client Lua
> (`<client install>/SGWGame\Content\UI\Core\Pet\{PetContainer,PetInfo}.lua`,
> `Content\UI\Core\Ability\Ability.lua`, `Content\UI\Core\ActionButtons\ActionButtons.lua`).
> No x64dbg attach, no client run, no repo edits. Builds on
> `docs/reverse-engineering/findings/pet-restoration.md`, `pet-wire-formats.md`,
> `docs/gameplay/pet-system.md`, and the knowledge-gap report at
> `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\kg-2026-09-27\pets.md`. Only genuinely new
> or corrected material is written up in detail below; items already fully covered by those docs
> are referenced, not repeated.

## A. How the client fills `Unit.Pet1..4` / recognizes "my pet"

**Not resolved by this pass — genuinely needs live capture.** `Unit.PetN` is a native unit-slot
accessor (string constants `Pet1`..`Pet4` at `0x01955a9c`-`0x01955ac0`); the client-side logic that
*decides* which live entity occupies each slot is native C++ (no Lua touches this — the Lua only
*consumes* `Unit.Pet1` as an already-resolved unit handle, e.g. `PetContainer.lua:429-432`,
`PetInfo.lua:59,73`). `GamePet__ctor` (`0x00d39cb0`) zero-inits fields but does not read an
ownerID off the wire at construction time — ownership must arrive via a subsequent property-update
message (SGWPet's `ownerID`/`ownerBase`, both `CELL_PUBLIC` per the .def) after `createOnClient`.
Ctor field layout (fastcall, `param_1` = `this`):

```text
*param_1 = GamePet::vftable
[0x5c]  = 1        (dword index into param_1, i.e. byte +0x170: the constant "IsPet" marker; see pet-client-contract.md §2.3)
[0x171] = 0        (stance-related, matches pet-restoration.md's noted init bytes)
[0x172] = 0
[0x173] = 0xff
[0x5d]/[0x5e] = 0  (likely ownerID / start of ownerBase MAILBOX — zeroed, not yet resolved)
FUN_00ec0620(this+0x5f)   -- array ctor, likely toggledAbilities
FUN_0043b050(this+0x62)   -- object ctor, likely abilityInformation (PYTHON) or a VECTOR3
```

**Recommendation**: the client-side "is this entity my pet / which slot" binding is a property-sync
question, not a construction-time one — close it with the same `Event_Net_EntityCreate`/property-sync
MercuryLogger capture already planned in `pet-restoration.md` (createEntity class_id=5 + first
property batch), watching specifically for which property write triggers the `Unit.PetN` slot
assignment (likely a `PetChanged` UI event fire — see `Events.PetChanged` subscription in
`PetContainer.lua:467`, `PetInfo.lua:348`).

## B. `onPetStanceList` semantics — resolved precisely from Lua source

`PetInfo.lua:104-156` (`PetMod.refreshPet`) is unambiguous:

```lua
local petStances = getPetStanceList( unitId )      -- returns the ARRAY<INT8>, 1-based Lua table
for i=1,PetMod.MAX_STANCES do                       -- MAX_STANCES = 5
    PetMod.setStance( unitId, i, petStances[i] )
end
```

`PetMod.setStance(unitId, i, stanceId)` (`PetInfo.lua:194-208`) sets `Pet_StanceIcon_i`'s window
**ID to the real `stanceId` value** (from `getPetStanceInfo(stanceId)`), and the click handler
(`onInfoStanceClicked`, line 341-344) reads that ID straight back and sends it via
`changePetStance(unitId, stanceId)`. **So: `onPetStanceList` array element `i` (1-based) is the
actual server-side stance id that should occupy display slot `i` — not an index, not an ability
id.** This confirms Q5 from the knowledge-gap report at HIGH confidence, no live capture needed.

**A genuine client-side inconsistency, found by comparing the two pet UIs**: `PetContainer.lua`
(the small always-on pet bar) wires its 5 `_StanceButton` widgets very differently
(`registerPetContainer`, lines 279-289):

```lua
for i=1, PetMod.MAX_STANCES do
    local stanceButton = _G[containerPrefix..'_StanceButton'..i]
    stanceButton:setID( i )                          -- ID = raw slot index, NOT a real stanceId
    stanceButton:subscribe( stanceButton.EventClicked, 'PetMod.onStanceClicked' )
end
-- onStanceClicked: changePetStance( unitId, window:getID() )   -- sends the slot index (1..5)
```

`setupPetContainer` even leaves a `-- TODO:` comment ("Set up the Stances") where it should be
loading `getPetStanceList` and calling something equivalent to `PetMod.setStance` for these
buttons — **it never does**. So the small pet-bar's stance row was left unfinished in the shipped
2009 client: clicking one of its 5 stance icons sends `changePetStance(unitId, 1..5)` (the display
slot number) rather than a real stance id, and the icons are never populated with the actual
per-slot stance (no icon/tooltip set at all for that row — they'd show whatever default CEGUI
imagery the .layout gave them). The correct, fully-wired flow only exists in the separate
`PetInfo` ("Pet Abilities") window. **This is an original-2009 client bug/incompleteness, not a
Cimmeria restoration gap** — worth a one-line callout in `pet-restoration.md` so nobody "fixes"
the small bar to match a spec the original client itself didn't follow, and worth deciding whether
Cimmeria's server should tolerate slot-index stanceId values arriving from that bar (defensive
clamp/validate, since a client sending `changePetStance(unitId, 4)` or `5` for a pet with only 3
real stances would otherwise be a bogus value the server must reject, not silently accept).

**Why 5 UI slots vs 3 `EPetStance` values**: still not fully closed, but two static findings
narrow it. (1) `getPetStanceList`'s wire array is genuinely variable-length (`ARRAY<INT8>`), and
`EEntityFlags` has three flags that gate individual stances per pet/mob —
`ENTITYFLAG_NoPassive`(64), `ENTITYFLAG_NoDefensive`(128), `ENTITYFLAG_NoAggressive`(256)
(`entities/defs/enumerations.xml:1482-1484`) — so different pet types plausibly ship different
subsets of the 3 real stances, which is why the wire format is an array rather than a fixed
3-element struct. (2) That still only explains variability up to 3, not the extra 2 UI slots;
`PetMod.MAX_STANCES = 5` reads as plain UI headroom (possibly for a 4th/5th stance that was
planned and cut), not something tied to a discovered content value — low-value to pursue further
statically.

## C. `onPetAbilityList` — command/ability split, and where command defs come from

`isPetCommand` on the native `getAbilityInfo(id)` result table is the *only* pet-command field
ever surfaced to Lua (confirmed by grep across all of `Content/UI` — no `isPetToggled` /
`isPetTrained` equivalent exists in script). It almost certainly reads
`EAbilityFlags.PetCommand` (`65536`, `enumerations.xml:53`) off the ability definition. Two
**other** pet-flavored bits exist in the same `EAbilityFlags` bitfield that are *not* exposed to
script at all: `PetToggled = 32` and `PetTrained = 64` (`enumerations.xml:42-43`) — native/server
only. `PetToggled` is the natural candidate for whatever gates an ability into the
`toggledAbilities` opt-out array / CM 89 `petAbilityToggle`; `PetTrained` likely gates
level-unlocked pet abilities (ties to `setPetLevel`/`ENTITYFLAG_NoPetLeveling`). Neither is
confirmed beyond the flag's existence — would need either a decompile of the native
`getAbilityInfo` marshalling function or a live ability-definition dump to confirm which table
field each maps to.

**Trap for anyone grepping blind**: there is a *second*, unrelated `PetCommand` token —
`EBehaviorEventFlags.PetCommand = 1` (`enumerations.xml:1589`), a `UINT64` bitfield used by the
mob AI behavior-event/Kismet-adjacent system, completely distinct from
`EAbilityFlags.PetCommand = 65536`. Do not conflate the two when grepping the defs.

Client behavior in `refreshPet` (`PetInfo.lua:104-133`) and `buildDefaultConfig`
(`PetContainer.lua:214-251`): every ability in the `onPetAbilityList` array is looked up once via
`getAbilityInfo(abilityId)` and routed to either the "ability" icon row or the "command" icon row
purely by the `isPetCommand` boolean — **there is no separate wire list or separate id-space for
commands**; commands are abilities with a flag bit set, sharing the same ability id space and the
same `abilityId` array. This matches — and strengthens — the existing doc's framing; no
`RESOURCE_PetCommand`/cooked-pak dependency was found anywhere in the client UI tree for
*populating* the command row (contrary to a plausible-sounding assumption): the row is populated
purely from `onPetAbilityList` + `getAbilityInfo`, i.e. the same ability-definition source
abilities already come from. `getPetCommandInfo(unitId, commandId)` is a *second*, parallel native
lookup that exists but — see section D — is never reached from the shipped UI's ability-list path.

## D. What the client sends on click — confirmed, with a load-bearing correction

CM 88/89/90 payloads and byte widths in `pet-wire-formats.md`/`pet-restoration.md` are unchanged
and correct (12B/9B/5B; `entityId` is the pet's id, ownership must be server-validated). What this
pass adds:

**`usePetCommand` / `dragPetCommand` / `getPetCommandInfo` / `ActionType.PetCommand` are dead code
in the shipped UI's mouse path.** Ghidra confirms all four exist as real native ScrFuncs/classes
(`getPetCommandInfo` @ `0x01954a94`, `usePetCommand` @ `0x019549f8`, `dragPetCommand` @
`0x01954a30`, RTTI class `PetCommandAction` @ `0x01e61e40`), and `PetInfo.lua` defines full
handlers for them (`onInfoCommandClicked` line 335-338 → `usePetCommand(unit, commandId,
Unit.Target)`; `onDragCommandStarted` line 261-290 → `dragPetCommand`). **But the subscription that
would wire the command icons to those handlers is commented out and replaced with the ability
handlers instead** (`PetInfo.lua:360-362`):

```lua
PetMod.initButtons( "Pet_AbilityIcon_", 'PetMod.onDragAbilityStarted', 'PetMod.onInfoAbilityClicked' )
--PetMod.initButtons( "Pet_CommandIcon_", 'PetMod.onDragCommandStarted', 'PetMod.onInfoCommandClicked' )
PetMod.initButtons( "Pet_CommandIcon_", 'PetMod.onDragAbilityStarted', 'PetMod.onInfoAbilityClicked' )
```

So clicking a "pet command" icon (stay/follow/attack-toggle, whatever the isPetCommand-flagged
abilities are) in the shipped 2009 client calls **`onInfoAbilityClicked` → `usePetAbility(unit,
abilityId, Unit.Target)`** — the exact same script function and (per the .def) the same wire
method, **CM 88 `petInvokeAbility`**, that ordinary pet abilities use. `PetContainer.lua`'s
`buildDefaultConfig` reinforces this independently: even when it detects `abilityInfo.isPetCommand`
(to pick a different default-button slot), it still always builds
`actionInfo = { abilityId = id, commandId = nil }` (line 244) — `commandId` is never populated from
that path, so `setupPetButton`'s `elseif actionInfo.commandId then dragPetCommand(...)` branch
(`PetContainer.lua:171-172`) is unreachable from auto-populated default layout too.

**Practical implication for the server**: build the pet-command feature entirely on CM 88
`petInvokeAbility` (server dispatches on the ability's `PetCommand` flag / a hardcoded id table to
decide "run stay/follow/attack logic" vs "resolve as a normal pet ability"), not on CM 89 or a
`petAbilityToggle`-style "toggle" semantics. **CM 89 `petAbilityToggle` has zero call sites
anywhere in the client Lua UI tree** (grepped whole `Content/UI`) — nothing drags, clicks, or
right-clicks ever calls `togglePetAbility`. If it's reachable at all in the shipped client, it's
via a path this static pass didn't find (see section H).

**A real, additional invocation path this pass discovered and the existing docs don't mention**:
slash commands. Three dedicated `SGWTextCommandMgr`-routed events exist and are wired at RTTI
level exactly like the confirmed GM `/summon` pattern already documented:

```text
Event_SlashCmd_PetInvokeAbility   (0x0184280c)
Event_SlashCmd_PetInvokeCommand   (0x0184282c)
Event_SlashCmd_PetAbilityToggle   (0x0184284c)
```

registered via `CMERegistry__RegisterAllEventEmitHandlers` (xref from `0x005cc434`), each with a
full `MemberCallback<...SGWTextCommandMgr...>` RTTI signature (e.g.
`.?AV?$MemberCallback@UNoSubject@EventSignal@CME@@VSGWTextCommandMgr@@P84@AEXPBVEvent_SlashCmd_PetInvokeAbility@@PAX@ZV5@@EventSignal@CME@@`
at `0x01e07060`). This means **typed slash commands are a second, independent route to pet
ability/command/toggle invocation**, separate from and probably *not* subject to the UI's
dead-code problem above — `Event_SlashCmd_PetInvokeCommand` and `_PetAbilityToggle` may be the
*only* live way to reach `usePetCommand`/`petAbilityToggle` semantics in the shipped client. The
actual typed keyword text (e.g. whatever a player would type — `/petcommand`, `/petinvoke`,
something else) was **not** found as a plain string near these addresses; `SGWTextCommandMgr`
resolves slash text through a runtime map (per existing agent memory: 256 `Event_SlashCmd_*`
classes, count is runtime not static), so the keyword needs either the runtime map dump technique
already used for the general slash-command work, or a live capture (section H).

## E. `EAbilityFlags.SpeedPet` (16384) — not resolved statically

No client-side usage site was found (bit values aren't greppable as strings, and the ability-flag
check code wasn't decompiled this pass — out of scope for a quick static pass, needs targeted
decompilation of the ability-resolution/animation-gating function). Pattern inference only, **LOW
confidence**: it sits in a family with `SpeedGrenade`(2048)/`SpeedDeploy`(4096)/`SpeedAttack`(8192)
— all "Speed*" bits immediately adjacent in the same bitfield — so by naming convention it most
likely means "no wind-up/travel-time animation gate when invoking a pet ability", mirroring
whatever the other three `Speed*` flags do for their respective ability categories. This is
consistent with, but does not newly confirm, the correction already recorded in agent memory that
this value was previously misassigned to `AF_CHANNEL_ALLOWS_MOVEMENT` during the ability-tree
campaign. Closing this needs the ability-resolution decompile, not more grepping.

## F. Leash/follow, onOwnerRespawn, despawn visuals, "X's Pet" nameplate

**Important negative finding, changes the plan in `pet-restoration.md`**: `onOwnerDeath`,
`onOwnerLeash`, and `onOwnerRespawn` produce **zero string/function hits anywhere in SGW.exe**
(searched all three by name). This is expected once you look at the `.def`: they're listed under
**Cell Methods**, i.e. server(cell)-side method calls the *server* invokes on the `SGWPet` entity
in response to server-observed owner events — they are never serialized to the client and have no
client-side representation at all. Likewise the leash-poll fields
(`lastOwnerPositionCheck`/`lastTeleportTime`/`ownerLastPosition`/`petLastPosition`) are
**CELL_PRIVATE** per the .def — server-internal state, not replicated. **Conclusion: there is
nothing in the client binary to capture for the leash-teleport distance/interval or the
`onOwnerRespawn` default decision** — these were server-side Python logic that (per
`deprecated/python/cell/SGWPet.py`) was never actually implemented, so the "constant" being
searched for may simply never have existed in a shipped, working form. The x64dbg plan in
`pet-restoration.md` items 2/3/5 (leash threshold, poll interval, onOwnerRespawn default) should be
retargeted or downgraded to "greenfield design decision", not "recover the original value" — there
may be no original value to recover from any client-observable source. The one thing that *would*
still be client-observable is the generic *visual* effect of a forced teleport/position snap (if
the server ever did move the pet), but that's the same generic AoI position-update path every
entity uses, not something pet-specific to isolate.

**"X's Pet" nameplate**: no `"'s Pet"` or similar format string was found anywhere in SGW.exe by
direct string search. The nameplate text is very likely built from two separately-fetched pieces —
the owner's display name (via `ownerID`/`ownerBase` → the same unit-name lookup used everywhere
else, e.g. `unitName()` as seen in `PetInfo.lua:75,85,95`) and a localized possessive/suffix string
— rather than a single hardcoded template string in the binary; those localized UI strings live in
the string tables (`.int`/localization files), not as raw ASCII/UTF-16 literals in the .exe. Not
resolved; would need either the localization tables or a live nameplate capture.

**Despawn visuals**: not found — no pet-specific despawn/dismiss effect function or string
surfaced in this pass (searches for "dismiss"/"despawn" pet-scoped strings came back empty). Given
section F's finding that owner-event handling is entirely server-side and never implemented, it's
plausible the original client never got to *show* a scripted pet despawn — an entity removal would
fall back to the same generic entity-destroy/AoI-leave visual every other entity uses. Low
confidence; not exhaustively ruled out.

## G. Entity flag values — resolved from source, no Ghidra needed

Exact values and inline dev comments, `entities/defs/enumerations.xml:1472-1496`
(`EEntityFlags`, `UINT32` bitfield, comment: `Bitfield db:ref_MOB_FLAGS` — confirms this is the
legacy `ref_MOB_FLAGS` column reborn as an enum):

| Flag | Value | Note |
|---|---|---|
| `ENTITYFLAG_NoPetLeveling` | 8 | |
| `ENTITYFLAG_NoPetTargeting` | 16 | |
| `ENTITYFLAG_DespawnOnOwnerLeash` | 32 | dev comment: *"You got too far from your owner"* |
| `ENTITYFLAG_NoPassive` / `NoDefensive` / `NoAggressive` | 64 / 128 / 256 | gate which `EPetStance` values are legal for this entity — see section B |
| `ENTITYFLAG_DetectionPet` | 512 | |
| `ENTITYFLAG_Pet` | 1024 | |
| `ENTITYFLAG_DespawnOnLeashFromOwner` | 32768 | dev comment: *"Your owner is a mob who is leashing"* |
| `ENTITYFLAG_PetUseOwnFaction` | 65536 | |
| `ENTITYFLAG_PetWaitToDespawn` | 131072 | |

**Not a naming bug** (flagged for verification, then cleared): `DespawnOnOwnerLeash` (32) and
`DespawnOnLeashFromOwner` (32768) look like a copy-paste/near-duplicate pair at first glance, but
the source comments confirm they're two distinct trigger directions — the former fires when *this*
entity (the pet) leashes away from its owner; the latter fires when *this* entity's owner is itself
a leashing mob and leashes back to its own spawn (e.g., a pet-of-a-summoned-thing, or anything
whose "owner" is an NPC rather than a player). Worth stating explicitly in
`docs/gameplay/pet-system.md` since the names invite confusion. None of these flag bits have any
Lua-visible surface (never grepped in `Content/UI` outside the .def/enum source) — they're pure
server/native gating, consistent with being CELL-side entity behavior, not client UI state.

## H. Live x64dbg captures still needed (non-freezing log breakpoints only)

1. **Ownership/slot-fill (section A)** — property-sync capture on `createEntity(class_id=5)` +
   first property batch; watch for whichever property write fires `Events.PetChanged` /
   assignment into `Unit.PetN`. Reuses the capture already scoped in `pet-restoration.md`.
2. **Slash-command keyword text (section D)** — dump the `SGWTextCommandMgr` runtime
   command-string→`Event_SlashCmd_*` map (same technique as the existing 256-command registry
   work) to find what a player types for `Event_SlashCmd_PetInvokeCommand` /
   `_PetAbilityToggle` — this is now the most promising route to observe a *live* CM 89
   `petAbilityToggle` and non-ability-routed `usePetCommand` call, since the mouse UI can't reach
   them.
3. **`GamePet__ctor` field semantics (section A)** — non-freezing log BP at `0x00d39cb0` return,
   dump `this+0x5c`/`0x5d`/`0x5e`/`0x171..0x173` at construction vs. after first property sync, to
   confirm which offset is `ownerID` vs `ownerBase` vs the stance-init bytes.
4. **`getPetStanceInfo`/`getPetCommandInfo` data source (section B/C)** — not statically located
   this pass (the ScrFunc→native-function binding table wasn't traced); a log BP on whichever
   native function backs these two names, or a MercuryLogger-style call-arg/return dump, would
   settle whether stance/command display info (name/icon) comes from cooked ability-definition
   data already synced to the client or a separate small static table.
5. **`EAbilityFlags.SpeedPet` effect (section E)** — needs the ability-resolution/animation-gate
   decompile (static, no x64dbg needed) rather than a live capture; flagged here only because it's
   still open, not because it needs dynamic analysis.
6. Sections F's `onOwnerRespawn`/leash-poll items are **downgraded, not scoped for capture** — see
   section F: there is no client-observable signal to capture for server-only cell methods that
   were never implemented server-side either.

## Confidence summary

- **HIGH, source-grounded, no further work needed**: B (onPetStanceList semantics + small-bar
  bug), C (command/ability share one id-space and one wire path), D (dead command-click code +
  CM88 unification + slash-command alternate route existing), G (entity flag values + the
  two-flags-not-a-bug clarification).
- **MEDIUM**: C's `PetToggled`/`PetTrained` flag→feature mapping (flag exists, exact consumer not
  traced).
- **LOW / open**: A (ownership resolution mechanism), E (SpeedPet effect), F (nameplate string
  construction, despawn visuals) — all listed in section H with the smallest capture that would
  close them, except F's owner-event items which are re-scoped as "nothing to capture."
