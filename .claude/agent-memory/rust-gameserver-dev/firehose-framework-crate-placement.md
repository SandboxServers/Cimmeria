---
name: firehose-framework-crate-placement
description: Log firehose framework split across crates (wire table, observability sampler, mercury emitter); const targets dodge the source-target scan
metadata:
  type: project
---

Since TG-MER-09 (2026-10-11): `cimmeria_wire::firehose` owns `FIREHOSES` and the
decrypt/udp_in/aoi emitters, but `FirehoseSampler` lives in
`cimmeria-observability` and the `mercury_packet` emitter/targets in
`cimmeria_mercury::firehose` (wire depends on mercury, never the reverse); wire re-exports both.

**Why:** a new firehose emitted below wire cannot import wire.
**How to apply:** a new firehose = consts + emitter + `FIREHOSES` row + `FILE_LAYERS`
`wire.firehose.<x>=trace` + a `firehose_sampling` parity test. Use `const` targets:
`target_scan_tests` skips const targets (the table covers them); a literal
`wire.firehose.*` target fails `every_source_target_reaches_signoz_at_its_level`.
Framework `suppressed` counts drops BEFORE a sample (first sample = 0), so
`sum(1+suppressed)` reaches the last sample, not n.
