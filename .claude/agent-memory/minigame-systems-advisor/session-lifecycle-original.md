---
name: session-lifecycle-original
description: Confirmed original SGW minigame session lifecycle — no server-side timeout, abort reports Canceled=0 (not Defeat), the three AbortReason triggers, and the two client-driven cleanup RPCs
metadata:
  type: project
---

Evidence from the C++ + Python reference tree (read 2026-09-17 during the CA04 review).
All line refs are `deprecated/` paths.

## There is NO pending-session timeout in the original

`MinigameRequestManager::QueueEntry` (`cpp/src/baseapp/minigame.hpp:25-39`) has **no
timestamp field**, and there is no timer/sweep anywhere in `minigame.cpp` /
`minigame_connection.cpp`. `queue_` entries leave the map only via:

- `remove()` (`minigame_connection.cpp:81-89`) ← called from `minigameClosed()` (:227)
- `cancel()` (`minigame_connection.cpp:59-79`) ← `Atrea.cancelMinigameSession`

`BaseService.config:27-34` has only `minigame_external_address` / `minigame_external_port`
/ `minigame_port`. No timeout knob.

**Do not be fooled by** the WARN string at `minigame_connection.cpp:386`:
`"Minigame login failed for entity ID %s, Key %s (maybe it timed out?)"` — that is the
author speculating about a client-side condition, not evidence of a server timer.

### The original's actual cleanup path is client-driven, two RPCs

1. `base/SGWPlayer.py:99 endMinigameForPlayer(ticket)` → `Atrea.cancelMinigameSession(entityId)`.
   Fired by the cell when the client reports the minigame window closed.
2. `cell/SGWPlayer.py:2010 minigameStartCancel()` — player dismisses the start dialog
   before the SWF ever connects. Reports `MINIGAME_RESULT_NotStarted` (3).

Both are **UNIMPLEMENTED in Rust**: `crates/services/src/cell/cell_methods/minigame.rs`
`START_CANCEL` (CM method 30) only logs `"UNIMPLEMENTED: minigameStartCancel"`, and there
is no `endMinigameForPlayer` path at all. Any TTL sweep in Cimmeria is a *substitute* for
these, not a port of original behaviour — say so when reviewing one.

## Result codes: Canceled(0) is distinct from Defeat(2)

`minigame.hpp:8-18`, mirrored in `python/common/Constants.py:12-15`:

| Code | Name | Meaning |
|---|---|---|
| 0 | `MINIGAME_RESULT_Canceled` | canceled *after* start |
| 1 | `MINIGAME_RESULT_Victory` | |
| 2 | `MINIGAME_RESULT_Defeat` | player actually lost |
| 3 | `MINIGAME_RESULT_NotStarted` | canceled *before* start |

## `aborted()` is a client-facing hook; the C++ wrapper does the upstream report

`Placeholder.aborted()` (`python/base/minigame/Placeholder.py:60-63`) only does
`self.started = False` (an original bug — shadows the `started` *method* with a bool) and
`sendFailure()` → xt `_cmd=failure`. It reports nothing upstream itself.

`PythonMinigame::abort()` (`cpp/src/baseapp/minigame.cpp:154-181`) is what reports:
calls `aborted()` → `running_=false` → `connection_->closeMinigame()` (onPlayerLeaveGame,
onGameEnd, roomDel — `minigame_connection.cpp:190-208`) → `handler_(this, MinigameCanceled)`.

So an abandoned game reports **Canceled (0), never Defeat (2)**. Ordering is:
aborted()'s xt output first, teardown second, upstream result last.

### Three AbortReason triggers (`minigame.hpp:64-78`)

| Reason | Value | Raised from |
|---|---|---|
| `AbortConnectionClosed` | 0 | `connectionClosed()` (`minigame.cpp:146-151`) |
| `AbortPlayerCanceled` | 1 | `MinigameRequestManager::cancel` (`minigame_connection.cpp:73`) |
| `AbortIllegalAction` | 2 | bad/oversized packet, or `message()` returning False (`minigame_connection.cpp:557-563, 567-573`) |

`AbortIllegalAction` matters for server authority: in the original an unknown extension
command returns False from `Placeholder.message` (`Placeholder.py:69-70`) → `handleMessage`
false → abort + socket close. Cimmeria's `MinigameInstance::message` returns
`Vec<GameOutput>` with no failure channel, so this path is unreachable there.

## No total play-time cap

Session lifetime is bounded only by the socket, `victory()`, `abort()`, or explicit
`cancel()`. Per-game time limits are *in-game* (e.g. Livewire `paramTimer`,
`Livewire.py:179`) and produce a normal Defeat result — not a server-side session kill.
Exempting a connected session from age-based expiry therefore matches the original.
Half-open TCP pinning a session forever is equally true of the original (asio
`async_receive` with no read timeout, `minigame_connection.cpp:534-541`).

See also [[difficulty-ranges]], [[chain-wiring-and-gaps]], [[game-implementation-status]].
