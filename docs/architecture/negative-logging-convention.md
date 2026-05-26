# Negative-Logging Convention

> **Last updated**: 2026-05-26
> **Audience**: Engineers writing or reviewing server-side handlers, packet
> dispatchers, DB writers, AoI fan-out, or anything where a downstream
> effect is *expected* to land
> **Type**: Reference + decision guide
> **Owner**: Server systems

## TL;DR

The server logs liberally when things happen and historically said almost
nothing when expected things failed to happen. A `let _ = tx.send(...)`
hides a dropped event; a `rows_affected == 0` silently treated as success
hides a corrupt state flag; a `trace!` on an AoI witness miss hides
invisible entities. Every one of those becomes a ticket with no repro.

**Pick a level by recoverability + visibility. Always include
expected + actual + player-facing consequence. Pair `rows_affected` with
`expected` so a single ops query surfaces every divergence.**

| Level | When to use |
|-------|-------------|
| `trace!` | **Never** for expectation failures. Reserve for high-volume sample-only diagnostics. |
| `debug!` | Expectation unmet, normal/transient (e.g. client disconnected mid-AoI-update). |
| `warn!` | Expectation unmet, player-visible, recoverable (e.g. NPC stuck, fragment will retry). |
| `error!` | Expectation unmet, player-visible, unrecoverable or corrupts state (e.g. `rows_affected == 0` on mission-complete UPSERT). |

## Why this exists

Issue [#304] audited the cell, base, and content/mission subsystems and
turned up 40+ *expectation seams* — call sites that assume a downstream
effect will land but emit no signal when it doesn't. The class of bug
this targets:

- NPC fails to spawn.
- Quest fails to advance.
- Teleport silently drops a packet.
- AoI packet vanishes into an empty witness map.
- `first_login` flag never clears, so the intro cinematic re-fires on
  every login.

Each is invisible until a player files a ticket with no repro. A
`warn!` or `error!` at the right seam turns every one of them into a
greppable ops signal.

[#304]: https://github.com/SandboxServers/Cimmeria/issues/304

## The three patterns

Three patterns account for most seams. Each has a uniform fix.

### Pattern A — `let _ = tx.send(...)` / `let _ = query.execute(...)`

Endemic across the content executor (`PlaySequence`, `StartMinigame`,
dialog dispatch, `SetActiveSlot`) and DB UPDATE paths. Errors are
silently dropped.

**Fix**: replace bare `let _` with `if let Err(e) = ... { warn!(...) }`.
No call-site needs to handle the error differently — they all just need
to log it.

```rust
// ✗ silent
let _ = tx
    .send(CellToBaseMsg::EntityMethodCall { entity_id, method_index, args })
    .await;

// ✓ greppable
if let Err(e) = tx
    .send(CellToBaseMsg::EntityMethodCall { entity_id, method_index, args })
    .await
{
    tracing::warn!(
        entity_id,
        sequence_id,
        chain_id,
        "PlaySequence: cell→base send failed: {e}"
    );
}
```

### Pattern B — `rows_affected == 0` inconsistently handled

Teleport and gate travel `warn!` on it. `first_login` clear used to
silently swallow it. Inventory `remove_instance` warned but dropped the
actual count from the structured fields.

**Fix**: always emit `rows_affected` and `expected` as paired structured
fields so a single ops query (`rows_affected != expected`) surfaces every
divergence in one place.

```rust
// ✗ count missing — the warn fires but ops can't aggregate
tracing::warn!(player_id, item_id, "RemoveInventoryItem: no rows changed");

// ✓ count present — `rows_affected != expected` is a single ops query
tracing::warn!(
    player_id,
    item_id,
    rows_affected = r.rows_affected(),
    expected = 1,
    "RemoveInventoryItem: no rows changed"
);
```

For state-corrupting cases (`first_login`, mission-complete UPSERT,
teleport position) use `error!` rather than `warn!` because the player
experience is visibly broken until manual intervention.

### Pattern C — Witness / lookup miss logs at `trace!`

`send_to_witness` (and its `_reliable` and bundle siblings) silently
dropped AoI packets at `trace!` level when `entity_to_addr` returned
`None`. Same shape repeats in delayed callbacks that look up an
entity_id after a sleep.

**Fix**: upgrade to `warn!` for the missing-addr case (something is
wrong: either the witness leaked from the list or the addr map drifted
out of sync), `debug!` for client-disconnected (normal but should be
queryable).

```rust
// ✗ trace! — production never sees this
let addr = match entity_to_addr.lock().unwrap().get(&witness_id).copied() {
    Some(a) => a,
    None => {
        tracing::trace!(witness_id, "AoI: no client addr -- skipping");
        return Ok(());  // also returns Ok — caller can't tell either
    }
};

// ✓ warn! + distinguishable Err so callers can fan out their own logging
let addr = match entity_to_addr.lock().unwrap().get(&witness_id).copied() {
    Some(a) => a,
    None => {
        let entity_count_in_map = entity_to_addr.lock().unwrap().len();
        tracing::warn!(
            witness_id,
            entity_id,
            action,
            entity_count_in_map,
            "AoI: no client addr for witness -- skipping"
        );
        return Err("no_client_addr");
    }
};
```

## Structured-field naming rules

| Field | Type | Notes |
|-------|------|-------|
| `player_id` | `i32` | DB id (matches `sgw_player.player_id`). Not the entity_id. |
| `entity_id` | `u32` | Runtime entity id. Use this when the operation targets the entity, not the persisted player row. |
| `mob_id` | `u32` | NPC entity id when the surrounding context could conflate with players. |
| `mission_id`, `chain_id`, `step_id`, `dialog_id`, `dialog_set_id` | `i32` | Content ids. Plain names, no `mission_uid` aliases. |
| `space_id`, `cell_id`, `world_name` | `i32` / `String` | World context. `world_name` for human-readable; `space_id` for the cell key. |
| `rows_affected` | `u64` | Always paired with `expected`. From `sqlx::query::Result::rows_affected()`. |
| `expected` | `u64` | The invariant the SQL is supposed to maintain. Usually `1`. |
| `phase` | `&'static str` | Short string enum naming the sub-step. Examples: `"create_base"`, `"cascade"`, `"display"`, `"start"`. |
| `reason` | `&'static str` | Short string enum naming why the expectation was unmet. Examples: `"entity_to_addr_miss"`, `"oneshot_dropped"`, `"rows_affected_zero"`. |
| `action` | `&'static str` | AoI fan-out classifier. Values: `"CREATE"`, `"METHOD"`, `"LEAVE"`. |

Use the same name everywhere — no aliases. `pid` / `eid` / `mid` are
forbidden because they prevent ops from grepping for one field across
the codebase.

## Message shape

Always include: **expected + actual + player-facing consequence**.

- ✗ `"send failed"`
- ✗ `"no rows changed"` (alone)
- ✓ `"MissionUpdate (complete) send failed -- completion will not persist to DB"`
- ✓ `"first_login flag NOT cleared -- cinematic will re-fire on next login (player_id missing from sgw_player?)"`

The consequence clause is the most important part. It tells the
on-call engineer what the player sees and what to triage first.

## Definition of done

A seam is done when:

1. The expectation-violation path emits a structured log at the right
   level.
2. Structured fields follow the naming convention above.
3. The message includes expected + actual + player-facing consequence.
4. A test asserts the log fires when the expectation is violated.
5. The test fails when the fix is removed (proves it guards the bug
   shape, not the happy path).

(5) is the load-bearing requirement. A "happy path" test that passes
both before and after the fix doesn't guard anything; see
[TESTING.md](../../TESTING.md#regression-guards-vs-happy-path-tests).

## Testing patterns

Two patterns satisfy the regression-guard requirement.

### Behaviour-based guards

When the fix changes a return type or side effect, assert the new
behaviour directly. Example from
[`crates/services/src/base/helpers.rs`](../../crates/services/src/base/helpers.rs)
(T1-1 / T1-2):

```rust
let result = send_to_witness(/* ... */).await;
assert_eq!(
    result,
    Err("no_client_addr"),
    "missing witness MUST produce a distinguishable Err so callers \
     can fan-out their own logging"
);
```

Reverting the fix to `Ok(())` fails the assertion.

### Log-emission guards

When the fix is a level promotion or a structured-field addition, use
[`tracing-test`](https://docs.rs/tracing-test) — a dev-dep that
installs a per-test subscriber and exposes `logs_contain!`. Example
from
[`crates/services/src/base/world_entry_appearance.rs`](../../crates/services/src/base/world_entry_appearance.rs)
(T1-5):

```rust
#[tokio::test]
#[traced_test]
async fn clear_first_login_logs_error_when_player_missing() {
    let pool = require_db_or_skip!();
    // ... insert account but no sgw_player row ...

    clear_first_login(&pool, missing_player).await;

    assert!(
        logs_contain("first_login flag NOT cleared"),
        "error! must fire for zero-row UPDATE — bug shape: silent re-fire of cinematic"
    );
    assert!(
        logs_contain("rows_affected=0"),
        "structured field `rows_affected = 0` must appear so ops queries surface the divergence"
    );
}
```

Reverting the `Ok(r) if r.rows_affected() == 0 => error!()` match arm
fails both asserts.

### Live-DB tests for `rows_affected` paths

Anything that changes a `rows_affected` invariant (Pattern B) needs a
**live-DB** regression guard. Unit tests can't construct a sqlx
`PgQueryResult` (the type has no public constructor), so the only way
to prove the guard exercises the real code path is to run against a
real Postgres. See
[integration-test-infra.md](integration-test-infra.md) for the local
setup and `require_db_or_skip!` pattern.

Live-DB tests share `sgw_player`, so each test family uses a sentinel
range to avoid collision. Current allocation:

| Range | Used by |
|-------|---------|
| `0x7000_1000+` | `crates/services/src/base/character.rs` |
| `0x7000_2000+` | `crates/services/src/base/world_entry_appearance.rs` (T1-5 guards) |

Pick the next free `0x7000_N000` slot when adding a new family.

## Anti-patterns

- **`let _ = ...` on anything that can fail.** Either log the error or
  call the function for its side effect with a sync API that can't
  fail. There's no third option.
- **`trace!` for a missed expectation.** Production never enables
  `trace!` — the bug is invisible.
- **`warn!` without an `entity_id` / `player_id` / `mission_id`.** An
  unattributable warn is a metric, not a log. Aggregate with metrics if
  that's what you want.
- **`error!` for a transient that auto-recovers.** Ops dashboards page
  on `error!`. A retried RTO is a `warn!` at most.
- **Field renames per call site.** `pid` here, `playerId` there,
  `player` somewhere else. Pick one and use it everywhere.

## Related

- Issue [#304] — the audit that produced this convention.
- [TESTING.md](../../TESTING.md) — regression-guard rules.
- [CLAUDE.md](../../CLAUDE.md) — doc-update map.
- [integration-test-infra.md](integration-test-infra.md) — live-DB test setup.
