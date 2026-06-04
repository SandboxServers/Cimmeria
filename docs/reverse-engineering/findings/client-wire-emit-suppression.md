# Client-Side Wire-Emit Suppression — Heal Focus + P90 Swap

> **Last updated**: 2026-06-04
> **Audience**: Engineers debugging server-silent player actions
> **Type**: RE finding + diagnosis
> **Confidence**: HIGH (Ghidra decompilation evidence) for the binary anatomy;
> MEDIUM for the proposed server-side mitigations (need playtest validation)

## Context

During lomiada's 2026-06-04 18:09–20:08 UTC session (server boot at
17:45:53 UTC), two reported failures show as **server-silent** —
i.e., the wire packet that would carry the action never reaches the
server's dispatcher:

- **Heal Focus (ability 597)** — pressing the hotbar slot bound to
  Heal Focus does nothing. Zero `useAbility(597)` events in 3 h of
  SigNoz logs (the `combat.use_ability` INFO span would have captured
  any dispatched call). The press doesn't make it onto the wire.

- **P90 bandolier swap (F1–F4)** — pressing the key to swap to the
  P90 bandolier slot does nothing. Zero `requestActiveSlotChange`
  events in the entire session.

The wire packets are emitted by client-side Lua → Mercury code paths.
This document records the Ghidra-recovered call chains for both, the
client-side gate that suppresses each emit, and the server-side
mitigation that could unblock the player without requiring a client
patch.

## Failure 1 — Heal Focus

### Call chain (Ghidra)

| Step | Address | What it does |
|---|---|---|
| 1 | Hotbar key/click | UI dispatches a Lua call to `useAbility(abilityId, targetId)` |
| 2 | `0x00aa2910` (`CEGUI__unknown_00aa2910`) | Lua → C++ thunk. Validates **three arguments**: arg1 + arg2 must be numbers (`CEGUI__unknown_00403330` @ `0x00403330`), arg3 must exist (`CEGUI__unknown_00403280` @ `0x00403280`). |
| 3 | `0x00ad78e0` (`FUN_00ad78e0`) | Resolves local player entity via `FUN_00c66ad0()`, resolves target via `FUN_00dd0de0` @ `0x00dd0de0` (entity-map lookup). |
| 4 | `0x00d2afc0` → `0x00d2ae40` (`FUN_00d2ae40`) | Reads target type discriminant at `param_1[0x12]` (offset 0x48). Allocates event object, sets `AbilityID` + `TargetID`, calls `FUN_00cacd50` to emit. |
| 5 | `0x00d2b020` (`FUN_00d2b020`) | **In-flight queue gate**: `if ((0 < abilityId) && (FUN_00d2a000(this+200) == 0))`. `FUN_00d2a000` returns the head of an in-flight ability queue at `this+0x228`. **If the queue is non-empty, the packet is suppressed.** |
| 6 | `FUN_00cacd50` | Mercury wire emit. `useAbility` Mercury method = `0x3a0` (928). |

### Diagnosis

Two suppression points before the wire emit:

1. **Lua arg-validation gate at `0x00aa2910`**. The Lua binding
   requires three arguments; if the action-bar binding for Heal
   Focus passes `nil` or omits arg2 (target id) — e.g., because
   Heal Focus is a self-cast and the binding doesn't pass `self` as
   target id — `CEGUI__unknown_00403330` returns 0 on the arg2
   check and the function silently drops to the error label
   (`"#ferror in function 'useAbility'."` at `0x01940b70`). **No
   wire packet, no server-side log, no client error popup.**

2. **In-flight queue gate at `FUN_00d2b020`**. If the queue at
   `GameEntityManager+0x228` is non-empty — e.g., a prior ability
   fired and its acknowledgement (`AbilityCooldownUpdate`) never
   cleared the queue entry — every subsequent `useAbility` call is
   suppressed until the queue drains. The clearing handler is
   `LAB_00cea050` in `FUN_00cc33f0`.

### Proposed server-side mitigation

- **Investigate hotbar binding for ability 597**: the action-bar
  Lua should pass the player's own entity id as arg2 (target id)
  for self-casts. We can't fix the Lua from the server, but we can
  RE the action-bar binding's argument-build path
  (likely under `ActionBarMod` Lua) and document the convention
  for whoever ends up patching client content.
- **Drain the in-flight queue proactively**: ensure every server
  ability resolution sends `AbilityCooldownUpdate` (and any other
  ack the client expects) so the in-flight queue at
  `GameEntityManager+0x228` doesn't accumulate. If we already
  send this for some abilities but not Heal Focus, that's the
  asymmetry to close.

Until either lands, **Heal Focus will appear non-functional from
the player's perspective**. The server is innocent.

## Failure 2 — P90 Bandolier Swap

### Call chain (Ghidra)

| Step | Address | What it does |
|---|---|---|
| 1 | F1–F4 keypress | Fires `Event_SlashCmd_ActivateBandolierSlot` (CME event, registered at `0x005c38d0` in `CMERegistry__RegisterAllEventEmitHandlers` @ `0x005ca888`) |
| 2 | `SGWTextCommandMgr::onActivateBandolierSlot` | Receives the event, dispatches to Lua |
| 3 | Lua `BandolierMod.ActivateBandolierSlotN` | **Guards on `getActiveSlotForContainer(containerId) ~= N`**. If the cached active slot already equals the pressed key's slot, the keypress is a Lua no-op. **`requestActiveSlotChange` is never called.** |
| 4 | `0x00aa6e40` (`CEGUI__unknown_00aa6e40`) | Lua → C++ thunk for `getActiveSlotForContainer` |
| 5 | `0x00ad8ad0` (`FUN_00ad8ad0`) | Walks the bandolier container map at `SGWPlayer+0x8c → *+0x24`, looks up container by id, returns `*(int*)(slot+0xc)` — the cached active slot index |

### How the cached value gets written

Server sends `onActiveSlotUpdate` (Mercury method 500 / `0x1F4` on
SGWPlayer); the NetIn handler `FUN_00da9ce0` @ `0x00da9ce0` →
`FUN_004649a0` @ `0x004649a0` walks the bag list and stores the
new value at `slot+0xc`.

### Diagnosis

The Lua gate is correct: if you're already on slot N, F-keying to
slot N should be a no-op. The bug is that **the cached client-side
value disagrees with reality**. Either:

1. The server's `onActiveSlotUpdate` packet for the player's
   current persisted slot never arrived (Mercury drop, ordering
   race with `mapLoaded`, etc.).
2. The packet arrived but for the wrong slot (off-by-one between
   wire-1-indexed and server-0-indexed slot encoding).
3. The packet arrived correctly but the client's container map
   wasn't initialized at the time it was processed — the NetIn
   handler at `FUN_00da9ce0` reads `SGWPlayer+0x8c → *+0x24`; if
   the bag-list map is still nil at processing time, the lookup
   no-ops and the new value is lost.

### Proposed server-side mitigation

The `map_loaded.rs` login burst at line 354 already sends
`onActiveSlotUpdate`:

```rust
args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
args.extend_from_slice(&(data.active_bandolier_slot + 1).to_le_bytes());
append_method!(method_idx::ON_ACTIVE_SLOT_UPDATE, &args);
```

But this fires DURING the world-entry packet bundle, before the
client has fully initialized `SGWPlayer.bagList`. The NetIn handler
at `FUN_00da9ce0` may see an uninitialized container map (option 3
above) and silently drop the value.

**Candidate fixes** (in priority order for playtest validation):

1. **Re-send `onActiveSlotUpdate` after `onClientReady`** — once
   the client has confirmed it's ready, force a fresh broadcast.
   The existing `cell::service::base_messages::player_init`
   handler runs after `onClientReady` and is the natural site.
2. **Re-send on every successful `request_active_slot_change`** —
   already done (the existing handler emits
   `onActiveSlotUpdate` per swap), so the asymmetry is just the
   initial state.
3. **Send a redundant `onActiveSlotUpdate` 100 ms after world
   entry** as a defensive resync. Worst case: a single redundant
   wire packet per login.

The simplest first-line: add the re-send to `InitPlayerState` after
the cell entity has hydrated. If lomiada's next session shows the
swap working, we've confirmed the diagnosis without RE-ing the
client further.

## Key addresses (Ghidra)

```
useAbility Lua binding                      0x00aa2910
  arg validation (number)                   0x00403330
  arg validation (existence)                0x00403280
useAbility emit path                        0x00ad78e0
  target resolve                            0x00dd0de0
  event build + emit                        0x00d2ae40
  in-flight queue gate                      0x00d2b020
    queue head accessor                     0x00d2a000
useAbility Mercury method                   0x3a0 (928)

requestActiveSlotChange Mercury method      0x3ec (1004)
getActiveSlotForContainer Lua binding       0x00aa6e40
  container map walk                        0x00ad8ad0
  active-slot field offset                  +0xc on slot node
onActiveSlotUpdate Mercury method (S→C)     500 (0x1F4)
  NetIn handler                             0x00da9ce0
    bag lookup                              0x004649a0
GameEntityManager in-flight queue field     +0x228
SGWPlayer bag-list field                    +0x8c → +0x24
```

## Investigation methodology

Findings recovered via Ghidra MCP analysis of `SGW.exe` in a session
on 2026-06-04. Decompilation evidence from
`CEGUI__unknown_00aa2910`, `FUN_00ad78e0`, `FUN_00d2ae40`,
`FUN_00d2b020`, `FUN_00ad8ad0`, `FUN_00da9ce0`. CME event
registration confirmed at `CMERegistry__RegisterAllEventEmitHandlers`
@ `0x005ca888`. Mercury method indices verified against the
client's wire-method table and cross-checked with
`docs/protocol/client-method-dispatch-table.md`.

## Next steps

1. **Heal Focus**: pure-RE follow-up — trace the action-bar Lua
   that builds the `useAbility` arg list to see whether it passes
   the player's entity id as arg2 for self-casts.
2. **P90 swap**: shipped via PR #502 — re-send
   `onActiveSlotUpdate` from `handle_init_player_state` after the
   client has confirmed `onClientReady`. Needs playtest to
   confirm diagnosis (the next session should show non-zero
   `requestActiveSlotChange` events after F-key presses).

## In-flight queue audit (2026-06-04 follow-up)

**Verdict: the success-path drain is correctly wired; the
rejection-path drain is missing.**

`AbilityCooldownUpdate` (Mercury method 12 / `onTimerUpdate` with
`TIMER_ABILITY_COOLDOWN`) is the packet that drains the client's
in-flight ability queue at `GameEntityManager+0x228` via the
handler `LAB_00cea050` in `FUN_00cc33f0`.

Audited every committed-path return from
`crates/services/src/cell/abilities/use_ability/mod.rs::handle_use_ability`:

| Outcome | `onTimerUpdate` emitted? | Drain ok? |
|---|---|---|
| Successful commit | ✅ line 530 (every shot) | yes |
| Unknown ability id | ❌ early return | gap |
| Ability not in known set + not weapon-granted | ❌ early return | gap |
| Ability on cooldown | ❌ early return | gap |
| Reload in flight | ❌ early return | gap |
| No ammo | ❌ early return | gap |
| Target dead | ❌ early return | gap |
| Out of range | ⚠ sends `onErrorCode(42)` only | likely gap |
| Mid-draw queued | ❌ early return | gap |
| Bandolier slot swap in progress | ❌ early return | gap |

If the client speculatively adds an entry to its in-flight queue
**before** receiving the server's response, every rejection-path
return leaves a stuck entry. The next `useAbility` call —
including Heal Focus — would then be suppressed by the gate at
`FUN_00d2b020`.

**Whether the client speculatively populates the queue is not yet
confirmed.** The path to validate is:

1. Inspect `FUN_00d2b020` callers — does any caller insert into
   `this+0x228` BEFORE the wire emit (speculative), or only on
   the response NetIn handler (reactive)?
2. If speculative: ship a synthetic drain packet on every
   `handle_use_ability` rejection. Could be a no-op
   `onTimerUpdate(ability_id, TIMER_ABILITY_COOLDOWN, dur=0, ...)`
   sent right before each `return false`, or a dedicated
   `onAbilityRejected` wire method (requires `entities/defs` +
   client RE to confirm a handler exists).
3. If reactive: the client only adds to the queue on
   `LAB_00cea050` — which means the queue should always be
   drainable by the NEXT `AbilityCooldownUpdate`, no fix needed.

This investigation is not blocking the P90 swap fix (#502),
which addresses an unrelated gate. The follow-up here is a
separate RE session against the client's `useAbility` send
pipeline at `FUN_00d2afc0` → `FUN_00d2ae40` → `FUN_00cacd50`,
specifically looking for inserts into the queue at offset
`+0x228` on the emit side.

**If lomiada's next session shows Heal Focus still failing AFTER
PR #502 lands**, the in-flight queue is the next suspect and
the RE follow-up above becomes priority.
