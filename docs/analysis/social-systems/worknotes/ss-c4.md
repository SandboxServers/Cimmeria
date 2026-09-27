# SS-C4 Worknote: Channel-id alignment (D-ORG14)

> Type: reference. Audience: the social-systems coordinator, the organizations coordinator (ORG-09) and the PR reviewers.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md#ss-c4-channel-id-alignment), [organizations D-ORG14](../../organizations/README.md#decisions).

## Contract

- **Packet:** SS-C4 (work-packets.md § SS-C4). By agreement between the social-systems coordinator and the organizations coordinator (cimmeria-1f), SS-C4 carries the whole D-ORG14 alignment. `world_entry_chat.rs` and the `CHAN_*` constants were released to this packet. ORG-09 keeps team/command/officer fan-out and any conditional officer registration.
- **Decisions in force:** D-ORG14 (CONFIRMED: the client hardcodes every built-in `UIChannel.*` id as the `enumerations.xml` value), D-SS17 (one change, agreed by both coordinators), D-SS16 (GM broadcast on the server channel).
- **Base:** `origin/main` @ `2076e482a` (SS-C1, SS-C2, SS-C3 merged). Branch `social/c4-channel-ids`.
- **Commits:**
  1. `440a61ec6` fix(chat): align CHAN_* with EChannel: server 8, tell 10, splash 11
  2. `1b9806f2c` refactor(chat): social paths use the aligned CHAN_* constants
  3. `de938bc90` fix(chat): register server 8 and tell 10 at login; welcome on CHAN_FEEDBACK (option A, superseded by 6)
  4. `3d262e5bb` docs(chat): channel table per EChannel; GM feedback uses the wire serializer
  5. `cffe1e6f1` docs(social): SS-C4 worknote and chat-channel agent memory
  6. `850e1941b` fix(chat): no onChatJoined for built-in channels at login (SS-C4 option B)
  7. this worknote update
- **Edited:**
  - `crates/wire/src/cell/chat.rs`: the constants, `CHAN_CHAT`, three new tests.
  - `crates/base/src/base/dispatch/{chat.rs,chat_gates.rs,tell.rs}` and `tests/{chat_channel_mute.rs,tell.rs}`.
  - `crates/base-session/src/base/{world_entry_chat.rs,gm_broadcast.rs,world_entry_appearance/builders.rs}`.
  - `crates/base-world-entry/src/base/world_entry_appearance/{mod.rs,client_ready/mod.rs}`.
  - `crates/cell-console/src/cell/console/{gm/feedback.rs,chat/feedback.rs}`.
  - Comments only: `crates/cell/src/cell/service/base_messages/lab_console.rs`, `crates/content-engine/src/loader/action_bark.rs`, `crates/server/src/logging/filters.rs`.
  - Docs: `docs/gameplay/chat-system.md`, `docs/content/content-engine.md`, `docs/architecture/mercury-bundle.md`.
- **Read set:**
  - Campaign: SS-WORKER-RULES.md; work-packets.md § SS-C4 and the contended list; README.md D-SS17; organizations README D-ORG14; `organization-restoration.md` ORG-E1 Q5 and `org-e1.md`; `chat-wire-formats.md`; worknotes ss-c1, ss-c2 and ss-c3.
  - Defs and legacy: `enumerations.xml:113-130`; `deprecated/python/base/Chat.py:141-190`; `deprecated/python/base/SGWPlayer.py:155-163`; `deprecated/python/cell/SGWPlayer.py:541`.
  - Client: `Content/UI/Core/ChatWindow/ChatWindow.lua` and `ChatWindow.int`.
  - Code: every `CHAN_*` and `serialize_on_player_communication` use in `crates/`.

## Evidence

- **Ids.** `enumerations.xml:116-127`: say 0, emote 1, yell 2, team 3, squad 4, command 5, officer 6, server 8, feedback 9, tell 10, splash 11, chat 12. Before this packet the Rust constants were server 7, feedback 9, tell 9 and splash 10 (`chat.rs:26-37` at the base sha), and `world_entry_chat.rs` had its own `CHAN_TELL = 9`.
- **Client display, from the client Lua** (`ChatWindow.lua`, not in git):
  - `:160-162`: a line on `UIChannel.Server` (8) calls `PromptMod.showPrompt("Server Message", text, "Ok", ...)`. That is a **modal prompt**, shown as well as a bright red line (`:1249`). This is the "red unknown-channel splash popup" that older comments blamed on 8 being unregistered.
  - `:1297-1312`: `ChatMod.ChannelMap` has entries for 0-6 and 8-12, and none for 7. For any id below 12, `onMessageReceived` (`:93-104`) calls `ChatMod.ChannelMap[channelId](...)`. **A line on 7 therefore calls nil**, raises a Lua error and shows nothing. SS-C2's `/gmshout` and `.announce` sent on 7, so they have never displayed.
  - `:1250`: feedback (9) is an ordinary sky-blue line, shown in the Info tab by default (`:1274`).
  - `:370-386`, with `ChatWindow.int:107`: `onChannelJoined(name, displayId)` computes `channelId = UIChannel.Chat + displayId`. It then prints "You have joined channel [displayId:name]" on feedback and adds a user channel keyed by name. **The client treats every `onChatJoined` as a user channel.**
- **Legacy python:**
  - `SGWPlayer.py:162-163` calls `client.onChatJoined(name, channelId - MIN_USER_CHANNEL)`, and only when `channelId >= MIN_USER_CHANNEL` (12).
  - `Chat.py:183-190` (`playerLoggedIn`) joins the server channel on the server side only.
  - So the legacy login sent **no** `onChatJoined`.
  - The welcome (`cell/SGWPlayer.py:541`) is `onPlayerCommunication(name, 0, 9, ...)`. Its channel is a literal 9, which is feedback.

## Design decisions

### Constants

`CHAN_*` in `crates/wire/src/cell/chat.rs` now equal `EChannel`, and `CHAN_CHAT = 12` was added. `CHAN_FEEDBACK` (9) and `CHAN_TELL` (10) are now distinct constants; before this packet both were 9.

### The welcome line stays on 9

Moving it to server (8) would open a modal prompt on every login. The byte is unchanged. It is now named `CHAN_FEEDBACK` instead of the local `CHAN_TELL`, and it matches the legacy literal.

### GM feedback stays on 9

Every GM-feedback and refusal path already used `CHAN_FEEDBACK` = 9, so their bytes are unchanged. The cell console's `gm/feedback.rs` had a private serializer and a private `CHAN_FEEDBACK` copy. Both are deleted in favour of the wire ones, which produce the same bytes.

### GM broadcast moves from 7 to 8

This is a visible behaviour change. SS-C2 sent `/gmshout` and `.announce` on 7. The client has no `ChatMod.ChannelMap` entry for 7 (`ChatWindow.lua:1297-1312`), and `onMessageReceived` calls `ChannelMap[channelId](...)` for every id below 12 (`:93-104`). Each broadcast therefore raised a client Lua error and **showed nothing**: SS-C2's broadcast has been invisible until now. On 8 the client prints a bright red line (`:1249`) and opens a modal "Server Message" prompt with an OK button (`:160-162`). That is the client's designed display for server messages, so a modal per broadcast is intended. It is flagged for UAT (item 3).

### No channel registration at login (option B, coordinator decision 2026-09-27)

The server sends **no** `onChatJoined` at login. `DEFAULT_CHAT_CHANNELS` and `build_chat_joined_args` are deleted, and the login burst is BeingAppearance, onEntityTint and the welcome: 3 messages, down from 11. The evidence:

- **The client hardcodes ids 0-11.** ORG-E1 Q5: every `UIChannel.*` getter returns a compiled-in literal equal to `EChannel`, so a built-in channel needs no registration to send or show.
- **The client treats every `onChatJoined` as a user channel.** `ChatWindow.lua:370-386`: `onChannelJoined(name, displayId)` computes `channelId = UIChannel.Chat + displayId`, prints "You have joined channel [displayId:name]" (`ChatWindow.int:107`) on feedback, and adds a user channel keyed by name.
- **The legacy server matched that.** `deprecated/python/base/SGWPlayer.py:162-163` sends `onChatJoined(name, channelId - MIN_USER_CHANNEL)` only when `channelId >= 12`. `deprecated/python/base/Chat.py:183-190` (`playerLoggedIn`) joins the server channel on the server side only.

So the old Rust burst of eight `onChatJoined` (say through command, server, tell) created eight bogus user channels on every login and printed a "joined channel" line for each. With option A's corrected ids it would still have done so, and `("server", 8)` would have announced user channel 20, not the Server channel.

Under B, ORG-09's plan to "register officer 6 conditionally" is moot: officer needs no registration. A user-channel feature, when one exists, sends `onChatJoined` from its own join path, with the display id (channel minus 12).

**Guards.**

- `on_client_ready_burst_registers_no_built_in_channel` reads the burst's method list (`on_client_ready_burst_messages`, which the bundle builder appends verbatim). It asserts there is no `ON_CHAT_JOINED` and the exact list `[BEING_APPEARANCE, ON_ENTITY_TINT, ON_PLAYER_COMMUNICATION]`, and that the bundle has exactly that many messages.
- `on_client_ready_burst_bundles_to_single_packet` now pins `num_messages == 3`.

### Social locals removed

- SS-C3's `dispatch::chat_gates::echannel` module and SS-C1's `dispatch::tell::TELL_CHANNEL` are deleted. Every use now reads `cimmeria_wire::cell::chat::CHAN_*`.
- `echannel_ids_match_enumerations_xml` is **folded** into `cimmeria_wire::cell::chat::tests::chan_constants_match_enumerations_xml`, which also checks the reverse direction: every `EChannel` token has a constant, and every constant names a token.
- The tell fan-out test sends and expects the client's literal byte 10 (`CLIENT_TELL_BYTE`), not the constant.

### No server-to-client send uses 7

Three tests cover this:

- `no_chan_constant_is_seven`: no constant is 7, and the XML still skips 7.
- `chan_constants_match_enumerations_xml`: pins every constant to the XML.
- `no_player_communication_call_uses_a_literal_channel`: walks every `.rs` file under `crates/`, parses each `serialize_on_player_communication(` call, and fails if the channel argument is a numeric literal. It also fails if it finds fewer than 20 calls, or any call it cannot parse.

Together they mean every channel byte handed to the one serializer comes from a pinned constant. The welcome builder pushes `CHAN_FEEDBACK` itself and is pinned byte for byte.

## Telemetry

- **No new event.** The first pass added a DEBUG `chat.channels_registered` row per login. Under option B nothing is registered, so a row that always says "none" carries no information, and it was dropped (the coordinator left this to me). The login itself is already visible through the existing `onClientReady` rows.
- **SigNoz:** chat refusals and broadcasts keep their SS-C2 and SS-C3 events (`chat.channel_rejected`, `chat.gm_broadcast`, `chat.gm_broadcast_delivered`). Query: `scope_name = chat AND attributes.player_id = <id>` around time T.
- No refusal seam was added, so there is no new type 12 test.

## Commands run

All from the worktree, through the lane. Exit 0 unless noted.

- `bash tools/build-lane/lane.sh cargo check -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console --all-targets`.
- `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-content-engine`:
  - Option A: 2108 passed, 0 failed, 0 skipped. An earlier run failed one test: the scan could not parse char literals. That was fixed before the commit.
  - Option B (after `850e1941b`): 2106 passed, 0 failed, 0 skipped. Three tests were removed with `DEFAULT_CHAT_CHANNELS` and `build_chat_joined_args`, and one was added.
- `bash tools/build-lane/lane.sh cargo fmt --all`, then `cargo fmt --all -- --check`: clean.
- `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-content-engine --all-targets -- -D warnings`: clean. Re-run after option B on `-p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-wire -p cimmeria-base`: clean.
- `cargo clippy -p cimmeria-server --all-targets -- -D warnings`: clean.
  - With sccache on, `utoipa-swagger-ui` failed with a `RustEmbed` folder path from another worktree's target (`bank-bv04`), because sccache had cached a build-script output path.
  - The clean run used `RUSTC_WRAPPER=`, after `cargo clean -p utoipa-swagger-ui`.
- No live-DB tier: the packet touches no SQL.

## Regression proof

Each mutation was applied to the committed tree and tested with `cargo nextest run --no-fail-fast -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-cell-console` (option B rows: `-p cimmeria-base-session on_client_ready_burst`). The file was then restored with `git checkout HEAD -- <file>` followed by `touch`. The first four rows ran on the option A tree, where `default_chat_channels_match_enumerations_xml` still existed; that test was deleted with option B, and every other listed test still guards.

| Mutation | Tests that failed |
|---|---|
| `CHAN_SERVER` 8 -> 7 | 6: `chan_constants_match_enumerations_xml`, `gm_broadcast_bytes_are_exact`, `no_chan_constant_is_seven`, `chat_rejects_unknown_channel_at_base`, `default_chat_channels_match_enumerations_xml`, `gm_broadcast_global_reaches_every_listed_session` |
| `CHAN_TELL` 10 -> 9 | 16: `chan_constants_match_enumerations_xml`, `default_chat_channels_match_enumerations_xml`, `allowlist_matches_the_tell_route`, `chat_rejects_system_channel_at_base`, `tell_reaches_exactly_one_recipient`, and 11 more `dispatch::tests::tell::*` |
| `send_channel_feedback` passes a literal `7` | `no_player_communication_call_uses_a_literal_channel` |
| The welcome pushes `CHAN_SERVER` | `build_welcome_message_args_emits_speaker_flags_chan_feedback_then_text`, `build_welcome_message_args_handles_fallback_speaker` |
| Option B: the old eight-channel `onChatJoined` loop re-added to `build_on_client_ready_burst_bundle` | `on_client_ready_burst_registers_no_built_in_channel`, `on_client_ready_burst_bundles_to_single_packet` |
| Option B: `ON_CHAT_JOINED` added to `on_client_ready_burst_messages` | `on_client_ready_burst_registers_no_built_in_channel`, `on_client_ready_burst_bundles_to_single_packet` |

## Tests added or changed

- **Added:**
  - `cimmeria-wire`: `chan_constants_match_enumerations_xml`, `no_chan_constant_is_seven`, `no_player_communication_call_uses_a_literal_channel`.
  - `cimmeria-base-session`: `on_client_ready_burst_registers_no_built_in_channel`.
- **Removed with option B:** `default_chat_channels_matches_sgwplayer_channel_set` (renamed `default_chat_channels_match_enumerations_xml` in the option A pass), `build_chat_joined_args_emits_wstring_then_u8_id`, `build_chat_joined_args_round_trips_for_all_default_channels`. The code they tested is gone.
- **Renamed:** `build_welcome_message_args_emits_speaker_flags_chan_tell_then_text` is now `..._chan_feedback_then_text`. The byte is unchanged: 9.
- **Folded:** `echannel_ids_match_enumerations_xml` (cimmeria-base), into the wire test.
- **Updated:**
  - `gm_broadcast_bytes_are_exact` pins `0x08`.
  - `gm_broadcast_global_reaches_every_listed_session` pins 8.
  - `feedback_wire_shape_is_system_on_feedback_channel` (in `gm/feedback.rs`) pins a literal 9.
  - The tell fan-out tests use the client's literal 10.
  - `on_client_ready_burst_bundles_to_single_packet` pins 3 messages.

## Known gaps

- Team (3) and command (5) still get "not supported yet" from the cell. Officer (6) is unregistered and routed the same way. This is ORG-09's scope.
- `CHAN_SPLASH` (11) has no sender, and the content-engine bark still accepts `say` only.

## UAT (for SS-UAT)

1. Log in. The welcome "Welcome to Stargate Worlds. Your player id is: N." appears as a sky-blue line in the Info tab, with **no** popup.
2. As a GM, run a `.` command such as `.location`. The feedback line is sky blue, in the Info tab, with no popup.
3. As a GM, send `/gmshout hello` and `.announce hello`. Every online player sees a **red line and a modal "Server Message" prompt** with an OK button. This is new: before SS-C4 nothing displayed. Also try `.announce space hello`.
4. Send `/tell <other player> hi`. The recipient sees a purple tell in the Info tab, the sender sees their "To X" confirmation, and `/reply` works on the recipient's side.
5. As a player, post on a refused channel if the UI lets you. The refusal line is sky blue, with no popup.
6. On login, there are **no** "You have joined channel [N:name]" lines in the Info tab, and say, team, squad and tell all still work (option B). If the client did need registration after all, one of these channels fails to display here.

## Integration edits for the coordinator

- **work-packets.md:** set the SS-C4 status; option B is done.
- **README.md:** D-SS17 can be marked as done by SS-C4.
- **Organizations ledger:** tell ORG-09 three things (the coordinator is relaying):
  - `CHAN_*` are aligned.
  - No channel is registered at login, so officer 6 needs no conditional registration.
  - `DEFAULT_CHAT_CHANNELS` no longer exists.
- **`docs/reverse-engineering/findings/organization-restoration.md:241-243`** says the Rust "must change"; it could now say "changed in SS-C4".
- **`docs/reverse-engineering/findings/chat-wire-formats.md`** could record that the `ChannelID` in `onChatJoined` is a display id (the channel minus 12), per the legacy python and the client Lua.
- I left both findings docs alone because they belong to the RE owners.

## Close-out edits for SS-99

- **`docs/gap-analysis.md:730`:** the "Pre-defined channels" row is wrong twice: it says server=7/tell=9, and that the channels are pushed as `onChatJoined`. It should say that built-in channels are client-hardcoded (ids per `EChannel`, SS-C4, D-ORG14) and that none is registered at login, matching the legacy server.
- **`docs/project-status.md`:** the chat row can say that the channel ids are aligned and that GM broadcasts now display.
- **Test count:** 4 added, 3 removed and 1 folded, a net 0, across `cimmeria-wire`, `cimmeria-base` and `cimmeria-base-session`.
