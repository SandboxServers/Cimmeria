# Negative-Logging Convention

> **Last updated**: 2026-09-26
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

## The four patterns

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
[`crates/base-session/src/base/helpers/mod.rs`](../../crates/base-session/src/base/helpers/mod.rs)
historically logged AoI packet drops at `trace!`, making them
invisible without `RUST_LOG=trace`. **Fix**: upgrade to `warn!` for the
entity-to-addr miss (player-visible bug) and `debug!` for the
client-disconnected case (normal during logoff races but should
remain queryable). Both carry a stable `reason` field for triage.

### Pattern D — High-frequency repeat, throttled with a suppressed count

A negative log on a **per-packet or per-tick** seam reports a condition
that usually persists. One player stuck against a wall re-reports the
same rejected position at the client's 10 Hz update rate; one NPC with
no route re-fails every AI tick for as long as the zone is up. Logging
each occurrence is not extra information — it is the same fact, and it
buries every other player's first occurrence underneath it.

Measured: in three days of production logs, `movement.validation_reject`
produced **146,760** WARN rows, **103,818** of them (71%) from a single
entity in Harset.

**Fix**: gate the emission per entity, and make the elision explicit.

| Rule | Why |
|---|---|
| The **first** occurrence for an entity emits immediately | The row that says a problem *started* is the most useful one; delaying it to the end of a window is the wrong trade |
| Subsequent occurrences inside the window are counted, not written | This is the repetition, not new information |
| The next row that does emit carries `suppressed = N` | The magnitude survives. "1 row, suppressed=847" and "1 row, suppressed=0" are very different incidents and a plain rate limiter cannot tell them apart |
| The **counter** increments on every occurrence, throttled or not | A throttle must never deflate the rate an operator alerts on. Log volume and metric volume are separate budgets |
| Throttle state is keyed by entity and released in `destroy_entity` | Bounded by the live entity population, and a recycled `entity_id` must not inherit a predecessor's open window — that would swallow the first reject of a fresh session, the exact row Pattern D exists to protect |

The shared primitive is `LogThrottle` in
[`crates/cell-world/src/cell/space_manager/movement_telemetry/`](../../crates/cell-world/src/cell/space_manager/movement_telemetry/mod.rs),
parameterised on the window so each caller picks its own
(`movement.validation_reject` uses 1 s against a 10 Hz packet rate;
`npc_ai.path_fail` uses 5 s against the AI tick). Reuse it rather than
hand-rolling a second one.

**Do not apply Pattern D to a one-shot seam.** A `rows_affected == 0`
on a mission-complete UPSERT happens once and matters every time;
throttling it would lose events, not repetition. The pattern is for
seams where the *same* entity can re-trigger the *same* condition many
times a second.

#### Testing a throttle

Two guards, both required — the first is the one reviewers forget:

1. **The burst.** N occurrences inside the window produce exactly one
   row, and the next emitted row carries `suppressed = N-1`. A test
   that only asserts "a row was emitted" passes with the throttle
   deleted.
2. **Independence.** A second entity's *first* occurrence is not
   swallowed by the first entity's open window. Without per-entity
   keying the throttle is strictly worse than no throttle.

A third, cheap: state is released on `destroy_entity`.

## Field naming rules

| Field | Required? | Notes |
|---|---|---|
| `player_id` | when applicable | The affected player. No aliases (`pid`). |
| `entity_id` | when applicable | The affected entity. No aliases (`eid`). |
| `mob_id` / `mission_id` / `chain_id` / `step_id` / `space_id` / `cell_id` / `world_name` | when applicable | Canonical names per the existing logging surface. |
| `rows_affected` + `expected` | always paired on DB writes | Pair so a single ops query catches divergence. |
| `phase` | optional | Short string naming a sub-step (e.g. `"create_base"` \| `"cascade"`). |
| `reason` | optional | Short string naming why the expectation was unmet (e.g. `"entity_to_addr_miss"`, `"oneshot_dropped"`, `"rows_affected_zero"`). |
| `world` | when the seam is space-scoped | The **world name**, not only `space_id`. A space id is a runtime allocation that means nothing outside the running process, so a log carrying only `space_id` cannot be grouped by zone after the fact. Pair them — `space_id` still identifies the instance. |
| `suppressed` | required on a Pattern D seam | Count of occurrences elided since this seam last emitted for this entity. `0` on the first row of an episode. |

### Credential fields

Never log a credential value in full, at any level: this covers SIDs, tickets, session keys, passwords and password hashes, and raw request bodies that carry them. Disk logs, the admin `/ws/logs` stream and SigNoz all keep what they receive, and a harvested SID or ticket is enough to hijack a pending login ([#440](https://github.com/SandboxServers/Cimmeria/issues/440)).

Log a redacted prefix under a `*_prefix` field instead, using `CredentialPrefix` from `crates/auth/src/credential_redaction.rs`:

```rust
tracing::debug!(ticket_prefix = %CredentialPrefix(&ticket), "Phase 2 generated session credentials");
```

The prefix is six characters, which is enough to correlate one login's events and far too short to replay. `CredentialPrefix` slices on character boundaries, so it is safe on client-supplied input of any length. Don't use `&value[..6]`: it panics on short or non-ASCII input. For a request body, log `body_len` rather than the body. `auth/credential_log_guard.rs` is the regression guard: it runs a full login under `LogCapture` and fails if any captured event contains a credential.

Two deliberate exceptions remain. Credentials of six characters or fewer appear in full in `CredentialPrefix` output — there is nothing left to truncate — and the `UDP_IN` TRACE hex dump in `base/connect_loop/mod.rs` captures pre-encryption packet bytes, including the ticket field in `baseAppLogin`. That dump is required for wire-level debugging; production login tickets are long enough that the prefix rule still holds in practice.

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

## Cross-IP session-binding seams (issue #442)

The SOAP login handoff records the issuing client IP on the Phase-1
`SessionRecord` and the Phase-2 `PendingLogin`. Consuming that SID or
ticket from a **different** IP is the replay signature of a harvested
session token (a stolen SID or ticket is enough to hijack a pending
login). Both consumption seams log at `warn!`:

| Seam | `reason` | Fields |
|---|---|---|
| Phase 2 SID consumption in `auth/handlers.rs::handle_server_selection` | `session_ip_mismatch` | `user`, `account_id`, `sid_prefix`, `session_ip`, `client_ip` |
| Phase 3 ticket consumption in `base/login/mod.rs::handle_login` | `ticket_ip_mismatch` | `account_id`, `ticket_ip`, `client_ip` |

These are **warn-first** by design: NAT and IPv4/IPv6 dual-stack can
surface a different IP for the same physical client, and the false
positive rate has to be measured (via these rows) before the gate
hardens to a rejection. The `client_ips_match` helper in
`auth/mod.rs` normalises IPv4-mapped-IPv6 (`::ffff:a.b.c.d`) to its
IPv4 form so the dual-stack case does not pollute the signal.

The two rows are **not** equal-confidence. Phase 2 compares two requests
on the same TCP SOAP listener minutes apart, so a mismatch there is a
strong signal and is the seam to harden first. Phase 3 compares the
Phase-2 **TCP** source against the Mercury **UDP** source: carrier-grade
NAT pools that map TCP and UDP to different egress addresses, and a
genuinely dual-stack client (IPv6 SOAP, IPv4 Mercury — `client_ips_match`
cannot reconcile those), both produce a legitimate mismatch on every
login. Expect a high benign rate on `ticket_ip_mismatch` and never harden
Phase 3 to a rejection on this comparison alone. `sid_prefix` joins a
Phase-2 mismatch to the Phase-1 `Phase 1 generated SID` row without
logging the credential.

## NPC attack-animation seams (NA43)

An NPC attack that cannot resolve its Ability_End `onSequence` still
deals its damage, so the player takes hits from a guard that never
visibly fires. `cell/abilities/use_ability/sequence.rs` WARNs on target
`abilities.sequence` (`event = "ability_end"`) for **NPC attackers
only**. Most player abilities have no event set (1851 of the 1886
seeded), so the same condition on a player click is not a defect. The
WARN fires when the cast fires (`use_ability/fire.rs`), which for a
warmup ability is the warmup tick, not the launch (AT-10).

| `outcome` | Seam | Fields |
|---|---|---|
| `no_ability_def` | the ability has no loaded `resources.abilities` row | `source_id`, `target_id`, `ability_id`, `suppressed` |
| `no_event_set` | `event_set_id` is NULL, so no sequence is looked up | same |
| `no_end_sequence` | the event set has no event-1001 sequence | same, plus `event_set_id` |
| `no_witnesses` | the Ability_End went out to zero AoI witnesses | same, plus `sequence_id` |

This is a Pattern D seam with one deliberate difference: the throttle
(`SpaceManager::ability_sequence_log`, 60 s window) is keyed by
**ability id**, not entity. The first three outcomes are facts about a
seed row, and every NPC firing that ability repeats the same fact. The
state is bounded by the ability table, so nothing is released in
`destroy_entity`. The success rows (`ability_begin`, `ability_end`,
`ability_interrupt`, at DEBUG) carry `witness_count`.

Ability_Begin and Ability_Interrupt are deliberately not WARN seams. A
missing Begin leaves only the charge unanimated, and the Ability_End
WARN names the same seed row when the shot fires. An interrupted cast
deals no damage, so a missing Interrupt is not a hit from an invisible
attacker.

The guards are `use_ability/tests/sequence.rs` (each WARN, the NPC-only
scope, and the burst and independence throttle guards),
`use_ability/tests/sequence_phases.rs` (the WARN rides the Ability_End
into the warmup tick; Interrupt never WARNs) and
`service/tests/npc_ai/attack_sequence.rs` (`witness_count = 2` on a real
fight tick). The seed side is linted by the live-DB
`spawner/tests/npc_ability_animation.rs`.

## Encrypted-channel decrypt rejects

A datagram from a registered session that fails to decrypt is dropped
and the session stays up. The legacy C++ `EncryptionFilter` did the
same, and a teardown here would let anyone who can spoof the client's
source address end the session with one garbage datagram. The seam is
`base/connect_loop/encrypted/decrypt_reject.rs`, and it logs at `warn!`:

| `reason` | Meaning | Fields |
|---|---|---|
| `login_retry_on_channel` | The datagram is the client's **plaintext** `baseAppLogin` arriving after the server registered the encrypted channel. The client retries every 300 ms until its login reply handler finishes, so a train of these means the server replied and the client never completed the login. Look client-side, not at the keys. | `addr`, `account_id`, `raw_len` |
| `decrypt_fail` | Anything else that fails the length, HMAC, or padding check: a key mismatch, a stale session, or a forged or corrupted packet. | `addr`, `account_id`, `raw_len`, `error` |

These rows carry `reason`, never `disconnect_reason`. That field is kept
for rows that report a real teardown, such as `session.end` and
`Client entities cleaned up`. A per-session disconnect query must not
count dropped datagrams.

## Cross-space cast refusal (#906)

`useAbility` resolves its target id with `SpaceManager::get_entity`, which searches every space. A player's cast at a target that is not in the caster's space is refused in `use_ability/fire_los.rs` (at launch, which is also the fire for a zero-warmup cast) with one row on target `abilities` at `debug!`: `event = "cast_refused"`, `reason = "target_other_space"`, `entity_id`, `account_id`, `player_id`, `ability_id`, `target_id`, `caster_space_id`, `target_space_id`, `error_code`. DEBUG, because only a forged or stale packet names such a target, and a WARN would let it flood the log. The refusal is not silent: the caster gets `onErrorCode(0, ability_id, 0)`. If that cannot be queued, a WARN `cast_refused_send_failed` with the same identity fields says so. The guard is `a_target_in_another_space_is_refused_at_launch` in `use_ability/tests/target_validity.rs`.

## Pet command seams (PT-04)

The owner's pet commands (cell methods 88-90, `cell_methods/player/pet/`) carry a client-supplied pet id. The ownership guard (CAT-C-11 / #462) resolves it through `SpaceManager::owned_pet`, which logs every mismatch once on target `pets.command` at `debug!` (`event = "ownership_rejected"`, `reason`, the caller as `caller_id` plus its `account_id` / `player_id`). DEBUG, because a client can name any id at will. The handler adds no second row, but the refusal is never silent: the caller also gets `onErrorCode`.

| `reason` | Meaning | `onErrorCode` |
|---|---|---|
| `not_owner` | The id is another player's pet. `owner_id` names the real owner | 236 `IsNotPetOwner` |
| `not_a_pet` | The id is an NPC, a player, or nothing | 236 |
| `pet_gone` | The registry still lists the pet, but its entity is gone (the teardown sweep has not run) | 190 `DoesNotHavePet` |
| `owner_identity_mismatch` | The caller holds the owner's entity id but is not the player who summoned the pet: the id was reused before the sweep (#870) | 236 |

Refusals after the guard use the same target. Two stay at WARN because an operator should see them: `ability_not_in_list` for an ability the server has a definition for (a stale bar or a seed bug), and `cast_refused` (a cast every pre-check passed but `handle_use_ability` still refused). Everything else logs at DEBUG. That covers ordinary play (a cooldown, a friendly or out-of-range target, a wall in the way, a slot 4 or 5 from the small pet bar) and values only a forged packet sends (an ability id with no definition, a target in another space, a bad toggle value). The split follows `useAbility`'s not-known path, which logs undefined ids at DEBUG so a client cannot flood the WARN index. Each refusal also sends the owner an `onErrorCode` paired with a `CHAN_FEEDBACK` chat line (`pets::order_feedback_text`: the shipped client has no Lua consumer for `onErrorCode`, AT-E1), or re-sends the pet bar or stance. The target rule is the pet AI's (`npc_ai::pet::fight_refusal`), so a refused target logs `target_not_combatant` (a player, pet or SGWBeing) or `target_not_hostile` (fails `combat::player_may_attack`). The full list is in [observability.md](observability.md) (`pets.command`).

`malformed_args` is a WARN and is not throttled. A flood of short packets writes one row per packet; a Pattern D throttle keyed by `(player_id, reason)` is the follow-up if that shows up in practice.

Every `pets.command` row the handlers write carries `owner_id` (the caller's entity id) and the owner's `account_id` / `player_id` from `SpaceManager::player_identity`. `player/pet/tests/telemetry.rs` pins each `reason`, its level and those identity fields in one table-driven `LogCapture` test, and the guard's `ownership_rejected` rows in a second one.

The guards are in `cell_methods/player/pet/tests/guard.rs`. They cover another player's pet, an NPC id, a nonexistent id, a negative id, a stale registry entry and a reused owner entity id, for each command, and they fail when the `owned_pet` call is removed, weakened to "is a pet", or narrowed to the registry map without the per-pet summoner check (worknote `docs/analysis/pets/worknotes/pt-04.md`).

## Bank move and grant refusals (BV-01)

Target `bank`, all WARN. Each refusal carries the player-activity pair
(`account_id`, `player_id`) plus `entity_id`, so "player X tried to move Y
at time T and it failed" is answerable from SigNoz alone.

| `event` (also the message prefix) | `reason` | Fields |
|---|---|---|
| `move_rejected` | `source_container_not_player_movable`, `target_container_not_player_movable` (the vault reasons are in the BV-03 section) | `account_id`, `player_id`, `entity_id`, `item_id`, `type_id`, `quantity`, `stack_size`, `source_container_id`, `source_slot_id`, `target_container_id`, `target_slot_id` |
| `move_resync_skipped` | `refused_item_not_owned` (the refused move named an item the player does not own: a forged packet), `lock_timeout` (the move lock or the item's row lock could not be taken; an unlocked resend could overtake a concurrent write, so the client keeps its optimistic position until the next update of that item), `resync_read_failed` | `account_id` (when it was read before the failure), `player_id`, `entity_id`, `item_id` |
| `move_rejected` (infrastructure) | `move_lock_begin_failed`, `move_lock_failed` (the move lock or the item's row lock), `refusal_context_query_failed`, `move_lock_release_failed` | `player_id`, `entity_id`, `item_id`; `account_id` only on `move_lock_release_failed`, the one failure after the account is read |
| `grant_rejected` | `grant_into_storage_container` | `account_id`, `player_id`, `entity_id`, `type_id`, `quantity`, `target_container_id` |
| `grant_rejected` (infrastructure) | `account_lookup_failed` | `player_id`, `entity_id`, `type_id`, `target_container_id` (no `account_id`: that is what failed to load) |

The item fields of `move_rejected` are read at refusal time under the
per-player move lock and the item's row lock, so they are the item's
committed position, and they are omitted (not zero) when the player does
not own the item. The `LogCapture` guards are in
`inventory/move_/allowlist_tests.rs`, `refusal_resync_tests.rs` and
`refusal_infra_tests.rs` (the infrastructure reasons, injected with an
unreachable pool or a lock held under a short `lock_timeout`), and
`inventory/grant/vault_guard_tests.rs`. `resync_read_failed` and
`move_lock_release_failed` have no guard: each needs the connection to
fail after both locks were taken on it, which nothing can inject.

## Vault open refusals and send seams (BV-02)

Target `bank`, WARN, on the cell. Every row carries `account_id` and
`player_id` (omitted, not zeroed, when the cell does not know them) and
`entity_id`. Each refusal also sends the player a `CHAN_FEEDBACK` chat line,
because `onErrorCode` has no Lua consumer in the shipped client (AT-E1).

| `event` | `reason` | Fields |
|---|---|---|
| `vault_open_rejected` | `out_of_range` (a Banker click from beyond the interact distance, or from another space), `not_gm` (`.bank` from a player), `banker_missing` (the Banker vanished between the range gate and the arm) | `banker_id`, `distance` (absent when the Banker is in another space or gone) |
| `vault_open_send_failed` | `base_channel_closed` (the `onVaultOpen` send to the base failed: the session is open but the window never appeared) | `banker_id`, `error` |
| `bank_feedback_send_failed` | `base_channel_closed` (a refusal line could not be queued) | `error` |

A lookup miss on the player's own entity is `vault_open_rejected` with
`reason = player_entity_missing`, `entity_id` and `banker_id`, and no
`account_id` or `player_id`, which are read from the entity that is missing
(BV-02 logged it on the crate's own target; BV-03 moved it under `bank` so a
query on the target finds every open refusal). The `LogCapture`
guards are in `cell-interactions` `cell/interactions/bank/telemetry_tests.rs`
(one per reason and seam) and `cell-console` `console/tests/bv02_bank.rs`
(`not_gm`).

## Team and Command vault open refusals (BV-07)

Target `bank`, WARN. `org_vault_open_rejected` carries `reason`, `org_id`
(when known), `org_type`, `scope`, `banker_id`, and on the base
`space_id` and `distance`. Each reason with a player to tell also sends a
`CHAN_FEEDBACK` line.

| Side | `reason` | Meaning |
|---|---|---|
| base | `not_in_org` | the player is in no Team or Command of that type |
| base | `not_a_member`, `no_such_org`, `wrong_org_type`, `player_missing` | the check under the organization lock failed |
| base | `player_unknown` | the cell sent no `player_id` |
| base | `open_query_failed` | a database error, logged at ERROR with it first |
| base | `cell_channel_closed` | the grant could not reach the cell; no window opens |
| cell | `banker_not_pinned`, `out_of_range`, `banker_missing` | the grant arrived after the player moved on |
| cell | `stale_entity`, `player_entity_missing` | the entity is another character, or gone; no line |

A refused Team or Command vault move is `org_move_rejected` (WARN,
`bank`), with the reasons listed in `docs/gameplay/inventory-system.md`
§ "Moving items in and out of a Team or Command vault", a line, and a
snap-back under the move's locks; `move_lock_begin_failed` and
`move_lock_failed` mean the snap-back was skipped. A committed move whose
vault rows could not be read back is `org_move_resync_failed
reason=vault_read_failed`. Guards: `base-methods`
`inventory/org_vault/tests/moves.rs`, one per reason.

A closed base channel on the cell's request is `vault_open_send_failed
reason=base_channel_closed` with the `scope`. The `LogCapture` guards are
`cell-interactions` `bank/org_open_tests.rs` and `bank/telemetry_tests.rs`,
and `base-methods` `inventory/org_vault/tests/open.rs`.

## Treasury transfer refusals (BV-08)

Target `bank`, WARN. `org_cash_rejected` carries `reason`, `account_id`,
`player_id`, `entity_id`, `org_id`, `direction`, `amount` and the balances
the refusal read (`player_cash_before` / `_after`, `org_cash_before` /
`_after`, equal because nothing moved). Every reason but `actor_mismatch`
sends a `CHAN_FEEDBACK` line; the table is in
`docs/gameplay/organization-system.md` § "The treasury". `zero_amount` is
logged on the cell, the rest on the base. `no_permission` adds `perm`
(`DepositCash` or `WithdrawCash`) and `permissions`; `query_failed` adds
`error`. The `LogCapture` guards, one per reason: `base-session`
`org_cash/tests/{refusals,transfers,race}.rs`, `base-world-entry`
`tests_dispatch_arms/org_arms.rs` (`db_unavailable`, `actor_mismatch` at
the arm), and `cell-methods` `organization/tests/router.rs`
(`zero_amount`).

## Team vault expansion refusals (BV-09)

Target `bank`, WARN `expand_rejected` with `scope`, `trigger = gm_console`,
`offered_slots`, and the organization's fields once read (`org_id`,
`org_type`, `rank`, `vault_slots`, `price`, `org_cash`). Every reason sends
a line; the table is in `docs/gameplay/inventory-system.md` § "Expanding
the Team vault". The `LogCapture` guards: `base-methods`
`inventory/org_vault/tests/expand/{purchase,refusals}.rs` (one per base
reason but `price_missing`, `row_changed`, `wrong_org_type`, `no_such_org`
and `player_missing`, which need shared seed rows or a racing delete; the
module doc says why), `base-world-entry` `tests_dispatch_arms/bank_arm.rs`
(`db_unavailable` at the arm), and `cell-console`
`tests/bv09_orgvaultexpand.rs` (`not_gm`, `bad_args`).

## Vault moves, use and removal (BV-03)

Target `bank`. The cell attaches a vault verdict to every forwarded
inventory request (`VaultAccess`); the base logs what it did with it. Every
bank refusal also sends a `CHAN_FEEDBACK` line before the snap-back.

| `event` | Level | `reason` | Fields |
|---|---|---|---|
| `move_rejected` | WARN | the verdict's label: `no_vault_session`, `banker_out_of_range`, `banker_gone`, `banker_other_space`, `vault_session_other_space`, `player_missing`, `vault_scope_mismatch`; and `target_slot_beyond_bank_slots`, `mission_item_not_bankable`, `item_not_allowed_in_container`, `split_onto_occupied_slot` | the BV-01 fields, plus `vault_end` (`source` or `target`), `banker_id`, `distance`, `gm_override`, and `bank_slots` on the slot refusal |
| `move_accepted` | DEBUG | none (success) | `account_id`, `player_id`, `entity_id`, `item_id`, `type_id`, `quantity`, `kind`, source and target container and slot, `source_stack_before`/`after`, `target_stack_before`/`after`, `bank_slots`, `banker_id`, `distance`, `gm_override` |
| `use_rejected` | WARN | `container_not_accessible` (a use or removal of an item in buyback, the org vaults, or the personal vault without an open verdict) | `account_id`, `player_id`, `entity_id`, `item_id`, `container`, `op` (`use` or `remove`), `vault_reason`, `banker_id` |
| `use_rejected` (infrastructure) | WARN | `account_lookup_failed` | `player_id`, `entity_id`, `item_id` (no `account_id`: that is what failed to load) |
| `bank_feedback_send_failed` | WARN | `no_client_address` (a refusal line had no session address to go to) | `player_id`, `entity_id`, `item_id` |

The `LogCapture` guards are in `inventory/move_/vault_move_tests.rs`,
`vault_move_shape_tests.rs`, `vault_refusal_tests.rs` and
`allowlist_tests.rs` (every `move_rejected` reason the base produces, and
`move_accepted`), and `inventory/core/access_tests.rs` (`use_rejected`, its
infrastructure reason with an unreachable pool, and `no_client_address`).
The verdict labels the base passes through are each pinned where they are
made, in `cell-interactions` `bank/tests.rs`
(`vault_access_maps_every_verdict`); `vault_scope_mismatch` in the `wire` and
`container_policy` unit tests.

## Grant and loot hand-back seams

Target `inventory`. A loot pickup takes the item off the corpse before the
base writes it to the looter's inventory, so every way the grant can fail
is a seam: the item goes back on the corpse, or its loss is logged.

| `event` (also the message prefix) | Level | `reason` | Fields |
|---|---|---|---|
| `grant_refused` | INFO | `container_full`, `database_error`, `not_grantable_container` (the grant resolved to buyback, 16) | `account_id`, `player_id`, `entity_id`, `type_id`, `quantity`, `container_id` |
| `grant_outcome_unknown` | WARN | `commit_outcome_unknown` | as `grant_refused` |
| `lookup_failed` | WARN | (`phase = placement`) | `player_id`, `entity_id`, `type_id`, `requested_container_id`, `error` |
| `loot_restored` | INFO | the refusal: `storage_only`, `container_full`, `no_database`, `database_error` | `account_id`, `player_id`, `entity_id`, `corpse_id`, `index`, `type_id`, `qty`, `container_id`, `reflagged` |
| `loot_restore_failed` | WARN | `corpse_gone`, `corpse_changed`, `index_taken` (cell); `cell_channel_closed` (base) | as `loot_restored`, plus `refusal` |
| `loot_restore_skipped` | WARN | `commit_outcome_unknown` | `account_id`, `player_id`, `entity_id`, `corpse_id`, `index`, `type_id`, `qty` |
| `loot_grant_send_failed` | WARN | `restored`, or the restore miss | `player_id`, `entity_id`, `corpse_id`, `index`, `type_id`, `qty`, `restored` |
| `feedback_send_failed` | WARN | `send_error` | `account_id`, `player_id`, `entity_id` |

Only a refusal raised before the commit is handed back. A `COMMIT` that
failed with a server error rolled back and is handed back too; one that
failed without an answer (I/O, a closed pool) may have landed, so the item
stays off the corpse (`loot_restore_skipped`) rather than risk a second
copy. The `LogCapture` guards are in `inventory/grant/fall_through_tests.rs`,
`inventory/grant/loot_refusal_tests.rs`, `vendor/purchase/tests.rs` and the
cell's `interactions/loot/restore_tests.rs`. `grant_outcome_unknown`,
`loot_restore_skipped`, `lookup_failed` and `feedback_send_failed` have no
`LogCapture` guard: each needs a connection or channel to fail at one
exact step, which nothing can inject; the commit classification itself is
unit-tested (`persist::tests`).

## Native consumable seams (decision 27)

A heal or buff item used from the bags ([consumables.md](../gameplay/consumables.md)). The cell rows use the module's own target (`cimmeria_cell_content::cell::content::consumable_use`), the base rows `cimmeria_base_methods::...::consume_for_use`, the stat-buff rows `abilities`. Every row carries `entity_id`, `account_id` and `player_id`; the use rows also `type_id` and, when known, `instance_id` and `ability_id`.

| `event` | Level | `reason` | Extra fields |
|---|---|---|---|
| `consumable_refused` | INFO | `already_at_max`, `dead` | `stat_id`, `stat_cur`, `stat_max` (at max). The player also gets `onErrorCode` and a chat line |
| `consumable_skipped` | DEBUG | `placeholder_ability` (the 597 filler), `effect_not_native`, `no_ability_def`, `no_effects` | chains decide the use |
| `consumable_skipped` | INFO | `chain_owns_item` | an `item_use` chain owns the item |
| `consumable_skipped` | WARN | `no_instance_id` | an `ItemUsed` without an instance; nothing consumed |
| `consumable_consume_send_failed` | ERROR | `cell_to_base_closed` | `error` |
| `consumable_feedback_send_failed` | WARN | `cell_to_base_closed` | `method_index`: the refusal could not be shown |
| `consumable_consume_refused` (base) | INFO | `not_removed` | `remove_instance` logged the cause (row gone, another type, inaccessible container) |
| `consumable_apply_send_failed` (base) | ERROR | `cell_channel_closed` | a consumed unit whose effect is lost |
| `consumable_apply_skipped` | WARN | `entity_gone`, `no_native_plan`, `dead_at_apply` | a consumed unit whose effect was not applied |
| `stat_buff_skipped` | WARN | `no_stat_nvps`, `no_duration`, `target_missing`, `stat_missing` | `effect_id`, `stat_id`: a seed defect |

The success rows are `consumable_used` (INFO, before and after HEALTH and FOCUS), `consumable_consumed` (base, DEBUG), and `stat_buff_applied` / `stat_buff_removed` (INFO, `reason` = `expired`, `replaced`, `removed` or `died`, with `stat_before` / `stat_after`). `LogCapture` guards: `the_refusal_logs_reason_already_at_max` and `a_user_who_died_before_the_consume_landed_is_not_healed` in `cell/content/consumable_use_tests.rs`, and the `no_stat_nvps`, `no_duration` and `replaced` rows in `cell-world`'s `effects/stat_buff/tests.rs`.

## Related

- [TESTING.md](../../TESTING.md) — Test-type picker; regression-guard rules.
- [CLAUDE.md](../../CLAUDE.md) — Doc-update map; pre-PR checklist.
- [crates/services/src/test_support.rs](../../crates/services/src/test_support.rs)
  — `LogCapture` helper used by guards.
