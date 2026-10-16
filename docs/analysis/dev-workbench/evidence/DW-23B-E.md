# DW-23B-E: dot network and AI debug command evidence

This is a source-only contract for the 14 `Spec` names in `crates/cell-console/src/cell/console/registry/commands/net_debug.rs`, not live client proof. Every command enters as GM say-chat through `crates/cell-console/src/cell/console/chat/mod.rs::handle_chat_message`, which checks server-side `access_level`; `dispatch.rs::handle_console_command` checks token count and target before `net::dispatch` or `aggro::toggle`. `dispatch.rs::resolve_target` confines a required selection to the caller's space and view. `Target::None` passes an optional in-view selection, so it does **not** mean the handler necessarily acts only on the caller. `Target::Spawnable` accepts every entity, but the client binds `onSequence` only to beings (`docs/protocol/client-method-dispatch-table.md`, “Client method binding hierarchy”). The parser uses `split_whitespace`; quotation marks have no escaping semantics. The direct `exec` test helper bypasses the chat GM gate and is not an authorization test.

The accepted-command telemetry logs command name, count and identities without raw arguments. The same path calls `cimmeria_discord::emit_gm_command` with `args.join(" ")` **before** the handler runs ([dispatch.rs](../../../../crates/cell-console/src/cell/console/dispatch.rs)); malformed typed arguments that pass count/target checks also reach that audit. Feedback is a second data sink: `send_gm_feedback` logs the first 400 characters of every feedback line at `console.feedback` DEBUG ([feedback.rs](../../../../crates/cell-console/src/cell/console/gm/feedback.rs)), and the server logging filter enables that target at DEBUG ([filters.rs](../../../../crates/server/src/logging/filters.rs)). Thus text, object identifiers, positions, IDs and echoed invalid arguments can reach Discord or telemetry; neither sink is redacted. Some paths, such as a vanished `.debug_follow` target, return without feedback. Feedback is never a delivery or state receipt. Before offering any free-text/private palette form, redact before both sinks and add negative-log tests that assert representative sensitive tokens are absent from Discord events and `console.feedback` telemetry, including refusal paths. No per-command invocation rate limit, expiry timer or automatic reset appears in these handlers; any external chat/transport limit is outside this evidence. The network commands below send game client methods, not a packet capture or connection setting. They were not invoked while collecting evidence.

## .net_seq

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::{sequence,build_sequence_args}`; `console/dispatch.rs::resolve_target`; `docs/protocol/client-method-dispatch-table.md` method 1 and binding hierarchy.
- **Arguments:** Required `sequenceId` signed `i32`; optional `viewType` signed `i8` (-128..127), default 0. One or two whitespace tokens; no sequence-ID domain allowlist.
- **Target/scope:** Required selected same-space, in-view `Spawnable`; source, destination and broadcaster are that entity. `send_entity_method_to_self_and_witnesses` fans `onSequence` to its own client if a player and its witnesses. A prop is allowed by the registry but has no `SGWBeing` binding, so visible delivery can fail.
- **Preconditions and authority:** Server GM gate, count and selection gate; numeric parse refuses invalid input. No separate Kismet sequence existence or target-class validation.
- **Effects and impact:** One `onSequence` with zero impact time, empty NVP array and instance 0; client-side animation/event may fire. No server sequence state or duration, no stop command, no per-command rate cap. Repetition replays it.
- **Verification:** Feedback `net_seq …` confirms construction only. Use read-only `server_packet_tap_read` for method 1 addressed to the intended witness/session, then client event/visual telemetry if available; no existing exact client-applied readback or sequence state query.
- **Compensation:** None proven; a sent sequence cannot be retracted.
- **Availability:** Disabled pending being-only selected-target validation: broad `Spawnable` accepts clients that cannot bind `onSequence`. Exact delivery/application receipt is also absent.

## .net_seqto

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::{sequence,build_sequence_args}` (`CallerToTarget`); `console/dispatch.rs::resolve_target`; client method table method 1.
- **Arguments:** Required signed `i32` `sequenceId`; optional signed `i8` `viewType` default 0; whitespace tokens, no sequence allowlist.
- **Target/scope:** `Target::None`: optional selected in-view entity, otherwise caller. Payload source/broadcaster is caller and destination is selected entity or caller. Fan-out is on caller to their client and witnesses, not the selected target's owner automatically.
- **Preconditions and authority:** Server GM/argument gates; target selection is optional and same-space/in-view when present. No content validation of destination's suitability.
- **Effects and impact:** One caller-sourced `onSequence`; may trigger client animation/event for any observer of the caller. One-shot, no expiry/reset/rate cap in handler.
- **Verification:** Feedback shows source and destination IDs but does not prove application. `server_packet_tap_read` can check method 1 per session; no exact client-applied readback.
- **Compensation:** None proven after client dispatch.
- **Availability:** Disabled pending being-only optional-destination validation; arbitrary sequence/destination still lacks a safe content contract or application readback.

## .net_seqfrom

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::{sequence,build_sequence_args}` (`TargetToCaller`); `console/dispatch.rs::resolve_target`; client method table method 1/binding hierarchy.
- **Arguments:** Required signed `i32` `sequenceId`; optional signed `i8` `viewType` default 0; no sequence allowlist.
- **Target/scope:** Required selected same-space/in-view `Spawnable` is source/broadcaster; caller is destination. Fan-out is selected entity's witnesses plus owner if a player; a prop has no `onSequence` binding.
- **Preconditions and authority:** Cell GM gate and target/count checks; parse refuses invalid numbers, but selection can still be a non-being.
- **Effects and impact:** One client `onSequence`, no server sequence lifecycle, expiry, rate cap or stop command.
- **Verification:** Feedback and `server_packet_tap_read` may show dispatch; client application/visible result must be observed separately and has no exact readback.
- **Compensation:** None proven.
- **Availability:** Disabled pending being-only selected-target validation; `Spawnable`/being mismatch and missing applied receipt remain.

## .net_timer

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::timer`; `console/tests/p38.rs` byte and clock tests; `docs/protocol/client-method-dispatch-table.md` method 12/clock note.
- **Arguments:** Required signed `i32` ID and signed `i8` type (-128..127). Optional finite `f32` total time defaults to 1 second **even when an invalid supplied token is parsed**; optional signed `i32` secondary ID defaults to 0 also on invalid input. Four whitespace tokens maximum. No positive-duration bound or semantic type allowlist.
- **Target/scope:** `Target::Spawnable` requires a selected entity, but the handler ignores it and sends `onTimerUpdate` directly to the caller's player entity. This is a real registry/handler mismatch.
- **Preconditions and authority:** Server GM/count/selected-target gate and required numeric parsing. The `i8` conversion explicitly refuses out-of-range types; malformed optional values silently default.
- **Effects and impact:** One client timer update. `BigWorldTimeComplete = game_time_secs() + totalTime`; protocol says a zero expiry clears a timer, and negative/past expiry may clear/expire immediately. No server timer state, rate cap or automatic reset beyond client timer expiry. Existing timer ID may be overwritten.
- **Verification:** `console/tests/p38.rs` proves 21-byte payload, secondary ID, default and absolute expiry. `server_packet_tap_read` can confirm method 12, ID/type/expiry to caller. No server query of active client timers and no exact applied readback.
- **Compensation:** A later timer update using the same ID may replace/clear it, but prior state is not captured; no proven inverse.
- **Availability:** Disabled: misleading selected-target policy, silent malformed-duration fallback, unbounded timer semantics and no readback.

## .net_mapinfo

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::map_info`; `console/dispatch.rs::resolve_target`; `docs/protocol/client-method-dispatch-table.md` method 123.
- **Arguments:** Required signed `i32` system ID, key ID and lifetime; optional `delete` is true only for `1`/`true`, otherwise false, including invalid words; optional signed `i32` system type defaults to 0 on invalid text. No nonnegative/range validation before conversion to unsigned wire fields.
- **Target/scope:** Required selected player receives `onMapInfo`; world ID and position are copied from the **caller**, not the recipient. Negative signed IDs/lifetime become unsigned wire values.
- **Preconditions and authority:** GM, three-to-five tokens and same-space/in-view player target; invalid required numbers refuse. Missing caller entity produces zero position/world fallback.
- **Effects and impact:** One client map-info add/update/delete instruction with a client lifetime; possible change to another player's map. No server map-info state, per-command rate cap or automatic rollback. `delete` may remove an existing entry.
- **Verification:** Feedback lists IDs, lifetime and delete flag, not recipient delivery. `server_packet_tap_read` can check method 123/payload to recipient; client map display/application lacks an exact read-only query.
- **Compensation:** A subsequent delete/add may alter the entry but original content is not captured; no proven inverse.
- **Availability:** Disabled: signed-to-unsigned wrap, silent optional fallbacks, cross-player client mutation and no applied readback.

## .net_speak

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::speak`; `console/dispatch.rs::handle_console_command`; client method table method 28.
- **Arguments:** One required message **word**; optional case-insensitive channel `say|emote|yell|team|squad`, default `say`. No quoting, length bound, content filter or multiline form. Unknown channel refuses after the accepted-command Discord audit.
- **Target/scope:** Required same-space/in-view `Spawnable` supplies speaker name (`npc_name` or `Entity N`); `onPlayerCommunication` fans from the selected entity to its own client and witnesses. Registry allows props, mobs, pets and players, but method 28 is bound under `SGWPlayer` ([client handler bindings](../../../protocol/client-method-dispatch-table.md#client-handler-bindings)); dispatch on a non-player entity is dropped by the client rather than displayed. Player targets can deliver a spoofed message.
- **Preconditions and authority:** GM/target/count gates; handler rejects missing message or unknown channel. It does not enforce a player-class target or obtain the selected player's consent.
- **Effects and impact:** One spoofable chat/communication message, potentially seen by others in AoI; channel byte is set but the audience comes from witness fan-out, not normal chat-channel membership routing. No duration, reset or rate cap in handler.
- **Verification:** Feedback echoes message into `console.feedback` DEBUG telemetry, but is not witness proof. `server_packet_tap_read` could see method 28 per witness; no independent exact readback of display. Treat message as private content.
- **Compensation:** No recall for a sent message.
- **Availability:** Disabled pending pre-dispatch Discord redaction and feedback-log redaction, each guarded by negative-log tests for accepted and refused messages (including an unknown channel). Spoofing and player-bound method/class mismatch also need fixes and safe receipt.

## .net_dialog

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::dialog`; `console/dispatch.rs::resolve_target`; `crates/cell-content/src/cell/interactions/dialog.rs::send_dialog_display` and its offered-dialog tests.
- **Arguments:** One required signed `i32` dialog ID. Whitespace token; no dialog-existence check in this handler.
- **Target/scope:** `Target::None` passes optional selected same-space/in-view entity. Without a selection the handler refuses; with one it calls `send_dialog_display(caller, selected entity, dialog ID)`, targeting the caller's client rather than all witnesses.
- **Preconditions and authority:** GM/count gate and numeric parse; a selected entity is required in practice but the registry does not enforce one or NPC class.
- **Effects and impact:** `send_dialog_display` first adds `dialogId` to caller's bounded `offered_dialog_ids` set, then best-effort sends client method `onDialogDisplay` (105). This changes server authorization for future `DialogButtonChoice`; a stream of calls can evict an older unanswered offer and strand its chain. It may alter client UI and trigger later content choices. No duration, automatic reset or per-command rate cap in this handler.
- **Verification:** `net_dialog …` feedback is sent even if the cell-to-base send fails. `server_packet_tap_read` can check method 105 addressed to caller; `dialog.send_display` log exposes IDs and an eviction warning. No existing read-only tool exposes the current offered-dialog set or proves client display.
- **Compensation:** Closing client UI does not undo `offered_dialog_ids`. No proven removal or inverse for an evicted offer.
- **Availability:** Disabled: required NPC target policy, dialog existence, offered-set bounds and delivery/readback need explicit guards before a palette form.

## .net_challenge

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::challenge`; `crates/cell-methods/src/cell/cell_methods/player/social.rs` `CLIENT_CHALLENGE_RESPONSE`; `docs/protocol/login-handshake.md` post-login challenge; client method table method 142 and cell method table method 101.
- **Arguments:** Exactly five tokens: signed `i32` challenge nonce, signed `i32` type, one object/path word, signed `i32` ID1 and ID2. No challenge-type enum check, object allowlist or length bound; no quoted object path.
- **Target/scope:** `Target::None` permits an optional selection but ignores it. Sends `onClientChallenge` to **caller**, not selected target; may ask client to hash a file/object and respond over cell method 101.
- **Preconditions and authority:** Server GM/count gate, numeric parse and nonempty third token; response path currently only logs challenge nonce as `UNIMPLEMENTED`, without correlation/validation.
- **Effects and impact:** Triggers client integrity work and possible response; not a read-only query. One-shot but repeated calls are not rate-limited by this handler. Object string and response hash are sensitive; supplied object goes to Discord GM audit before validation. No server challenge lifecycle/reset.
- **Verification:** `server_packet_tap_read` can inspect outbound method 142 and inbound method 101; server handler's `UNIMPLEMENTED` log is not a verified result, and no accepted-result readback exists.
- **Compensation:** None; a challenge cannot be unsent or its computed response retracted.
- **Availability:** Disabled pending audit redaction and response-handler proof: response processing, type/object validation and rate control are absent. Registry help incorrectly says “target” but handler sends to caller.

## .debug_velocity

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::debug_velocity`; `console/parse.rs::parse_f32`; `crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot`.
- **Arguments:** Exactly three finite `f32` coordinates `x y z`; no magnitude or speed bound. Whitespace tokens.
- **Target/scope:** Required selected same-space/in-view `Spawnable`; handler changes its cell entity `velocity`, including players or props.
- **Preconditions and authority:** GM/selection/count gate and finite parsing. If entity disappears between resolution and mutation, handler still sends success-looking feedback.
- **Effects and impact:** In-memory motion state mutation, potentially reflected in movement/AI updates; no dedicated wire send here, persistence, duration, rate cap or automatic reset. AI or movement ticks may overwrite it.
- **Verification:** `server_entity_get` exposes `velocity` by entity ID (`crates/lab-mcp/src/tools/entities.rs`, wire snapshot). Compare before/after promptly; feedback alone is insufficient. Position may change afterward.
- **Compensation:** Manually restore captured velocity in same entity epoch if still valid; movement side effects are not reversible.
- **Availability:** Disabled: no magnitude bound and success feedback can mask a vanished target.

## .debug_controller

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::dispatch` `debug_controller` branch; `console/dispatch.rs::resolve_target`.
- **Arguments:** None.
- **Target/scope:** Required selected same-space/in-view `Spawnable`; selection is checked then ignored.
- **Preconditions and authority:** Server GM/target/count gate. A missing/stale target refuses before the no-op branch.
- **Effects and impact:** Handler only emits `no debug movement controller in the Rust server (no-op)` feedback; no controller toggled, duration/reset/rate behavior. Accepted-command and Discord audit still occur.
- **Verification:** The explicit no-op feedback is the only result; `server_entity_get` should show no state change but cannot prove absence of every side effect. Source contains no mutation in this branch.
- **Compensation:** None needed for state; audit/feedback already emitted.
- **Availability:** Hidden: obsolete no-op command should remain inventory-visible but not offer a fake toggle.

## .debug_follow

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::debug_follow`; `crates/cell-combat/src/cell/service/npc_ai/follow.rs`; `crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot`.
- **Arguments:** None.
- **Target/scope:** Required selected same-space/in-view `Spawnable`; target entity gets `follow_target_id = caller` and `AiState::Follow`, or a second invocation against the same caller clears the ID and sets `Idle`. Registry does not constrain this to NPCs.
- **Preconditions and authority:** Server GM/target/count gate; entity may vanish before mutation, then handler returns without feedback. No prior AI state captured.
- **Effects and impact:** Persistent-in-memory AI mode until second toggle, AI transition, entity removal or reset; **no duration/automatic cleanup** in handler. Following can move the entity and affect other observers. No per-command rate cap.
- **Verification:** Feedback says “following you” or “cancelled”; `server_entity_get` exposes `ai_state` but **not** `follow_target_id`, so it is only partial independent readback. The NPC AI trace fields include `follow_target_id`; use `server_log_tail` if that trace is enabled and identity-fenced. No exact current follower binding query.
- **Compensation:** Reissue only if the same target still follows this caller; otherwise a second toggle could start following. No general inverse or position restoration.
- **Availability:** Disabled: non-NPC target permitted, no expiry and no exact follower-state readback.

## .threaten

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::threaten`; `console/parse.rs::parse_f32`; `crates/cell-combat/src/cell/combat/threat/aggro.rs`; `crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot`.
- **Arguments:** One finite `f32` amount, no nonnegative or maximum bound. Zero/negative values pass. The existing threat total is not checked; even a finite increment can overflow the stored `f32` to infinity, and a later opposite-signed increment can make it NaN.
- **Target/scope:** Required selected same-space/in-view `Mob`; `TargetPolicy::matches` means any non-player entity, not a proven NPC class. Adds amount to its `threat_list[caller]`.
- **Preconditions and authority:** Server GM/count/selection and finite parse; if entity vanishes after resolution, feedback still claims addition.
- **Effects and impact:** In-memory combat threat mutation, possibly changes aggro and movement on later AI ticks. The handler adds directly to `threat_list`; non-finite totals can corrupt target ordering because NPC selection compares threat values with `partial_cmp(...).unwrap_or(Equal)` ([fight target](../../../../crates/cell-combat/src/cell/service/npc_ai/fight_target.rs), [lifecycle](../../../../crates/cell-combat/src/cell/service/npc_ai/lifecycle/mod.rs)). These production comparators do not panic on NaN, but treat an unordered comparison as equal. No duration, rate cap or automatic inverse; combat/leash/release paths may later clear threat.
- **Verification:** Feedback reports requested increment, not actual stored result. `server_entity_get` has AI state but no threat-list values; `server_log_tail` NPC AI trace can show top threat/count, not caller's exact value in general. No exact independent readback tool.
- **Compensation:** No safe general inverse: negative amount is accepted but concurrent threat changes make subtraction unsafe.
- **Availability:** Disabled: a future form must bound the increment and reject a non-finite resulting total before mutation, then provide exact readback. Signed arbitrary threat and absent exact readback remain unsafe.

## .aggression

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/net.rs::aggression`; `console/tests/na13_aggro.rs` set/clear/refusal and witness tests; `docs/protocol/client-method-dispatch-table.md` mob methods 27/28.
- **Arguments:** Required case-insensitive `clear` or signed `i32` level 0..5; other values visibly refuse with no mutation. One token.
- **Target/scope:** Required selected same-space/in-view `Mob` (registry's non-player check accepts props as well as NPCs). Handler sets `e.aggro.override_level` and sends `onAggressionOverrideUpdate` or `onAggressionOverrideCleared` to witnesses of that entity. Methods 27/28 are bound under `SGWMob`, so a prop dispatch is dropped by the client ([mob method table](../../../protocol/client-method-dispatch-table.md#sgwmob-client-method-dispatch-table)). A palette form requires an actual NPC-class guard, not `Target::Mob` alone.
- **Preconditions and authority:** Server GM/count/selection gate and enum conversion. If target disappears after resolution, feedback says `entity gone -- nothing changed`.
- **Effects and impact:** In-memory aggression override until `clear`, entity reset/removal or other write; no timed expiry or per-command rate cap. It can change AI hostility and visible mob state. Level 0 maps passive/neutral behavior; `clear` returns to faction-derived reaction.
- **Verification:** Handler feedback labels effective level/source; `na13_aggro` tests prove field changes, invalid-level refusal and witness method fan-out. `server_entity_get` omits override; packet tap can show method 27/28 but not lasting server field. No exact independent override readback identified.
- **Compensation:** `clear` removes the override, but does not restore a previously nonempty override or undo AI consequences. Capture prior state through a new read query before offering restore.
- **Availability:** Disabled: actual NPC-class validation, exact prior/current override readback and safe automatic compensation are missing.

## .aggro

- **Sources:** `registry/commands/net_debug.rs::SPECS`; `console/aggro.rs::toggle`; `console/tests/na13_aggro.rs` query/toggle/refusal tests; `crates/cell-combat/src/cell/service/npc_ai/aggro_gates.rs`.
- **Arguments:** Zero tokens queries; optional case-insensitive `on|off` toggles. Any other one-token value visibly refuses without change, but the refusal echoes the bad token into `console.feedback` DEBUG telemetry; no quoting.
- **Target/scope:** No selected target. Character-keyed `SpaceManager::gm_aggro_off` affects only the caller's eligibility for **idle proximity** aggro scans while the entity retains GM access. Damage and scripted threat still engage, and existing fights continue.
- **Preconditions and authority:** Server GM gate; `toggle` requires caller `player_id`, otherwise refuses with `nothing changed`. `off` inserts ID; `on` removes it; same value is idempotent.
- **Effects and impact:** Bare query is a state read plus feedback/audit. `on|off` changes server in-memory AI targeting across zone changes/relogs until toggled or server restart; no timed expiry or per-command rate cap. It does not alter network connectivity.
- **Verification:** Each invocation replies with ON/OFF and `set`/`unchanged`; tests inspect the keyed set and bad-input refusal. No independent `server_entity_get` field for this switch, so palette readback is not exact without a new read tool. `server_log_tail` can observe `npc_ai.aggro` toggle events but log presence is not current state.
- **Compensation:** Opposite explicit `on`/`off` restores the switch if the same character still has GM access; it cannot unwind encounters that occurred meanwhile.
- **Availability:** Disabled in palette pending independent current-state readback, an explicit auto-reset/session policy and feedback-log redaction with a negative-log refusal test. Direct `.aggro` remains registered. Query-only form could be enabled once identity-bound readback exists.
