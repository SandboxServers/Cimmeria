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
  3. `de938bc90` fix(chat): register server 8 and tell 10 at login; welcome on CHAN_FEEDBACK
  4. `3d262e5bb` docs(chat): channel table per EChannel; GM feedback uses the wire serializer
  5. this worknote
- **Edited:**
  - `crates/wire/src/cell/chat.rs`: the constants, `CHAN_CHAT`, three new tests.
  - `crates/base/src/base/dispatch/{chat.rs,chat_gates.rs,tell.rs}` and `tests/{chat_channel_mute.rs,tell.rs}`.
  - `crates/base-session/src/base/{world_entry_chat.rs,gm_broadcast.rs}`.
  - `crates/base-world-entry/src/base/world_entry_appearance/{mod.rs,client_ready/mod.rs}`.
  - `crates/cell-console/src/cell/console/{gm/feedback.rs,chat/feedback.rs}`.
  - Comments only: `crates/cell/src/cell/service/base_messages/lab_console.rs`, `crates/content-engine/src/loader/action_bark.rs`, `crates/server/src/logging/filters.rs`.
  - Docs: `docs/gameplay/chat-system.md`, `docs/content/content-engine.md`.
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

This is a visible behaviour change. On the client, `/gmshout` and `.announce` now show a red line and a modal "Server Message" prompt with an OK button. Before, they showed nothing. The modal is the client's designed display for server messages.

### Registration (`DEFAULT_CHAT_CHANNELS`): option A implemented, option B recommended

As briefed, the list now reads the constants: say 0, emote 1, yell 2, team 3, squad 4, command 5, server 8, tell 10. No other entry was wrong. The order and names are unchanged, so the login bundle is still 11 messages. It is pinned by literal and against the XML by `default_chat_channels_match_enumerations_xml`, and the doc comment explains how it departs from the python.

**Finding (sent to the coordinator on 2026-09-27, before implementation):** the evidence above says built-in channels should get **no** `onChatJoined` at all.

- The legacy server sent none.
- The client files each one as a user channel at `12 + ChannelID` and prints a "You have joined channel" line.
- `("server", 8)` announces a user channel `[8:server]`, which is id 20, not the Server channel.
- The current burst most likely prints eight "joined channel" lines on every login. It is cosmetic and was the same before this packet, but it is not what the original server did.

**Option B** drops the built-in entries: `DEFAULT_CHAT_CHANNELS` becomes empty or is removed, the bundle shrinks to 3 messages, and a test asserts that no `onChatJoined` carries an id below 12. B changes the burst numbers in `mercury-bundle.md` and the count test in `builders.rs`. It needs the coordinator's decision, and should be confirmed on a live client first (UAT item 6). No answer had arrived by the time the rest was done, so I implemented A by default.

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

- **New event:** `chat.channels_registered`, DEBUG, on the `chat` target, once per `onClientReady`, right after the burst is sent.
  - Fields: `account_id`, `player_id`, `entity_id`, `channel_ids` (a Debug list, `[0, 1, 2, 3, 4, 5, 8, 10]`) and `channel_count`.
  - The `chat` target is already in `OTEL_FILTER` at `debug`; the `filters.rs` comment now lists this row.
- **SigNoz query:** `scope_name = chat AND attributes.event = 'chat.channels_registered' AND attributes.player_id = <id>`, then that player's other `chat.*` rows around time T.
- No refusal seam was added, so there is no new type 12 test.

## Commands run

All from the worktree, through the lane. Exit 0 unless noted.

- `bash tools/build-lane/lane.sh cargo check -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console --all-targets`.
- `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-content-engine`: 2108 passed, 0 failed, 0 skipped. An earlier run failed one test: the scan could not parse char literals. That was fixed before the commit.
- `bash tools/build-lane/lane.sh cargo fmt --all`, then `cargo fmt --all -- --check`: clean.
- `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-content-engine --all-targets -- -D warnings`: clean.
- `cargo clippy -p cimmeria-server --all-targets -- -D warnings`: clean.
  - With sccache on, `utoipa-swagger-ui` failed with a `RustEmbed` folder path from another worktree's target (`bank-bv04`), because sccache had cached a build-script output path.
  - The clean run used `RUSTC_WRAPPER=`, after `cargo clean -p utoipa-swagger-ui`.
- No live-DB tier: the packet touches no SQL.

## Regression proof

Each mutation was applied to the committed tree and tested with `cargo nextest run --no-fail-fast -p cimmeria-wire -p cimmeria-base -p cimmeria-base-session -p cimmeria-cell-console`. The file was then restored with `git checkout HEAD -- <file>` followed by `touch`.

| Mutation | Tests that failed |
|---|---|
| `CHAN_SERVER` 8 -> 7 | 6: `chan_constants_match_enumerations_xml`, `gm_broadcast_bytes_are_exact`, `no_chan_constant_is_seven`, `chat_rejects_unknown_channel_at_base`, `default_chat_channels_match_enumerations_xml`, `gm_broadcast_global_reaches_every_listed_session` |
| `CHAN_TELL` 10 -> 9 | 16: `chan_constants_match_enumerations_xml`, `default_chat_channels_match_enumerations_xml`, `allowlist_matches_the_tell_route`, `chat_rejects_system_channel_at_base`, `tell_reaches_exactly_one_recipient`, and 11 more `dispatch::tests::tell::*` |
| `send_channel_feedback` passes a literal `7` | `no_player_communication_call_uses_a_literal_channel` |
| The welcome pushes `CHAN_SERVER` | `build_welcome_message_args_emits_speaker_flags_chan_feedback_then_text`, `build_welcome_message_args_handles_fallback_speaker` |

## Tests added or changed

- **Added** (`cimmeria-wire`): `chan_constants_match_enumerations_xml`, `no_chan_constant_is_seven`, `no_player_communication_call_uses_a_literal_channel`.
- **Renamed:**
  - `default_chat_channels_matches_sgwplayer_channel_set` is now `default_chat_channels_match_enumerations_xml`, and it also parses the XML.
  - `build_welcome_message_args_emits_speaker_flags_chan_tell_then_text` is now `..._chan_feedback_then_text`. The byte is unchanged: 9.
- **Folded:** `echannel_ids_match_enumerations_xml` (cimmeria-base), into the wire test.
- **Updated bytes:**
  - `gm_broadcast_bytes_are_exact` pins `0x08`.
  - `gm_broadcast_global_reaches_every_listed_session` pins 8.
  - `build_chat_joined_args_emits_wstring_then_u8_id` pins tell 10.
  - `feedback_wire_shape_is_system_on_feedback_channel` (in `gm/feedback.rs`) pins a literal 9.
  - The tell fan-out tests use the client's literal 10.

## Known gaps

- Option B (dropping the built-in `onChatJoined`) is not done. See Design decisions.
- Team (3) and command (5) still get "not supported yet" from the cell. Officer (6) is unregistered and routed the same way. This is ORG-09's scope.
- `CHAN_SPLASH` (11) has no sender, and the content-engine bark still accepts `say` only.

## UAT (for SS-UAT)

1. Log in. The welcome "Welcome to Stargate Worlds. Your player id is: N." appears as a sky-blue line in the Info tab, with **no** popup.
2. As a GM, run a `.` command such as `.location`. The feedback line is sky blue, in the Info tab, with no popup.
3. As a GM, send `/gmshout hello` and `.announce hello`. Every online player sees a **red line and a modal "Server Message" prompt** with an OK button. This is new: before SS-C4 nothing displayed. Also try `.announce space hello`.
4. Send `/tell <other player> hi`. The recipient sees a purple tell in the Info tab, the sender sees their "To X" confirmation, and `/reply` works on the recipient's side.
5. As a player, post on a refused channel if the UI lets you. The refusal line is sky blue, with no popup.
6. On login, look for "You have joined channel [N:name]" lines in the Info tab. Whether they appear decides option B.

## Integration edits for the coordinator

- **work-packets.md:** set the SS-C4 status and decide between options A and B.
- **README.md:** D-SS17 can be marked as done by SS-C4.
- **Organizations ledger:** tell ORG-09 three things:
  - `CHAN_*` are aligned.
  - `CHAN_OFFICER` = 6 is unregistered.
  - `DEFAULT_CHAT_CHANNELS` is now `pub` in `cimmeria_base_session::base::world_entry_chat`.
- **`docs/reverse-engineering/findings/organization-restoration.md:241-243`** says the Rust "must change"; it could now say "changed in SS-C4".
- **`docs/reverse-engineering/findings/chat-wire-formats.md`** could record that the `ChannelID` in `onChatJoined` is a display id (the channel minus 12), per the legacy python and the client Lua.
- I left both findings docs alone because they belong to the RE owners.

## Close-out edits for SS-99

- **`docs/gap-analysis.md:730`:** change "server=7/tell=9" to "server=8/tell=10 (SS-C4, D-ORG14)", and note that the legacy server sent no `onChatJoined` for the built-in channels.
- **`docs/project-status.md`:** the chat row can say that the channel ids are aligned and that GM broadcasts now display.
- **Test count:** 3 added and 1 folded, a net +2, in `cimmeria-wire` and `cimmeria-base`.
