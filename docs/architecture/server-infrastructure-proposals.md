---
title: "Server Infrastructure Proposals"
type: explanation
audience: architects
last_updated: 2026-07-25
---

# Server Infrastructure Proposals

> **Type**: explanation (design proposals — nothing here is implemented)
> **Audience**: architects
> **Companions**: [gap-analysis.md](../gap-analysis.md) §"Server Infrastructure
> (Cross-Cutting)" tracks the current status of each of these;
> [server-systems.md](server-systems.md) is the pointer page these proposals were
> extracted from.

Five pieces of server-side infrastructure that Cimmeria still does not have, and
a concrete design for each. These are the parts of the 2026-03 server-systems
survey that survived the Rust rewrite as *design thinking* — the survey's
current-state verdicts went stale, but nobody has built these five, so the
proposals are still on the table.

Read this when you are about to build one of them. Everything below is a
proposal, not a description of running code. Where a proposal cites the Rust
tree, that citation describes what exists **today** as the starting point.

Ordering is by how much pain the absence causes during a live test session, not
by implementation cost.

---

## 1. Session resume across a network blip

### The problem

When a client's connection drops, its session is invalidated immediately. There
is no window in which the player can come back. A transient network drop — which
is exactly what a zone transition over a flaky link looks like — costs the player
a full trip back through character selection, and costs the server a destroy and
recreate of the cell entity.

The pieces that exist today: the base service reaps idle channels on a
**60-second inactivity timeout**, and the disconnect path already carries a
structured `disconnect_reason` (`"client_disconnect"`, `"inactivity_timeout"`,
`"duplicate_login"`, `"send_error"`, `"logoff"`) that every call site pins — see
[`crates/services/src/base/helpers/mod.rs`](../../crates/services/src/base/helpers/mod.rs).
Duplicate-login prevention runs at character select. What is missing is any
notion of a session that is *temporarily* gone rather than over.

### The proposal

Add a 30–60 second reconnection window. When the transport drops, mark the
session `RECONNECTING` instead of tearing it down. Mint a random 128-bit session
token at login, return it to the client in the login response, and require it
back in the reconnect handshake. If the client presents a valid token inside the
window, re-attach the existing entity to the new channel rather than destroying
and recreating the cell entity.

Use the existing inactivity timeout as the hard ceiling: a session that has sat
in `RECONNECTING` past that point is destroyed for real, with a
`disconnect_reason` of its own so the two cases stay distinguishable in SigNoz.

No schema change is needed — the token lives in base-side memory for the life of
the session window.

### Why this ordering

Everything else on this page is an improvement. This one is a
quality-of-service floor: without it, no sustained multi-player test session
survives ordinary network weather.

---

## 2. Per-player rate limiting

### The problem

Ability cooldowns are enforced per-ability, server-side, and they cover the
single most important abuse vector. Nothing else is throttled. A modified client
can send unlimited chat messages, unlimited trade proposals, unlimited mail, and
unlimited position updates, and the server will process every one.

There is no player-facing rate limiter anywhere in `crates/`. The only token
bucket in the tree is in
[`crates/discord/src/sender/token_bucket.rs`](../../crates/discord/src/sender/token_bucket.rs),
and it paces outbound webhook posts — it is not reusable as-is for player
actions, but it is a working reference for the algorithm.

### The proposal

Per-player, per-category token buckets, checked on the cell tick. Four
categories to start: chat, ability-use *attempts* (distinct from successful uses,
which cooldowns already govern), trade requests, and mail sends. Each category
gets a refill rate and a burst capacity — chat at roughly 5/second with a burst
of 10, ability attempts nearer 20/second to absorb latency-driven client retries.

**Log before you enforce.** Run the buckets in warn-only mode first, emitting a
negative log per violation (see
[negative-logging-convention.md](negative-logging-convention.md)), and let
SigNoz tell you what legitimate play actually looks like before any request gets
refused. This is the same calibration path the movement speed validator took —
it is still warn-only for exactly this reason, and that has worked out well
enough to copy. See [movement-validation.md](movement-validation.md).

The cell message loop ticks at 100 ms
([`crates/services/src/cell/service/message_loop.rs`](../../crates/services/src/cell/service/message_loop.rs)),
so 10 Hz is your measurement resolution.

---

## 3. World-state persistence

### The problem

Nothing about the interactive world survives a restart. Player position and
inventory persist; a gate left open does not, nor does a door, an elevator
button, a destroyed object, or the fact that a space event already fired.
Entities respawn in their template default state every boot.

There is no world-state table in `db/` and no code path that writes one.

### The proposal

One narrow table, keyed by space and entity tag:

```sql
CREATE TABLE sgw_world_state (
    space_id    VARCHAR(64)   NOT NULL,
    entity_tag  VARCHAR(64)   NOT NULL,
    state_key   VARCHAR(64)   NOT NULL,
    state_value TEXT          NOT NULL,
    updated_at  TIMESTAMP     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (space_id, entity_tag, state_key)
);
```

Space logic saves a key on change and loads it during space setup. The
discipline that matters more than the schema: **do not persist all entity
state.** Most objects should reset on restart. Only state that is meaningful
across sessions earns a row — gate open/closed, mission-critical interactables,
door states that gate area access.

Gate state is the case to build first. It is the most visible failure (a player
dials a gate, the server restarts, the gate is closed with no explanation) and
it is a testability problem as much as a persistence one.

> Note on schema changes: this repo does not use `db/scripts/*.sql` migrations —
> new tables go into the seed under `db/resources/` directly. See
> [write-a-database-migration.md](../guides/write-a-database-migration.md).

---

## 4. Global event scheduler

### The problem

Every timed behaviour in the server is attached to an entity's lifetime.
Cooldowns, crafting timers, ring-transport animation steps, the auction expiry
sweep — each is owned by the thing that created it and dies with it. There is no
singleton that outlives entities, and no persistent record of what is scheduled.

That rules out daily resets, vendor restocks, periodic world events, announced
maintenance windows, and seasonal content — none of which exist today, which is
precisely why nobody has missed the scheduler.

### The proposal

A scheduler owned by the base service, backed by a table so schedules survive
restarts:

```sql
CREATE TABLE sgw_scheduled_events (
    event_id    SERIAL        PRIMARY KEY,
    event_name  VARCHAR(64)   NOT NULL,
    next_run    TIMESTAMP     NOT NULL,
    interval_s  INTEGER,      -- NULL for one-shot events
    enabled     BOOLEAN       NOT NULL DEFAULT TRUE
);
```

Game systems subscribe to named events (`daily_reset`, `vendor_restock`); the
scheduler fires the event and reschedules from `interval_s`. Its own tick can
ride the existing cell/base tick rather than introducing a new timer source.

**Build this when the first feature that needs it is designed, not before.** A
scheduler with no subscribers is speculative infrastructure, and the schema above
is cheap enough to write on the day it is wanted.

---

## 5. Economy instrumentation before economy balance

### The problem

Currency enters the game through mission rewards, cash loot drops, and vendor
sell-back. It leaves through vendor purchases and (indirectly) through crafting
consumption. Player-to-player trade is zero-sum, not a sink. There is no repair
cost, no travel cost, no mail postage, and no auction fee — so the sink side is
thin, and nobody can say by how much, because **no currency transaction is
logged anywhere with its source tagged**.

### The proposal

Instrument first; do not balance yet. Every currency gain and loss gets a
structured log event carrying the amount, the resulting balance, and a source
discriminator (`mission_reward`, `loot`, `vendor_buy`, `vendor_sell`,
`trade`, `auction_fee`, …). Follow
[instrumentation-discipline.md](instrumentation-discipline.md) for the field
shape and keep the player id out of the metric labels — put it in the event.

That gives you the flow data to reason about before you touch a single price.
Static pricing is fine until you can answer "where is the naquadah actually
coming from?"

**Only then** consider sinks. The standard auction-house model (a
non-refundable listing fee plus a percentage cut on a successful sale) is
well-understood and easy to tune, and the Black Market is the natural first
sink because the fee has a place to live in the listing flow already — see
[black-market.md](../gameplay/black-market.md#economy-sink-design-unbuilt).
Repair costs, if they ever land, should scale with item level and durability
lost, not a flat fee.

### Why this is last

There is no functioning economy to balance. The player base is small enough
that accumulation is not a practical problem. Instrumentation is cheap and
makes every later decision defensible; balancing before instrumenting is
guesswork.

---

## Related documents

- [gap-analysis.md](../gap-analysis.md) — §"Server Infrastructure
  (Cross-Cutting)" is the live status tracker for all four of these.
- [server-systems.md](server-systems.md) — the superseded survey these
  proposals came from.
- [movement-validation.md](movement-validation.md) — the anti-cheat work that
  *did* ship, and the warn-then-enforce calibration pattern proposal 2 copies.
- [observability.md](observability.md) — where the metrics half of the original
  survey went.
- [negative-logging-convention.md](negative-logging-convention.md) — how to log
  a rejection so it is greppable in SigNoz.
