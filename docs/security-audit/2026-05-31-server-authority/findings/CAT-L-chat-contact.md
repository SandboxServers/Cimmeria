# CAT-L — Chat / Contact list / Communication — Findings

## Overall trust posture

The chat surface in Cimmeria is **partially implemented**: exactly five base methods
are dispatched (`chatJoin`, `chatLeave`, `chatSetAFKMessage`, `chatSetDNDMessage`,
`sendPlayerCommunication`), and three of those (`chatJoin`/`chatLeave`/`chatSetAFK`)
are log-only stubs. `sendPlayerCommunication` is the **only path that actually
broadcasts** — and it does so unconditionally to all AoI witnesses without any
rate limit, length cap, mute / ignore filter, or per-channel authorization. The
contact-list surface (6 indices), the channel-admin surface (mute / kick / ban /
op / password / friend / ignore), `Petition`, `Who`, `SendGMShout`, and
`BroadcastMinimapPing` are all **completely unimplemented** — the bytes are
consumed, an `UNIMPLEMENTED:` log line fires, and the handler returns. No state
mutates.

This produces two distinct risk shapes:

1. **Demonstrable today (live exploit surface):** `sendPlayerCommunication` will
   broadcast attacker-controlled `text` to every witness in the sender's AoI with
   no rate limit, no length cap, and no `chatIgnore` mute-list filter. The
   `speaker_name` and `speaker_flags` are server-authoritative
   (`crates/services/src/base/dispatch.rs:106,126-141`) — that part is correct —
   but the message body is forwarded verbatim, and a single attacker can
   ratchet chat-spam volume up to whatever the connection / Mercury bundle
   sustains. The `text_len` field bypass on the spec's WSTRING parser is bounded
   only by the WSTRING reader's overall buffer-size check, so messages up to the
   single-bundle limit (~65 KB) per call are forwardable.
2. **Likely-exploitable theoretical (lands the moment the stubs are filled):** the
   chat-admin commands (`chatOp`, `chatMute`, `chatKick`, `chatBan`,
   `chatPassword`), `SendGMShout`, `BroadcastMinimapPing`, `Petition`, `Who`, and
   every contact-list operation will trust attacker-controlled fields (target
   channel name, target player name, ping x/y/z, org_id, list_id) when those
   handlers are wired up, unless the implementor knows in advance which fields
   need server-side validation. The same systemic gap that landed on CAT-N
   applies here: `access_level` is not in scope for cell-method dispatch
   ([[reference-gm-auth-plumbing-gap]]), so a naive future implementation of
   `SendGMShout` or any "op-only" channel admin command lacks the bit to gate on
   even if the author remembers to gate.

The Python reference (`deprecated/python/`) and the Ghidra `Event_NetOut_*`
classes confirm the wire shapes — for example, `Event_NetOut_BroadcastMinimapPing`
carries the full client-asserted `(org_id, x, y, z)` quad. Once a handler is
wired up to actually broadcast or persist, every one of these fields needs an
explicit server-side cross-check (org membership, channel-op bit, friend
list size cap, self-add guard) that the current stubs do not encode.

Findings below are numbered in order of demonstrability — CAT-L-01 and CAT-L-02
are live today, the rest are "lands when the stub is implemented."

---

### CAT-L-01 — `sendPlayerCommunication` has no rate limit, length cap, or ignore-list filter

**Severity**: Medium
**Class**: Chat-spam / DoS / harassment vector (no anti-flood)
**Wire surface**: `Event_NetOut_sendPlayerCommunication`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The base-layer handler reads `(channel, target, text)` from the wire and forwards
`text` verbatim to every AoI witness via `chat::handle_chat_message ->
broadcast_to_witnesses`. There is no per-sender rate limit (no "messages per
window" counter on `ConnectedClientState`), no per-message length cap (the
WSTRING parser only bails on truncation, not on size), and no consultation of
any ignore list before the witness fan-out (no `ChatIgnore` machinery exists
anywhere — `grep -ri ChatIgnore crates/` returns zero hits). A modified client
can drive `sendPlayerCommunication` at packet rate, with each `text` payload up
to the Mercury bundle's u16 length-field max (~65 KB), and the server will fan
each one out to every witness in AoI. There is also no profanity filter / muted-
account filter.

**Evidence**
- Ghidra: `019b9b14` `Event_NetOut_sendPlayerCommunication` — class name string
  registered via `register_NetOut_sendPlayerCommunication @ 00cfda00`. The
  client emits the typed payload `(UINT8 channel, WSTRING target, WSTRING text)`
  through `SGWNetworkManager::EventHandler<Event_NetOut_sendPlayerCommunication>`
  (vtable installer at `00d56350`). No client-side length cap was located in
  Ghidra string scan for `MAX_CHAT_LEN` / `MAX_MESSAGE_LENGTH` / `MAX_TEXT_LENGTH`
  (zero hits) — so the client UI is the only natural truncation point and a
  modified client bypasses it.
- Client behavioral log: n/a (no spam observable in `SGWDebugLog.log` from a
  vanilla session — the gap is in what the server fails to enforce, not in what
  the client routinely sends).
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/base/dispatch.rs:77-156` (decode + forward),
  `crates/services/src/cell/chat.rs:65-95` (channel match, no rate limit),
  `crates/services/src/cell/chat.rs:101-157` (fan-out to every witness with no
  filter).

**Attack scenario**
1. Attacker modifies their client (or scripts a wire-level emitter) to call
   `sendPlayerCommunication(channel=0 /*SAY*/, target="", text="<spam body>")`
   in a tight loop.
2. The base handler consumes the bundle, calls `handle_chat_message`, which
   broadcasts `onPlayerCommunication` to every witness via
   `space_mgr.get_entity(sender_id).witnesses` (server-authoritative AoI list).
3. Observable effect: every player in AoI of the attacker receives a flood of
   `onPlayerCommunication` callbacks at the attacker's send rate, multiplied by
   the witness count. The server's outbound side has no per-recipient
   throttling on this path; each fan-out goes through one
   `CellToBaseMsg::EntityMethodCall` send per witness, then through the per-
   client Mercury channel. The attacker can also vary `text` to slip
   harassment / off-color content past any client-side moderation that a future
   chat-window UI might layer.

**Suggested remediation (one line)**
Add a per-sender token-bucket (e.g. 5 messages/3s for SAY/EMOTE/YELL, separately
per channel) on `ConnectedClientState` plus a length cap (`text.chars().count()
<= 256` matches the original SGW UI), and consult a server-side `ignore_list`
(when ChatIgnore is implemented) before each per-witness `EntityMethodCall`.

**Would benefit from x64dbg trace?**
No — Rust handler shape is fully readable; no additional client behavior needed
to confirm the absence of rate-limit code.

---

### CAT-L-02 — `chatSetDNDMessage` stores attacker-controlled WSTRING with no length cap

**Severity**: Low
**Class**: Memory / log-pollution
**Wire surface**: `Event_NetOut_ChatSetDNDMessage` (base method 0xC4)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The base handler at `crates/services/src/base/dispatch.rs:191-233` reads a
WSTRING from the wire and stores it directly on `ConnectedClientState.dnd_message:
Option<String>`. The only filter is `if message.chars().count() > 1` (i.e.,
2+ chars sets DND, ≤1 clears it). There is no upper bound — the WSTRING parser
admits any `char_count: u32` that fits in the remaining buffer (decoded by
`read_wstring` at `mercury/mod.rs:336`). A single packet can store up to ~65 KB
of attacker-controlled UTF-16 in the server's per-connection state, and that
buffer persists for the entire session. Today the `dnd_message` is only consulted
for a boolean ("is DND active" — sets the SPEAKER_DND flag), so the body never
reaches any other player; but if the auto-reply-tell path is ever wired up
(the comment at `dispatch.rs:198` says "future work"), it becomes a DM-spam
vector. Worse, the WSTRING is logged with `text_len` on the base span and the
body itself is unbounded for in-memory storage.

**Evidence**
- Ghidra: `Event_NetOut_ChatSetDNDMessage` class name appears as a registered
  NetOut event (alongside `ChatSetAFKMessage` at the same dispatch range). The
  payload is a single WSTRING (no length cap field).
- Client behavioral log: n/a (vanilla client UI truncates DND message; modified
  client bypasses).
- Cross-ref to Rust handler: `crates/services/src/base/dispatch.rs:207-232`.

**Attack scenario**
1. Attacker modifies client (or replays a packet) to send `chatSetDNDMessage`
   with a 32,000-character WSTRING.
2. Server stores the entire string on `ConnectedClientState.dnd_message`.
3. Observable effect: per-connection memory inflates by ~64 KB per shaped
   message; if the future auto-reply-tell path lands without retroactively
   capping the field, every chat-tell to the attacker triggers a 64 KB
   `onPlayerCommunication` to the sender. Also: the server log captures
   `dnd_active=true` and otherwise discards the body, so an audit trail of
   "what did the user actually set" is lost.

**Suggested remediation (one line)**
Validate `message.chars().count() <= 128` server-side after `read_wstring`;
truncate or reject longer strings (matches the SGW UI input cap).

**Would benefit from x64dbg trace?**
No.

---

### CAT-L-03 — `sendPlayerCommunication` channel byte not validated against allowlist

**Severity**: Low
**Class**: Channel-impersonation (theoretical)
**Wire surface**: `Event_NetOut_sendPlayerCommunication`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The client-supplied `channel: u8` is decoded at
`crates/services/src/base/dispatch.rs:82` and forwarded unchanged through
`BaseToCellMsg::ChatMessage { channel, .. }` to `chat::handle_chat_message`. The
cell-side match at `chat.rs:74` only acts on `CHAN_SAY | CHAN_EMOTE | CHAN_YELL`
and otherwise debug-logs and drops — so today, only those three channels actually
broadcast. The `channel` byte is then serialized verbatim into the witness-bound
`onPlayerCommunication(speaker, flags, channel, text)` args
(`chat.rs:127,187-188`). A client could *try* setting `channel = CHAN_SERVER (7)`
or `CHAN_FEEDBACK (8)` to make the witnesses' client-side UI render the message
as a server / system broadcast — but because `handle_chat_message` filters at
the dispatch step, the wire never carries it. **Today this is defended by the
filter, but the defense is brittle:** if a future change adds CHAN_TEAM or
CHAN_SQUAD to the cell-side broadcast arm without also gating on team/squad
membership, a non-member can post into a team channel by setting the channel
byte. Document the invariant before that future change lands.

**Evidence**
- Ghidra: `019b9b14` `Event_NetOut_sendPlayerCommunication` — wire-format
  preserves `channel: UINT8` as the first byte after the SGWPlayer base msg_id
  0xC2.
- Cross-ref: `crates/services/src/base/dispatch.rs:82,150` and
  `crates/services/src/cell/chat.rs:74-94`.

**Attack scenario** (theoretical, requires future code change)
1. A future PR adds `CHAN_TEAM | CHAN_SQUAD` arms to `handle_chat_message` to
   broadcast team / squad chat.
2. Attacker sends `sendPlayerCommunication(channel=CHAN_TEAM, target="", text=...)`
   without being in any team.
3. Without an explicit "is sender in a team?" check before broadcast, the message
   reaches whoever the team-broadcast scope selects.

**Suggested remediation (one line)**
Add an explicit `validate_channel_membership(entity_id, channel) -> Result<()>`
check at the head of every new channel arm, and document at the match site that
the client-supplied `channel` byte must be paired with a server-side membership
check (not just a "channel id is in range" check).

**Would benefit from x64dbg trace?**
No.

---

### CAT-L-04 — Contact list operations are full client-trust stubs (no self-add guard, no list-size cap, no ownership check)

**Severity**: High (when implemented)
**Class**: Friend/ignore-list integrity, harassment vector, DB-row inflation
**Wire surface**: `Event_NetOut_contactListCreate`, `contactListRename`,
`contactListDelete`, `contactListFlagsUpdate`, `contactListAddMembers`,
`contactListRemoveMembers`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
All six contact-list cell methods at
`crates/services/src/cell/cell_methods/contact_list.rs:14-73` are
`UNIMPLEMENTED:` log-only stubs that decode the leading `list_id` (i32) and
`flags` (u32) and return. None of the following invariants are encoded
anywhere in the codebase, and a future implementor working from the wire shape
alone is likely to miss them:

- **Self-add prevention:** `contactListAddMembers` decodes a member list; the
  implementor needs an explicit guard that no member entry resolves to
  `entity_id` (the caller's own player id). Without it, a player can add
  themselves to their own friend list — wasteful but, more importantly, can be
  used to enumerate side-effects of "is X my friend?" lookups in any future
  ChatIgnore / DND path that checks self-membership.
- **Add-without-consent:** the wire shape adds members by name / player id with
  no proposal / accept handshake (compare `tradeRequest` → `tradeRequestCancel`
  in CAT-H, which at least has an explicit two-step). A naive port would let a
  client unilaterally add anyone to its "ignore" list (harmless) AND its
  "friend" list (privacy violation — the target's "Who shows me online?" state
  may key on someone-friended-me).
- **List-size cap:** there is no upper bound on the number of members per list,
  the number of lists, or the total bytes per list. A client can call
  `contactListCreate` / `contactListAddMembers` in a loop and bloat the row
  count for that account indefinitely.
- **Caller-owns-list check:** `list_id` is client-supplied. `contactListRename`,
  `contactListDelete`, `contactListFlagsUpdate`, `contactListAddMembers`, and
  `contactListRemoveMembers` all decode `list_id` without any check that the
  caller (`entity_id`) actually owns that list row. When persistence lands, the
  WHERE clause MUST include `account_id = caller.account_id`, not just
  `list_id = ?`, or one player can rename / delete another's lists.
- **Caller-can-query-others:** the wire shape doesn't expose a "read another
  player's contact list" message, so this isn't an immediate concern — but the
  contact-list state is itself privacy-sensitive (who I have ignored / friended),
  so any future debug / GM accessor must be `access_level >= GM` gated.

**Evidence**
- Ghidra: `019c2528` `contactListCreate`, `019c25ac` `contactListAddMembers`,
  `019be13c` `Event_NetIn_onContactListAddMembers` (the server-to-client mirror,
  confirming the message family exists). The client UI dispatcher
  `Event_UI_ContactListAddMembers` at `01e0de90` confirms the field set comes
  straight from the UI without an authentication / consent step.
- Client behavioral log: n/a (vanilla client constructs these but never sends
  them in the captured session because the QA-build UI flow is incomplete).
- Cross-ref: `crates/services/src/cell/cell_methods/contact_list.rs:14-73` (all
  six methods stub-only).

**Attack scenario** (lands when the handler is implemented)
1. Attacker scripts `contactListAddMembers(list_id=<victim's list_id>,
   members=[attacker])` against a list_id discovered through observation /
   guess.
2. Server persists attacker into victim's list because the handler trusts
   `list_id` without `account_id = caller` cross-check.
3. Observable effect: victim's friend list silently gains the attacker as a
   member; if the future Who / online-status path keys "show me to my friends"
   on this list, the attacker also harvests presence info.

**Suggested remediation (one line)**
At each handler's head, persist `list_id`-owning lookup as
`SELECT account_id FROM contact_lists WHERE id = ? AND account_id = caller.account_id`
and bail on row-miss; reject add-list-members payloads where any entry equals
the caller's own player id; cap `member_count` at e.g. 100 per list and
`list_count` at e.g. 10 per account.

**Would benefit from x64dbg trace?**
Yes — trace `Event_UI_ContactListAddMembers` to enumerate the exact wire
field set (member name strings vs. player ids vs. account ids), because the
Ghidra string table names the event but the payload struct isn't fully
labeled and the fields the server will need to validate depend on which
identifier the client actually sends.

---

### CAT-L-05 — `BroadcastMinimapPing` is a stub that will trust client-supplied (org_id, x, y, z) when implemented

**Severity**: High (when implemented)
**Class**: Cross-org information leak, position-spoof, griefing-spam
**Wire surface**: `Event_NetOut_BroadcastMinimapPing` (cell method index 10 in
OrganizationMember interface)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The handler at `crates/services/src/cell/cell_methods/organization.rs:48-64`
decodes `(org_id: i32, x: f32, y: f32, z: f32)` from the wire and logs
`UNIMPLEMENTED:`. When the handler is wired up to broadcast a "ping" to org
members, it must validate **three** invariants the wire encoding leaves
attacker-controlled:

1. `org_id` matches the caller's actual organization (per
   `[[reference-gm-auth-plumbing-gap]]`, the org-membership lookup currently
   has no DB-backed canonical source — when added, must check `caller.player_id
   IN (SELECT player_id FROM org_members WHERE org_id = ?)`). Without this,
   any player can broadcast a ping into any org's minimap.
2. The `(x, y, z)` are within the caller's currently-occupied space and bounded
   by the space's coordinate envelope. The current decoded args are raw `f32`s
   with no spatial sanity check (no `is_finite`, no in-bounds check). A NaN or
   `f32::INFINITY` value would propagate to client UI rendering.
3. Per-caller rate limit — the original SGW UI throttles minimap ping at one
   per few seconds; a wire-level attacker can spam pings at packet rate to
   griefing-spam an org's UI.

**Evidence**
- Ghidra: `0195faf0`, `019bfa48`, `019c3fb0` — `Event_NetOut_BroadcastMinimapPing`
  / `BroadcastMinimapPing` class name strings. Handler chain installer at
  `00d669d0`. The class name encoding `Event_NetOut_BroadcastMinimapPing` plus
  the SGW spec convention of `(EntityId org, Vector3 pos)` as the args list
  matches the server handler's 16-byte (i32 + 3×f32) decode shape at
  `organization.rs:49`.
- Cross-ref: `crates/services/src/cell/cell_methods/organization.rs:48-64`.

**Attack scenario** (lands when the handler is implemented)
1. Attacker discovers a target org's `org_id` (via Who, gossip, or guess).
2. Attacker calls `BroadcastMinimapPing(org_id=<target>, x=0, y=0, z=0)` from a
   foreign org / no-org character.
3. Server fans `onBroadcastMinimapPing` to every member of the target org with
   the attacker-supplied coordinates — appearing in their minimap UI as a
   legitimate ping from an org member.

**Suggested remediation (one line)**
At the handler head: `assert_org_membership(entity_id, org_id)?` (DB-backed),
`assert_finite_position(x, y, z, current_space_bounds)?`, and apply a
per-caller token bucket of (e.g.) 1 ping per 3s.

**Would benefit from x64dbg trace?**
No — wire shape is unambiguous from the decoded byte count.

---

### CAT-L-06 — `SendGMShout` is a registered NetOut event with no server handler — implicit GM check missing

**Severity**: High (when implemented)
**Class**: GM-channel impersonation
**Wire surface**: `Event_NetOut_SendGMShout`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The `Event_NetOut_SendGMShout` class is registered in the client
(`019b9b50`, `01e1d880`) and dispatched through the
`SGWNetworkManager::EventHandler<Event_NetOut_SendGMShout>` chain
(`01e4a328`). On the server side **no handler exists** — no msg_id arm for it
in `dispatch.rs`, no cell-method arm in any `cell_methods/*` dispatcher. The
unhandled message currently lands in the `Unhandled SGWPlayer base method` warn
arm at `dispatch.rs:333-346` or the `Unhandled cell method call` warn arm at
`crates/services/src/cell/dispatch/router.rs:101-106`, depending on whether the
client emits it via base or cell. Either way, the server today does nothing —
which is safe by accident.

The risk is **when the handler is implemented**. `SendGMShout` should be an
`access_level >= GameMaster` privileged path (broadcasts to all players in the
shard / region), and the server-side gate is the canonical example of "GM bit
lives on the session, not the inbound packet" from the agent prompt. Cimmeria's
existing `SPEAKER_GM` derivation at `dispatch.rs:131-133` uses the right pattern
(`if c.access_level > 0 { flags |= speaker_flags::GM; }`) — `SendGMShout` MUST
do the same: `if c.access_level < ACCESS_LEVEL_GM { return; }` at the handler
head, NEVER deriving GM-ness from any wire field. The
[[reference-gm-auth-plumbing-gap]] applies if the handler is added to the cell
layer (no access_level in scope) — pin the implementation to the base layer or
plumb `access_level` first.

**Evidence**
- Ghidra: `019b9b50` `Event_NetOut_SendGMShout`, `0184259c`
  `Event_SlashCmd_GMShout` (slash-command source), `01e05cc0` MemberCallback
  binding through `SGWTextCommandMgr` (the `/gmshout` slash handler emits the
  NetOut event).
- Cross-ref: no Rust handler exists. Lands in the catch-all warn arm at
  `crates/services/src/base/dispatch.rs:333` or
  `crates/services/src/cell/dispatch/router.rs:101`.

**Attack scenario** (lands when the handler is implemented)
1. A future PR adds an `SGW_GM_SHOUT` msg_id (or cell method index) arm. If the
   author misses the access_level gate (because the cell layer has no access to
   it, or because they trust the "the wire shape only fires from /gmshout"
   client-side path), every player can send a server-wide GM-styled broadcast.
2. Attacker emits the wire bytes directly without typing `/gmshout`.
3. Observable effect: GM-flagged broadcast reaches every player on the shard,
   impersonating real GMs.

**Suggested remediation (one line)**
Implement `SendGMShout` in the **base** layer (so `access_level` is in scope),
and gate with `if c.access_level < ACCESS_LEVEL_GM { return Ok(()); }` as the
first thing the handler does — matching the chat-speaker-flag idiom at
`dispatch.rs:131`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-L-07 — `Petition` / `Who` / `chatFriend` / `chatIgnore` are NetOut events with no server handler — sender-identity discipline must be set before they're wired

**Severity**: Medium (when implemented)
**Class**: Sender-identity spoof / GM-petition spam / privacy
**Wire surface**: `Event_NetOut_Petition`, `Event_NetOut_Who`,
`Event_NetOut_ChatFriend`, `Event_NetOut_ChatIgnore`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
Four NetOut events are registered in the client but have no server handler:

- `019b9b80` `Event_NetOut_Petition` — `/petition` to GM, no handler in Rust.
- `019be840` `Event_NetOut_ChatIgnore` — `/ignore <player>`, no handler.
- `019b30c0` `Event_NetOut_ChatFriend` — `/friend <player>`, no handler.
- `Event_NetOut_Who` (cell method index 73 — `WHO` constant) — handler exists
  at `crates/services/src/cell/cell_methods/player/interaction.rs:17-20` as a
  bare `tracing::info!("UNIMPLEMENTED: who"); true` stub.

For each, the canonical security invariants are:

- **Petition:** sender identity MUST come from session (`player_name`,
  `account_id`), NEVER from any wire-supplied name field — the GM receiving the
  petition needs to know who actually sent it. Today's `sendPlayerCommunication`
  pattern (server reads `player_name` from `ConnectedClientState` at
  `dispatch.rs:106`) is the right shape; reuse it. Rate limit at e.g. 1 petition
  / 30s per account.
- **Who:** must respect a `/hide` flag (when implemented — currently no `hide_gm`
  / `invisible` plumbing exists outside of GM constants), and may want to
  faction-scope the response (i.e. an Ori-faction player shouldn't see SGC
  players in /who by default). Today the handler is a stub, so the bug is
  latent.
- **ChatFriend / ChatIgnore:** server-side persistence will need the same
  ownership / size-cap / self-add discipline as the contact-list operations
  (see CAT-L-04). Self-ignore is harmless; self-friend wastes a row. List size
  must cap (the original SGW limit was 50 friends / 50 ignores).

**Evidence**
- Ghidra: `019b9b80` `Event_NetOut_Petition`, `019b30c0` `Event_NetOut_ChatFriend`,
  `019be840` `Event_NetOut_ChatIgnore`, `019c2974` `chatFriend`, `019c295c`
  `chatIgnore`, `019c2a18` `petition`. Slash-command sources
  `Event_SlashCmd_Petition`, `Event_SlashCmd_ChatFriend`, `Event_SlashCmd_ChatIgnore`
  bind via `Communicator` `MemberCallback` (`01e1f508`, `01e1fb88`), so the
  client-side emission goes Communicator → NetOut.
- Cross-ref: WHO handler stub at
  `crates/services/src/cell/cell_methods/player/interaction.rs:17-20`; Petition
  / ChatFriend / ChatIgnore have no handler at all (catch-all warn arm).

**Attack scenario** (lands when the handlers are implemented)
- Petition: a sender-name-spoof attack (attacker claims to be a different
  player in the petition body) only works if the implementor reads the sender
  from the wire instead of from `c.player_name`. Document the requirement
  before the handler is added.
- Who: a faction-leak / hide-flag-bypass becomes possible if the handler returns
  all-players-unfiltered. The mitigation is to compute the response from the
  server's authoritative space / faction state, not from any client-supplied
  filter.
- ChatFriend / ChatIgnore: see CAT-L-04 for the list-size and ownership
  failure modes.

**Suggested remediation (one line)**
Before any of these four handlers are filled in, add a `// SECURITY` block in
the stub naming the server-authority source (`c.player_name`, `c.account_id`,
`c.player_entity_id`) and the cap / scope rule (faction scope, list size,
rate limit) — so the next contributor doesn't trust the wire by default.

**Would benefit from x64dbg trace?**
Yes for Petition — trace `Event_SlashCmd_Petition` → `Event_NetOut_Petition` to
confirm whether the client embeds a target-GM-name or just a free-text body, so
the server-side validation (or lack of need for one) can be encoded precisely.

---

### CAT-L-08 — `chatOp`/`chatMute`/`chatKick`/`chatBan`/`chatPassword` are NetOut events with no handler — channel-op authorization will need session-side op-bit tracking

**Severity**: Medium (when implemented)
**Class**: Channel-admin privilege escalation
**Wire surface**: `Event_NetOut_ChatOp`, `ChatMute`, `ChatKick`, `ChatBan`,
`ChatPassword`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
Five channel-admin events are registered in the client (`019b9a90` ChatOp,
`019b9a38` ChatMute, `019b9a64` ChatKick, `019b9ab8` ChatBan, `019b9ae4`
ChatPassword) with no server handler. When implemented, each handler MUST
verify the caller's **op-bit for the specific channel** before applying the
admin action. The op-bit must live on a server-side per-channel state
(`channel_ops: HashMap<ChannelName, HashSet<account_id>>` or equivalent) — NOT
on any client-supplied flag. The agent-prompt's "central question #4" applies
verbatim: *the client cannot claim "I'm op" without being one*.

A second-order risk: the wire shape carries `channel_name` (WSTRING) and
`target_player` (WSTRING) — both attacker-controlled. The op-bit check must
be keyed on the channel_name AFTER normalizing (case-fold, whitespace-trim),
or an attacker could claim op on `"FOO"` while the canonical op record is
for `"foo"`.

**Evidence**
- Ghidra: `019b9a38..019b9ae4` block of `Event_NetOut_Chat{Mute,Kick,Op,Ban,Password}`
  strings, plus `Event_SlashCmd_*` mirror at `01842078..01842110`. Communicator
  `MemberCallback` bindings at `01e1f508`-range route from slash command to
  NetOut.
- Cross-ref: no Rust handler exists; unhandled message falls into the warn arm.

**Attack scenario** (lands when the handler is implemented)
1. Attacker joins channel "lfg" as a normal user.
2. Attacker sends `chatKick(channel="lfg", target=<random_player>)` directly.
3. If the handler doesn't check whether `caller` is in the
   `channel_ops["lfg"]` set, the kick lands and the victim is removed from the
   channel.

**Suggested remediation (one line)**
At the handler head: `if !channel_ops.get(&normalize(channel)).map_or(false,
|ops| ops.contains(&caller.account_id)) { return; }` — and document that
op-bit is **per-channel, server-tracked**, never derived from the inbound
packet.

**Would benefit from x64dbg trace?**
No.

---

### CAT-L-09 — `chatJoin` / `chatLeave` are stubbed as auto-acknowledged — future channel-password gate inherits no verification

**Severity**: Low (when chat-password is implemented)
**Class**: Channel-password bypass (theoretical)
**Wire surface**: `Event_NetOut_ChatJoin` (base method 0xC0), `ChatLeave` (0xC1)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`chatJoin` at `dispatch.rs:158-169` reads `(channel_name, password)` and emits
`debug!(..., "chatJoin -- acknowledged (channels auto-joined)")`. The comment
correctly notes that today all channels are auto-joined — but when a channel-
password gate is implemented (`Event_NetOut_ChatPassword` at `019b9ae4`), the
join handler MUST cross-check `password` against the server-side stored
password for the channel before recording the join.

The latent risk: a future "channel password" implementation that lives on a
per-channel struct outside `ConnectedClientState` could be missed by the
`chatJoin` handler, because the current handler discards `password` entirely.
The two paths (set-password and check-password-on-join) need to be wired up
in lockstep, or a `chatJoin(channel="vip", password="")` succeeds.

**Evidence**
- Ghidra: `019b9b04` `Event_NetOut_ChatJoin` (inferred from sequence;
  alternative addresses near the `ChatMute/Kick/Ban/Password` block at
  `019b9a38..019b9ae4`).
- Cross-ref: `crates/services/src/base/dispatch.rs:158-169` (the entire body is
  a debug log; `_password` is intentionally discarded by the `let (_password,
  _)` binding).

**Attack scenario** (lands when channel-password is implemented)
1. GM creates a password-protected channel `"raid_planning"` with password
   `"secret"`.
2. Attacker sends `chatJoin(channel_name="raid_planning", password="")` and the
   handler — still tracking the "channels auto-joined" stub posture — accepts
   the join without checking against the per-channel stored password.

**Suggested remediation (one line)**
When channel-password is implemented, replace the `_password` discard at
`dispatch.rs:164` with a server-side lookup
(`channel_passwords.get(channel_name)`) and a constant-time string compare;
on mismatch, return without inserting the caller into the channel-member set.

**Would benefit from x64dbg trace?**
No.

---

## Not Filed

- **Channel-byte forwarding into `onPlayerCommunication` is fine for spatial channels** — the cell-side match at `chat.rs:74` strictly gates broadcast to SAY/EMOTE/YELL, so a client setting `channel=CHAN_SERVER` results in a debug log and no fan-out. The brittleness is captured in CAT-L-03 as a future-change warning, not a live exploit.
- **`chatSetAFKMessage` is log-only and stores nothing** — the comment at `dispatch.rs:178-189` confirms AFK is not a speaker_flags input and the auto-reply path is unimplemented. No state mutates, so the "client can set AFK without being AFK" concern is degenerate.
- **`cancelLogOff` exposed as a base method** — present at `dispatch.rs:329-331` as a debug-only acknowledgement. Not chat surface, and CAT-A owns the logoff path.
- **Mercury bundle size DoS via gigantic `text` WSTRING** — bounded by the WSTRING parser's buffer-tail check (`read_wstring` at `mercury/mod.rs:336-364`), so a malformed `char_count = u32::MAX` is rejected before allocation. Real risk is "many small messages" (CAT-L-01), not "one gigantic message".
- **`Who` returning all-players-unfiltered** — handler is a single info-log stub at `interaction.rs:17-20`. No response is emitted, so the privacy / faction-scope concern is theoretical; captured in CAT-L-07 alongside Petition.
- **`onSpaceQueuedResponse` / other space-queue mechanics potentially as chat-side** — those are CAT-O scope (World / Space), not CAT-L.
- **GM-bit on `sendPlayerCommunication`** — server correctly computes `SPEAKER_GM` from session `access_level > 0` (`dispatch.rs:131-133`), not from any client-supplied flag. The check is byte-for-byte correct against the Python reference and is regression-pinned by tests 1–4 at `dispatch.rs:588-660`. No finding.
- **Speaker-name spoof on `sendPlayerCommunication`** — server reads `player_name` from `ConnectedClientState.player_name`, populated server-side from the DB at `play_character.rs:133`. The client-supplied WSTRING fields are `target` and `text`; `speaker_name` is never on the wire from the client. No finding.
- **Witness fan-out using attacker-supplied entity list** — chat broadcast uses `space_mgr.get_entity(sender_id).witnesses`, which is server-computed AoI. No client-trusted target list. No finding.
- **`cell_methods::being::dispatch` SET_MOVEMENT_TYPE and similar** — owned by CAT-B (movement) and CAT-C (combat), respectively. Out of scope for CAT-L.
