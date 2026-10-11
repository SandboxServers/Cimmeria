# Rust Gameserver Dev Memory

One line per topic file; the detail lives in the file. Keep hooks short (this index must stay under ~17 KB).

## Build environment

- [build-environment.md](build-environment.md) — rust-lld override obsolete; worktrees need the `external/` junction; PowerShell `bash` may be WSL.
- [stale-branch-clippy-toolchain-drift.md](stale-branch-clippy-toolchain-drift.md) — CI clippy floats to current stable; idle branches fail on new lints. Update the branch first.
- [lane-sh-masks-cargo-exit-code.md](lane-sh-masks-cargo-exit-code.md) — `lane.sh` / `live-db-test.sh` the old `%TEMP%` lane exited 0 on a failed cargo.
- [mutation-restore-mtime-trap.md](mutation-restore-mtime-trap.md) — restoring from a backup copy leaves an old mtime; cargo keeps the mutated build. Touch restored files.
- [dependency-dedupe-blockers.md](dependency-dedupe-blockers.md) — duplicate dep versions pinned upstream (sqlx, axum ws, reqwest, rmcp); machete false positives.
- [services-split-extraction-traps.md](services-split-extraction-traps.md) — extracting a crate from services: allowlist edges, unreachable_pub, privacy errors in phases.
- [launcher-core-path-include-extraction](launcher-core-path-include-extraction.md) — #[path] to crate: cfg(test) items need a `test-fixtures` feature; telemetry moved whole; ignored tests hide from nextest list.
- [desktop-engine-tests-need-cargo-test](desktop-engine-tests-need-cargo-test.md) — nextest cannot list the desktop engine (harness=false PASS lines); use `cargo test` via the lane.

## Working environment

- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — other sessions on the repo: work in `.claude/worktrees/<slug>/`, junction `external/`.
- [shared-scratchpad-name-collisions.md](shared-scratchpad-name-collisions.md) — sibling workers share the scratchpad; prefix script names with the packet id.
- [stacked-branch-rebase-traps.md](stacked-branch-rebase-traps.md) — a handed-down base sha may not be an ancestor.
- [shepherd-merge-traps.md](shepherd-merge-traps.md) — clean merges of main can duplicate a doc section or index line; const-slice ptr::eq fails on i686.
- [rebase-keep-both-regex-drops-braces.md](rebase-keep-both-regex-drops-braces.md) — scripted "keep both" conflict fixes can drop a `}` mid-hunk; inspect + `cargo check` before `--continue`.
- [resuming-a-dead-workers-wip.md](resuming-a-dead-workers-wip.md) — a `wip(...) unverified` commit may not compile; its tests encode the starting design; port hunks by hand.
- [crate-split-extraction-traps.md](crate-split-extraction-traps.md) — moving code out of services: `pub(crate)` turns dead, `unreachable_pub` hits pub fields.
- [pre-split-branch-port-traps.md](pre-split-branch-port-traps.md) — porting a June branch onto the split: pull cell-visible types into wire (no sqlx there).

## Tooling quirks

- [tooling-quirks-index](tooling-quirks-index.md) — sub-index: rustfmt/clippy/sqlx traps, worktree shell limits, client file case, patchset/ring rigs, Python UTF-8.
- [launcher-native-owner-handle.md](launcher-native-owner-handle.md) — native launch reads installation identity through its locked handle; Windows revert proof must run natively.

- [patchset-supersede-and-restore-to-stock](patchset-supersede-and-restore-to-stock.md) — apply skips target==result before source check; a delta back to an LZO stock map ships CME bytes.
- [ring-rig-clone-to-new-map](ring-rig-clone-to-new-map.md) — patch 010: region 3 rig roots, donor pinned to 007's result, central chunk for streaming, floor heights.
- [client-file-case-and-stock-listing](client-file-case-and-stock-listing.md) — game UI lookups are case-sensitive on Windows (eula.lua = no login); rename keeps target spelling; DATA.INF = stock names.
- [offline-client-event-trace-and-udp-port-trap](offline-client-event-trace-and-udp-port-trap.md) — no Ghidra: client Lua + PE bytes + RTTI name the CME event a handler raises.
- [mail-escrow-lock-order-and-proof-traps](mail-escrow-lock-order-and-proof-traps.md) — inventory lock order is advisory → item row → sgw_player.
- [bash-heredoc-backslash-and-metric-tests](bash-heredoc-backslash-and-metric-tests.md) — a doubled backslash in a heredoc arrives as one: use Edit for backslash text; metric tests use a per-test world label.
- [python-write-mangles-utf8-and-crlf](python-write-mangles-utf8-and-crlf.md) — `write_text` encodes cp1252: use bytes + restore CRLF.
- [i686-test-exe-uac-installer-detection](i686-test-exe-uac-installer-detection.md) — a 32-bit test exe named `*patch*` fails with os error 740 under UAC.
- [rustfmt-trailing-line-comment-quirk](rustfmt-trailing-line-comment-quirk.md) — rustfmt pulls a standalone comment into the previous line's trailing column.
- [rustfmt-skips-sigil-tracing-macros](rustfmt-skips-sigil-tracing-macros.md) — a `%`/`?` field makes rustfmt skip the whole tracing call; reflow scripted inserts by hand.
- [rustfmt-reorders-mod-declarations](rustfmt-reorders-mod-declarations.md) — `reorder_modules` sorts `mod` lines, so "append at the end" never survives `cargo fmt`.
- [clippy-items-after-test-module](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be last.
- [tooling-filter-and-path-traps](tooling-filter-and-path-traps.md) — `live-db-test.sh` takes positional substrings, not filtersets.
- [sqlx-dynamic-sql-string](sqlx-dynamic-sql-string.md) — `sqlx::query` needs `&'static str`.
- [sqlx-chain-id-is-i32-vacuous-guards](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is i32; a wrong decode type hides inside "no rows" guards.
- [gitignore-swallows-new-dirs](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules hide a new `foo/mod.rs`.
- [worktree-shell-and-external-binary-tests](worktree-shell-and-external-binary-tests.md) — worktree Bash refuses `env VAR=x cmd`, heredoc appends, chained commits.
- [powershell-git-commit-stdin-trap](powershell-git-commit-stdin-trap.md) — `git commit -F - @'..'@` takes the text as a pathspec; use `-F <file>` or `-m @'..'@`.
- [powershell-strict-mode-empty-index](powershell-strict-mode-empty-index.md) — lab CLI strict mode: `@(..)[0]` on empty throws, use Select-Object -First 1; test git repos eat fixture files.
- [powershell-ctrl-c-test-pipeline-stopped](powershell-ctrl-c-test-pipeline-stopped.md) — Ctrl+C = PipelineStoppedException: no catch sees it, only finally; test it from a stub in a child pwsh.
- [powershell-dotnet-relative-path-trap](powershell-dotnet-relative-path-trap.md) — `[IO.File]` relative paths ignore `cd` and hit the main checkout; use absolute paths.

## Wire format

- [error-strings-served-from-pak-not-seed.md](error-strings-served-from-pak-not-seed.md) — client error text = served ErrorStrings.pak (216), not the seed; patch Text or add the entry in attribute_patches.
- [client-error-codes-and-ammo-chatter.md](client-error-codes-and-ammo-chatter.md) — create-failed codes are served ErrorStrings.pak ids, not error_texts; check the PAK text; "out of ammo" is the client reacting to AmmoSlotN = 0.
- [wire-names-generated-tables.md](wire-names-generated-tables.md) — `cimmeria_wire::names` is generated from defs (regen cmd), doc-checked; naming rules for unknown entity types.
- [gm-tail-dispatch-doc-filename-trap.md](gm-tail-dispatch-doc-filename-trap.md) — client- vs cell-method dispatch tables are different files; GM tail is `109 + K`.
- [method-idx-duplicate-table-drift.md](method-idx-duplicate-table-drift.md) — `cell/client_methods/` is authoritative; `mercury::method_idx` is a drifted partial copy; `def_conformance` guards both (#801).
- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns bytes consumed: `offset += n`, never `offset = n`.
- [dialog-set-bind-carries-no-dialog-id.md](dialog-set-bind-carries-no-dialog-id.md) — a bind pushes only `InteractionType`; method 104 is never emitted.
- [cooked-pak-and-dialog-override-traps.md](cooked-pak-and-dialog-override-traps.md) — `data/cache/*.pak` IS in git; fail-closed patcher vs seed linter diverge silently.
- [cooked-override-client-cache-persistence.md](cooked-override-client-cache-persistence.md) — the client caches pushed overrides on disk; removed ids are never evicted.
- [gm-feedback-cell-base.md](gm-feedback-cell-base.md) — four method-28 serializers.
- [witness-entity-method-dual-fn.md](witness-entity-method-dual-fn.md) — two `witness_entity_method` fns; idbase 61 player / 62 NPC matters for index >= 61.
- [request-ammo-change-instance-id.md](request-ammo-change-instance-id.md) — method 42 `ItemId` is the weapon instance id; match `instance_id`, whitelist by the slot's design id (#534).
- [cell-entity-direction-semantics.md](cell-entity-direction-semantics.md) — `direction` is `[pitch, yaw, roll]` radians for all entities; `[i8; 3]` param zeroes facing.
- [login-handshake-acks-and-pre-channel-sends.md](login-handshake-acks-and-pre-channel-sends.md) — seqs 1/2 in the TX window (#842); clients ack them in 3 of 5 captures; testing a pre-channel drop.
- [game-clock-and-timer-expiry-tests.md](game-clock-and-timer-expiry-tests.md) — client clock is ticks / hertz; expiries = `game_time_secs() + d`.
- [cooked-data-full-resync.md](cooked-data-full-resync.md) — #840: paced full resync (RequiredUpdates=0), misses jump the stream, Play held for 6 no-miss-path categories.
- [cooked-item-additions-shape.md](cooked-item-additions-shape.md) — real shipped COOKED_ITEM shape (not alphabetical); new ids via ITEM_ADDITIONS; AmmoType_Icons = EAmmoType labels.
- [client-gm-slash-xml-and-dhd-address-resolution.md](client-gm-slash-xml-and-dhd-address-resolution.md) — /gm* need InternalSlashCommands.xml; property 7 = command mask; updateStargateAddress resolves async (open DHD shows "Unknown").
- [ability-telemetry-coverage-gate.md](ability-telemetry-coverage-gate.md) — AB-C7: script set + four scanned code tables + client declaration; AB-C6 timing tables are process-global.
- [entity-tint-packed-colours.md](entity-tint-packed-colours.md) — onEntityTint args are packed 0xRRGGBB__ colours; seed bigints = signed i32, send low 32 bits; opt-in send_tint.

## Lab supervisor

- [client-lua-cegui-global-subscribe](client-lua-cegui-global-subscribe.md) — client Lua `window:subscribeGlobal(ns, ev, 'Fn')`; no unsubscribe, so once per Lua state; args (this, eventWindow, ...).
- [lab-native-cegui-input-and-levels](lab-native-cegui-input-and-levels.md) — cursor/drag via native CEGUI injectors; `cegui_fake.rs` test seam; new native-level words need a `from_reported` row.
- [stacked-pr-ship-title-and-ab-lab-tools.md](stacked-pr-ship-title-and-ab-lab-tools.md) — ship.py mistitles stacked PRs; AB-T5 snapshot builder, lab dummy = AI-skip extension, cooldown clear is type 2.
- [lab-uat-runner-in-process-tool-calls.md](lab-uat-runner-in-process-tool-calls.md) — call rmcp tools by name in-process via the RequestContext extractor; chat marks before the action; capability table.
- [lab-daemon-lease-gate.md](lab-daemon-lease-gate.md) — lease gate is hand-written call_tool; new tools need a policy line; runner touches its own lease; per-test lease books.
- [ability-uat-staging-limits.md](ability-uat-staging-limits.md) — warmup and cleanse rows need .dummy caster (#1188); 1462/4306/2827 are effect ids; one graded press per row.
- [uat-runner-grade-neutral-capture.md](uat-runner-grade-neutral-capture.md) — reasons name step indices; opt-in capture records go to the row at the end, re-indexed (GD-06).
- [uat-run-dir-sibling-json-trap](uat-run-dir-sibling-json-trap.md) — `RunDir::rows` parses every json beside the row JSON (skip `.snapshot.json`); attest re-grade must re-apply post-grade rules.
- [lab-mcp-raw-args-and-live-db-crate](lab-mcp-raw-args-and-live-db-crate.md) — rmcp arg-parse errors skip audit (raw Value + delegated schema); first live-DB test in a crate: dev-dep, shim last, test-live-db lists.
- [lab-calibrate-sweep-test-seams](lab-calibrate-sweep-test-seams.md) — runner fakes are private: write a geometry fake; paused time makes each stand cost 250 ms.
- [lab-flow-first-live-run-findings.md](lab-flow-first-live-run-findings.md) — busy client after Create = bridge `dispatch timeout`; only SelfStatusWin is the HUD; outcome reads `ability.*`.

## Launcher

- [launcher-state-key-and-egui-wake.md](launcher-state-key-and-egui-wake.md) — sgw_game patches are recorded as `<id>@sgw_game`; worker events must wake egui via EventSender.
- [launcher-extracted-mtimes-and-ue3-ini-version.md](launcher-extracted-mtimes-and-ue3-ini-version.md) — extracted files must keep archive DOS times or UE3 flags Default*.ini outdated; zip 1980 placeholder; MakeCAB fixture.
- [desktop-settings-panel-ipc-traps](desktop-settings-panel-ipc-traps.md) — a panel that invokes on mount breaks view.test.ts call pins and sync-throwing fakes; inspect on first snapshot.
- [launcher-self-update-handoff-traps.md](launcher-self-update-handoff-traps.md) — viewport Close waits for a frame; handoff releases instance_lock + process::exit; relaunch kills stuck old exe only.
- [launcher-preferences-schema-change-fanout](launcher-preferences-schema-change-fanout.md) — desktop Preferences change: strict TS decode breaks ~10 fixtures; migrate in serde try_from; adoption Plan digest goes through `digest_form`.
- [launcher-windows-adoption-path-traps](launcher-windows-adoption-path-traps.md) — Windows adoption: `<root>\game` invariant, verbatim-root joins, zip `enclosed_name` backslashes, Windows file identity, UAT binary path.
- [desktop-repair-update-terminal-hooks](desktop-repair-update-terminal-hooks.md) — Repair/Update `finish_failure` runs after every commit (dropped handoff); run-end hooks must check the op was open.
- [launcher-release-feed-traps](launcher-release-feed-traps.md) — release zip needs the exe at its root; key/sig base64 forms; Tauri CLI keys work; prove feeds with `examples/release_feed.rs`.

## Injected client DLLs

- [injected-client-dlls-index](injected-client-dlls-index.md) — sub-index: detours and unwind, CME handler ABI, lab bridge/watchdog, telemetry anchors, injector bitness.

## UE3 packages and navmesh

- [ue3-and-navmesh-index](ue3-and-navmesh-index.md) — sub-index: UE3 package decode, BSP/StaticMesh, NavBuilder OBJ, Recast/Detour, map placement, occluders.

## Seeds and content chains

- [seeds-and-content-chains-index](seeds-and-content-chains-index.md) — sub-index: template/cover/name seeds, chain conditions and edge triggers, dialog binds, inventory locks, pet and trainer seeds.

## Debug Area

- [debug-area-plaza-and-training-dummies.md](debug-area-plaza-and-training-dummies.md) — tags fire in any world; TrainingDummy mark; gm_ability_bulk; hub once-placed guards scoped to the hub.

## Base sessions

- [mail-expiry-and-notify-seams.md](mail-expiry-and-notify-seams.md) — every mail writer sets `expires_at`; `NOT quarantined` on every player path.
- [connected-map-view-over-parallel-index.md](connected-map-view-over-parallel-index.md) — online lookups: a view over `connected` + `listed_online`, not a parallel map.
- [trade-results-client-ignores-space-cash.md](trade-results-client-ignores-space-cash.md) — client trade window acts only on results 1/2; space/cash codes 3-6 show nothing, so send a feedback line.
- [tell-channel-and-ignore-copies.md](tell-channel-and-ignore-copies.md) — client /tell is byte 10 (CHAN_TELL since SS-C4); the Ignore list has 3 copies synced by one resync.
- [chat-channel-client-display.md](chat-channel-client-display.md) — server 8 opens a modal prompt, 9 is plain feedback, 7 shows nothing (nil ChannelMap).

## Cell systems

- [cell-systems-index](cell-systems-index.md) — sub-index: grants and loot, per-session state, abilities and effects, NPC AI, missions, pets, crafting, black market, duels, respawn, and the ammo campaign (#1026) notes.
- [adding-a-cimmeria-world-touchpoints](adding-a-cimmeria-world-touchpoints.md) — new world id = one ADDED_WORLDS row + seed/spaces data; nav/occ client-map fallback; bounds unused.

## Observability

- [named-telemetry-sweep-traps](named-telemetry-sweep-traps.md) — Rule 6 sweeps: chain/loot/event-set names exist in seed; keys holding the wrong ID; rescan without rebuild.
- [rule6-sweep-scanner-and-naming-traps](rule6-sweep-scanner-and-naming-traps.md) — list unpaired sites by zeroing baseline rows; `$level!` rows escape the scan; never name a deferred effect's invoker live.
- [discord-named-pairs-resolvers](discord-named-pairs-resolvers.md) — Discord Event objects are `Named` pairs (NT-10); base/cell/content resolvers; button text and minigame ids are unnameable.
- [observability-test-and-throttle-traps](observability-test-and-throttle-traps.md) — counters unobservable in tests.
- [rule6-name-pairing-traps](rule6-name-pairing-traps.md) — base names via known_names; NameBook borrow vs owned; rustfmt bails on >100-col macro lines.
- [ability-row-event-guard-and-cast-join](ability-row-event-guard-and-cast-join.md) — ability rows need `event` (source-scan guard); pre-mint rows lack cast_id; client_sent joins by payload.
- [discord-noise-and-teardown-race](discord-noise-and-teardown-race.md) — SIGNOZ_ONLY_EVENTS; logOff witness-send race is DEBUG via departed_witnesses; colo warns that are real faults.
- [rule6-name-pairing-sweep-techniques](rule6-name-pairing-sweep-techniques.md) — list unpaired sites by zeroing the baseline; lazy fields + `enabled!`; base NPC names only from EnteredAoI.
- [base-side-log-naming-and-lock-traps](base-side-log-naming-and-lock-traps.md) — base Rule 6 resolvers (session_identity), base can't name NPCs bar AoI name_id; match-scrutinee guard + name lookup = nested lock.
- [unpaired-id-sweep-workflow-traps](unpaired-id-sweep-workflow-traps.md) — NT sweep: per-site dump patch, lane quiet-mode log-dir race, B: <10 GB refusals, heredoc edits eat `\` continuations.
- [firehose-framework-crate-placement](firehose-framework-crate-placement.md) — sampler in observability, mercury_packet emitter in mercury; const targets; suppressed = drops before.
- [log-filter-parity-traps](log-filter-parity-traps.md) — `EnvFilter::new` drops a bad directive silently.
- [tracing-span-fields-not-on-log-records](tracing-span-fields-not-on-log-records.md) — span fields are not flattened onto OTLP log records.
- [client-telemetry-index-and-upload-traps](client-telemetry-index-and-upload-traps.md) — cimmeria-client routing; base-vs-chunk URL, blank `${VAR:-}` env; opted-in player launch mints the session before the game.
- [client-telemetry-governor-classify-table](client-telemetry-governor-classify-table.md) — every DLL event passes the governor; new targets default to Budgeted; must-keep rows need a server priority prefix too.
- [telemetry-ingest-naming-traps](telemetry-ingest-naming-traps.md) — replay: 32-field event cap, no router state (global cell sender), docs/ not in the image, lane log-dir race.

- [otel-log-redaction-wrap-provider](otel-log-redaction-wrap-provider.md) — redact OTLP log fields by wrapping the bridge's LoggerProvider (LogProcessor can't edit attrs); event names hold registry paths.
- [rule6-name-pairing-patterns](rule6-name-pairing-patterns.md) — tracing field exprs are lazy (inline names are free); book() guard in closures fails E0515.

## Testing patterns

- [testing-patterns-index](testing-patterns-index.md) — sub-index: nextest vs cargo test, revert proofs, live-DB races/ports, chain replay, encrypted test sessions, LogCapture.
- [tracing-capture-single-dispatcher-trap](tracing-capture-single-dispatcher-trap.md) — scoped tracing capture records nothing under `cargo test` unless a 2nd Dispatch lives; static async locks couple tests.
- [cap-guard-sized-from-the-constant](cap-guard-sized-from-the-constant.md) — a cap guard sized `CAP + 1` passes when the cap is loosened; use a literal size.
- [aoi-fixture-introducible-and-wire-ledger](aoi-fixture-introducible-and-wire-ledger.md) — account_id without archetype_id hides a test player from AoI; ability sends go through `wire_ledger` (AB-T4).
- [damage-apply-miss-gate-and-seeded-rolls](damage-apply-miss-gate-and-seeded-rolls.md) — since AB-06 a miss lands nothing; a literal effect_seq may roll a miss; use `seq_rolling`.
- [table-driven-live-db-refusal-cases](table-driven-live-db-refusal-cases.md) — one character per refusal case + collected failures, so each dropped check shows; rolled-back dry run as precheck.
- [player-cast-fixtures-need-a-mechanic](player-cast-fixtures-need-a-mechanic.md) — since AB-12 a player cast with no mechanic is refused; effectless fixtures need `seed_mechanic_effect`.
- [lab-runner-paused-time-poll-tests](lab-runner-paused-time-poll-tests.md) — lab runner poll/timeout tests use `start_paused = true`; deadlines must be `tokio::time::Instant`.

## Campaign judgment

- [legacy-command-parity-scoping-judgment](legacy-command-parity-scoping-judgment.md) — porting a legacy dot command: verify the reference files, don't port diverged enums.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — the egui-only dependabot PR is a no-op; the eframe PR carries the breakage.
- [training-points-cache-absolute-write.md](training-points-cache-absolute-write.md) — `handle_grant_xp` writes training_points absolutely from the session cache.
- [loot-seed-pins-and-grant-stack-cap.md](loot-seed-pins-and-grant-stack-cap.md) — new loot rows break exact pins on tables 3/7/8/9 (filter by NOT EXISTS); GrantItem does not cap at max stack.
- [admin-api-ingest-test-seams.md](admin-api-ingest-test-seams.md) — admin-api has no dev-deps: test ingest through injected `*_inner` fns and a local capture layer; statics and env are shared under `cargo test`.
