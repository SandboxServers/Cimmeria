---
name: project-chat-contact-audit-2026-05-31
description: CAT-L audit findings; chat/contact/communication trust posture as of 2026-05-31
metadata:
  type: project
---

## CAT-L audit snapshot — 2026-05-31

**Why:** captured the chat / contact-list / communication surface trust posture at
the time of the 2026-05-31 server-authority audit so future re-reviews can pick up
incrementally.

**How to apply:** when re-auditing CAT-L or implementing a chat / contact / org-ping
handler, start from this snapshot rather than re-discovering the wire surface.

### Implemented (live exploit surface)

- `sendPlayerCommunication` (base method `0xC2`, `dispatch.rs:77-156`) — the
  ONLY chat path that broadcasts. Server-authoritative speaker_name +
  speaker_flags (from session `access_level` / `dnd_message`). Cell-side
  broadcast at `cell/chat.rs` filters channel to SAY/EMOTE/YELL only.
  **Gaps:** no rate limit, no text length cap, no ignore-list filter, no
  profanity filter. → CAT-L-01.
- `chatSetDNDMessage` (base method `0xC4`, `dispatch.rs:191-233`) — stores
  WSTRING on `ConnectedClientState.dnd_message` with no size cap. → CAT-L-02.

### Stubbed / log-only (latent traps)

- `chatJoin` / `chatLeave` / `chatSetAFKMessage` (base methods `0xC0`/`0xC1`/`0xC3`)
  — debug-log only, discard the `password` field on chatJoin. → CAT-L-09.
- All six contact-list cell methods (indices 55-60 in `cell_methods/contact_list.rs`)
  — log-only stubs that decode the leading 4-8 bytes and return. No self-add
  guard, no list-size cap, no ownership check. → CAT-L-04.
- `BroadcastMinimapPing` (cell method index 10 in OrganizationMember interface,
  `cell_methods/organization.rs:48-64`) — decodes `(org_id, x, y, z)` and logs.
  No org-membership check, no NaN guard, no rate limit. → CAT-L-05.
- `WHO` (cell method index 73, `cell_methods/player/interaction.rs:17-20`) —
  single-line info-log stub. → CAT-L-07.

### Unhandled (warn-arm only — no exploit until handler is added)

- `Event_NetOut_SendGMShout` — registered NetOut event, no Rust handler;
  needs `access_level >= GameMaster` gate when implemented. → CAT-L-06.
- `Event_NetOut_Petition` — sender-identity must be session-authoritative.
  → CAT-L-07.
- `Event_NetOut_ChatFriend`, `Event_NetOut_ChatIgnore` — need same
  ownership / cap / self-add discipline as contact-list. → CAT-L-07.
- `Event_NetOut_ChatOp/Mute/Kick/Ban/Password` — need per-channel
  server-tracked op-bit before applying admin action; channel name MUST be
  normalized (case-fold, trim) before the op-bit lookup. → CAT-L-08.

### Authority sources used correctly

- `player_name` reads from `ConnectedClientState.player_name`, set from DB at
  `play_character.rs:133`. **Authoritative.**
- `access_level` reads from `ConnectedClientState.access_level`, set from DB at
  `auth/handlers.rs:486-488` and `login/mod.rs:156`. **Authoritative** — used
  to compute `SPEAKER_GM` bit, regression-pinned by tests 1-4 in
  `dispatch.rs:588-660`.
- Witness list reads from server-computed AoI
  (`space_mgr.get_entity(sender_id).witnesses`). **Authoritative.**

### Key dispatch entry points (for incremental re-audit)

- Base layer: `crates/services/src/base/dispatch.rs:dispatch_sgw_player_base_method`
  — handles msg_id 0xC0..=0xD8 range. Catch-all warn arm at `dispatch.rs:333-346`.
- Cell layer: `crates/services/src/cell/dispatch/router.rs:dispatch_cell_method`
  — routes by `method_index` through inheritance order. Catch-all warn arm at
  `router.rs:101-106`.
- Slash-command layer (separate path): `crates/commands/src/registry.rs` +
  `crates/commands/src/permissions.rs` enforces `access_level` for typed `/give`,
  `/spawn`, etc. — NOT used by the wire `Event_NetOut_*` path. Two GM dispatch
  paths exist. → see [[reference-gm-auth-plumbing-gap]].

### Tests covering chat correctness

- `dispatch.rs:599-660` — speaker_flags = 0 default, =SPEAKER_GM when
  access_level>0, =SPEAKER_DND when dnd_message set, combined when both. Tests
  pin the byte-exact wire encoding.
- `world_entry_chat.rs:62-237` — `DEFAULT_CHAT_CHANNELS` byte-exact channel set
  + `onChatJoined` / welcome-message wire layout.
- `chat.rs:200-339` — onPlayerCommunication serialization byte-exact;
  witness fan-out; CHAN_TELL ignored by CellService.

No rate-limit test, no length-cap test, no ignore-list test — because those
features don't exist yet.
