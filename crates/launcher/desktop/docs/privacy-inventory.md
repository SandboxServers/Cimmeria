# Launcher and game diagnostics inventory

> **Type:** Reference · **Audience:** Players, maintainers and reviewers · **Updated:** 2026-10-23
> **Companions:** [Player controls](../../../../docs/client/launcher-guide.md#main-page-preferences-and-changes), [summary contract](launcher-summaries.md), [native timing](launcher-telemetry.md), [client architecture](../../../../docs/architecture/client-telemetry.md), [session transport](../../../../docs/architecture/dev-session-telemetry.md), [operator runbook](../../../../docs/operations/telemetry.md)

This source audit covers the current desktop launcher, player telemetry DLL,
shared launcher-core transport and server ingest. Source constructors/selectors
linked below define exact fields; open event bags and log text are not fixed
schemas. This is not proof of what a particular installation transmitted, the
live deployment's retention, or every behavior of the original game.

## Controls

| Control/action | Effect and limits | Source |
|---|---|---|
| Launcher diagnostics | Off by default. Enables fixed attempt summaries, independently of game diagnostics. Local launcher logging continues. | [Summary tracker](../engine/src/storage/launcher_summary/tracker.rs), [consent](../engine/src/storage/launcher_summary/consent.rs) |
| Game diagnostics | Off by default in its own game-telemetry.json. Enables supported game telemetry/injection plus allowed launcher/adoption events before Play. | [Choice/identity](../engine/src/storage/game_telemetry/mod.rs), [host gate](../shell/src/host/game_telemetry.rs), [launcher target](../engine/src/storage/game_telemetry/launcher_session.rs) |
| Summary withdrawal | Closes exporter gate, cancels request/backoff and purges pending summaries. Opt-in empties the old queue first. Received bytes cannot be recalled. | [Consent](../engine/src/storage/launcher_summary/consent.rs), [exporter](../engine/src/storage/launcher_summary/export.rs) |
| Game withdrawal | Discards launcher's unsent queue and invalidates its consent generation; session loop stops without final flush/bundle. An already posted chunk can finish. Already injected DLL stays loaded with its own uploader/session token; checkbox changes injection at next Play. Exit the running game to stop that DLL. | [Choice setter](../engine/src/storage/game_telemetry/mod.rs), [consent generation](../engine/src/storage/game_telemetry/transport/queue.rs), [session loop](../engine/src/storage/game_telemetry/session_loop/mod.rs), [DLL uploader](../../../client-telemetry/src/uploader.rs), [DLL session reads](../../../client-telemetry/src/session.rs) |
| Upload debug logs | Press authorizes one upload regardless of either toggle or unfinished install. Reads existing logs, including earlier sessions. Closing launcher cancels at a chunk boundary. Received chunks remain received. | [Admission/lifecycle](../engine/src/storage/log_upload/ingest_flow.rs) |
| Client patches | Separate gameplay-fix preference, not diagnostics consent. Patches can write a local diagnostic log; boot/counter observations use the game-consented transport. | [Patch-event reader](../engine/src/storage/game_telemetry/patch_events.rs), [transport consent](../engine/src/storage/game_telemetry/transport/queue.rs) |

Turning off is not deletion of received server data, local game logs, retained
identity, or a manually requested upload. Normal login/update/download traffic
is independent of these diagnostics switches.

## Launcher collectors

| Channel | Data/categories | Source |
|---|---|---|
| Attempt summaries | Schema version, random event/attempt UUIDs; install/prepare_runtime/repair/uninstall/launch operation; final phase/outcome; closed failure code; optional duration and starting/running/download/extraction timings; retries; normalized launcher version, OS/architecture; aggregate overflow/expired/rejected counts. No adoption/update summary variant, arbitrary text, path, URL, account, persistent install/machine ID, token or cookie in this payload. | [Complete schema](../engine/src/storage/launcher_summary/schema.rs), [request](../engine/src/storage/launcher_summary/exchange.rs), [server schema](../../../admin-api/src/routes/telemetry/launcher_summary/dto.rs) |
| Local rotating log | Times/severity/target/messages/fields for launcher INFO/WARN/ERROR and other WARN/ERROR. Startup/build/configuration; catalog/download/archive/install/repair/update/uninstall/prerequisite/PhysX; UI/settings/login/adoption; resource/helper/launch/patch outcomes and timing. Recognized home/username path segments replaced before writing; other paths and arbitrary error/panic text may remain. Seven daily files, not a guaranteed seven-day age or byte cap. | [Writer/filter/redaction](../shell/src/telemetry.rs), [panic hook](../shell/src/panic_hook.rs) |
| Consented launcher events | Exactly startup/crash/update/game_update/install/repair/launch/settings/telemetry/UI/login/adoption target families, INFO and above. Timestamp/sequence, scalar fields, local operation/phase correlation, start/finish/outcomes/durations/files/bytes; manifest hash/overlay IDs; versions/errors/reasons/status/counts. Raw panic text is local-only; forwarded panic uses closed location/thread classes. | [Exact targets/scrub](../engine/src/storage/game_telemetry/launcher_event.rs), [forwarder](../shell/src/telemetry.rs), [phase contract](launcher-telemetry.md), [applied content](../engine/src/storage/game_telemetry/install_event.rs), [panic](../shell/src/panic_hook.rs) |
| Adoption scan/comparison | Reads selected install entries, identities/attributes/sizes and bytes/hashes needed for safety/comparison/patch checks, including unrelated files inside the selected tree. Emits modes/stages/index outcomes/missing/changed/unsafe counts, bounded relative path samples, expected/actual hashes. Relative names can themselves contain personal text. Unexpected extras are excluded from index samples; refused unsafe-entry paths can be named. Not an upload of every game file's contents. | [Inventory](../engine/src/storage/adoption/inventory.rs), [unsafe survey](../engine/src/storage/adoption/survey.rs), [index samples](../engine/src/storage/adoption/index_compare.rs), [in-place checks](../engine/src/storage/adoption/in_place.rs) |
| Deferred install/patch events | Per-patch ID/verdict/reason and failed-file/hash evidence; client-patches injection outcome/version/fingerprint, claimed/delivered/dropped Black Market counters and reasons. Can queue at setup and ship with next opted-in game session. | [Install event](../engine/src/storage/game_telemetry/install_event.rs), [result builder](../../../launcher-core/src/telemetry/install_result.rs), [patch watcher](../engine/src/storage/game_telemetry/patch_events.rs) |
| Manual debug-log lines | Newest up to 20,000 launcher records plus 20,000 game records per press, from saved byte offsets. launcher.*.log; top-level game-binaries sgwdebuglog*; recursive sessions/**/*.log. File name/relative path, physical line, optional timestamp/level/target, message/truncation. Messages capped at 2,048 bytes after scrub. Chat and character names are deliberately not content-filtered. | [Launcher reader](../engine/src/storage/log_upload/lines.rs), [game reader](../engine/src/storage/log_upload/game_lines.rs), [wire/caps](../engine/src/storage/log_upload/ingest.rs), [discovery](../../../launcher-core/src/logs.rs) |

Only the fixed summary payload lacks persistent sender identity. Rich event
telemetry/manual logs are not anonymous. Any HTTP connection exposes its source
address to receiving network infrastructure, including summary requests.

## Game diagnostics collectors

The following are code capabilities, not evidence that every hook activated.
Executable fingerprints, platform/Wine behavior, hook/capture overrides, game
state, sampling, quotas and drops change the actual scope.

| Family | Data read/emitted | Source |
|---|---|---|
| Boot/session/hook identity | Install UUID, hostname-derived machine ID, session UUID; launcher/DLL version/branch/revision/tags/flavor; game/helper PIDs, exit status/time; game EXE and DLL paths; hook availability, expected/observed instruction bytes/addresses, IAT/vtable/protection/install outcomes. | [Desktop identity/mint](../engine/src/storage/game_telemetry/session.rs), [DLL attach](../../../client-telemetry/src/boot.rs), [DLL session fields](../../../client-telemetry/src/session.rs), [fingerprint](../../../client-telemetry/src/fingerprint/mod.rs) |
| Live game-log tail | New complete sgwdebuglog* lines under binaries, and .log **or .txt** recursively under sessions. Source-file label, timestamp/sequence, level/category, optional packet number, unfiltered text. Existing files start at end; new session files at beginning. Can contain chat/names, commands, paths, addresses or sensitive text from game/other components. | [Session loop](../engine/src/storage/game_telemetry/session_loop/mod.rs), [actual selector/parser](../../../launcher-core/src/telemetry/runner.rs), [tailer](../../../launcher-core/src/telemetry/tail.rs) |
| Automatic exit bundle | Raw ZIP of selected top-level sgwdebuglog* and sessions/**/*.log modified since session start. Whole selected file bytes, not just new lines, with relative filenames/timestamps. Metadata: session/install/machine IDs, launcher version, session start/end, event/drop counts, ZIP hash/size. No manual line scrub on raw ZIP. Server expands/replays bounded lines. | [Bundle/metadata](../../../launcher-core/src/telemetry/bundle.rs), [ZIP selector](../../../launcher-core/src/logs.rs), [server expansion](../../../admin-api/src/routes/telemetry/bundle_unzip.rs) |
| Engine/UI/Lua logs | BigWorld messages/unknown-entity notices, UE3 categories/text/assertions/fatal errors, log4cxx logger/file/line/text, OutputDebugString, CEGUI messages, Lua call/error/status/source/line and Debug:log/warn/error text. Game/Lua-produced stack traces can be text. No comprehensive chat/name/secret exclusion. | [Sink family](../../../client-telemetry/src/hooks/sinks/mod.rs), [CEGUI](../../../client-telemetry/src/hooks/cegui_log.rs), [Lua errors](../../../client-telemetry/src/hooks/iat_hooks/lua_error.rs), [Lua debug](../../../client-telemetry/src/hooks/inline_hooks/lua_debug_log.rs) |
| Frame/resource health | Tick samples/counts, hitches/last phase age, process working/private/peak memory, free virtual memory, frame percentiles; map/package/object/class names, streaming/loading/async archive durations/failures; rollup/repeat/drop/upload status/failure counts. | [Frame/memory](../../../client-telemetry/src/hooks/seams/frame_health.rs), [ticks](../../../client-telemetry/src/hooks/inline_hooks/engine_frame.rs), [loading](../../../client-telemetry/src/hooks/inline_hooks/engine_loading.rs), [objects](../../../client-telemetry/src/hooks/seams/objects.rs), [governor](../../../client-telemetry/src/governor/mod.rs) |
| Entities/world/sequences | Entity ID/type and create/enter/leave/destroy, appearance scheduling, world/cache/deferred membership/queue counts, message delivery/drop/replay, state flags, CME registry event names/catalog, animation/notify, sequence IDs/timing/drop reasons. Actor name/class/position and cinematics start/stop/input flags/timeline position/length. | [Entity builders](../../../client-telemetry/src/hooks/entity_trace/mod.rs), [lifecycle](../../../client-telemetry/src/hooks/inline_hooks/entity_lifecycle.rs), [messages](../../../client-telemetry/src/hooks/inline_hooks/entity_messages.rs), [flags](../../../client-telemetry/src/hooks/inline_hooks/state_flags.rs), [CME](../../../client-telemetry/src/hooks/cme_catalog.rs), [actors](../../../client-telemetry/src/hooks/seams/actors.rs), [cinematics](../../../client-telemetry/src/hooks/seams/matinee.rs), [animation](../../../client-telemetry/src/hooks/inline_hooks/anim_notify.rs) |
| Ability/combat/UI | Press source/route, attempted sends/drop reason, allowlisted outbound arguments, decoded inbound ability/effect/sequence/cooldown/stats/known-abilities/feedback messages, target/cast IDs, applied UI results and sequence/timing joins. Ability decoder excludes ordinary chat channels; other logs do not share that exclusion. | [Ability family](../../../client-telemetry/src/hooks/ability_trace/mod.rs), [receive field table](../../../client-telemetry/src/hooks/ability_trace/recv_methods.rs), [decoder](../../../client-telemetry/src/hooks/ability_trace/recv.rs), [application hooks](../../../client-telemetry/src/hooks/inline_hooks/ability_apply/mod.rs), [outgoing route](../../../client-telemetry/src/hooks/inline_hooks/net_out.rs) |
| Navigation/input/game camera | Controller mode/button bits, left/right mouse edges/held duration, drag offsets/input axes, hover/selection/UI ownership; pawn position/orientation/movement/attachment transitions; virtual camera/pawn pose, zoom, yaw/pitch offsets/flags at interact/useItem and sequence joins. Console hook emits bare invocation, not decoded command. Not a system-wide keystroke transcript. | [Navigation](../../../client-telemetry/src/hooks/nav/mod.rs), [buttons](../../../client-telemetry/src/hooks/nav/buttons.rs), [pose](../../../client-telemetry/src/hooks/nav/pose.rs), [camera](../../../client-telemetry/src/hooks/camera/emit.rs), [console](../../../client-telemetry/src/hooks/inline_hooks/console_command.rs) |
| Protocol/network | Outgoing method/route/message/entity IDs; inbound method/property IDs/lengths; receive flags/acks/sequences/window/gaps/fragments/bundle parsing, socket/errors, remote IPv4 peer address and port, datagram lengths/fingerprints; bounded raw byte samples at parse faults (up to 24 bytes at failing message). Not a guarantee that no payload bytes leave. recvfrom observes the game's process-wide Winsock import, not system-wide traffic. | [Outgoing](../../../client-telemetry/src/hooks/inline_hooks/net_out.rs), [receive](../../../client-telemetry/src/hooks/mercury_recv/mod.rs), [socket](../../../client-telemetry/src/hooks/iat_hooks/recvfrom.rs), [wire](../../../client-telemetry/src/hooks/mercury_recv/wire.rs), [fault bytes](../../../client-telemetry/src/hooks/inline_hooks/mercury_unpack_fault.rs) |
| Cooked cache/file I/O | PAK/category/entry names, versions read/set/held, metadata extraction/stream/CRT results, file-open paths/errors/access/disposition/existence, cache writes/resync evidence. A path opened by the game need not be inside install root. Not an upload of every opened file's contents. | [Cache family](../../../client-telemetry/src/hooks/inline_hooks/cooked_cache/mod.rs), [writes](../../../client-telemetry/src/hooks/inline_hooks/cooked_cache/write.rs), [file opens](../../../client-telemetry/src/hooks/seams/file_io.rs) |
| Graphics/media/physics | D3D9 adapter index, resolution/format/windowed/buffers/multisample/interval/refresh, HRESULT/device-loss/reset timing; Bink movie name/open/close/tick; FMOD sound-event name/start/stop/result; PhysX code/message/source-file/line/assert. Metadata/events, not microphone audio or video-frame capture. | [D3D9](../../../client-telemetry/src/hooks/seams/d3d9.rs), [Bink](../../../client-telemetry/src/hooks/seams/bink.rs), [FMOD](../../../client-telemetry/src/hooks/seams/fmod.rs), [PhysX](../../../client-telemetry/src/hooks/seams/physx.rs) |
| OS/exceptions | Thread stack-size/flags, library names/paths, sampled foreground-window query occurrence (not other window titles), first-chance exception code/address/module/RVA/thread/access-kind/target/flags. First-chance exception does not establish fatal crash. | [OS imports](../../../client-telemetry/src/hooks/iat_hooks/os_detours.rs), [exception fields](../../../client-telemetry/src/hooks/sinks/exceptions.rs) |

### Important selector difference

Manual uploads and automatic session ZIPs use `collect_log_inputs`, omitting
current-session.json, .dmp and *-keys.txt by selecting session .log files only.
**Live tail also accepts session .txt, with no special *-keys.txt exclusion.**
Those lines become ordinary ClientLog messages, not protected KeyDump events.
Therefore no blanket key-file exclusion is established for live streaming.
The separate KeyDump wire variant's DEBUG-only/OTLP-off server target does not
protect secret text already classified as an ordinary log. Sources:
[selector](../../../launcher-core/src/logs.rs), [live predicate/conversion](../../../launcher-core/src/telemetry/runner.rs), [server replay](../../../admin-api/src/routes/telemetry/replay.rs).

## Identity and destination

| Layer | Behavior | Source |
|---|---|---|
| Stored identity | Random install UUID on first game opt-in, retained after opt-out. machine_id is SHA256(hostname), first 16 hex characters, not a random machine UUID. Raw hostname is not this minted field; hash is persistent and potentially guessable. Manual upload reuses stored identity or mints temporary unsaved identity when none exists. | [Identity](../engine/src/storage/game_telemetry/mod.rs), [manual target](../engine/src/storage/game_telemetry/launcher_session.rs) |
| Session mint/marker | First login server /api/auth/dev-session receives install/machine IDs, branch/revision/launcher version/tags. Reply gives session UUID/bearer/expiry/cadence/endpoint. Desktop mint currently has no account-name field. Session marker beside game contains identity and bearer/upload configuration for DLL. | [Desktop mint](../engine/src/storage/game_telemetry/session.rs), [DLL marker reader](../../../client-telemetry/src/session.rs) |
| Account/character joins | Server protocol supports optional account label; desktop currently does not send it. Server can still join active telemetry to world entry using machine/address correlation and enrich entity IDs with character/NPC/ability/item names. Game logs can independently include chat/names/inventory/trade/mission data. Not anonymous or free of character linkage. | [Mint semantics](../../../admin-api/src/routes/dev_session/handlers.rs), [correlation contract](../../../../docs/architecture/dev-session-telemetry.md), [entity labels](../../../admin-api/src/routes/telemetry/entity_labels.rs), [name lifting](../../../admin-api/src/routes/telemetry/replay_native.rs) |
| Summaries | Resolved once at launcher startup from the first effective login server (adopted list, saved list or defaults). The destination is that server's HTTPS origin; for an HTTP login server on a DNS name it is the same name over HTTPS on 443. Plain HTTP is never used, and IPs, localhost, `*.localhost`, single-label names and credentials get no endpoint. Later entries are not searched; a first HTTP/second HTTPS list sends no summary. Current fallback is unset. Changes apply next launcher start. Payload has no token/cookie/persistent sender ID. HTTP accepted only for explicit loopback fixtures. | [Endpoint](../engine/src/storage/launcher_summary/endpoint.rs), [exchange](../engine/src/storage/launcher_summary/exchange.rs) |
| Rich events/manual logs | Minted telemetry endpoint; remote server advertising loopback is normalized. Native policy allows HTTPS, or HTTP only to loopback/configured HTTP login-server exact host/port. **Not unconditionally HTTPS-only.** Bearer on chunks/bundles; DLL adds PID/send-clock headers. | [Policy](../../../launcher-core/src/telemetry/endpoint.rs), [normalization](../engine/src/storage/game_telemetry/endpoint_sanity.rs), [sender](../engine/src/storage/game_telemetry/launcher_session.rs), [DLL POST](../../../client-telemetry/src/uploader_post.rs) |
| Receiver/infrastructure | Connection address used for in-memory quotas/correlation; trusted loopback proxy header can supply caller address. Current ingest rows omit the receiving HTTP caller's connection address; this does not exclude addresses within game events/logs, including the remote game socket peer. Proxies/CDNs/hosting still see connection metadata; independent access logs/retention are outside payload guarantees. Ingest replays via server logging/OTLP into SigNoz when configured. | [Admission](../../../admin-api/src/routes/telemetry/upload_gate.rs), [summary handler](../../../admin-api/src/routes/telemetry/launcher_summary/handlers.rs), [session route](../../../admin-api/src/routes/dev_session/handlers.rs), [no-address policy](../../../../docs/operations/telemetry.md#no-address-in-any-log) |

Normal game-server processing/logs are independent of launcher consent. Explicit
`.bug <note>` records the free-text note, caller/selected target, mission/region,
nearby named entities and ability-state snapshots. Server movement/combat,
missions/inventory/network records can provide context even without the DLL.
Sources: [bookmark snapshot](../../../cell-console/src/cell/console/bookmark.rs),
[server target catalog](../../../../docs/architecture/observability-target-catalog.md).

## Storage, limits and retention

| Store/limit | Behavior | Source |
|---|---|---|
| Summary queue | launcher-summaries.json; 64 entries; 24-hour expiry; up to 32 summaries/batch; bounded body. Local journal tracking is separate from exported fields. Grant empties old queue. | [Queue](../engine/src/storage/launcher_summary/queue.rs), [consent](../engine/src/storage/launcher_summary/consent.rs) |
| Launcher forwarding | In-memory 512-record channel, batches up to 64, no disk retry queue. Default receiver 300 rows/launcher session and 3,000/minute globally, configurable. | [Forwarder](../shell/src/telemetry.rs), [budget](../../../admin-api/src/routes/telemetry/launcher_budget.rs) |
| Game-event backlog | Plain JSONL queue/inflight files, 100 MiB drop-oldest cap; survives restart while opted in, can ship under later session token. Retries can duplicate rows. Opt-out discards unsent; deletion failures logged. No general age expiry specified. | [Transport](../engine/src/storage/game_telemetry/transport/mod.rs), [queue consent/recovery](../engine/src/storage/game_telemetry/transport/queue.rs), [shared queue](../../../launcher-core/src/telemetry/queue.rs) |
| DLL queue/governor | 4,096 ring entries/in-memory retry batch; overflow loss; sampling, family rate budgets, repeated-event collapse/rollups. Session/environment firehose/raw/unfilter switches broaden capture; unfilter also changes game's logging thresholds. Bounded exit flush can lose tail. | [Ring](../../../client-telemetry/src/queue.rs), [governor](../../../client-telemetry/src/governor/mod.rs), [switches](../../../client-telemetry/src/capture.rs), [shutdown](../../../client-telemetry/src/shutdown.rs) |
| DLL local log | cimmeria-client-telemetry.log beside SGW.exe, rewritten each launch, up to 2,000 boot/diagnostic lines; also OutputDebugString. Separate from rotating launcher log. | [DLL logger](../../../client-telemetry/src/log.rs) |
| Manual upload cursors | uploaded-log-offsets.json: byte offsets and game-file head/tail fingerprints, up to 2,048 files. Repeated presses normally send new complete records; rewritten/truncated logs can start again. Upload does not delete local logs. | [Offsets](../engine/src/storage/log_upload/offsets.rs), [game reader](../engine/src/storage/log_upload/game_lines.rs) |
| Manual upload budgets | Sender 40,000 rows/press, 400 chunks, 2,048-byte messages, 3.5-second pacing. Receiver defaults: 40,000 kept/session, 80,000/install/day, 400,000 received/address/day, 40,000 received/minute globally; configurable. | [Sender](../engine/src/storage/log_upload/ingest.rs), [receiver](../../../admin-api/src/routes/telemetry/launcher_log_budget.rs) |
| Stream/bundle receiver | Default 30,000 events/minute/session plus 3,000 priority; bundle request 32 MiB, expanded 64 MiB, 1,024 replayed entries/250,000 lines with separate hard ZIP-entry guard. Truncation bounds resources, not sensitive content. | [Session budget](../../../admin-api/src/routes/telemetry/session_budget.rs), [caps](../../../admin-api/src/routes/telemetry/mod.rs), [field caps](../../../admin-api/src/routes/telemetry/field_caps.rs) |
| Remote retention | Server sinks/SigNoz/ClickHouse configured by operator. Guidance proposes 30-day logs/14-day traces/90-day metrics; actual TTL, backups and other sinks were not inspected and are not controlled by launcher toggles. | [Retention guidance](../../../../docs/operations/signoz-deployment.md#retention), [operator runbook](../../../../docs/operations/telemetry.md) |

## Redaction and verified exclusions

- **Summary schema:** closed fields exclude arbitrary text, names, paths and
  persistent sender identifiers. This does not apply to rich bags/logs/ZIPs.
- **Forwarded launcher events:** sender replaces recognized home/username
  segments, removes recognized remaining absolute paths/IP literals and bounds
  strings. Receiver restricts targets/scalars/keys and suspicious address/path/
  account-like words, with a narrow public manifest-hash exception. Relative
  names/unrecognized free text can remain. [Sender](../engine/src/storage/game_telemetry/launcher_event.rs), [IP guard](../engine/src/storage/game_telemetry/ip_scrub.rs), [receiver](../../../admin-api/src/routes/telemetry/launcher_rows.rs).
- **Manual log lines:** sender/receiver apply pattern-based home/path/IP/long-token
  scrubbing. Ordinary chat/character names remain. Recognition cannot guarantee
  removal of every personal value, credential or secret in text.
  [Game reader](../engine/src/storage/log_upload/game_lines.rs), [receiver scrub](../../../admin-api/src/routes/telemetry/launcher_log_rows.rs).
- **DLL/live logs/raw ZIP:** do not share the manual line-scrub guarantee.
  Generic server field caps truncate rather than anonymize. Paths, text and
  bounded protocol samples can survive. [DLL bags](../../../client-telemetry/src/events.rs), [live parser](../../../launcher-core/src/telemetry/runner.rs), [ZIP](../../../launcher-core/src/telemetry/bundle.rs), [field caps](../../../admin-api/src/routes/telemetry/field_caps.rs).
- **Camera/audio/graphics:** virtual game camera and pawn pose are not webcam
  capture; FMOD observes game sound metadata, not microphone recording; D3D9
  observes device parameters/state, not screenshots/video frames. The audited
  collectors do not add a browser-history/clipboard/document crawler or
  system-wide keyboard listener. This is not a sandbox guarantee: game input,
  file calls and free-text logs can still describe personal activity/data.
  [Camera](../../../client-telemetry/src/hooks/camera/emit.rs), [FMOD](../../../client-telemetry/src/hooks/seams/fmod.rs), [D3D9](../../../client-telemetry/src/hooks/seams/d3d9.rs), [controller](../../../client-telemetry/src/hooks/nav/controller.rs), [hook registration](../../../client-telemetry/src/hooks.rs).
- **Player versus lab:** releases build default player DLL and refuse the
  lab-bridge marker at staging/release/native admission. Separate lab feature
  can provide authenticated memory read/write, Lua evaluation, native calls,
  injected input and crash minidump/marker capture. Those capabilities are not
  in distributed player DLL. Player exception rows are not full-memory dumps.
  [Feature gate](../../../client-telemetry/src/lib.rs), [release](../../../../tools/launcher-release/build-desktop.sh), [staging](../tools/stage-helper.py), [native resources](../engine/src/storage/launch/resource.rs), [lab crash](../../../client-telemetry/src/bridge/crash.rs).

No claim is made that the original game's own logs/crash handling, user mods,
external tools or separately built lab binaries obey these player-collector
boundaries. No complete hardware inventory was found in these collectors;
actual fields include OS/architecture, memory health and graphics adapter/mode,
not a claim that all environmental information is absent.

## Ready-to-use disclosure coverage

The expanded privacy section should include all paragraphs below and the exact
fixed-schema field lists. Keep the full disclosure available offline. A short
summary plus a link alone does not cover the data categories above.

**Your choices.** Launcher diagnostics and game diagnostics are optional and off
by default. Client patches are a separate gameplay setting. Local launcher and
game logs can exist while uploads are off. Normal login, downloads, updates and
game-server operational logs are independent of these switches.

**Launcher diagnostics.** Sends fixed setup/play attempt outcomes, timings,
retries/failure codes, launcher version, OS/architecture, random event/attempt
IDs and queue-drop totals. Its payload has no name, path, chat, token or
persistent machine/install ID. Network services still see connection metadata.

**Game diagnostics.** Adds detailed launcher/adoption/install/launch events,
content/patch/fingerprint evidence and upload health; persistent install ID,
hostname-derived machine ID, session/build/process identifiers; newly written
game-log lines and an automatic session log ZIP. It can be linked to your
character and game-server activity and is not anonymous.

**Inside the game.** Can report maps/positions/virtual camera, game navigation
and input state, entities/abilities/effects/stats, UI/sequences/cinematics,
object/cache/file names/versions, protocol IDs/timing/limited fault bytes,
graphics/sound/physics events, frame/memory health, errors/exceptions and log
text. Free-text logs can include chat, names, inventory/trade/mission data,
commands, paths, addresses and other sensitive values. Some paths are raw or
only length-bounded, rather than comprehensively redacted.

**Log-file scope.** Live streaming can read .log and .txt beneath game sessions,
including key-like text files; that folder is not a safe place for secrets.
Automatic ZIP and manual game-log selection use .log files and SGWDebugLog,
excluding the session JSON/key-text/dump filenames by that selector. Raw exit
ZIPs contain whole selected files and do not use manual line scrubbing.

**Manual upload.** Upload debug logs sends existing launcher and selected-game
logs when pressed even with both toggles off, possibly including earlier
sessions. Pattern-based path/address/token scrubbing preserves ordinary chat
and names and cannot guarantee removal of every sensitive value.

**Destination and retention.** Session diagnostics use your configured login
server's telemetry service. HTTPS is supported; native policy also permits HTTP
to loopback/configured HTTP login origins. Summaries consider only the first effective login server at launcher startup, need HTTPS and currently
have no fallback destination. Local queues/logs and server logging/SigNoz can
retain data. Server retention is configured by the operator, not the launcher.

**Stopping diagnostics.** Turning a toggle off closes its future launcher-send
gate and clears its unsent queue; already received/in-flight data may remain.
Exit the running game to stop an already attached telemetry DLL. The checkbox
changes DLL injection for the next Play. Received server records, local logs
and retained install identity are not erased by turning it off.

**What these collectors do not do.** They do not record a webcam/microphone or
screenshot/video feed, crawl browser history/clipboard, or capture a system-wide
keystroke transcript. They still observe the game input/camera/file/log data
listed above. The distributed player DLL has no lab remote-control bridge or
lab crash-dump handler.

Avoid blanket “anonymous diagnostics”, “no chat”, “no paths”, “no key files”,
“nothing outside the game is read”, “always encrypted HTTPS”, or “turning off
immediately stops a running DLL”. None is an audited global guarantee.
