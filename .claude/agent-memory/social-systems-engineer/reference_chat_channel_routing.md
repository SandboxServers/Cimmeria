---
name: reference-chat-channel-routing
description: EChannel routing table, legacy onError feedback precedent, and which channels are truly wired vs. registered-but-unsupported
metadata:
  type: reference
---

## Chat channel routing — what's actually wired (2026-09-26, chat-channel-echo fix)

`crates/services/src/cell/chat.rs::handle_chat_message` is the single cell-side
chat dispatch point. Before this fix it only handled 3 of the 11 `EChannel`
values and silently dropped everything else (debug-log only, no client reply)
— including the sender's own message. That silent drop is what the tester
report ("i dont see my own message sent to the server channel") was hitting.

### EChannel ids (python/Atrea/enums.py, mirrored as consts in cell/chat.rs)

| Id | Name | Registered client-side (`DEFAULT_CHAT_CHANNELS`) | Cell distribution |
|----|------|---|---|
| 0 | say | yes | Spatial AoI witness broadcast |
| 1 | emote | yes | Spatial AoI witness broadcast |
| 2 | yell | yes | Spatial AoI witness broadcast (same radius as say — no wider range impl) |
| 3 | team | yes | None — no group backing. Feedback line instead of drop (since this fix) |
| 4 | squad | yes | None — no squad backing. Feedback line instead of drop |
| 5 | command | yes | None — no org backing. Feedback line instead of drop |
| 6 | officer | **no** | None. Feedback line instead of drop (feedback always rides ch. 9, safe regardless of the unregistered input channel) |
| 7 | server | yes | System-broadcast-only (`CHANNEL_FLAG_DisallowPlayerMessages` in legacy `ChatChannelManager.__init__`). Player sends now get a feedback line explaining it's system-only |
| 9 | tell | yes | Not routed (no player-to-player tell implementation exists at all — only wire decoders for `onTellSent`). Feedback line instead of drop |
| 10 | splash | no | Not handled |
| 8 | (unused/feedback) | no (unregistered) | `CHAN_FEEDBACK` const aliases to 9 (tell), not a distinct id — sending on literal 8 triggers the client's red unknown-channel splash popup, so nothing in this codebase ever sends on 8 |

### Legacy precedent: `onError` feedback, not silence

`python/cell/SGWPlayer.py::processPlayerCommunication` (deprecated/python) only
special-cases say/emote/yell; everything else falls to
`self.onError("Speaking on channel %d is not supported yet!")`, and `onError`
is `self.client.onPlayerCommunication('', 0, CHAN_server, msg)` — i.e. the
legacy server ALWAYS replied to the sender, even for "unsupported" channels.
Only the server-channel-speak rejection in
`python/base/Chat.py::ChatChannelManager.sendPlayerMessage` was silent
(`canPlayerSpeak()` false → server-side `warn()` only, no client reply) — that
one gap is intentionally *not* legacy-faithful in the Rust port; the project's
"every button press gets visible feedback" rule wins over strict parity there.

### Established feedback-send pattern (reused, not duplicated)

Single-recipient system lines all follow the same shape across the codebase:
`onPlayerCommunication(speaker="SYSTEM", flags=0, channel=CHAN_FEEDBACK(9), text)`
sent as one `CellToBaseMsg::EntityMethodCall` to the sender's own entity id
only (no witness fan-out). Three independent implementations of this exact
shape exist: `base/gm_feedback.rs`, `cell/cell_methods/gm/feedback.rs`
(`send_gm_feedback`, re-exported via `cell/console/mod.rs`), and now
`cell/chat.rs::send_channel_feedback` (added for the non-GM chat-rejection
case — deliberately NOT reusing `send_gm_feedback` since that name is
GM-console-specific even though the wire shape is identical).

### Real gaps still open (documented in docs/gameplay/chat-system.md, not fixed here)

- Player-to-player `tell` (channel 9) has zero routing — no online-player
  lookup, no cross-instance delivery. Only wire *decoders* for `onTellSent`
  exist (`wire_log/decoders/generated.rs`), no *handler*.
- team/squad/command have no group/organization backing to route into.
- No slash-command or UI mechanism for channel selection was found in the
  `docs/analysis/sgw-handoff-pack-v1.2` slash-command table — "how do you
  speak to different channels" is very likely a client chat-window UI
  question (channel tabs/dropdown), not a server routing gap.

### Test gotcha

`SpaceManager::create_entity(entity_id, world_name, pos, rot)` — 2nd arg is
the **world/space name** (must match a space parsed via `parse_spaces_xml`,
e.g. `"Agnos"`), NOT the player's display name. The display name is a
separate arg to `handle_chat_message`. Mixing these up produces
`Err("Unknown world: <name>")` at `create_entity(...).unwrap()`.
