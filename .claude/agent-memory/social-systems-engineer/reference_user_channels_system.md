---
name: reference-user-channels-system
description: User chat channels (issue #1039) — registry design, id mapping, echo/feedback rules, cleanup wiring, legacy divergence from Chat.py
metadata:
  type: reference
---

Implemented end-to-end in PR for issue #1039 (`chatJoin`/`chatLeave` were acknowledged no-ops; #1035/#840 made them reachable in-world).

## Where the code lives

- `crates/base-session/src/base/user_channels/registry.rs` — `UserChannelRegistry`, a `LazyLock<Mutex<...>>` singleton exactly like `crates/base-session/src/base/mutes/mod.rs::MuteTable`. Keyed by wire channel id (`u8`, `CHAN_CHAT`=12 and up); a second `by_key: HashMap<String, u8>` maps case-folded name (`org_text::name_key`) to id for join-by-name lookup.
- `crates/base/src/base/dispatch/chat.rs` — `handle_chat_join`, `handle_chat_leave` (real logic, replacing the old ack-only stubs), and `post_to_user_channel` (called from `send_player_communication_at` when `channel >= CHAN_CHAT`).
- `crates/base/src/base/dispatch/chat_gates.rs::check_channel` — a user channel id now passes the allowlist (`Ok(())` for `c >= CHAN_CHAT`), shape-only; membership is `post_to_user_channel`'s job.
- `crates/wire/src/cell/chat.rs` — `serialize_on_chat_joined`/`serialize_on_chat_left` (method indices `ON_CHAT_JOINED`=31, `ON_CHAT_LEFT`=32, already defined in `cell/client_methods/communicator.rs`, unused until this PR).
- `crates/entity/src/organization/org_text.rs` — new `TextField::ChannelName` variant, routed through the exact same charset+normalize branch as `TextField::Name` (`validate()`'s `if field != TextField::Name && field != TextField::ChannelName` check). New limits: `MAX_CHANNEL_NAME_UNITS`=32, `MAX_CHANNELS_PER_PLAYER`=10, `MAX_USER_CHANNELS`=200 (all `crates/entity/src/organization/limits.rs`, all project policy).

## Id mapping (legacy-confirmed)

`deprecated/python/base/SGWPlayer.py:162-163,185-188`: `onChatJoined` carries `channelId - Constants.MIN_USER_CHANNEL`; `chatLeave(channelId)` looks up `channelId + Constants.MIN_USER_CHANNEL`. `MIN_USER_CHANNEL` == `CHAN_CHAT` == 12. Confirmed against `entities/defs/enumerations.xml` via the existing `chan_constants_match_enumerations_xml` test.

## Legacy divergence: auto-create on join

`deprecated/python/base/Chat.py::ChatChannelManager.joinChannel` (lines 234-247) ONLY joins a pre-existing channel; on a miss it just `warn()`s server-side with **no feedback to the player at all**. The one path that creates a channel, `requestCreateChannel`, is dead code — grep confirms no base method ever calls it. So the legacy server's own players could never create a channel through any client-reachable path. Cimmeria's `chatJoin` deliberately creates-or-joins (auto-create), which is what makes the client's login auto-joins (`channel-chat`/`channel-roleplay`/`channel-alliance` for a GM, `chat`/`roleplay`/`alliance` for a normal player — both variants exist client-side, gated on `AccessLevel`, `ChatWindow.lua:1109-1141`) actually produce working channels. This is flagged explicitly in code comments and docs as an intentional improvement, not a port.

## No extra feedback on success — this resolved an apparent contradiction in the issue

The issue asked for "no feedback line for each auto-join" AND "a manual /chatjoin gets visible feedback". These are NOT in tension: `onChatJoined`/`onChatLeft` themselves ARE the client's feedback (`ChatWindow.lua::onChannelJoined`/`onChannelLeft` print "You have joined/left channel" unconditionally whenever those calls arrive — verified by reading the Lua directly, not inferred). The wire cannot distinguish an auto-join chatJoin from a manual one — both are byte-identical `chatJoin(name, password)` calls. So the rule is simply: **never send a second (CHAN_FEEDBACK) line on top of a successful onChatJoined/onChatLeft** — success is self-announcing; only a *refusal* (where onChatJoined/onChatLeft never fires) gets a feedback line. This applies uniformly regardless of auto vs manual.

## Server-authority / scope decisions

- **Global, not per-world/per-space.** Legacy `ChannelManager` is a single process-wide singleton (`Chat.py:365`); membership is keyed on the base `SGWPlayer` entity id, which survives gate travel (only the cell/space changes on gate travel, confirmed by reading `base-world-entry/gate_travel/mod.rs` — same `entity_id` before/after `RESET_ENTITIES`). So a user channel is never scoped to a world or cell.
- **Post reaches every member including the speaker.** Unlike say/emote/yell (see `chat-speaker-echo.md` — client shows its own `say` locally, never echo it), a user channel post has NO client-side local echo, and legacy `ChatChannel.sendMessage` (`Chat.py:125-138`) always includes the sender in the iteration. So `post_to_user_channel` sends to every member with no self-skip.
- **Server authority on post.** `members_if_joined(wire_id, entity_id) -> Option<Vec<u32>>` returns `None` for BOTH "channel doesn't exist" and "exists but caller isn't a member" — deliberately not distinguished, so a client can't probe which channel names/ids exist by trying to post to them.
- **Cleanup on disconnect, not gate travel.** `UserChannelRegistry::leave_all(entity_id)` is called at the exact same 3 sites `drop_player_inductions` already uses for session teardown: `crates/base/src/base/dispatch/session.rs::handle_log_off` (both branches — disconnect=0 char-select AND disconnect=1 full-exit, since the `if let Some(entity_id)` block that captures `drop_player_inductions` runs before the branch split) and `crates/base-session/src/base/helpers/mod.rs::destroy_client_entities` (disconnect/timeout/dup-login). Deliberately NOT called from `base-world-entry/gate_travel/mod.rs` (also calls `drop_player_inductions`, but for `DropReason::WorldChange` — gate travel reuses the same entity id, so channel membership must survive it).
- **Empty channels always deleted**, no `CHANNEL_FLAG_KeepIfEmpty` equivalent — legacy only needed that flag because it pre-created 3 channels at boot before anyone joined; Cimmeria never pre-creates, so nothing needs protecting from immediate deletion.

## Testing gotcha: the registry is a shared global in tests too

`UserChannelRegistry::new()` is public — unit tests in `registry/tests.rs` each construct their own fresh instance instead of touching the process-wide `user_channel_registry()` singleton, avoiding all cross-test interference. Dispatch-level tests in `crates/base/src/base/dispatch/tests/chat_user_channels.rs` DO share the real singleton (needed, since `chat.rs`'s handlers call it directly) — every test there uses a channel name AND entity-id range unique to that test function, never reused across tests in the file, because `cargo test` runs them in parallel by default. Learned the hard way: a "channel count before == count after" assertion is flaky under parallel execution on shared global state; use a targeted check instead (`contains_name(key)` for one exact key, `members_if_joined` for one exact wire id) — never assert on the registry's *total* size across a test boundary.

Related: [[reference_chat_channel_routing]], [[reference_chat_speaker_flags]].
