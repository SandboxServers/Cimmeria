---
title: "NPC Aggression Broadcast — SGWMob ClientMethod indices and the legacy wire-format bug"
type: reference
audience: engineers doing RE or wiring NPC AI content
last_updated: 2026-09-25
---

# NPC Aggression Broadcast

> **Last updated**: 2026-09-25
> **Source**: SGW.exe Ghidra decompilation, `entities/defs/SGWMob.def` +
> `entities/defs/interfaces/Lootable.def`, `deprecated/python/cell/SGWMob.py`
> and `deprecated/python/cell/commands/Entity.py`
> **Confidence**: HIGH for the flattened method indices (structural derivation
> plus Ghidra confirmation of the handler pair); HIGH for the legacy
> wire-format divergence (read directly off the python reference); MEDIUM for
> the client-visible effect of the level (no consuming Lua script located)
> **Campaign**: NA33 ([work-packets.md](../../analysis/npc-ai-restoration/work-packets.md#na33), D-NA16)
> **Supersedes**: the "index is not binary-verified" open item in
> [npc-ai-state-machine.md](npc-ai-state-machine.md) (NA13, 2026-09-25) and
> [docs/gameplay/npc-ai.md](../../gameplay/npc-ai.md) §"Wire: not broadcast
> yet"

---

## Summary

The client has a fully-wired, working handler for an NPC's aggression
override — `onAggressionOverrideUpdate`/`onAggressionOverrideCleared`,
SGWMob's own two `ClientMethods` at flat indices 27 and 28. The 2009 python
server's `SGWMob.createOnClient` used it correctly (once, at spawn, only
when an override was already seeded), but its runtime-change path,
`setAggression`, called a different message —
`onEntityProperty(GENERICPROPERTY_MobAggression)` — for which no client
consumer was found. So in the shipped game, telling a mob to stand down
after it was already visible to a player never actually reached that
player: the override took effect server-side, but the client kept showing
whatever aggression state it last received at spawn or reconnect. NA33
verifies the correct indices in the binary and wires Cimmeria to the
ClientMethod that actually works, on every runtime change and on AoI entry.

## Plain-language explanation

Every mob template on the server has a "how does this thing feel about
you" setting: hostile, neutral, friendly, and so on. Sometimes a mission
script changes that mid-fight — a guard surrenders, a normally-passive
drone gets armed for an ambush. The client needs to be told about it so a
player's UI (nameplate, reticle, whatever reads the value) reflects the
change. The 2009 developers built the client-side plumbing for this
correctly, and then the server-side code that was supposed to use it sent
the wrong message — one nothing on the client was listening for. The
override still worked mechanically (the mob still fought or didn't), but
the visible state on screen was stale. Cimmeria fixes this by using the
message the client's own code was built to receive.

## Technical detail

### Flattened ClientMethod index derivation

BigWorld's `entity_description.cpp:parseInterface()` flattens each entity's
inherited `<Implements>` interfaces (in document order) before its own
`<ClientMethods>`, root to leaf. SGWMob's inheritance chain is:

```
SGWEntity → SGWSpawnableEntity (12 own) → SGWBeing
    Implements: SGWBeing-interface (8), SGWAbilityManager (0), SGWCombatant (6)
    SGWBeing own: 1 (BeingAppearance)
  → SGWMob
    Implements: Lootable (0 client methods — entities/defs/interfaces/Lootable.def
                 has an empty <ClientMethods/> block)
    SGWMob own: 2 (onAggressionOverrideUpdate, onAggressionOverrideCleared)
```

`entities/defs/SGWMob.def:554-562`:

```xml
<ClientMethods>
    <onAggressionOverrideUpdate>
        <Arg>    INT8    <ArgName>    aAggressionLevel    </ArgName>    </Arg>
    </onAggressionOverrideUpdate>
    <onAggressionOverrideCleared>
    </onAggressionOverrideCleared>
</ClientMethods>
```

Indices 0-26 are identical for SGWMob and SGWPlayer — both share the
`SGWSpawnableEntity → SGWBeing` prefix, and that prefix's parse is a
property of the *ancestor classes*, not the leaf entity, so it cannot
differ between the two. (This was already documented, pre-NA33, in
`crates/services/src/mercury/mod.rs`'s `method_idx` module comment.) From
27 the two entity types diverge: SGWPlayer continues into
Communicator/OrganizationMember/etc., while SGWMob's `Implements` is just
the empty-client-method `Lootable`, so SGWMob's own two methods begin
immediately at 27:

| Index | Method | Args |
|-------|--------|------|
| 27 | `onAggressionOverrideUpdate` | `INT8 aAggressionLevel` |
| 28 | `onAggressionOverrideCleared` | *(none)* |

Both fall well under any plausible SGWMob `idbase` (SGWMob exposes 29
methods total, so `idBase = 0x3E - (29 + 0xC0) / 0xFF = 62`), so both use
**direct** wire encoding: `msg_id = 0x80 | 27 = 0x9B` and
`msg_id = 0x80 | 28 = 0x9C`.

### Ghidra confirmation

- `Event_NetIn_onAggressionOverrideUpdate` / `..._onAggressionOverrideCleared`
  string constants at `0x019bdf64` / `0x019bdf8c` (and duplicated RTTI-mangled
  copies at `0x019c0458`+ and `0x019c9e60`+).
- `MemberCallback<GameMob, Event_NetIn_onAggressionOverrideUpdate>` RTTI
  mangled name at `0x01e252c8`; the paired `...Cleared` variant at
  `0x01e253b8`.
- Registration function `FUN_00d31cd0` wires both callbacks together (one
  `FUN_00d31fd0`/`FUN_00d31bd0` pair for Update, one for Cleared via
  `LAB_00d31770`).
- The Update handler, `FUN_00d31bd0`:

```c
void __thiscall FUN_00d31bd0(void *this, void *param_1)
{
    ...
    bVar1 = FUN_00d434d0(param_1, auStack_28, &cStack_29); // reads "aAggressionLevel" arg
    ...
    *(int *)((int)this + 0x16c) = (int)cStack_29;          // GameMob + 0x16c = level
    GameEntity__unknown_00e6e330(this, (undefined4 *)0x1);
}
```

Reads the `aAggressionLevel` INT8 argument and stores it at `GameMob +
0x16c` — an SGWMob-instance field, confirming this is the SGWMob-specific
handler and not a shared `SGWBeing` method.

### UI consumption

`UIAggressionLevel` (RTTI string `.?AVUIAggressionLevel@@` at `0x01de972c`)
is registered as a Lua-scriptable enum type at `0x00ab1a5e`, in the same
registration function and alongside the same family as `UIArchetype`,
`UIStatType`, `UIDamageType`, `TargetType` and `ActionType` — enum types
exposed to the CEGUI/Lua UI layer, not CEGUI widget classes themselves (the
function also registers genuine widgets like `CEGUI::PushButton` in the
same list, but `UIAggressionLevel` sits with the other `UI*`-prefixed
enums, not the `CEGUI::*` ones). This is consistent with the level driving
nameplate color, target-reticle color, or an interaction-verb choice
("Attack" vs. "Talk") in Lua UI script, matching the family it was
registered next to. **No specific consuming Lua script was located** — the
client's Lua source is not present in this repo's tree, only the compiled
binary's RTTI/enum registration — so the exact visual effect (which
color, which icon) is inferred with medium confidence, not directly
observed.

### The legacy wire-format bug

`deprecated/python/cell/SGWMob.py:36-60`:

```python
def createOnClient(self, mailbox):
    super().createOnClient(mailbox)
    if self.aggressionOverride is not None:
        mailbox.onAggressionOverrideUpdate(self.aggressionOverride)
        mailbox.onEntityProperty(Atrea.enums.GENERICPROPERTY_MobAggression, self.aggressionOverride)
    mailbox.onEntityProperty(Atrea.enums.GENERICPROPERTY_AmmoTypeId, self.ammoTypeId)

def setAggression(self, aggressionLevel):
    self.aggressionOverride = aggressionLevel
    if self.client is not None:
        self.client.onEntityProperty(Atrea.enums.GENERICPROPERTY_MobAggression, self.aggressionOverride)
    self.witnesses.onEntityProperty(Atrea.enums.GENERICPROPERTY_MobAggression, self.aggressionOverride)
```

`createOnClient` (fired once, when a mob first becomes visible to a
client) calls **both** the ClientMethod and the property broadcast, and
only when `aggressionOverride is not None` — a faction-derived mob sends
neither. `setAggression` (the runtime-change path — the only one the
`.aggression` console command, `Entity.py:660`, ever calls) sends **only**
the property broadcast, never the ClientMethod.

[npc-ai-state-machine.md](npc-ai-state-machine.md) (NA13) already
established that no client handler for `onEntityProperty` type 6
(`GENERICPROPERTY_MobAggression`) was found in the binary. Taken together:
the *only* wire path that ever reached a working client handler was the
conditional send inside `createOnClient` — a one-time snapshot at spawn or
reconnect. Every subsequent `setAggression` call (console command, mission
script) updated the server's authoritative state but never refreshed what
an already-connected witness saw.

## Evidence trail

| Claim | Evidence |
|---|---|
| SGWMob indices 0-26 match SGWPlayer's | `entities/defs/SGWBeing.def`, `entities/defs/SGWSpawnableEntity.def` — ancestor classes, parsed identically regardless of leaf entity; already noted in `crates/services/src/mercury/mod.rs` pre-NA33 |
| Lootable contributes 0 client methods | `entities/defs/interfaces/Lootable.def:1-7` — `<ClientMethods></ClientMethods>` empty |
| SGWMob's own methods are `onAggressionOverrideUpdate`, `onAggressionOverrideCleared`, in that order | `entities/defs/SGWMob.def:554-562` |
| Ghidra confirms a live paired handler reading `aAggressionLevel` and storing `GameMob + 0x16c` | Ghidra decompile of `0x00d31bd0` / `0x00d31cd0`, string/RTTI search for `onAggressionOverride*` |
| `UIAggressionLevel` is a Lua-enum registration, not a widget | Ghidra decompile of `0x00ab1a00` (`CEGUI_ButtonBase_2`), full RTTI registration list |
| Legacy `createOnClient` vs. `setAggression` diverge in which wire call they use | `deprecated/python/cell/SGWMob.py:36-60` |
| No client consumer for `onEntityProperty` type 6 | [npc-ai-state-machine.md](npc-ai-state-machine.md) (NA13, prior finding) |
| `.aggression` console command never had a "clear" option in legacy | `deprecated/python/cell/commands/Entity.py:660-668` |

## 2009-vs-2026 notes

Cimmeria's implementation (`crates/services/src/cell/content/executor/world/mod.rs::set_aggression`,
`crates/services/src/cell/console/net.rs::aggression`,
`crates/services/src/cell/service/npc_ai/lifecycle/mod.rs::npc_ai_submit`)
broadcasts `onAggressionOverrideUpdate`/`onAggressionOverrideCleared` on
every runtime change, not the client-dead `onEntityProperty` property.
This is a deliberate divergence from the *literal* legacy wire call, in
favor of finishing the *intent* `createOnClient` already had — the client
binary has a complete, functioning handler for exactly this value; the
2009 server-side `setAggression` simply used the wrong message. No client
patch is needed; the handler has been there since 2009.

The AoI-entry replay (`crates/services/src/cell/space_manager/aoi.rs`)
mirrors `createOnClient`'s conditional send exactly: only sent when an
override is active, nothing for a faction-derived mob. The `.aggression
clear` console verb (new in Cimmeria; legacy's console command had no
clear option) uses `onAggressionOverrideCleared` — a method the client has
carried, unused, since 2009.

## Open questions

- The exact Lua script (or scripts) that read `UIAggressionLevel` and what
  they render (nameplate color? reticle color? interaction verb?) were not
  located — no client Lua source is present in this tree. A live-client
  capture (change a mob's override while an SGW.exe client is attached and
  watch what changes visually) would resolve this.
- Whether the 2009 shipped client ever actually displayed a "changed"
  aggression state outside of relog/respawn is now answered (no — the bug
  above), but whether any 2009 mission content was tuned around that
  limitation (e.g., always relying on relog to refresh the UI) is unknown.

## Cross-reference targets

- [docs/gameplay/npc-ai.md](../../gameplay/npc-ai.md) §"Aggression System" —
  update the "Wire: not broadcast yet (open item)" subsection to reflect
  that it now is, with this finding's evidence.
- [docs/protocol/client-method-dispatch-table.md](../../protocol/client-method-dispatch-table.md) —
  add the SGWMob section (done in this PR).
- [npc-ai-state-machine.md](npc-ai-state-machine.md) — the NA13 "index is
  not binary-verified" line is now resolved; a follow-up annotation there
  should point at this file.
- [docs/analysis/npc-ai-restoration/README.md](../../analysis/npc-ai-restoration/README.md) —
  D-NA16.
