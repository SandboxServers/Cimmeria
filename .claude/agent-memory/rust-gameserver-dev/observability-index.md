---
name: observability-index
description: Sub-index of observability memories (Rule 6 name pairing sweeps, Discord pairs, OTLP redaction, client telemetry, log filters) moved out of MEMORY.md to keep it small
metadata:
  type: reference
---

# Observability

- [named-telemetry-sweep-traps](named-telemetry-sweep-traps.md) — Rule 6 sweeps: chain/loot/event-set names exist in seed; keys holding the wrong ID; rescan without rebuild.
- [rule6-sweep-scanner-and-naming-traps](rule6-sweep-scanner-and-naming-traps.md) — list unpaired sites by zeroing baseline rows; `$level!` rows escape the scan; never name a deferred effect's invoker live.
- [discord-named-pairs-resolvers](discord-named-pairs-resolvers.md) — Discord Event objects are `Named` pairs (NT-10); base/cell/content resolvers; button text and minigame ids are unnameable.
- [observability-test-and-throttle-traps](observability-test-and-throttle-traps.md) — counters unobservable in tests.
- [fixed-window-throttle-steady-retrier](fixed-window-throttle-steady-retrier.md) — fixed 10 s window let a 12 s retrier post 1,363 Discord rows; escalate, reset on quiet-since-last-seen; credential refusals SigNoz-only.
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
