---
name: tracing-span-fields-not-on-log-records
description: Span fields never reach SigNoz Logs (opentelemetry-appender-tracing does not flatten ancestor spans); Option<T> tracing fields are omitted when None; LogCapture only sees event-own fields
metadata:
  type: project
---

Three facts that decide the shape of any "stamp X onto every log" task in
this repo. Each cost real investigation; none is visible from reading a
single file.

**Why:** a span-based approach to log enrichment looks correct locally
(`RUST_LOG` fmt output shows the span context) and is silently useless in
production, where the operator queries the SigNoz **Logs** view.

**How to apply:** reach for explicit event fields, not spans, whenever the
goal is "filterable in SigNoz Logs".

## 1. `opentelemetry-appender-tracing` does NOT flatten ancestor span fields

`crates/server/src/otel.rs` wires two separate layers:
- `tracing_opentelemetry::OpenTelemetryLayer` → spans become OTel **spans**
  (fields become span attributes, visible in the Traces view).
- `OpenTelemetryTracingBridge` (`opentelemetry-appender-tracing` 0.32) →
  each `tracing` event becomes one OTel **log record** carrying *that
  event's own fields* plus `trace_id`/`span_id`. It does **not** walk the
  parent span chain.

So a field declared only on `#[tracing::instrument(fields(account_id))]`
is **invisible to a SigNoz Logs filter**. Put correlators on the event.

Compounding: `base.datagram`, `base.encrypted_datagram`,
`base.player_method`, `cell.dispatch` are all `level = "debug"`, so under
the default info filter those parent spans aren't recorded at all.

## 2. Spans do not cross the base↔cell boundary

`orchestrator.rs` joins the two services with
`mpsc::channel::<BaseToCellMsg>` / `CellToBaseMsg`. Separate tokio tasks —
a span entered on the base side is not in scope when the cell task
dequeues the message. Ambient propagation into `cell/service/`,
`cell/console/`, `cell/space_manager/` is structurally impossible; data
must ride in the message or be re-resolved from `SpaceManager`.

## 3. `Option<T>` as a tracing field value is OMITTED when `None`

`tracing` has `impl<T: Value> Value for Option<T>` whose `record` is a
no-op on `None`. So:

```rust
tracing::warn!(account_id = id.account_id /* Option<u32> */, ...);
```

emits `account_id=6` for a player and **no `account_id` key at all** for
an NPC. This is the right idiom for "field only when known" — never
`unwrap_or(0)`, which makes a sentinel indistinguishable from a real id
and matches every NPC in a query.

## 4. `LogCapture` records only the event's own fields

`crates/services/src/test_support.rs`'s `CaptureLayer::on_event` visits
the event's fields and does not walk span context. Consequences:
- A test **cannot** assert a span-inherited field on an event.
- `on_new_span` is captured separately under target `span:<name>`, and
  `on_record` under `span_record:?` (see `span_recorded()`).
- Asserting `!event.fields.contains_key("account_id")` is a valid and
  useful guard — it catches the `unwrap_or(0)` regression shape.

## Applied

The identity-propagation convention (`account_id` + `player_id` on every
player-activity log) is Rule 5 of
`docs/architecture/instrumentation-discipline.md`. Resolvers:
`SpaceManager::player_identity` (cell) and
`base::session_identity::identity_for_entity` (base). Cell entities are
identity-stamped at `BaseToCellMsg::CreateEntity`, not `InitPlayerState`
(the latter arrives only after `onClientReady`, too late for world-entry
and gate-travel logs).
