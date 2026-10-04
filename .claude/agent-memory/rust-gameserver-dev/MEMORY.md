# Rust Gameserver Dev Memory

One line per topic file; the detail lives in the file. Keep hooks short (this index must stay under ~17 KB).

## Build environment

- [build-environment.md](build-environment.md) — rust-lld override obsolete; worktrees need the `external/` junction.
- [stale-branch-clippy-toolchain-drift.md](stale-branch-clippy-toolchain-drift.md) — CI clippy floats to current stable; idle branches fail on new lints. Update the branch first.
- [lane-sh-masks-cargo-exit-code.md](lane-sh-masks-cargo-exit-code.md) — `lane.sh` / `live-db-test.sh` the old `%TEMP%` lane exited 0 on a failed cargo.
- [mutation-restore-mtime-trap.md](mutation-restore-mtime-trap.md) — restoring from a backup copy leaves an old mtime; cargo keeps the mutated build. Touch restored files.
- [dependency-dedupe-blockers.md](dependency-dedupe-blockers.md) — duplicate dep versions pinned upstream (sqlx, axum ws, reqwest, rmcp); machete false positives.
- [services-split-extraction-traps.md](services-split-extraction-traps.md) — extracting a crate from services: allowlist edges, unreachable_pub, privacy errors in phases.

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

- [patchset-supersede-and-restore-to-stock](patchset-supersede-and-restore-to-stock.md) — apply skips target==result before source check; a delta back to an LZO stock map ships CME bytes.
- [client-file-case-and-stock-listing](client-file-case-and-stock-listing.md) — game UI lookups are case-sensitive on Windows (eula.lua = no login); rename keeps target spelling; DATA.INF = stock names.
- [offline-client-event-trace-and-udp-port-trap](offline-client-event-trace-and-udp-port-trap.md) — no Ghidra: client Lua + PE bytes + RTTI name the CME event a handler raises.
- [mail-escrow-lock-order-and-proof-traps](mail-escrow-lock-order-and-proof-traps.md) — inventory lock order is advisory → item row → sgw_player.
- [bash-heredoc-backslash-and-metric-tests](bash-heredoc-backslash-and-metric-tests.md) — a doubled backslash in a heredoc arrives as one: use Edit for backslash text; metric tests use a per-test world label.
- [python-write-mangles-utf8-and-crlf](python-write-mangles-utf8-and-crlf.md) — `write_text` encodes cp1252: use bytes + restore CRLF.
- [i686-test-exe-uac-installer-detection](i686-test-exe-uac-installer-detection.md) — a 32-bit test exe named `*patch*` fails with os error 740 under UAC.
- [rustfmt-trailing-line-comment-quirk](rustfmt-trailing-line-comment-quirk.md) — rustfmt pulls a standalone comment into the previous line's trailing column.
- [rustfmt-reorders-mod-declarations](rustfmt-reorders-mod-declarations.md) — `reorder_modules` sorts `mod` lines, so "append at the end" never survives `cargo fmt`.
- [clippy-items-after-test-module](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be last.
- [tooling-filter-and-path-traps](tooling-filter-and-path-traps.md) — `live-db-test.sh` takes positional substrings, not filtersets.
- [sqlx-dynamic-sql-string](sqlx-dynamic-sql-string.md) — `sqlx::query` needs `&'static str`.
- [sqlx-chain-id-is-i32-vacuous-guards](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is i32; a wrong decode type hides inside "no rows" guards.
- [gitignore-swallows-new-dirs](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules hide a new `foo/mod.rs`.
- [worktree-shell-and-external-binary-tests](worktree-shell-and-external-binary-tests.md) — worktree Bash refuses `env VAR=x cmd`, heredoc appends, chained commits.

## Wire format

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
- [ability-telemetry-coverage-gate.md](ability-telemetry-coverage-gate.md) — AB-C7: script set + four scanned code tables + client declaration; AB-C6 timing tables are process-global.

## Lab supervisor

- [stacked-pr-ship-title-and-ab-lab-tools.md](stacked-pr-ship-title-and-ab-lab-tools.md) — ship.py mistitles stacked PRs; AB-T5 snapshot builder, lab dummy = AI-skip extension, cooldown clear is type 2.
- [lab-uat-runner-in-process-tool-calls.md](lab-uat-runner-in-process-tool-calls.md) — call rmcp tools by name in-process via the RequestContext extractor; chat marks before the action; capability table.
- [ability-uat-staging-limits.md](ability-uat-staging-limits.md) — warmup and cleanse rows need .dummy caster (#1188); 1462/4306/2827 are effect ids; one graded press per row.

## Launcher

- [launcher-state-key-and-egui-wake.md](launcher-state-key-and-egui-wake.md) — sgw_game patches are recorded as `<id>@sgw_game`; worker events must wake egui via EventSender.
- [launcher-extracted-mtimes-and-ue3-ini-version.md](launcher-extracted-mtimes-and-ue3-ini-version.md) — extracted files must keep archive DOS times or UE3 flags Default*.ini outdated; zip 1980 placeholder; MakeCAB fixture.
- [launcher-self-update-handoff-traps.md](launcher-self-update-handoff-traps.md) — viewport Close waits for a frame; handoff releases instance_lock + process::exit; relaunch kills stuck old exe only.

## Injected client DLLs

- [offline-disasm-and-minhook-detour-tests.md](offline-disasm-and-minhook-detour-tests.md) — capstone under `py -V:3.13` reads SGW.exe offline; MinHook stand-in tests for detours; game calls outside catch_unwind.
- [injected-dll-unwind-and-lua-error-rules.md](injected-dll-unwind-and-lua-error-rules.md) — `thiscall-unwind` detours for C++-EH prologues.
- [entity-method-stream-is-memory-ostream.md](entity-method-stream-is-memory-ostream.md) — onEntityMethod's live stream is a queued MemoryOStream subobject (cursor +0x14, end +0xc), not MemoryIStream.
- [client-handler-abi-and-static-disassembly.md](client-handler-abi-and-static-disassembly.md) — CME handlers are `ret 8` (event, subject); verify `ret N` with capstone on the local QA exe; event-bag getters.
- [lab-probe-traffic-starves-watchdog.md](lab-probe-traffic-starves-watchdog.md) — per-field mem_reads starved the lab heartbeat and killed a healthy client; keep bridge reads coarse.
- [lab-event-store-and-ui-lua-hooks.md](lab-event-store-and-ui-lua-hooks.md) — events_read drains; read via the supervisor store; one UI Lua subscription per window per event.
- [lab-ui-reader-lua-traps.md](lab-ui-reader-lua-traps.md) — stock UI Lua facts behind the UI readers (right-click use, Ctrl-drag split, one chat capture via the events store; lupa offline check.
- [client-patch-send-natives-traps.md](client-patch-send-natives-traps.md) — startEntityMessage sends even offline; microseh masks the ABI; no cpcall around C-function args.
- [telemetry-anchor-audit-and-hookgate.md](telemetry-anchor-audit-and-hookgate.md) — telemetry anchors never ran and 5 were wrong (IAT hint/name RVAs, COL-shifted vtable.
- [cme-registry-is-a-factory-not-subscribe.md](cme-registry-is-a-factory-not-subscribe.md) — 0x00a5c0f0/0x00a5c150 are the CME event-factory map (create/count by std::string); the CME subscribe never worked.
- [dll-boot-testhost-traps.md](dll-boot-testhost-traps.md) — sgw-testhost harness: console children hold a piped stdout (start32::run blocks), restage after DLL edits, derive site counts.
- [cargo-artifact-hardlink-cp-trap.md](cargo-artifact-hardlink-cp-trap.md) — `cp` over a built DLL writes through the hardlink into deps/; `rm` first, `touch` a source to recover.
- [injector-bitness-and-start32-helper.md](injector-bitness-and-start32-helper.md) — x64 launcher injects via the i686 sgw-start32 helper; the WOW64 resolver fails on suspended targets.
- [client-unit-slots-and-actor-pose.md](client-unit-slots-and-actor-pose.md) — Lua units are slots (map at mgr+0x130), pin private slots 7700+; actor pose at +0xDC; worldToPixel only in PreRender.

## UE3 packages and navmesh

- [ue3-and-navmesh-index](ue3-and-navmesh-index.md) — sub-index: UE3 package decode, BSP/StaticMesh, NavBuilder OBJ, Recast/Detour, map placement, occluders.

## Seeds and content chains

- [seeds-and-content-chains-index](seeds-and-content-chains-index.md) — sub-index: template/cover/name seeds, chain conditions and edge triggers, dialog binds, inventory locks, pet and trainer seeds.

## Base sessions

- [mail-expiry-and-notify-seams.md](mail-expiry-and-notify-seams.md) — every mail writer sets `expires_at`; `NOT quarantined` on every player path.
- [connected-map-view-over-parallel-index.md](connected-map-view-over-parallel-index.md) — online lookups: a view over `connected` + `listed_online`, not a parallel map.
- [trade-results-client-ignores-space-cash.md](trade-results-client-ignores-space-cash.md) — client trade window acts only on results 1/2; space/cash codes 3-6 show nothing, so send a feedback line.
- [tell-channel-and-ignore-copies.md](tell-channel-and-ignore-copies.md) — client /tell is byte 10 (CHAN_TELL since SS-C4); the Ignore list has 3 copies synced by one resync.
- [chat-channel-client-display.md](chat-channel-client-display.md) — server 8 opens a modal prompt, 9 is plain feedback, 7 shows nothing (nil ChannelMap).

## Cell systems

- [cell-systems-index](cell-systems-index.md) — sub-index: grants and loot, per-session state, abilities and effects, NPC AI, missions, pets, crafting, black market, duels, respawn, and the ammo campaign (#1026) notes.

## Observability

- [observability-test-and-throttle-traps](observability-test-and-throttle-traps.md) — counters unobservable in tests.
- [discord-noise-and-teardown-race](discord-noise-and-teardown-race.md) — SIGNOZ_ONLY_EVENTS; logOff witness-send race is DEBUG via departed_witnesses; colo warns that are real faults.
- [log-filter-parity-traps](log-filter-parity-traps.md) — `EnvFilter::new` drops a bad directive silently.
- [tracing-span-fields-not-on-log-records](tracing-span-fields-not-on-log-records.md) — span fields are not flattened onto OTLP log records.
- [client-telemetry-index-and-upload-traps](client-telemetry-index-and-upload-traps.md) — cimmeria-client routing; base-vs-chunk URL, blank `${VAR:-}` env; opted-in player launch mints the session before the game.
- [client-telemetry-governor-classify-table](client-telemetry-governor-classify-table.md) — every DLL event passes the governor; new targets default to Budgeted; must-keep rows need a server priority prefix too.

## Testing patterns

- [testing-patterns-index](testing-patterns-index.md) — sub-index: nextest vs cargo test, revert proofs, live-DB races/ports, chain replay, encrypted test sessions, LogCapture.
- [aoi-fixture-introducible-and-wire-ledger](aoi-fixture-introducible-and-wire-ledger.md) — account_id without archetype_id hides a test player from AoI; ability sends go through `wire_ledger` (AB-T4).
- [damage-apply-miss-gate-and-seeded-rolls](damage-apply-miss-gate-and-seeded-rolls.md) — since AB-06 a miss lands nothing; a literal effect_seq may roll a miss; use `seq_rolling`.
- [player-cast-fixtures-need-a-mechanic](player-cast-fixtures-need-a-mechanic.md) — since AB-12 a player cast with no mechanic is refused; effectless fixtures need `seed_mechanic_effect`.

## Campaign judgment

- [legacy-command-parity-scoping-judgment](legacy-command-parity-scoping-judgment.md) — porting a legacy dot command: verify the reference files, don't port diverged enums.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — the egui-only dependabot PR is a no-op; the eframe PR carries the breakage.
- [training-points-cache-absolute-write.md](training-points-cache-absolute-write.md) — `handle_grant_xp` writes training_points absolutely from the session cache.
- [loot-seed-pins-and-grant-stack-cap.md](loot-seed-pins-and-grant-stack-cap.md) — new loot rows break exact pins on tables 3/7/8/9 (filter by NOT EXISTS); GrantItem does not cap at max stack.
