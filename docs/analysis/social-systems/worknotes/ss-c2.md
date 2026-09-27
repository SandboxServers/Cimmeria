# SS-C2 Worknotes

> Type: reference. Audience: social-systems coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md).

## Contract

- **Packet:** SS-C2, GM broadcast.
- **Decisions in force:** D-SS16 (who may broadcast, scopes, delivery, the `chat.gm_broadcast` log), D-SS12 (text rules), D-SS17 (channel ids belong to D-ORG14; use whatever `CHAN_SERVER` holds).
- **Security row:** CAT-L-06 (audit § 6). Guard names kept: `gm_shout_rejected_for_player`, `gm_shout_space_scope_stays_in_space`.
- **Depends on:** SS-00 (#880): `OnlinePlayerIndex`, `FeedbackCtx`.
- **Base:** `origin/main` @ `88d7da73a`. Branch `social/c2-gm-broadcast`, worktree `.claude/worktrees/ss-c2`.
- **Owned paths (new):**
  - `crates/wire/src/cell/messages/chat_cell_to_base.rs` (`ChatCellToBase`)
  - `crates/base-session/src/base/gm_broadcast.rs` (global fan-out + its type-8 test)
  - `crates/base-world-entry/src/base/world_entry/cell_dispatch/chat_dispatch.rs`, `.../tests_dispatch_arms/gm_broadcast_arm.rs`
  - `crates/cell-console/src/cell/console/gm/shout.rs`, `.../gm/tests/shout.rs`
  - `crates/cell-console/src/cell/console/social.rs` (`.announce`), `.../registry/commands/social.rs`, `.../tests/ss_c2_announce.rs`
  - this file
- **Edited:** `wire/src/cell/chat.rs` (`SPEAKER_GM`, `serialize_gm_broadcast`, two tests), `wire/src/cell/messages/{mod.rs, cell_to_base.rs}` (the `Chat` variant), `base-session/src/base/mod.rs`, `base-world-entry/src/base/mod.rs` (re-exports `feedback`, `gm_broadcast`), `base-world-entry/.../cell_dispatch/{mod.rs, tests_dispatch_arms/mod.rs}`, `cell-console/src/cell/console/{mod.rs, dispatch.rs, gm/mod.rs, gm/tests/mod.rs, registry/commands/mod.rs, tests/mod.rs}`, `cell/src/cell/dispatch/tests.rs` (the gate guard), `server/src/logging/{filters.rs, target_scan_tests.rs}` (doc comment and one scan pin; no filter change, `chat=debug` already covers INFO). **Not edited:** `cell-console/.../console/chat.rs` (SS-C1 splits it), `base-session/.../world_entry_chat.rs` and the `CHAN_*` values (ORG-09).
- **Read set:** the ledger (README D-SS12/16/17, work-packets Contract / contended files / SS-C2, audit A-28 and § 6 CAT-L-06); SS-E1's `chat-wire-formats.md` C-Q2 on `origin/social/se1-re`; `entities/defs/SGWGmPlayer.def:650-697`, `interfaces/Communicator.def:227-231`, `enumerations.xml` (`EChannel`, `ESpeakerFlags`); `docs/protocol/cell-method-dispatch-table.md:694`; `docs/security-audit/.../CAT-L-chat-contact.md` CAT-L-06; `cell-world/.../dispatch/gm_gate.rs`; `cell/.../dispatch/router.rs`; `cell-console/.../gm/{mod.rs, feedback.rs, physics.rs, query.rs}`, `console/{mod.rs, dispatch.rs, chat.rs}`, `registry/`; `base-session/.../{feedback.rs, gm_feedback.rs, player_index/mod.rs, mod.rs}`; `base/.../dispatch/{mod.rs, chat.rs}`; `base-world-entry/.../cell_dispatch/{mod.rs, org_dispatch.rs}`; `server/src/logging/filters.rs`.

## Evidence

1. **Wire.** `sendGMShout(UINT8 isGlobal, WSTRING Text)` is the 114th own `<Exposed/>` CellMethod of `SGWGmPlayer.def` (line 650, right after `onPhysics` = 221), so index 222, matching the dispatch table. Pinned in `gm_indices_match_def_document_order` (`109 + 113`).
2. **Client path.** SS-E1 C-Q2: `Event_SlashCmd_GMShout` → `NetOut_SendGMShout`, no Lua. The typed-text-to-`(isGlobal, Text)` split is **unresolved**; this packet only decodes the two def fields.
3. **No legacy implementation.** `grep -ri gmshout deprecated/python` finds nothing. The def's base method `sendGMShout(ChannelID, BroadcastScope, SpaceID, Text)` and `Communicator.hearGMShout` are the original server-internal hops; D-SS16 delivers `onPlayerCommunication` directly, so neither is used. There is no legacy `.announce`, so there is no parity row to deviate from; `.announce` is a new console command.
4. **Channel.** `enumerations.xml` says `CHAN_server = 8`; Rust's `CHAN_SERVER = 7` and the client joins "server" as 7 (`world_entry_chat::DEFAULT_CHAT_CHANNELS`). That is the D-ORG14 drift. The broadcast uses the constant, so it follows ORG-09 when the value moves; the byte-exact test builds its expectation from `CHAN_SERVER` for that reason.
5. **Speaker flag.** `SPEAKER_GM = 1` in `ESpeakerFlags`; the new Rust constant is pinned by parsing the def (`speaker_gm_matches_enumerations_xml`), not by a copy of itself.
6. **Gate.** CAT-L-06 recommended a base-side implementation "so `access_level` is in scope". Since #475 the cell has `CellEntity::access_level` (from `account.accesslevel` at `InitPlayerState`) and `gm_gate::requires_gm` gates every index >= 109, GameMaster and up. That is exactly D-SS16's rule, so the handler relies on the existing gate and does not re-check; the guard proves the gate covers 222 for Player and Moderator.

## Design decisions

- **One implementation.** CM 222 and `.announce` both call `gm::shout::broadcast(caller, scope, text, source, ..)`. It validates, logs the audit row, then sends.
- **Space scope on the cell, global scope on the base.** The cell knows its space's player entities (`all_player_entity_ids` filtered by `get_entity_space_id`, i.e. the space *instance*, not the world name); each gets `EntityMethodCall(onPlayerCommunication)`. Global goes out as `CellToBaseMsg::Chat(ChatCellToBase::GmBroadcast)`, and the base sends it to every session `OnlinePlayerIndex` lists that has a `player_entity_id`: character-select and mid-world-entry sessions are not listed, so they never get a method call on an entity their client lacks. Sends are reliable (registered with each session's channel).
- **One payload builder.** `cimmeria_wire::cell::chat::serialize_gm_broadcast(speaker, text)` = `serialize_on_player_communication(speaker, SPEAKER_GM, CHAN_SERVER, text)`. The cell serializes once; the base only fans the bytes out.
- **New nested message.** `ChatCellToBase` (organizations pattern) instead of another top-level variant: `cell_to_base.rs` is already 799 lines, and SS-C3/C4 can add chat variants without touching it. It carries `entity_id`, `player_id`, `account_id` from the cell's `CellEntity`, a fixed `source` label, and the payload; no privilege bit.
- **`isGlobal`.** A BigWorld boolean byte: 0 is the space, anything else is global.
- **Text.** `org_text::validate(TextField::ChatText, ..)` (255 UTF-16 units, no control or format characters), plus a blank-text refusal. Rejected with a GM feedback line, never truncated. No flood limit, matching D-SS14's GM exemption on chat.
- **Speaker.** The GM's `CellEntity::character_name`; `"GM"` only if it was never threaded in.
- **Feedback.** The GM is always a recipient (their own space; listed online for global), so the GM sees the line on the first press. Refusals send a feedback line.
- **`.announce` parsing.** `.announce [space] <text>`: `space` (any case) only as the first word selects the space scope; words re-join with single spaces (the console tokenizes on whitespace). `.announce space` alone is refused with usage.
- **Log text.** `chat.gm_broadcast` includes the text. Chat bodies are otherwise kept out of SigNoz as private; a GM broadcast is a public staff announcement and the audit needs what was said. Already capped at 255 units.

## Telemetry added

| Event | Level | Target | Where | Fields |
|---|---|---|---|---|
| `chat.gm_broadcast` | INFO | `chat` | `cell-console` `gm/shout.rs` | `entity_id`, `account_id`, `player_id`, `speaker`, `source` (`native` / `console`), `scope` (`space` / `global`), `space_id`, `text_units`, `text` |
| `chat.gm_broadcast_delivered` | INFO | `chat` | space: `gm/shout.rs`; global: `base-world-entry` `chat_dispatch.rs` | actor ids, `source`, `scope`, `delivered`, `failed`, (`space_id` / `not_in_world`) |
| `chat.gm_broadcast_rejected` | WARN | `chat` | `gm/shout.rs`, `console/social.rs` | `entity_id`, `account_id`, `player_id`, `source`, `scope`, `text_units`, `reason` = `empty_text` / `malformed_args` / `no_text` / `caller_not_found` / a `TextReject::reason()` (`too_long`, `control_char`, ...) |
| `chat.gm_broadcast_send_failed` | WARN | `chat` | `gm/shout.rs` (global hand-off to the base; each space recipient), `gm_broadcast.rs` (each global recipient) | the GM's `entity_id`, `account_id`, `player_id`; `scope`; `reason` = `base_channel_closed` / `send_error`; per-recipient rows add `target_player_id`, `target_entity_id` (and `addr` on the base) |
| `chat.gm_broadcast_skipped` | WARN | `chat` | `gm_broadcast.rs` | the GM's ids, `reason = session_map_poisoned` |

A non-GM attempt is the existing gate row, `GM-gated cell method rejected` with `method_index = 222`.

SigNoz query for a tester: logs where `event IN ('chat.gm_broadcast', 'chat.gm_broadcast_delivered', 'chat.gm_broadcast_rejected')`, grouped by `player_id`; the audit row and the delivery row of one shout share `player_id` and `source`. For a refused player: `method_index = 222` on the gate's WARN.

## Tests

| Test | Crate | Type | Guards |
|---|---|---|---|
| `gm_shout_rejected_for_player` | `cimmeria-cell` (`dispatch::tests`) | 12 + dispatch | CAT-L-06: Player (0) and Moderator (1) get only `onErrorCode`; nothing broadcast, nothing to the base |
| `gm_shout_space_scope_stays_in_space` | `cimmeria-cell-console` (`gm::tests::shout`) | 8 | `isGlobal = 0` reaches exactly the GM's space instance (GM + one player, byte for byte); a player in another world and one in a **second instance of the same world** get nothing; no global forward |
| `gm_shout_global_forwards_one_broadcast_to_base` | same | unit + 12 | `isGlobal = 1`: one `GmBroadcast` with the cell's ids and payload; the audit row's fields |
| `gm_shout_base_channel_closed_logs_send_failed` | same | 12 | base channel gone: global logs one `chat.gm_broadcast_send_failed` for the lost hand-off, space one per recipient; all carry the GM's `account_id` / `player_id` and `reason`, the space rows name the recipient (Copilot review, PR #887) |
| `help_announce_shows_usage_and_argument_detail` | `cimmeria-cell-console` (`console::tests::ss_c2_announce`) | unit | `.help announce` prints the summary and a `text (str)` detail row (new `ANNOUNCE_ARGS` in `registry/mod.rs`) |
| `gm_shout_refusals_send_feedback_and_nothing_else` | same | 12 | blank, 256-unit, truncated WSTRING, empty args: `reason` logged, one GM feedback line, no broadcast |
| `gm_broadcast_global_reaches_every_listed_session` | `cimmeria-base-session` | 8 | two listed sessions each get one reliable packet with the exact payload on their own entity; an unlisted session and a listed one with no entity get nothing |
| `gm_broadcast_arm_fans_out_and_logs_delivery` | `cimmeria-base-world-entry` | dispatch arm | `CellToBaseMsg::Chat` reaches the fan-out and logs `chat.gm_broadcast_delivered` |
| `gm_broadcast_bytes_are_exact` | `cimmeria-wire` | 2 | the byte-exact line |
| `speaker_gm_matches_enumerations_xml` | `cimmeria-wire` | 2 | `SPEAKER_GM` against the def |
| `announce_parse_defaults_to_global`, `announce_parse_space_keyword_selects_space_scope`, `announce_parse_rejects_scope_without_text` | `cimmeria-cell-console` (`console::social`) | unit | the `.announce` parse test |
| `announce_space_sends_the_gm_line_to_the_space`, `announce_without_scope_is_global` | `cimmeria-cell-console` (`console::tests::ss_c2_announce`) | unit, end to end through `handle_console_command` | both scopes from the console |

`gm_indices_match_def_document_order` and `implemented_indices_are_in_gm_tail` gained the 222 row; `every_spec_is_dispatched` covers `.announce`.

## Commands run (worktree root, all through the lane)

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo check -p cimmeria-cell-console -p cimmeria-base-world-entry -p cimmeria-base-session -p cimmeria-wire --all-targets` | 101, then 0 | first run: `base-world-entry` lacked the `feedback` / `gm_broadcast` re-exports; fixed |
| `lane.sh cargo test -p cimmeria-cell --lib gm_shout` | 0 | 1 passed |
| `lane.sh cargo test -p cimmeria-cell-console --lib` | 0 | 275 passed |
| `lane.sh cargo test -p cimmeria-wire --lib chat` | 0 | 5 passed |
| `lane.sh cargo test -p cimmeria-base-session --lib gm_broadcast` | 0 | 1 passed |
| `lane.sh cargo test -p cimmeria-base-world-entry --lib gm_broadcast` | 0 | 1 passed |
| `lane.sh cargo test -p cimmeria-server --bins logging` | 0 | 52 passed |
| `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-base` | 0 | 1283 passed, 0 skipped |
| `lane.sh cargo fmt --all -- --check` | 0 | clean |
| Review round (PR #887): `lane.sh cargo nextest run -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell` | 0 | 992 passed, 0 skipped |
| Review round: `lane.sh cargo fmt --all -- --check`; `lane.sh cargo clippy -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell --all-targets -- -D warnings` | 0 | clean |
| `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-server --all-targets -- -D warnings` | 0 | clean |

No live-DB tier: the packet adds no SQL.

## Regression proof

Each fix reverted on top of the commits, the guard run, then `git checkout --` restored:

| Revert | Guard | Result with the revert |
|---|---|---|
| `gm_gate::requires_gm` exempts 222 (`index >= 109 && index != 222`) | `gm_shout_rejected_for_player` | FAILED: "a non-GM sendGMShout must log the gate rejection" (the shout ran) |
| space filter widened to every player entity | `gm_shout_space_scope_stays_in_space` | FAILED: recipients `[1, 2, 3, 4]`, expected `[1, 2]` |
| global fan-out iterates every session instead of the online index | `gm_broadcast_global_reaches_every_listed_session` | FAILED: `delivered: 3` (the character-select session got the line), expected 2 |
| the `if !forwarded` check disabled (review fix) | `gm_shout_base_channel_closed_logs_send_failed` | FAILED: "global: a lost hand-off must log chat.gm_broadcast_send_failed" |
| `account_id` / `player_id` dropped from the space send-failure row (review fix) | `gm_shout_base_channel_closed_logs_send_failed` | FAILED at the `account_id` assertion on the `space` row |

## Known gaps

- **No in-client test.** The `/gmshout` argument split is unrecovered (SS-E1 C-Q2 PARTIAL). If the client always sends one `isGlobal` value, `/gmshout` covers only that scope; `.announce` covers both either way. First UAT: type `/gmshout` with a second client online, then `.announce` and `.announce space`, and check SigNoz for the three events.
- **Gate travel window.** A session stays listed across gate travel and is addressed at its current `player_entity_id`; a line sent while the client is swapping entities may be dropped by the client. Not handled.
- **Discord.** `.announce` is relayed to the GM Discord channel by the console's existing `emit_gm_command`; the native `/gmshout` is not (the same as every other native `gm*` method).
- **No type-11 wireclient test.** The packet's acceptance does not ask for one.
- The base-side per-recipient `send_error` row (`gm_broadcast.rs`) now carries the GM's ids, but has no test: `TestTransport` has no send-failure injection.
- No advisor consulted: the design reuses the existing gate and fan-out primitives, and the security invariant has its named guard.

## Integration edits for the coordinator

1. **Contract:** the ledger's "Messages" section could name `CellToBaseMsg::Chat(ChatCellToBase)` next to the duel pattern, so SS-C3 / SS-C4 add chat variants there.
2. **Merge points:** `crates/wire/src/cell/messages/cell_to_base.rs` gains `Chat(..)` as its last variant, and `base-world-entry/.../cell_dispatch/mod.rs` a `Chat` arm after `Org`; SS-D1's `Duel(..)` lands in the same two places (adjacent-line merge). `console/dispatch.rs`, `console/mod.rs` and `registry/commands/mod.rs` each gain one `social` line; SS-C1 does not own them, but check the merge if it touched them during the `chat/` split.
3. **`docs/gap-analysis.md`:** §21 "GM broadcast" and Admin/GM "Announcement broadcast" moved KM → NT; the two matrix rows and the TOTALS line (NT 60 → 62, KM 140 → 138) were updated. The "Summary Percentages" table already disagreed with the TOTALS line before this packet (NT 58 vs 60, KM 142 vs 140), so it was left for the SS-99 recount.
4. **`docs/commands.md`:** the `/gmsendgmshout` row was renamed `/gmshout` (the slash command is `Event_SlashCmd_GMShout`, `docs/technical/slash-commands.md:344`) and marked working; "At a glance" counts moved by one (65 / 162 / 45 tests).
5. **CAT-L-06 finding doc** (`docs/security-audit/.../CAT-L-chat-contact.md`) still reads as open; the fix is this packet via the cell gate, not the base-side check it suggested. Worth a status line when the security table is reconciled.
