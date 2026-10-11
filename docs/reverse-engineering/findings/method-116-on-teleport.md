---
type: reference
audience: engineers deciding what the server sends with a teleport, and anyone asking why the client's drop oracle reports `onPlayerTeleport` on `SGWGmPlayer`
last_updated: 2026-10-11
companion_docs:
  - ../../protocol/client-method-dispatch-table.md
  - ../../analysis/telemetry-gaps/README.md
  - ../../analysis/telemetry-gaps/reviews/movement.md
  - ability-client-hook-anchors.md
---

# Method 116 (`onPlayerTeleport`): the client binds it for no class

> **Diátaxis type**: reference
> **Packet**: TG-MOV-09 of the telemetry-gaps campaign; feeds decision D-TG17
> **Binary**: the QA `SGW.exe` (ASLR off, so every address is the live address)
> **Method**: repo docs first, then headless Ghidra 12.0.4 (`tools/re/ghidra-headless/Probe.java`), 2026-10-11. Static only; no live client was attached.
> **Confidence legend**: **VERIFIED** = read in a decompile or string search in this pass. **INFERRED** = follows from verified code. **UNRESOLVED** = not found; the next action is named.

## Summary

The client drops `onPlayerTeleport` (method 116) on a GM player entity because **it drops it on every entity class**. No `Event_NetIn_*` handler is bound to method 116 under any class, so the `SGWGmPlayer` drops are not a GM quirk: GM `.gotoxyz` and ring travel are simply where the 195 rows came from. A regular `SGWPlayer` receiving the same bundle drops it the same way. The server's "streaming pre-load" hint therefore never reaches any stock client, and nothing is lost for a GM that a normal player has.

## Plain-language version

The server tells the client "you were just teleported, wait for the terrain at this spot" by sending a message numbered 116 next to the engine's position snap. The client keeps a phone book of messages it knows what to do with, filled in once at startup. Message 116 has no entry for any kind of player, so the client reads the number, finds no entry, and throws the message away without error. The position snap beside it is a different message with its own entry, and it is handled normally.

## What the client does with 116 (VERIFIED)

1. **One startup sweep binds every inbound entity method.** `0x00db3390` calls `FUN_00c6f1e0(class, method, event)` 162 times. The 162 call sites were parsed from the headless decompile as (event, method name, class name) triples. They split as `Account` 6, `SGWSpawnableEntity` 8, `SGWBeing` 13, `SGWPlayer` 124, `SGWMob` 2, `SGWPet` 3, `SGWGmPlayer` 6.
2. **`onPlayerTeleport` is in none of them.** None of the 162 triples names it, and a string search of the binary for `playerteleport` returns no match. Method names are plain strings in this sweep (a control search finds `onCharacterList`, `onLOSResult` and the rest), so absence is real. The same holds for `onTopSpeedUpdate`, `giveAbility`, `giveXPForLevel` and the other methods the dispatch table lists as "bound nowhere".
3. **The six `SGWGmPlayer` registrations are the GM-only client methods** and nothing else: `onLOSResult`, `onShowWaypoints`, `onShowPath`, `onDisableShowPath`, `onSetTarget`, `onShowNavigation`. `SGWGmPlayer.def` inherits `SGWPlayer` and appends them after index 156 (see the GM table in [client-method-dispatch-table.md](../../protocol/client-method-dispatch-table.md)).
4. **The dispatcher walks the parent chain, then drops.** `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) looks up `(clientIndex, methodIndex)` in the receiving entity description's handler tree (`desc+0xe0`). On a miss it fetches the parent class via `FUN_0158eca0` and retries, so a `SGWGmPlayer` is tried as 3, then `SGWPlayer`, and so on. When the chain ends with no node it calls `FUN_01590f30(desc+0xe0, index)` and returns. Nothing else in the bundle is lost. The telemetry DLL turns that miss into `client.dispatch.method_dropped`.

So the answer to "why is it dropped" is: the client's C++ never subscribed a handler to index 116, for any class. The `SGWPlayer.def` comment ("causes the client to force a waiting state flag ... otherwise the streaming level loading code will not know which chunks to wait for") describes what the BigWorld-side design wanted; the shipped client has no receiving end. Whether the handler was removed late or never written is not recoverable from the binary (UNRESOLVED, no evidence either way).

## Why 195 of 200 rows are `SGWGmPlayer` (INFERRED)

The drop oracle in [reviews/movement.md](../../analysis/telemetry-gaps/reviews/movement.md) counted 195 `onPlayerTeleport` drops in 7 days: 170 with class `SGWGmPlayer`, 21 with an empty class. The server sends 116 to every client through the class-agnostic bundle builder (`build_teleport_bundle`, `crates/base-world-entry/src/base/world_entry/teleport.rs`, which addresses it with `IDBASE_SGW_PLAYER`), so the class split reflects who teleports in the observed window (GMs using `.gotoxyz`), not who drops. The binary gives no class where the drop would not happen.

## Does the drop lose the streaming pre-load?

Not relative to a normal player: nobody ever had it. Relative to the original design, the 116 hint is inert on this client. What the client does use for a teleport is the engine-level `FORCED_POSITION` message, handled by `ServerConnection_forcedPosition` (`0x00dd9ee0`, VERIFIED): it queues the move through `ServerConnection_addMove` with the forced flag, records the physics byte in `sentPhysics_`, and calls the connection listener at `this+0x168` (vtable slot `+0x28`) when one is set. It does not reference any streaming state. Whether that listener, or the map-load code, is what holds the player for terrain at a long-distance teleport is **UNRESOLVED**: `StreamingLevel` has 14 string hits but no link to a teleport path was traced here. Next action: a live lab run of `.gotoxyz` to a far, unloaded chunk, watching for a hitch or fall-through, or an x64dbg logging breakpoint on `0x00dd9ee0` against the first chunk load.

## Server side today

- `handle_teleport_player` in `crates/base-world-entry/src/base/world_entry/teleport.rs` builds one bundle: `FORCED_POSITION` plus method 116 (24 argument bytes: position, then a zero direction). It stamps `streaming_hint = "method_116"` on the forced-position row (`teleport.rs:164` and `:540`), which claims a hint that the client discards.
- The burst-shape test `teleport_bundles_forced_position_and_player_teleport_to_single_packet` pins the two-message bundle. Ring transport (`crates/cell-content/src/cell/ring_transport/dispatch.rs`) also expects the pair.
- The `streaming_hint` field exists in the SigNoz row, so any option below can change its value to be honest.

## Options for D-TG17

The owner chose to decide after this RE. The finding removes the premise of the "GM only" framing in the original packet text: class 0x03 is not special.

| Option | What it changes | Pros | Cons |
|---|---|---|---|
| **A. Keep sending 116 to all classes, relabel the hint** (`streaming_hint = "method_116_unbound"`) and teach the client drop oracle to treat 116 as a known-unbound method | One string, plus the oracle allowlist (client telemetry DLL side) | No wire change, no new test shape, behaviour identical to the original server, and a future client patch that binds 116 works with no server change | Keeps about 30 wasted bytes per teleport and, until the oracle allowlist ships, one WARN per teleport |
| **B. Skip 116 for class 0x03 only** (the packet's original wording) | Branch on `player_class_id` in `build_teleport_bundle`, `streaming_hint = "skipped_gm_class"` | Removes about 85% of observed drop rows | Not supported by the binary: `SGWPlayer` drops it identically, so the other 15% stay and GM and player bundles diverge for no client reason. If a patch ever binds 116 for players, GMs silently lose it. **Not recommended** |
| **C. Skip 116 for every class** | `build_teleport_bundle` becomes `FORCED_POSITION` only; `streaming_hint = "skipped_unbound"`; update the burst-shape test and the ring-transport expectations; update wire-log expectations | Removes all drop rows and the wasted bytes; the server stops claiming a hint the client ignores | Changes the wire bundle the ring-transport UAT and its tests pin; discards the hook a future patch (D below) would want; the saving is tiny, so the gain is mostly log hygiene |
| **D. Route the hint another way** | Needs an RE result first, then possibly a new client patch | Could restore the intended terrain wait | No alternative client mechanism has been found (see "Does the drop lose the streaming pre-load?"). A new client patch needs a maintainer decision per the repo rules, and the benefit is unmeasured. Not actionable until the lab run above shows a real streaming problem |

**Recommendation:** A. It is the smallest change, it is the only option that does not rely on a claim the binary contradicts, and it fixes the telemetry lie (`streaming_hint = "method_116"`) without touching the wire. Choose C only if the owner wants the oracle quiet and accepts reopening the bundle shape; take D only if the lab run shows long-distance teleports actually misload terrain.

If A is chosen, the test is a LogCapture check that the forced-position row carries `streaming_hint = "method_116_unbound"` (the existing test at `teleport.rs:540` asserts `method_116` and fails when the value changes back). If C is chosen, the test is the existing bundle-shape test inverted to assert exactly one message.

## Corrections to existing docs

- The movement review ([reviews/movement.md](../../analysis/telemetry-gaps/reviews/movement.md)) and the TG-MOV-09 packet frame the drop as specific to `SGWGmPlayer` and ask whether to skip "class 0x03". The binary shows the drop is class-independent.
- [client-method-dispatch-table.md](../../protocol/client-method-dispatch-table.md) already lists 116 as bound nowhere (correct). This finding adds the independent re-parse of all 162 registrations.

## Evidence trail

| Claim | Evidence |
|---|---|
| 162 startup registrations, none for 116 | headless `D:0x00db3390`, triples parsed from the decompile; `S:playerteleport` no match |
| Unbound by class | same parse: per-class counts above; the `SGWGmPlayer` six listed |
| Dispatcher walks parents then drops | `D:0x00c6f8f0` (`FUN_0158eca0` parent step, `FUN_01590f30` miss path) |
| Forced position path | `D:0x00dd9ee0` (`ServerConnection_forcedPosition`) |
| Server sends 116 to all classes | `teleport.rs` `build_teleport_bundle`, `IDBASE_SGW_PLAYER` |
| 195 of 200 drops, class split 170 / 21 | [reviews/movement.md](../../analysis/telemetry-gaps/reviews/movement.md) (SigNoz, 7 days) |

## Open questions

1. What, if anything, makes the client wait for terrain after `FORCED_POSITION`? Resolve with the lab run or a logging breakpoint described above.
2. Was the 116 handler removed or never written? Unanswerable from the binary; the legacy server code only shows the sender.
