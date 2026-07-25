# Negative-Logging Convention

> **Last updated**: 2026-07-25
> **Status**: Convention adopted in issue #304 PR1 (2026-05-24). Applies to
> every new patch that touches an expectation seam.

## What this document covers

When a code path **assumes** a downstream effect will land — a packet will
send, a row will update, a channel send will deliver, a function will
return success — and the assumption fails silently, the failure is
**invisible until a player reports symptoms with no repro**. NPC fails to
spawn, quest fails to advance, teleport silently drops a packet, AoI
packet vanishes into an empty witness map.

This document defines the convention for **negative logs** — log lines
emitted at the point where an expectation is unmet. The goal: every
failure mode of this shape is greppable as a single structured log
event, with enough context for ops to act.

Source: issue #304 (`Negative-logging audit: 40+ expectation seams`).

## The three patterns

### Pattern A — `let _ = tx.send(...)` / `let _ = query.execute(...)`

Endemic in `crates/services/src/`. Errors are silently dropped. **Fix**:
replace bare `let _` with `if let Err(e) = ... { warn!(...) }`. No
call-site needs to handle the error differently — they all just need to
log it. The exception is broadcast channels with optional subscribers
(e.g. `audit.rs::emit_login_event` over `broadcast::Sender`) where "no
subscribers" is the normal case — those stay silent intentionally.

### Pattern B — `rows_affected == 0` inconsistently handled

Some sites warn, some don't. When they do, the count itself is often
omitted from the structured fields. **Fix**: always emit `rows_affected`
and `expected` as paired structured fields so a single ops query
(`rows_affected != expected`) surfaces every divergence in one place.

### Pattern C — Witness / lookup misses logged at `trace!`

The `send_to_witness` family in
[`crates/services/src/base/helpers/mod.rs`](../../crates/services/src/base/helpers/mod.rs)
historically logged AoI packet drops at `trace!`, making them
invisible without `RUST_LOG=trace`. **Fix**: upgrade to `warn!` for the
entity-to-addr miss (player-visible bug) and `debug!` for the
client-disconnected case (normal during logoff races but should
remain queryable). Both carry a stable `reason` field for triage.

## Field naming rules

| Field | Required? | Notes |
|---|---|---|
| `player_id` | when applicable | The affected player. No aliases (`pid`). |
| `entity_id` | when applicable | The affected entity. No aliases (`eid`). |
| `mob_id` / `mission_id` / `chain_id` / `step_id` / `space_id` / `cell_id` / `world_name` | when applicable | Canonical names per the existing logging surface. |
| `rows_affected` + `expected` | always paired on DB writes | Pair so a single ops query catches divergence. |
| `phase` | optional | Short string naming a sub-step (e.g. `"create_base"` \| `"cascade"`). |
| `reason` | optional | Short string naming why the expectation was unmet (e.g. `"entity_to_addr_miss"`, `"oneshot_dropped"`, `"rows_affected_zero"`). |

## Level discipline

| Level | When |
|---|---|
| `trace!` | **Never** for expectation failures. Reserve for high-volume sample-only diagnostics. |
| `debug!` | Expectation unmet, normal/transient (e.g. client disconnected mid-AoI-update). |
| `warn!` | Expectation unmet, player-visible, recoverable (e.g. NPC stuck, fragment will retry). |
| `error!` | Expectation unmet, player-visible, unrecoverable or state-corrupting (e.g. `rows_affected == 0` on mission-complete UPSERT, reward grant failure). |

## Message shape

Every negative log message MUST include:

1. **The system** (e.g. `"PlaySequence: …"`, `"AoI reliable: …"`).
2. **What was expected vs what actually happened**.
3. **The player-facing consequence** (or "may hold stale state" if no immediate user impact).

| | |
|---|---|
| ❌ Bad | `"send failed"` |
| ✅ Good | `"MissionUpdate (complete) send failed -- completion will not persist to DB"` |

## Defensible exceptions

Not every silent send is a bug. The following patterns are intentionally
silent and should NOT be promoted:

- **`broadcast::Sender` with optional subscribers** — `audit.rs:109`
  emits login events to whoever's listening on the WebSocket bus. Zero
  subscribers is normal.
- **`oneshot` reply channels where the receiver may have timed out** —
  `world_entry_db.rs:242`, `base_messages/mod.rs:69` fall back to a
  default reply when the requester has dropped. Logging here would spam
  during load-shedding.
- **`sqlx::Transaction::rollback()`** — `let _ = tx.rollback().await`
  inside an error arm. Rollback failure during error handling is
  recoverable noise; the caller has already logged the originating
  error.

When in doubt, add a `// Defensible silent send: <reason>` comment so
the next sweep doesn't churn the site.

## Regression-guard testing

Per [TESTING.md](../../TESTING.md), every PR that changes a negative-log
seam MUST include at least one guard that fails when the fix is
reverted. For log-only changes, use the `LogCapture` helper in
[`crates/services/src/test_support.rs`](../../crates/services/src/test_support.rs):

```rust
let capture = LogCapture::install();
some_function_that_logs().await;
assert!(
    capture
        .find_event(Level::WARN, "AoI", "entity_to_addr_miss")
        .is_some(),
    "issue #304: must emit WARN with reason=entity_to_addr_miss; \
     reverting to trace! breaks ops visibility"
);
```

Pin both **level** and a **stable structured field** (typically
`reason`) so a generic level-only revert AND a field-removing revert
both trip the test.

## Application: issue #304 PR series

The convention landed alongside PR1, which swept ~25 silent-drop seams
across `cell`, `base`, and `content/`. Subsequent PRs in the series:

- **PR1.5** — `world_entry_appearance/mod.rs:378` `rows_affected==0` guard
  on the relocated `first_login` UPDATE (landed with PR1).
- **PR2** — mission rewards dispatch implementation (T1-12 `todo!()`).
- **PR3** — Tier 2 state-desync logging, split by subsystem.
- **PR4** — Tier 3 operational canaries.

See issue #304 for the full per-seam catalog and tier ratings.

## Related

- [TESTING.md](../../TESTING.md) — Test-type picker; regression-guard rules.
- [CLAUDE.md](../../CLAUDE.md) — Doc-update map; pre-PR checklist.
- [crates/services/src/test_support.rs](../../crates/services/src/test_support.rs)
  — `LogCapture` helper used by guards.
