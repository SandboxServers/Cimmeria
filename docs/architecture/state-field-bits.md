# `bStateField` bit map — what the SGW client actually reads

Status: verified against the live `SGW.exe` binary (image base `0x00400000`, ASLR-fixed build).

`bStateField` is the 32-bit `EStateField` flag word on every `SGWBeing`-derived entity. It rides the wire via `onStateFieldUpdate` as the entity property `bStateField`. The client stores the full 32-bit value at `GameBeing+0x158` and dispatches per **changed bit** — i.e. `uVar2 = current ^ new`, then a series of `TEST` checks against masks.

**Only bits 0-7 trigger any client-side side effect.** Verified by decompiling `GameBeing_OnStateFieldUpdate` at `ghidra://SGW.exe@0x00e01c90` and byte-pattern-scanning every `TEST` instruction over the `bStateField` read range.

## Verified bit map (server-emit contract)

| Bit | Mask | Name | Client-side effect on change |
|---|---|---|---|
| 0 | `0x01` | `BSF_Dead` | `GameBeing_OnDeadStateChanged` + `ApplyDeadInteraction` |
| 1 | `0x02` | `BSF_AutoCycling` | `EmitAutoCycleStateChanged` |
| 2 | `0x04` | `BSF_Crouching` | `UpdateMovementSpeed` (via the `0xC4` group mask) |
| 3 | `0x08` | `BSF_InCombat` | `UpdateCombatStanceWeaponSet` |
| 4 | `0x10` | `BSF_PlayingMinigame` | `FUN_00e31aa0` (minigame UI lock) |
| 5 | `0x20` | `BSF_InStealth` | `EmitStealthStateChanged` |
| 6 | `0x40` | `BSF_MovementLock` | `UpdateMovementSpeed` (via the `0xC4` group mask) |
| 7 | `0x80` | `BSF_Walking` | `UpdateMovementSpeed` (via the `0xC4` group mask) |
| 8 | `0x100` | — *(was `BSF_Holster`)* | **Nothing.** See "Bit 8 retirement" below. |
| 9–31 | `0x200`–`0x80000000` | Reserved | Nothing. |

The `0xC4` group mask (bits 2 + 6 + 7) is the movement-speed recompute — any of crouch / movement-lock / walking changes triggers the same code path.

## Bit 8 retirement (`BSF_Holster`)

`BSF_Holster` (bit 8) was the SGW client's documented holster flag per the legacy Python source, but the 2009 client binary **does not test bit 8 anywhere**. The dispatch in `GameBeing_OnStateFieldUpdate` uses `TEST BL, imm8` instructions — bytes operations against the low byte of `bStateField` only. The full 32-bit value is still stored at `+0x158`, but bits 8-31 never trigger any side effect.

The actual visual holster mechanism is in the appearance compositor:

- `CompositedAppearanceProxy::ApplyToPawn` at `ghidra://SGW.exe@0x00ec0840` (source `.\Src\CompositedAppearanceProxy.cpp:0x27`) writes `entity+0x3D2` (the animation-key weapon-category byte) from `proxy+0x34` (a weapon-category code).
- `proxy+0x34` is filled from the appearance compositing job's `job[0x1e]` field — which sources its weapon-category from the `Event_NetIn_BeingAppearance` `ComponentList`.
- `USGWAnim_BlendByPosture` keys off `entity+0x3D2` to pick the armed-stance vs. holstered-stance animation blend.

So **holster is a `BeingAppearance` `ComponentList` concern, not a state-flag concern.** To render a player as holstered, omit the weapon visual from `ComponentList`; to render armed, include it. The Cimmeria server tracks this on `CellEntity::weapon_holstered` and filters in `CellEntity::appearance_components()` / `PlayerLoadData::appearance_components()`.

The `BSF_HOLSTER` constant was removed in PR for issues #249 + #333 (consolidated). The dead writes in `use_ability/mod.rs` (clear-on-fire) and `cell_methods/player/world.rs` (clear-on-reload) were removed at the same time — they wrote bit 8 onto the wire, but the client discarded it.

## How to verify a future change

If a future patch adds a new dispatch path that reads `bStateField`, re-run the byte-pattern scan:

- Find every `TEST` instruction whose first operand is `byte ptr [reg+0x158]` (bits 0-7) or `[reg+0x159]` (bits 8-15) or further offsets, with `reg` being a `GameBeing*`.
- Each hit is a dispatch path. The mask immediate in the `TEST` tells you which bit.
- Cross-check against the bits actually OR'd into `bStateField` by the server's `onStateFieldUpdate` emit sites — any bit emitted but never tested is dead-on-the-wire.

Ghidra MCP queries for the next verification pass:

```text
mcp__ghidra__decompile_function 0x00e01c90      // GameBeing_OnStateFieldUpdate
mcp__ghidra__decompile_function 0x00ec0840      // CompositedAppearanceProxy::ApplyToPawn
```

## Related

- Source-of-truth for what each flag does: `crates/services/src/cell/combat/state.rs`.
- Holster mechanism (visible state on `CellEntity::weapon_holstered`): see the docstring on that field in `crates/entity/src/cell_entity/mod.rs`.
- Bible — Mercury wire format ([`docs/drafts/spec/mercury-wire-format.md`](../drafts/spec/mercury-wire-format.md)): packet framing, encryption, sequencing — the layer that carries `onStateFieldUpdate`.
- Bible — Entity property sync ([`docs/drafts/spec/entity-property-sync.md`](../drafts/spec/entity-property-sync.md)): `BeingAppearance.ComponentList` is replicated through this layer; this doc is what defines the `holster = omit weapon entry` contract on the wire.
- Issue history: #249 (visible "weapon never holsters"), #333 (cleanup, the issue that produced this doc).
