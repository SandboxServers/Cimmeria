---
name: log-filter-parity-traps
description: Traps when changing the server's tracing/OTLP filters (NA25) — EnvFilter::new silently drops a malformed directive, per-layer filters can be tested with runtime targets via a leaked callsite, a custom target leaves its module-path file, and the Bash tool eats backslash-newline in heredocs
metadata:
  type: project
---

Learned landing NA25 (file/SigNoz log parity, 2026-09-25).

**`EnvFilter::new` never fails.** A malformed directive (e.g. two
directives run together with spaces) is dropped silently and the filter
just stops matching that target; only `"...".parse::<EnvFilter>()`
returns the error. Any test over a directive string must parse it.
**How to apply:** when a filter test fails as "0 indexes", check the
string parses before hunting the routing logic.

**Per-layer filters are testable with targets taken from a table.** The
`tracing` macros need a literal target, so the parity test in
`crates/server/src/logging/parity_tests.rs` leaks a `Callsite` +
`Metadata` per target and dispatches the macro way: `d.enabled(meta)`
first (records each per-layer filter's verdict), then
`d.event(&Event::new(meta, &meta.fields().value_set(&[..; 0])))`.
Reuse that harness rather than inventing another.

**Moving a row to a custom `target:` removes it from its log file.**
Every `logs/*.log` layer is `off,<module path>=trace`, so a custom target
matches none of them. The firehose targets (`wire.firehose.*`) are named
in `FILE_LAYERS` for that reason; `every_firehose_is_kept_in_full_by_a_file`
guards it.

**A new literal `target: "…"` must be named in `OTEL_FILTER` at its
level** or `logging/target_scan_tests.rs` fails (NA25 round 2 closed the
14 that were silently dropped). A new crate directory must be added to its
in-/out-of-process list there. `launcher.key_dump` is deliberately `off`
(session key); never widen `launcher` past it.

**Bash-tool heredocs drop a backslash-newline.** A python heredoc
replacing a Rust `"...,\` continuation line joined it to the next line
(spaces and all), which is exactly the malformed-directive case above.
**How to apply:** edit Rust string-continuation lines with the Edit tool,
not a scripted heredoc. See also [[python-write-mangles-utf8-and-crlf]].
