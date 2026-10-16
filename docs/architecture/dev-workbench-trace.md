# Developer workbench causal trace reference

> **Type:** Reference
> **Status:** DW-13 offline assembler and passive read API
> **See also:** [Developer workbench campaign](../analysis/dev-workbench/README.md), [local workbench authentication](dev-workbench-auth.md)

## Read API

`GET /workbench/api/v1/trace?instance=<label>&click_id=<epoch:seq>` uses the
existing local workbench cookie and instance gate. It reads up to 8,192
events already held in the supervisor's bounded ring. It does not pump the
client, acquire a lab lease, start a packet tap, or advance the
`client_events_read` cursor. The response is the versioned workbench
`{schema_version, context, data, gaps}` envelope. `data` contains the
requested `click_id`, all eight ordered stages, the first unobserved stage,
the largest competing candidate count, `capture_gap`, and an optional
`click_to_visible_ms` interval. Up to 64 later ring-order
`unjoined_context_events` retain raw `cme.event` and `mercury.*` rows
without claiming they belong to the click. Each stage contains `status` (`observed`,
`unknown`, or `ambiguous`), raw sourced evidence when uniquely matched,
reason, and candidate count. A ring eviction or any bridge drop is reported
in `gaps`; historical drops remain a conservative completeness warning.

The endpoint is an exportable JSON evidence view. The trace GUI and arm/stop
controls are separate packets. A missing `ui.click` row can mean the Lua
ring was not sampled yet; a passive read cannot claim the click never
happened.

## Stage and join rules

| Stage | Current observation | Gap meaning |
|---|---|---|
| `UiEvent` | Validated `ui.click` with exact `event_id` | Click absent or evicted from sampled history |
| `ClientSend` | `net.out`, `ability.sent`, or `ability.sent_seq` | No uniquely correlated outgoing row |
| `ServerReceive` | Trusted, bound server evidence when supplied to the pure assembler | No server source is wired to the read API yet |
| `ServerState` | Trusted, bound server state evidence | No server source is wired to the read API yet |
| `ServerSend` | Trusted, bound server send evidence | No server source is wired to the read API yet |
| `ClientReceive` | `mercury.entity_method` or `ability.recv` | No uniquely correlated inbound row |
| `ClientApply` | `ability.applied` | No domain-specific apply observation |
| `VisibleResult` | `ability.shown` handler event | No domain-specific shown observation; pixels remain unverified |

`client.cme.event` is retained as unjoined context but is not
classified as an apply or visible result: raising an event does not prove a
handler changed client state or drew a frame. The assembler uses exact
click, press, send, cast or Mercury sequence identity where available.
Mercury range joins also require the same bound server session because a
sequence counter can be reused across sessions.
Across different clock domains, the assembler also requires an exact unique
click ID or a trusted projection into one bounded domain. A matching send
ID on unprojected client and server clocks remains unknown.
Method and arguments need an entity or bound session as well. Entity,
method, or nearby time alone cannot promote a candidate to observed.
Multiple eligible candidates make the stage `ambiguous` and preserve their
count. A later uniquely keyed stage may still be observed after an earlier
gap; `first_unobserved_stage` is an evidence boundary, not a root-cause
diagnosis.

`ui.click.ts_ms` is the host's ingest time for a Lua ring row. Bridge rows
use the client's wall clock. Their offset and Lua sampling delay have no
measured finite bound, so a live click-to-visible interval stays null.
When an offline or future trusted source supplies one clock domain and
finite uncertainty for both endpoints, the export reports lower and upper
latency bounds plus their combined uncertainty. Cross-clock timestamps are
never subtracted directly.

The current read API has only the client ring. Server stages stay unknown
until a uniquely bound, independently read server feed is available. An
ordinary client event does not prove a server receive, state write, or
send. Live client behavior and pixels await approved lab UAT.
