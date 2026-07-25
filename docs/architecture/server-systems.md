---
title: "Server-Only Infrastructure Systems (superseded)"
type: explanation
audience: engineers
last_updated: 2026-07-25
---

# Server-Only Infrastructure Systems (superseded)

> [!IMPORTANT]
> **This document has been superseded. Its content lives in the pages below.**
>
> Nothing was deleted — the survey was split, because half of it aged well and
> half of it did not. Use the routing table to find what you came for.

## What this was

From 2026-03 to 2026-07 this page surveyed eight server-side infrastructure
systems that have no client-facing wire format — session management, rate
limiting, anti-cheat, economy, world-state persistence, scheduling, admin/GM
tools, and metrics. Each section had an Overview, a Current State, a Gaps list,
a Recommended Approach, and a Priority.

It was written against the **deprecated Python/C++ server**. Every "Current
State" section cited `python/…`, `config/BaseService.config`, and `db/sgw.sql`
— a tree that now sits under [`deprecated/`](../../deprecated/) with the config
files gone entirely. Active development is Rust under [`crates/`](../../crates/).

The survey then drifted in the way single-file surveys always drift: its
current-state verdicts went stale one PR at a time, while nothing forced them to
be revisited. By mid-2026 it was asserting that movement had no speed
validation, that the recommended next step was to enable a Python console, and
that the server had no performance metrics — all three untrue, and all three
untrue for months before anyone noticed. Its design *reasoning*, meanwhile, was
mostly still sound.

So it was split along that seam. Current state now lives where it gets updated
as a side effect of the work; design reasoning lives with the system it argues
about.

## Where each section went

| Original section | Current state | Design reasoning |
|---|---|---|
| §1 Session Management | [gap-analysis.md](../gap-analysis.md) §"Session Management" | [server-infrastructure-proposals.md §1](server-infrastructure-proposals.md#1-session-resume-across-a-network-blip) — session resume, reconnect token |
| §2 Rate Limiting | [gap-analysis.md](../gap-analysis.md) §"Rate Limiting" | [server-infrastructure-proposals.md §2](server-infrastructure-proposals.md#2-per-player-rate-limiting) — per-category token buckets, warn-then-enforce |
| §3 Anti-Cheat and Server-Side Validation | [movement-validation.md](movement-validation.md) — the four-layer validator that shipped | [movement-validation.md §"Adjacent validation gaps"](movement-validation.md#adjacent-validation-gaps-not-movement) — damage cap, line of sight |
| §4 Economy Sinks and Faucets | [gap-analysis.md](../gap-analysis.md) §"Economy Sinks / Faucets" | [server-infrastructure-proposals.md §5](server-infrastructure-proposals.md#5-economy-instrumentation-before-economy-balance) — instrument before balancing; auction fees in [black-market.md](../gameplay/black-market.md#economy-sink-design-unbuilt) |
| §5 World State Persistence | Still absent — no world-state table exists | [server-infrastructure-proposals.md §3](server-infrastructure-proposals.md#3-world-state-persistence) — `sgw_world_state`, gate state first |
| §6 Event and Scheduler System | Still absent — no scheduler exists | [server-infrastructure-proposals.md §4](server-infrastructure-proposals.md#4-global-event-scheduler) — `sgw_scheduled_events`, build on demand |
| §7 Admin and GM Tools Backend | [gm-cell-method-gating.md](gm-cell-method-gating.md), [dev-console-channel.md](dev-console-channel.md), [../tools/admin-api.md](../tools/admin-api.md) | [gm-cell-method-gating.md §"Moderation surface still missing"](gm-cell-method-gating.md#moderation-surface-still-missing) — audit log, ban/mute, broadcast, rollback |
| §8 Metrics and Telemetry | [observability.md](observability.md), [instrumentation-discipline.md](instrumentation-discipline.md) | [observability.md §"Known gaps"](observability.md#known-gaps) — `perfStats` sink, gameplay counters, alerting |

## Corrections worth carrying forward

If you find this survey quoted anywhere — an old issue, a PR comment, an agent
memory — these are the four claims most likely to be repeated as though they
were still true:

- **"No movement speed validation, no teleport detection."** False since #437
  and #478. A four-layer validator (bounds → navmesh → speed → teleport) gates
  every inbound client position. See [movement-validation.md](movement-validation.md).
- **"Enable the Python console — one config value unlocks 50+ commands."**
  There is no Python console. The Rust server embeds no interpreter. GM commands
  run through the client's native `/` console and the `.`-prefixed dev console;
  see [dev-console-channel.md](dev-console-channel.md) and
  [python-console.md](python-console.md) for what the old console actually was.
- **"No server performance metrics, no anomaly alerting."** Half false. OTLP
  export and Mercury packet instrumentation ship against a SigNoz backend
  ([observability.md](observability.md)). Alerting is still genuinely absent.
- **"The black market is entirely stubbed."** True on `main`. A full Phase 1
  implementation lives on the unmerged `feat/571-black-market-phase1` branch
  (PR #586) — see [black-market.md](../gameplay/black-market.md), which is
  explicit about which half is which.

## The framing that still holds

Two ideas from the original Overview survived the rewrite and are worth keeping
in mind when you pick up any of the proposals:

**These systems have no client awareness.** None of the eight has an entity
`.def` file, a client RPC method, or a wire format. That is what makes them
tractable to add — no client patch, no protocol archaeology, no compatibility
risk. It is also what makes them easy to defer indefinitely, since nothing
visibly breaks without them.

**Implementation order should follow test-session pain, not completeness.** The
original priority ordering put GM tooling and session resume first because they
are what a live multi-player test session actually stumbles over, and pushed
economy and scheduling to the back because there is no economy to balance and no
scheduled content to run. That ordering is preserved in
[server-infrastructure-proposals.md](server-infrastructure-proposals.md), which
is sequenced by pain rather than by cost.

## Related documents

- [server-infrastructure-proposals.md](server-infrastructure-proposals.md) — the
  five unbuilt designs extracted from this survey.
- [gap-analysis.md](../gap-analysis.md) — §"Server Infrastructure
  (Cross-Cutting)" is the live status tracker that replaced this page's
  "Current State" sections.
- [service-architecture.md](service-architecture.md) — the Auth / Base / Cell
  topology these systems sit inside.
- [scaling-analysis.md](scaling-analysis.md) — why single-process is the right
  target for all of them.
