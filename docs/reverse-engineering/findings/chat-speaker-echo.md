---
title: "Chat Speaker Echo — say/emote/yell"
type: reference
audience: engineers
---

# Finding: Spatial Chat Doubled the Speaker's Own `say` Line

**Confidence**: HIGH (the legacy server's own source comment states the client behaviour directly; corroborated by SigNoz evidence from the reported incident and a negative check of the client's `ChatWindow.lua`)
**Date**: 2026-09-28
**Sources**:

- `deprecated/python/cell/SGWPlayer.py:1825-1844` (`processPlayerCommunication`) — the 2009 developers' own comment
- `crates/cell-console/src/cell/console/chat/spatial.rs` (`broadcast_to_witnesses`, before the fix)
- SigNoz logs, `cimmeria-server` service, 2026-09-27/28 (see Evidence)
- `Content/UI/Core/ChatWindow/ChatWindow.lua` (client tree, `onTextAccepted`/`onMessageReceived`)

## Description

A tester reported (colo, 2026-09-27 ~23:20 local) that their own `say` line
appeared twice in the Info chat tab, while the other player's line appeared
once:

```text
[a4ga4g] says test
[4ta4w] says test
[4ta4w] says test
```

The tester's hypothesis was that the client shows its own line locally and
the server also echoes it back. That hypothesis is correct for `say`
specifically — but Cimmeria's implementation had it backwards for all three
spatial channels.

### What the legacy server did

`deprecated/python/cell/SGWPlayer.py:1825-1844`:

```python
def processPlayerCommunication(self, speaker, speakerFlags, target, channelId, message):
    if channelId in [Atrea.enums.CHAN_say, Atrea.enums.CHAN_emote, Atrea.enums.CHAN_yell]:
        ...
        self.witnesses.onPlayerCommunication(speaker, speakerFlags, channelId, message)
        # The client only echoes back messages in CHAN_say for some reason
        # For all other channels we need to notify the client as well
        if channelId != Atrea.enums.CHAN_say:
            self.client.onPlayerCommunication(speaker, speakerFlags, channelId, message)
```

`self.witnesses.onPlayerCommunication(...)` is the BigWorld witness broadcast
— every client currently observing the speaker, which never includes the
speaker itself. `self.client.onPlayerCommunication(...)` is an explicit send
to the speaker's own client, gated to fire for every spatial channel
**except** `say`.

Read literally: **the 2009 client shows its own `say` line locally** (the
"echoes back" in the comment), so the original server never needs to, and
must not, send it a second copy. For `emote` and `yell`, the client does
**not** show its own line locally, so the legacy server explicitly notifies
the speaker's own client for those two channels.

### What Cimmeria did (the bug)

`crates/cell-console/src/cell/console/chat/spatial.rs::broadcast_to_witnesses`
unconditionally sent the speaker's own echo for all three spatial channels:

```rust
// Also send to the sender themselves (client needs server echo for say channel,
// and sending for all spatial channels is harmless)
let _ = tx.send(CellToBaseMsg::EntityMethodCall {
    entity_id: sender_id,
    method_index: ON_PLAYER_COMMUNICATION,
    args,
}).await;
```

This was introduced deliberately during the social-systems campaign (SS-C1,
commit `92cdeddaa`, worknote `docs/analysis/social-systems/worknotes/ss-c1.md`
§"The lone-speaker echo"), on the stated premise "the client does not echo
say" — the *opposite* of the legacy comment above. That worknote records no
independent verification against the real client; it appears to be a
transcription error reading the same Python comment the wrong way round.

Once the server also echoes `say` unconditionally, and the (unmodified, real
2009) client independently shows its own `say` line locally, the speaker
sees their own line **twice**: once from the client's native local echo,
once from the server's `onPlayerCommunication`. Witnesses only ever get the
one copy the server sends them, so only the speaker is affected — exactly
the screenshot's shape (`4ta4w`'s own line doubled; `a4ga4g`'s line, seen
only as a witness, not doubled).

## Evidence

### The client shows nothing locally through `ChatWindow.lua`

The chat window's own text log (the Info tab) is populated by exactly one
path in `Content/UI/Core/ChatWindow/ChatWindow.lua`:

```lua
function ChatMod.onMessageReceived( this, speaker, speakerFlags, channelId, channelName, text )
```

subscribed at `Inst1ChatWin:subscribe(Events.MessageReceived,
'ChatMod.onMessageReceived')` (line 1320) — a network event. The input-submit
handler, `ChatMod.onTextAccepted` (line 320), never calls
`ChatMod.onMessageReceived` or any other chat-window text-append function; it
hands the raw text to the native `processTextCommand(text)` and returns. So
whatever "echoes back messages in CHAN_say" in the legacy comment is native
client code below the Lua layer (not independently re-derived here — no
Ghidra MCP session was available for this investigation; treat the exact
client-side mechanism as UNRESOLVED, the *fact* of the echo as HIGH-confidence
from the first-party comment).

### SigNoz: the server sent exactly one `onPlayerCommunication` to the speaker, not two

Server logs from the reported incident (`cimmeria-server`, `service.version =
c4ab2cb1...`, 2026-09-28T04:21-04:30Z ≈ 2026-09-27 23:21-23:30 local,
`account_id` 3/4, entities `4ta4w`=2 and `a4ga4g`=4, both from
`72.206.34.241`):

- One `sendPlayerCommunication` INFO log per chat line per player (base
  received the client's packet exactly once — no double-dispatch, no Mercury
  retransmit duplication observed).
- One `Broadcasting chat to witnesses` DEBUG log per chat line
  (`cell_console::cell::console::chat`), with `witness_count: 1` for each
  (the two players see each other).

This rules out a double-send bug in the application layer or an unfiltered
self-witness bug (`entity.witnesses` always excludes self —
`crates/cell-world/src/cell/space_manager/aoi.rs:96-97`,
`if cid == player_id { continue; }`). The old `spatial.rs` code sent the
witness broadcast (1 message, to the other player) plus exactly one
unconditional echo (1 message, to the sender) — one send per recipient,
matching the log evidence. The doubling the tester saw was therefore not a
double *send*; it was one (correct, by the old code's own logic) server send
landing on top of the client's own pre-existing local echo.

## Implementation Impact

Fixed in `crates/cell-console/src/cell/console/chat/spatial.rs`
(`broadcast_to_witnesses`): the sender-echo `EntityMethodCall` is now gated
on `channel != CHAN_SAY`, restoring the legacy per-channel split exactly.
`emote` and `yell` are unaffected (the speaker still gets an explicit echo,
since the client does not show those locally). A lone `say` speaker (nobody
in range) now gets nothing from the server for their own line — matching the
legacy server exactly, and relying on the client's own local echo to show it
at all.

Tests: `crates/cell-console/src/cell/console/chat/tests/spatial.rs`:

- `say_does_not_echo_to_speaker_but_emote_and_yell_do` — the direct
  regression guard; fails if the `CHAN_SAY` skip is removed, or if
  `emote`/`yell` stop being echoed.
- `lone_speaker_gets_no_say_echo_but_does_get_emote_and_yell_echo` — the
  zero-witness case for all three channels.
- `broadcast_say_to_witnesses`, `spatial_chat_skips_ignoring_witness`
  updated for the corrected `say` count; `broadcast_say_skips_npc_witnesses`,
  `spatial_chat_reaches_witness_the_speaker_ignores`,
  `spatial_chat_ignore_matches_case_insensitively` switched to `CHAN_EMOTE`
  where the assertion was about something other than the say/emote split
  (NPC filtering, Ignore directionality/case-folding), so they stay valid
  regardless of future say/emote echo changes.

## Open Questions

- The exact native client mechanism that shows a speaker's own `say` line
  locally (bubble-only vs. also into the Info tab text log) is not
  independently confirmed in this pass — Ghidra MCP was unavailable. If a
  future session can reach it, corroborate against `processTextCommand`'s
  native implementation and the `EChannel::Say` handling path.
- Whether the 2009 client also has any client-side dedup for a duplicate
  `onPlayerCommunication` (in case of a future protocol-layer retransmit)
  is unknown; this fix addresses the observed cause and does not add
  server-side dedup.
