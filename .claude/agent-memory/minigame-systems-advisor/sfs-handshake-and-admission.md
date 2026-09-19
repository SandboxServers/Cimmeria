---
name: sfs-handshake-and-admission
description: Handshake timing and admission facts for the SFS minigame port — verChk is library-sent on connect, the in-band cross-domain policy is inert, the original had no read timeout/conn cap, one socket per entity
metadata:
  type: project
---

Gathered 2026-09-19 while consulting on issue #532 (minigame DoS hardening).

## Handshake timing (what the client does between accept and login)

- SWF load, the pre-game dialog (`onStartMinigameDialog`, `minigameStartCancel`) and
  `gotoAndPlay("Start")` all happen BEFORE the TCP connect
  (`docs/reverse-engineering/findings/minigame-architecture.md:13-24`). None of that time
  falls between accept and verChk.
- The SFS 1.x client library sends `verChk` itself from its socket-connect handler.
  `API_VERSION = 154` is its version string 1.5.4 run together. This comes from knowing
  the SFS 1.x AS API. It has NOT been confirmed from the SGW SWFs.
- Whether game code calls `login` straight from `onConnection` is UNCONFIRMED: no SWF
  has been decompiled for it yet. Decompile the shipped `.upk` SWFs with JPEXS to settle it.

## The original never handled `<policy-file-request/>`

`handleMessage` rejects any frame that has no `<msg>` root
(`deprecated/cpp/src/baseapp/minigame_connection.cpp:247-251`), and the connection then
closes. The original worked with the retail client, so that client never sends one. This
also points to Scaleform not enforcing the Flash socket-policy sandbox.

## The cross-domain policy is inert

Its `domain='*'` is sent in reply to verChk (`minigame_connection.cpp:318-323`), after the
socket is already open. A policy file sent at that point cannot gate anything, and it
never protects the server from a raw socket. Keep the original bytes. The CAT-K audit
lists it as "Not filed" (`docs/security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md:542-545`).

## Admission in the original

- Reads have no timeout (`asio async_receive`, `minigame_connection.cpp:534-541`), there
  is no connection cap, and no per-IP limit. Any cap or timeout Cimmeria adds is
  hardening, not a port of the original.
- Each entity has at most one authenticated socket. `queue()` refuses a second entry for
  the same entity (`minigame_connection.cpp:33-44`), and the room is `maxu='1'`
  (`:415`). Spectator and helper sockets were never served (see the `isSpectator: False`
  flag in `Alignment.py:208`), and reconnecting with a used ticket fails because the
  entry is removed on close.

## A second login with a ticket already in play: the original allowed it

- `find()` (`minigame_connection.cpp:47-58`) checks only the entity id and the key.
- `handleLogin`'s "Duplicate login" guard (`:341-345`) only covers a second login on the
  same connection.
- `entry->session = this` (`:409`) simply overwrites the previous connection.

So a second socket gets its own game instance, and each instance fires the handler it
copied (`minigame.cpp:75`, `:117-127`). The original was saved further down the chain:
cell `handleMinigameResults` (`python/cell/SGWPlayer.py:1926-1935`) accepts only the first
result and drops any later one with a WARN, "no minigame is running".

Rust has no cell-side guard. `cell/service/base_messages/minigame.rs` fires
`on_victory_chains` for every victory it receives. The fix for #532 item 4 is to refuse a
claim on an already-connected session (first connection wins). A reconnect using the same
ticket was never possible in the original, because closing the connection removes the
entry (`:227`).

See also [[session-lifecycle-original]].
