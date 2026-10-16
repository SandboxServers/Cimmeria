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
One pump stores bridge rows before Lua click rows, so store seq does not
order the click before its send. The click has `event_id`, but stock
`ability.sent` and `net.out` lack it; apply/shown rows also lack a stable
receive key in most paths. These are separate blocked DLL dependencies,
not present live functionality. Press/send/cast IDs require proven common
process or session scope and cannot cross relog by numeric equality alone.
Failed shown handlers remain failure evidence, not visible result. Time,
entity and method alone cannot join stages. The complete offline latency
fixture assumes future verified identity and clock projection.
