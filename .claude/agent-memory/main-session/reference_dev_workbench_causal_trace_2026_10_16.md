# DW-13 causal trace boundary (2026-10-16)

`crates/lab/src/workbench/causal.rs` is a conservative eight-stage
observation assembler. The passive local API reads `EventStore` through
`Supervisor::trace_snapshot`, which does not pump the bridge or advance
`client_events_read`; it surfaces eviction and the bridge's cumulative drop
count. The store currently provides client rows only. A trusted server
session-bound feed is still required for server receive/state/send stages.

`ui.click.ts_ms` is supervisor host ingest time, while bridge rows use
client wall time. Lua sampling delay has no finite bound; live
click-to-visible latency must remain null until a calibrated source exists.
`client.cme.event` alone is neither client-apply nor visible-result proof.
Click, press, send, cast, or Mercury sequence identity can correlate
observations; timestamp, entity, or method alone cannot. Ambiguous matches
remain ambiguous, and first unobserved stage is not a causal diagnosis.
