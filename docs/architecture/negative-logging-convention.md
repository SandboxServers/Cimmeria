# Negative-Logging Convention

> **Last updated**: 2026-10-04
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
| `player_id` | when applicable | The affected player. No aliases (`pid`). Log it, and every other numeric id (`design_id`, `account_id`, `entity_id`), **as a number**: pass an `Option<i32>` as the value itself (`player_id = e.player_id`), which omits the field when `None`, never `?e.player_id` or `%`. SigNoz types an attribute by the values it receives, so one Debug-formatted `"Some(72)"` creates a second, string-typed `player_id` key that a `player_id = 72` filter silently misses (2026-09-29: the bandolier equip-display rows; `design_id: "Some(5224)"` on `Player looted item`). |
| `entity_id` | when applicable | The affected entity. No aliases (`eid`). |
| `mob_id` / `mission_id` / `chain_id` / `step_id` / `space_id` / `cell_id` | when applicable | Canonical names per the existing logging surface. |
| `<prefix>_name` next to every ID (`ability_name`, `item_name`, `template_name`, `player_name`, `account_name`, `target_name`, …) | always, when the name resolves | The ID's name, under the same prefix, per [instrumentation-discipline.md § Rule 6](instrumentation-discipline.md#rule-6--every-id-field-is-paired-with-its-name), which holds the full key table and its exceptions. Pass an `Option<&str>` so an unresolved name is left out; never `"unknown"`, `""` or `"None"`. Use `player_name`, not `character_name`. Social sweep renames (NT-26): `type_id` on items is `item_type_id`, `item_def_id` is `item_type_id`, the chat `target` text is `chat_target`, and a Black Market `sequence_id` logged as an auction is `auction_id`. An NPC line carries `entity_name` (or `npc_name` beside `npc_id`) **and** the `template_id` + `template_name` pair. An ID with nothing to name is marked `// nt:id-only <reason>`. |
| `<key>_names` next to every bitflag word (`state_field_names`, `flags_names`, `interaction_flags_names`, `recipient_flags_names`, `from_mask_names`, …) | always, beside the raw word | The set bits' names joined `A\|B\|C` in the enum's order, from the flag set's one `FlagSet` table (`cimmeria_common::flag_names`): `STATE_FLAGS` (`cimmeria_wire::state_field`), `EFFECT_FLAGS` (cell-combat `cone_aoe`), `INTERACTION_FLAGS` (`cimmeria_entity::interaction_flags`), `MAIL_FLAGS` (`cimmeria_wire::cell::mail::codes::flags`), `ORG_PERMISSIONS` (`cimmeria_entity::organization`), `PACKET_FLAGS` (`cimmeria_mercury::packet`). Log it with `%`: `state_field, state_field_names = %STATE_FLAGS.render(state_field)`. Bits no token names render as one hex remainder (`BSF_Dead\|0x200`), and a zero word renders `0x0`. Each table is pinned to `entities/defs/enumerations.xml` by a test. A numeric enum code gets a name the same way: `error_code` pairs with `error_name` (`cimmeria_names::book().error_code`), and a code from another vocabulary logs its Rust enum with `?` under its domain's key (the Black Market's `bm_error = ?error`). Leave `_names` off rows exported per packet or per tick: the OTLP appender allocates a string per field per event. NT-31. |
| `<domain>_arg` (`design_arg`, `mission_arg`, `template_arg`, `name_or_id_arg`), `target_raw` | GM console input that failed to parse | The raw text or out-of-range number a GM typed where an ID was expected. It is not an ID, so it has no name and never takes an `_id` key: a typed `DesignId` that isn't a number is logged as `mission_arg` / `template_arg` / `design_arg` (item), never `design_id`. A parsed one is logged under its domain key (`mission_id`, `template_id`, `item_type_id`) with its name (NT-27). |
| `account_name` on login lines | when the account resolved | The login name beside `account_id`. The auth (`auth/handlers.rs`) and Phase 3 (`base/login`) lines that resolved an account log it as `account_name`; NT-24 retired their old `user` key. A failed login with no matching account has no `account_id` and keeps `user`: the name the client typed, which may name no account. Never a password, key or ticket (see [Credential fields](#credential-fields)). |
| Names for closed enum IDs (`bag_name`, `stat_name`, `target_type_name`, `event_name`) | beside the ID | A small client enum logged as an ID is named from its `entities/defs/enumerations.xml` token, through one table each, pinned to the XML by a test: `cimmeria_entity::inventory::bag_name` (`EInventoryContainerId`, beside `bag_id`), `cimmeria_entity::stats::stat_name` (`EStats`, beside `stat_id`), `cimmeria_entity::abilities::target_type_name` (`ETargetType`, beside `target_type_id`) and `sequence_event_name` (`ESequenceEventType`, beside `event_id`). Content IDs that may be `None` resolve through `cimmeria_cell_world::cell::effects::content_names` (`ability_name`, `effect_name`, `template_name`, `item_name`). NT-20. |
| `rows_affected` + `expected` | always paired on DB writes | Pair so a single ops query catches divergence. |
| `phase` | optional | Short string naming a sub-step (e.g. `"create_base"` \| `"cascade"`). |
| `reason` | optional | Short string naming why the expectation was unmet (e.g. `"entity_to_addr_miss"`, `"oneshot_dropped"`, `"rows_affected_zero"`). An expected miss logs at DEBUG under its own reason: a witness-send miss for a witness whose session just ended is `"witness_session_ended"`, not a WARN. |
| `world` | when the seam is space-scoped | The **world name**, not only `space_id`. A space id is a runtime allocation that means nothing outside the running process, so a log carrying only `space_id` cannot be grouped by zone after the fact. Pair them — `space_id` still identifies the instance. `world` is Rule 6's name key for `space_id` and `world_id`; don't use `world_name`. |
| `chain_name` | beside every `chain_id` | `content_chains.description`, from `cimmeria_names::book().chain` or, inside the content engine, `Chain::label`. A chain with no description leaves the field off (the loader no longer invents `chain_<id>`). NT-21 |
| `suppressed` | required on a Pattern D seam | Count of occurrences elided since this seam last emitted for this entity. `0` on the first row of an episode. |

Keys the named-telemetry sweeps renamed while pairing them (Rule 6), so a saved query on the old key finds nothing after the change:

| Event | Old key | New key | Sweep |
|---|---|---|---|
| `navmesh_mode_summary` (target `movement.navmesh`, startup) | `world_name` | `world` | NT-23 |
| `AoI: dynamicUpdate InteractionType (base→merged)` | `player_id` (held the witness's entity id) | `witness_id` + `witness_name` | NT-23 |
| `TeleportPlayer: persistence UPDATE matched 0 rows` / `failed to persist position` | `pid` | `player_id` + `player_name` | NT-23 |
| `Content: adding dialog set` / `removing dialog set` and the other `AddDialogSet` / `RemoveDialogSet` executor rows | `dialog_set_id` (always held a `dialog_set_maps` row) | `dialog_set_map_id` + `dialog_set_map_name` (the topic text) | NT-21 |
| `consumable_*` cell rows, the `RemoveItem` executor rows, `content.send_system_mail`, `fire_item_equipped` | `type_id` | `item_type_id` + `item_name` | NT-21 |
| Cell loot rows: `Player looted item`, `loot_restored`, `loot_restore_failed`, `loot_grant_send_failed` | `type_id`, `design_id` | `item_id` + `item_name` | NT-21 |
| The interact dispatcher's target-resolved row | `tmpl_id` | `target_template_id` + `target_template_name` | NT-21 |
| `send_loot_display`, the vendor open rows, `send_dialog_display` | `player_id` (held the player's entity id) | `entity_id` + `entity_name` | NT-21 |
| The `spawn_entity` executor rows and the DHD rows | `world_name` | `world` | NT-21 |
| The spawn rows in `space_manager/npc_population.rs` | `name` (held the template name) | `template_name` | NT-25 |
| `npc_respawn_recreate` (`npc_respawn` tick) and the stargate loader (`spawner/stargates.rs`); also the `npc_respawns_total` metric label | `world_name` | `world` | NT-25 |
| The ability loader (`spawner/abilities.rs`) | `type_id` (an `abilities.type_id` enum label) | `ability_type` | NT-25 |
| `Loaded player data for mapLoaded` (`player_load/core/player_data.rs`) | `name` (held the character name) | `player_name` | NT-28c |
| `Added Cimmeria item definition` (`resources/apply_overrides.rs`, boot) | `name` (held the item name) | `item_name` | NT-50a |
| `triggerClientHintedGenericRegion` (accepted) | `tag` | `region_name` (left off when the tag is blank) | NT-28a |
| `triggerClientHintedGenericRegion refused` | `region_tag` | `region_name` (left off when the region is unknown) | NT-28a |
| `interact: no items_event_sets binding` (`event = weapon_unbound`) | `item_id` (held the weapon's design id) | `item_type_id` + `item_name` | NT-28a |
| `setRingTransporterDestination` | `destination_id` | `destination_region_id` + `destination_region_name` | NT-28a |
| `PersistPosition` rows (`cell_dispatch/position.rs`) | `position` (the coordinate array, `?position`) | `x`, `y`, `z` (floats) | TG-DB-04 |
| `useItem` and `UseInventoryItem send to base failed` | `target_id` (the client's raw target) | `wire_target_id` + `wire_target_name` | NT-28a |
| `friction: server sees the player inside a region…` (`signal = region_dwell_no_hint`) | `region_tag` | `region_name` | NT-28a |

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
| Phase 2 SID consumption in `auth/handlers.rs::handle_server_selection` | `session_ip_mismatch` | `account_id`, `account_name`, `sid_prefix`, `session_ip`, `client_ip` |
| Phase 3 ticket consumption in `base/login/mod.rs::handle_login` | `ticket_ip_mismatch` | `account_id`, `account_name`, `ticket_ip`, `client_ip` |

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

## Database startup and credential-check seams

A configured database (`DB_URL` not empty) is required. The server
refuses to start without it, and Phase 1 never accepts a credential it
could not check. These rows say which state the server is in:

| Seam | Level | `reason` | Meaning |
|---|---|---|---|
| `orchestrator_database.rs::connect_database` | `error!` | `database_connect_failed` | `DB_URL` is set and the connection failed or timed out (refused, wrong user, password or database). `start_all` returns `DatabaseFailed` and the process exits 1. Field: `error`. |
| `orchestrator_database.rs::connect_database` | `warn!` | `no_database_configured` | `DB_URL` is empty: the server starts with no database. Field: `developer_mode`. |
| `AuthService::start` (once) and `handle_user_auth` (each login) | `warn!` | `dev_mode_no_db_login` | Developer mode with no database configured: every well-formed login is accepted unchecked as account 1, access level 99. Developer use only. |
| `AuthService::start` (once) and `handle_user_auth` (each login) | `error!` | `db_pool_missing` | A database is configured but the auth service has no pool. The orchestrator never starts this way, so it is a wiring fault; every login is refused with code 10. |
| `AuthService::start` (once) and `handle_user_auth` (each login) | `error!` | `no_database` | No database and developer mode off: every login is refused with code 10. |

The guards are the `start_all_fails_when_configured_database_*` tests in
`crates/services/src/orchestrator_database.rs` and the
`dev_mode_*` / `no_dev_mode_*` tests in `crates/auth/src/auth/handlers.rs`.

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
| `no_witnesses` | the Ability_End went out to zero AoI witnesses, and a player is present (a player has the shooter within their AoI radius (the target's range does not count: a player who sees only the target cannot see the shooter), or the target or a threat-list entry is a player or a player's pet, deployable or lab dummy: `SpaceManager::player_present`, DA-F2). An NPC shooting an NPC with no player around writes nothing | same, plus `sequence_id` |
| `stance_not_announced` | the Ability_End reached a witness but the NPC's `BSF_InCombat` stance had not (the shot came from outside the Fighting pass, or the stance sync regressed): the client draws no muzzle flash, tracer or weapon sound | same, plus `sequence_id`, `witness_count` |

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
fight tick). The stance row is guarded by `use_ability/tests/sequence.rs`
(`an_npc_shot_before_its_stance_was_announced_warns`) and
`service/tests/npc_ai/combat_stance.rs` (a real fight tick announces
`BSF_InCombat` before the shot, once per fight, and announces the clear). The seed side is linted by the live-DB
`spawner/tests/npc_ability_animation.rs`.

## Encrypted-channel decrypt rejects

A datagram from a registered session that fails to decrypt is dropped
and the session stays up. The legacy C++ `EncryptionFilter` did the
same, and a teardown here would let anyone who can spoof the client's
source address end the session with one garbage datagram. The seam is
`base/connect_loop/encrypted/decrypt_reject.rs`, and it logs at `warn!`:

| `reason` | Meaning | Fields |
|---|---|---|
| `login_retry_on_channel` | The datagram is the client's **plaintext** `baseAppLogin` arriving after the server registered the encrypted channel. The client retries every 300 ms until its login reply handler finishes, so a train of these means the server replied and the client never completed the login. `reply_outstanding = true` means the server has not seen the client ACK the reply (seq 1): it was probably lost, and a resend is pending. `false` means the client acked it and is retrying anyway. Look client-side, not at the keys. | `addr`, `account_id`, `account_name`, `raw_len`, `reply_outstanding` |
| `decrypt_fail` | Anything else that fails the length, HMAC, or padding check: a key mismatch, a stale session, or a forged or corrupted packet. | `addr`, `account_id`, `account_name`, `raw_len`, `error` |

These rows carry `reason`, never `disconnect_reason`. That field is kept
for rows that report a real teardown, such as `session.end` and
`Client entities cleaned up`. A per-session disconnect query must not
count dropped datagrams.

Only a login whose ticket is already consumed reaches
`login_retry_on_channel`. A plaintext `baseAppLogin` with a fresh ticket
on an established channel is a client killed and relaunched on its fixed
UDP port, and goes to `handle_login` instead
([login-handshake.md](../protocol/login-handshake.md#a-client-relaunched-on-the-same-addressport)).
Its rows:

| `reason` | Level | Meaning | Fields |
|---|---|---|---|
| `relaunch_takeover` | `info!` if `old_idle_ms >= 15000`, else `warn!` | The fresh ticket is for the live session's account: the old session is evicted and the new login accepted. One row per takeover; the old session's `session.end` row carries `disconnect_reason = relaunch_takeover`. INFO means the old session had gone silent (a crash and relaunch); WARN means it was still receiving packets. | `addr`, `account_id`, `account_name`, `player_id`, `player_name`, `old_session_secs`, `old_idle_ms`, `old_in_world` |
| `duplicate_login` | `info!` if `old_idle_ms >= 15000`, else `warn!` | A fresh login for an account whose live session sits on another address evicts it, after a `LOGGED_OFF` to that address. The same INFO/WARN split as `relaunch_takeover`: INFO for a crashed client that relaunched on a new port, WARN for a real concurrent login. | `account_id`, `account_name`, `player_id`, `player_name` (the old session), `old_addr`, `addr`, `old_session_secs`, `old_idle_ms`, `old_in_world` |
| `send_failed` (on `LOGGED_OFF`) | `debug!` | The `LOGGED_OFF` sent to a displaced session on another address failed. The eviction goes ahead regardless. | `old_addr`, `error` |
| `relaunch_account_mismatch` | `warn!` | The fresh ticket is for another account and may not displace the live session. Refused, the ticket burned, the live session kept. A spoofed source address looks like this. Also `error!`, with only the ticket fields, when the session lock is poisoned. | `addr`, `account_id`, `account_name`, `player_id`, `player_name` (the live session), `ticket_account_id`, `ticket_account_name`, `ticket_ip_matches` |
| `address_reclaimed` | `warn!` | The session on the address is another account's that registered with a ticket issued to a different IP, and this login's ticket was issued to this address's IP: a squatter on a spoofed address. It is evicted (its teardown carries `disconnect_reason = address_reclaimed`) and the login accepted. | `addr`, `account_id`, `account_name`, `player_id`, `player_name` (the squatter), `ticket_account_id`, `ticket_account_name` |
| `ticket_expired` | `warn!` | Any Phase 3 login whose ticket is older than `TICKET_TTL` (30 s). The ticket is burned and nothing registered. | `addr`, `account_id`, `account_name`, `ticket_age_ms` |
| `address_still_occupied` | `error!` | A login found a session it may not displace on its address, before eviction (`stage = before_eviction`) or still there after it (`after_eviction`), and was refused rather than orphan it. The address claim rules this out, so a row means a bug. | `addr`, `account_id`, `account_name`, `stage` |
| `session_replaced` | `info!` | A task that resolved the address before a takeover found a newer session there and left it alone: the old tick-sync loop's teardown (with `disconnect_reason`), or a gate-transfer step (with `entity_id` and `step`: `reset_entities`, `store_pending_world_entry` or `abandon_unspaced_session`). | `addr`, `account_id`, `account_name`, `player_id`, `player_name` (the new session), plus `disconnect_reason` or `entity_id` and `step` |

## Cross-space cast refusal (#906)

`useAbility` resolves its target id with `SpaceManager::get_entity`, which searches every space. A player's cast at a target that is not in the caster's space is refused in `use_ability/fire_los.rs` (at launch, which is also the fire for a zero-warmup cast) with one row on target `abilities` at `debug!`: `event = "cast_refused"`, `reason = "target_other_space"`, `entity_id`, `account_id`, `player_id`, `ability_id`, `target_id`, `caster_space_id`, `target_space_id`, `error_code`. DEBUG, because only a forged or stale packet names such a target, and a WARN would let it flood the log. The refusal is not silent: the caster gets `onErrorCode(0, ability_id, 0)`. If that cannot be queued, a WARN `cast_refused_send_failed` with the same identity fields says so. The guard is `a_target_in_another_space_is_refused_at_launch` in `use_ability/tests/target_validity.rs`.

## Abandoned reliable resends (#842)

Every reliable packet the base sends resends until it is acked, except
the two login handshake packets: the reply (seq 1) and the time-sync
(seq 2) are capped at `HANDSHAKE_RETRANSMIT_CAP` (6) resends, because
some clients never ack them. When a capped packet expires again at its
cap, the channel drops it from the TX window and
`collect_pending_retransmits` (`base-session` `base/helpers`) logs one
`warn!` for it, never repeated:

| `event` | `reason` | Fields |
|---|---|---|
| `reliable_resend_abandoned` | `retransmit_cap_reached` | `addr`, `account_id`, `account_name`, `seq`, `retransmit_count` |

A row with `seq = 1` means the client never acked the login reply
through seven sends. If the login also stalled, look for
`login_retry_on_channel` rows from the same `addr`. A row with `seq = 2`
on a session that played normally is a client that ignores the
time-sync ACK, not a fault. `login/tests/handshake_retransmit.rs` pins the
cap, the fields and the one-row-per-packet rule.

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

Every ID on these rows, and on the BV-03, BV-07 and grant rows below,
carries its Rule 6 name when the server knows it: `account_name`,
`player_name`, `entity_name` (the player's character), `item_name`,
`org_name`, and `container_name` under each container key's prefix
(`target_container_name`: `MAIN`, `BANK`, ...). The base names players,
accounts and organizations from `cimmeria_entity::known_names`, filled at
login, `playCharacter` and the organization row reads, so a helper deep in
a transaction can name them without the session. An instance `item_id`
whose type the line has not read yet, slot indexes, the banker and the
corpse (NPC entities the base cannot name) are marked `// nt:id-only`.
`type_id` became `item_type_id` on every base inventory row (NT-22).

| `event` (also the message prefix) | `reason` | Fields |
|---|---|---|
| `move_rejected` | `source_container_not_player_movable`, `target_container_not_player_movable` (the vault reasons are in the BV-03 section) | `account_id`, `player_id`, `entity_id`, `item_id`, `item_type_id`, `quantity`, `stack_size`, `source_container_id`, `source_slot_id`, `target_container_id`, `target_slot_id` |
| `move_resync_skipped` | `refused_item_not_owned` (the refused move named an item the player does not own: a forged packet), `lock_timeout` (the move lock or the item's row lock could not be taken; an unlocked resend could overtake a concurrent write, so the client keeps its optimistic position until the next update of that item), `resync_read_failed` | `account_id` (when it was read before the failure), `player_id`, `entity_id`, `item_id` |
| `move_rejected` (infrastructure) | `move_lock_begin_failed`, `move_lock_failed` (the move lock or the item's row lock), `refusal_context_query_failed`, `move_lock_release_failed` | `player_id`, `entity_id`, `item_id`; `account_id` only on `move_lock_release_failed`, the one failure after the account is read |
| `grant_rejected` | `grant_into_storage_container` | `account_id`, `player_id`, `entity_id`, `item_type_id`, `quantity`, `target_container_id` |
| `grant_rejected` (infrastructure) | `account_lookup_failed` | `player_id`, `entity_id`, `item_type_id`, `target_container_id` (no `account_id`: that is what failed to load) |

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
the arm), and `cell-org` `organization/tests/router.rs`
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
| `move_accepted` | DEBUG | none (success) | `account_id`, `player_id`, `entity_id`, `item_id`, `item_type_id`, `quantity`, `kind`, source and target container and slot, `source_stack_before`/`after`, `target_stack_before`/`after`, `bank_slots`, `banker_id`, `distance`, `gm_override` |
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

Target `cimmeria_base_methods::base::world_entry::methods::inventory::ammo` (module path). A loot pickup takes the item off the corpse before the
base writes it to the looter's inventory, so every way the grant can fail
is a seam: the item goes back on the corpse, or its loss is logged.

| `event` (also the message prefix) | Level | `reason` | Fields |
|---|---|---|---|
| `grant_refused` | INFO | `container_full`, `database_error`, `not_grantable_container` (the grant resolved to buyback, 16) | `account_id`, `player_id`, `entity_id`, `item_type_id`, `quantity`, `container_id` |
| `grant_outcome_unknown` | WARN | `commit_outcome_unknown` | as `grant_refused` |
| `lookup_failed` | WARN | (`phase = placement`) | `player_id`, `entity_id`, `item_type_id`, `requested_container_id`, `error` |
| `loot_restored` | INFO | the refusal: `storage_only`, `container_full`, `no_database`, `database_error` | `account_id`, `player_id`, `entity_id`, `corpse_id`, `index`, `item_id` + `item_name` (the item type; `type_id` before NT-21), `qty`, `container_id`, `reflagged` |
| `loot_restore_failed` | WARN | `corpse_gone`, `corpse_changed`, `index_taken` (cell); `cell_channel_closed` (base) | as `loot_restored`, plus `refusal`; the base row's item key is `item_type_id` |
| `loot_restore_skipped` | WARN | `commit_outcome_unknown` | `account_id`, `player_id`, `entity_id`, `corpse_id`, `index`, `item_type_id`, `qty` |
| `loot_grant_send_failed` | WARN | `restored`, or the restore miss | `player_id`, `entity_id`, `corpse_id`, `index`, `item_id` + `item_name` (`type_id` before NT-21), `qty`, `restored` |
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

## Silent loot drops (TG-ITM-09)

Target `interactions`. The cell takes a loot item off a corpse before it hands
it to the base, so three losses on the cell side were silent: the corpse gone
after its item left the list, a cash grant that never reached the base, and an
`onLootDisplay` send that failed. Each is a WARN now. `loot_item_lost` is
defensive: the corpse was found a moment earlier with no await between, so it
is not expected to fire. A container that vanished before its roll is a
`loot.container_refused` INFO row with `reason = container_gone` (the content
executor, `open_loot`). That row is also defensive and logs only; it is
unreachable on a press because the range check before it already refuses a
missing container with the out-of-range feedback line.

| `event` (also the message prefix) | Level | `reason` | Fields |
|---|---|---|---|
| `loot_item_lost` | WARN | `entity_missing` | `account_id`, `player_id`, `entity_id`, `target_eid`, `index`, `item_id` + `item_name`, `quantity` |
| `loot_cash_lost` | WARN | `base_channel_closed` | `account_id`, `player_id`, `entity_id`, `target_eid`, `index`, `amount` |
| `loot_display_lost` | WARN | `base_channel_closed` | `entity_id`, `entity_name`, `npc_entity_id`, `npc_entity_name` |

The guard for the cash row is `loot/tests.rs`, `loot_cash_send_failure_warns`.

## Native consumable seams (decision 28)

A heal or buff item used from the bags ([consumables.md](../gameplay/consumables.md)). The cell rows use the module's own target (`cimmeria_cell_content::cell::content::consumable_use`), the base rows `cimmeria_base_methods::...::consume_for_use`, the stat-buff rows `abilities`. Every row carries `entity_id`, `account_id` and `player_id`, each with its name; the cell's use rows also `item_type_id` + `item_name` (`type_id` before NT-21) and, when known, `instance_id` and `ability_id` + `ability_name`; the base rows carry `item_type_id` + `item_name` (NT-22).

| `event` | Level | `reason` | Extra fields |
|---|---|---|---|
| `consumable_refused` | INFO | `already_at_max`, `dead` | `stat_id`, `stat_cur`, `stat_max` (at max). The player also gets `onErrorCode` and a chat line |
| `consumable_refused` | WARN | `consumable_not_implemented` | `item_id` (the instance), `cause` (why the ability is not native). A bag consumable (`{1,17}`) with no chain whose effect is not implemented; the player gets the chat line only. Mission items and the 597 filler never log it |
| `consumable_skipped` | DEBUG | `placeholder_ability` (the 597 filler), `effect_not_native`, `no_ability_def`, `no_effects` | chains decide the use |
| `consumable_skipped` | INFO | `chain_owns_item` | an `item_use` chain owns the item |
| `consumable_skipped` | WARN | `no_instance_id` | an `ItemUsed` without an instance; nothing consumed |
| `consumable_consume_send_failed` | ERROR | `cell_to_base_closed` | `error` |
| `consumable_feedback_send_failed` | WARN | `cell_to_base_closed` | `method_index`: the refusal could not be shown |
| `consumable_consume_refused` (base) | INFO | `not_removed` | `remove_instance` logged the cause (row gone, another type, inaccessible container) |
| `consumable_apply_send_failed` (base) | ERROR | `cell_channel_closed` | a consumed unit whose effect is lost |
| `consumable_apply_skipped` | WARN | `entity_gone`, `no_native_plan`, `dead_at_apply` | a consumed unit whose effect was not applied |
| `stat_buff_skipped` | WARN | `no_stat_nvps`, `no_duration`, `target_missing`, `stat_missing` | `effect_id`, `stat_id`: a seed defect |

The success rows are `consumable_used` (INFO, before and after HEALTH and FOCUS), `consumable_consumed` (base, DEBUG), and `stat_buff_applied` / `stat_buff_removed` (INFO, `reason` = `expired`, `replaced`, `removed` or `died`, with `stat_before` / `stat_after`; since ability mechanics AB-04 also `toggled_off`, `removed_by_moniker`, `damage`, `revive`, `bandolier_swap` and `cleansed`, and an `effect_bar_overflow` INFO row when one side of the client's effect bar passes ten icons). `LogCapture` guards: `the_refusal_logs_reason_already_at_max`, `the_not_implemented_refusal_logs_its_reason` and `a_user_who_died_before_the_consume_landed_is_not_healed` in `cell/content/consumable_use_tests.rs`, and the `no_stat_nvps`, `no_duration` and `replaced` rows in `cell-world`'s `effects/stat_buff/tests.rs`.

## Disconnect-teardown `DisconnectEntity` seam (issue #999)

`destroy_client_entities` (`crates/base-session/src/base/helpers/session_teardown.rs`) used
to return the player's entity id to `EntityManager`'s free list *before*
telling the cell to tear the mirrored cell entity down, over a bare
`let _ = tx.try_send(BaseToCellMsg::DisconnectEntity { .. })`. A full or
closed Base→Cell channel dropped the notice silently, and — independent of
channel capacity — a concurrent login could recycle the freed id via
`EntityManager::allocate_id`'s FIFO free list before the cell had even seen
the disconnect, so the old session's `DisconnectEntity` landed on and
destroyed the *new* player's cell entity.

`BaseToCellMsg::DisconnectEntity` now carries a `reply_tx: oneshot::Sender<()>`
that the cell fires once `handle_disconnect_entity` runs to completion (same
shape as `CreateEntity`'s existing `reply_tx`). `destroy_client_entities`
spawns a task that sends the message and awaits that reply before returning
the id to the free list -- it does **not** await inline. The function is
called from the base's single UDP receive loop (`client_disconnect`,
`duplicate_login`) and the per-session tick-sync loop
(`inactivity_timeout`), so waiting on a cell round trip there would pause
packet intake for every connected player whenever the cell is busy or the
shared Base→Cell channel is backpressured. Either failure WARNs (no
`target:` override) with `entity_id`, `entity_name`, `account_id`,
`account_name` and `disconnect_reason`:

| Level | Message | Meaning |
|---|---|---|
| WARN | `destroy_client_entities: DisconnectEntity send failed` | The Base→Cell channel send itself failed (closed or, in practice, a receiver that has shut down) |
| WARN | `destroy_client_entities: cell dropped the DisconnectEntity reply without confirming teardown` | The send succeeded but the oneshot reply was dropped without firing (the cell task ended mid-handler) |

In both cases the id is **withheld from the free list permanently** rather
than freed on a best-effort basis: an entity id is an `i32` counter with
effectively unbounded headroom, so leaking one costs nothing, while reusing
one whose teardown was never confirmed can hand a live cell entity's identity
to a brand-new session. A caller that never returns the id to a free list in
the first place (`gate_travel::abandon_unspaced_session`, the `SGWPlayer`
`logOff` arm in `base/dispatch/session.rs` — neither holds an
`EntityManager` handle) constructs the `reply_tx` and drops the receiver
immediately; the cell's `let _ = reply_tx.send(())` is a defensible silent
send in that case; nobody is waiting.

The `LogCapture` guards are
`base-session` `base/helpers/disconnect_teardown.rs`:
`disconnect_entity_send_failure_warns_and_withholds_the_id` (the WARN and the
withheld id) and `player_entity_id_is_withheld_from_reuse_until_the_cell_confirms_teardown`
(the concurrency guard — drives `destroy_client_entities` on its own task,
intercepts the `DisconnectEntity` before replying, and proves a concurrent
`EntityManager::create_entity` call does not receive the id under teardown).

## Logout position-save seam (TG-DB-03)

On `DisconnectEntity`, the cell's `persist_last_position` (`crates/cell/src/cell/service/base_messages/lifecycle.rs`) hands a player's live position to the base as `PersistPosition`. A cell-side skip and a base-side write that never ran look the same in the database, so each outcome logs on its own. NPCs stay silent: they have no `sgw_player` row to update.

| Level | Message | Meaning |
|---|---|---|
| WARN | `DisconnectEntity: entity not found — last position not persisted` (`reason = "entity_missing"`, `entity_id`) | The entity id is no longer in the cell's space manager when the teardown runs, so there is no position to read |
| WARN | `DisconnectEntity: player has no player_id — last position not persisted` (`reason = "player_id_missing"`, the identity fields) | A player entity that never received its `player_id` (a session that dropped before character select). The position cannot be stored, and the next login resumes at the previous stored position |
| DEBUG | `DisconnectEntity: last position queued for persistence` (`event = "persist_position_queued"`, the identity fields, `world`, `x`, `y`, `z`) | The `PersistPosition` was queued on the Cell→Base channel. This is the normal path, so it is DEBUG |

The identity fields are `account_id`, `account_name`, `player_id` and `player_name` (Rule 5), with `entity_id` + `entity_name` (Rule 6). The `player_id_missing` row omits `player_id` when it is `None`; it is never sent as `"None"`. A failed send keeps its existing WARN (`PersistPosition send failed`, `error = %e`). Pinned by `persist_position_skip_and_queue_rows` in the same file.

## NPC-vs-NPC target refusals and NPC-only kills (#1009)

Two seams, both carrying both entities' ids and factions so a standoff can be debugged from SigNoz alone:

- **A hostile NPC the Idle scan passed over.** `npc_ai.aggro_scan` at `debug!`, `event = "npc_candidate_rejected"`, Pattern D: at most one row per `(npc, target, reason)` per 10 s, with `suppressed = N`. Fields: `npc_id`, `tag`, `npc_faction`, `target_id`, `target_tag`, `target_faction`, `target_ai_state`, `reason` (`dead`, `target_evading`, `target_unavailable`, `out_of_vertical_band`, `out_of_radius`, `no_los`), `npc_to_target`, `dy`, `aggro_radius`. DEBUG because it is the normal state of a standoff whose sides stand just out of range. Only HOSTILE pairs are candidates: a same-faction neighbour is not a refusal and writes nothing, or every guard post would log a row per neighbour per tick. Written by `detectors::aggro_scan::report_npc_rejects` (`cimmeria-cell-world`). Guard: `service::tests::npc_ai::npc_vs_npc::a_hostile_npc_out_of_radius_is_refused_with_a_row` pins the level, the reason and both factions, and `friendly_rows_never_engage_and_log_nothing` pins the silence for non-hostile pairs.
- **An NPC-only kill.** `loot.drop` at `info!`, `event = "skipped"`, `reason = "npc_only_kill"`: `target_eid`, `target_tag`, `target_faction`, `loot_table_id`, `attacker_id`, `attacker_tag`, `attacker_faction`. INFO like the death itself (low frequency, and it explains a corpse with no loot cursor). Guard: `abilities::death::npc_only_kill_tests::an_npc_only_kill_says_why_nothing_dropped`.

## Combat and effects key changes (NT-20)

- `threat` `enter_combat` / `exit_combat` logged the player's **entity** id as `player_id`. It is now `entity_id` (with `entity_name`), and `player_id` / `account_id` carry the character's and account's ids per Rule 5.
- The loot roll rows (`loot_generated`, `loot_entry_bad_quantity`) logged the item type as `design_id = ?Some(..)`. They now log `item_type_id` (a number; `item_id` is for instances) with `item_name`, and keep `design_id`, as a number, for the cell/base loot join.

A deferred effect row (a pulse, an expiry, a removal) names its invoker from the snapshot the effect took when it landed (`SpaceManager::caster_label`, `invoker_identity` and `invoker_name` on `TimedEffect` and `ActiveEffectInstance`; the name is an NPC's too, so a mob's DoT and debuff rows name it after it died), never from whoever holds the invoker's entity id now: the id may have been reused. A pet's `owner_name` comes from the summon-time identity for the same reason (#889).

## Bandolier ammo flush (TG-ITM-11)

`crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs` sends one `BandolierAmmoUpdate` per dirty bandolier slot. It is called from world transitions, respawn, ring transport and logout (`flush_dirty_bandolier_ammo_at_logout`, from `lifecycle.rs`).

| Level | Row | When |
|---|---|---|
| WARN | `event = "bandolier_slot_dropped"`, `reason = "no_item"`, identity, `slot_id` | A dirty slot has no item any more (swapped out first): its ammo marker is dropped, nothing is persisted |
| WARN | `BandolierAmmoUpdate send failed; leaving slot dirty for retry`, `error`, item pair, `current_ammo`, `account_id`, `account_name`, `unsent`; on the logout path also `reason = "ammo_lost_at_logout"` | The base channel refused the update. Elsewhere the next flush retries; at logout the entity is destroyed next, so that ammo is lost |
| DEBUG | module-path target, `event = "bandolier_flush"`, identity, `sent`, `dropped`, `unsent`, `at_logout` | Once per flush with a non-empty dirty set |

Guards: `active_slot_flush_tests::bandolier_flush_reports_dropped_and_unsent` and `bandolier_flush_at_logout_warns_ammo_lost`.

## Character delete cascade (TG-SOC-02)

Target `org`, `character.delete_cascade`, emitted by `delete_character` after its commit: WARN when held cash, mail attachments, listings or mail cash were lost (the player can feel it), else DEBUG. Fields: `player_id`, `player_name`, `account_id`, `account_name`, and the counts `auctions_destroyed` (listings the character sold), `held_cash_destroyed` and `bids_held_lost` (open bids the character stood on), `mail_destroyed`, `mail_attachments_destroyed` (escrowed items), `cod_mails` (unpaid COD mail), `mail_cash_destroyed` (cash on non-COD mail) and `bid_history_rows` (`sgw_auction_bid` rows the character placed). Restorable detail, so an operator can restore by hand: `auction_sequence_ids`, `auction_item_ids`, `held_bid_auction_ids` with `held_bid_amounts` (same order), `mail_ids` and `mail_attachment_item_ids`, each comma-joined and capped at 50 entries, with `truncated = true` when any list was capped. The other cascades (inventory, missions, abilities, cooldowns, contacts, expertise) are not itemised. The counts are read inside the delete transaction, at READ COMMITTED before the DELETE, before the cascade and the Black Market trigger run; they can differ slightly from what the cascade removes when mail expires concurrently. A failed count is logged at WARN as `character.delete_cascade_count_failed` and the delete goes on without the counts. This row is the only record of what a delete destroys (D-TG9). Guard: `base-session` `organization/persistence/tests/delete_cascade.rs`, `live_db_character_delete_logs_cascade_counts`, `live_db_character_delete_with_nothing_logs_debug`, `live_db_character_delete_seller_with_bidder_refunds_bidder`.

## Outbox replays (TG-ITM-03)

`crates/base-session/src/base/outbox/mod.rs` holds base→cell events that must not be lost (for example `ItemUsed`) until the cell takes them. Every WARN on the outbox (dispatch failed, poison row, mark-delivered failed, send failed) carries `outbox_id`, `entity_id` and `event_type`, so a stuck row names whose event it is. A successful replay from `drain_undelivered` writes INFO `event = "outbox_replayed"` with `outbox_id`, `entity_id`, `event_type` and `attempts`. A replayed `ItemUsed` re-runs its chain, so this row is how a possible double consume shows up. The entity id is a runtime id with no stored name (`nt:id-only`). Guard: `outbox::tests::live_db::live_db_outbox_replay_logs_event_type`.

## Bandolier ammo writeback seam (TG-ITM-06)

Target `cimmeria_base_methods::base::world_entry::methods::inventory::ammo` (module path). The bandolier ammo writeback (`update_bandolier_ammo`, `base-methods` `inventory/ammo.rs`) is a write guard keyed on the slot's instance id. A zero-row result means the slot is empty or its item was swapped, so the ammo the cell computed for that slot is dropped. That is a loss the player can feel, so it is a WARN, not a DEBUG.

| `event` | Level | `reason` | Fields |
|---|---|---|---|
| `ammo_writeback_stale` | WARN | `slot_empty_or_instance_swapped` | `player_id`, `player_name`, `rows_affected` (0), `expected` (1), `slot_id`, `expected_instance_id`, `current_ammo`, `cur_ammo_type` |

Guard: `live_db_update_no_op_when_slot_holds_different_type` in `inventory/ammo.rs` pins the level, the event, both counts and the reason.

## Player-load degraded fallbacks (TG-DB-07)

Every fallback in the player-load loaders logs `event = "player_load_degraded"` with a stable `reason` and `error` (`error = %e`) at `error!`, except `no_db_pool` (offline mode, expected), which logs at `debug!`. The message states the consequence. Identity fields are the ones in scope: `account_id` / `account_name` where the loader has the account (`query_player_load_data`), `player_id` / `player_name` everywhere.

| `reason` | Site | Fields |
|---|---|---|
| `player_row_missing` | `player_load/core/player_data.rs`, no `sgw_player` row for the account and player | `player_id`, `player_name`, `account_id`, `account_name` |
| `no_db_pool` | `player_data.rs`, no database pool configured (`debug!`) | the above |
| `player_query_failed` | `player_data.rs`, the base row read | the above, plus `error` |
| `equipment_visuals_query_failed` | `player_data.rs`, the equipment-slot visuals | the above, plus `error` |
| `weapon_visual_query_failed` | `player_data.rs`, the active bandolier weapon visual | the above, plus `error` |
| `inventory_query_failed` | `player_load/core/inventory_items.rs` | `player_id`, `player_name`, `error` |
| `bandolier_query_failed` | `player_load/meta.rs`, `query_bandolier_items` | `player_id`, `player_name`, `error` |
| `missions_query_failed` | `missions/mod.rs`, `query_saved_missions` (read-only callers); at world entry `missions/state_lock.rs`, after the retries, with `attempts` (TG-DB-12) | `player_id`, `player_name`, `error` |

Only `missions_query_failed` says a re-accept will overwrite saved progress, the #411-class data loss; at world entry it now locks the missions instead (see TG-DB-12 below). Guard: `player_load/core/player_data.rs` `degraded_tests::player_load_fallbacks_log_degraded_reason` pins the level, `event` and `reason` for the three reasons reachable with the database down. `player_row_missing` is pinned by the live-DB test `live_db_query_player_load_data_account_mismatch_returns_default` (`player_load/core/mod.rs`). The equipment and weapon visual reasons need fault injection and are untested. `no_db_pool` and `bandolier_query_failed` have no guard.

### Shutdown unsaved-state report (TG-DB-01)

When the cell's message loop is told to shut down (every colo deploy), it reports what it is about to lose before it breaks: `crates/cell/src/cell/service/shutdown_report.rs`, called from the `shutdown.notified()` arm of `message_loop.rs`.

| Level | Row | When |
|---|---|---|
| WARN | `event = "player_state_not_persisted"`, `reason = "server_shutdown"`, identity, `world`, `x`/`y`/`z`, `dirty_ammo_slots` | Once per in-world player the shutdown flush (below) did not save. Its position and any dirty bandolier ammo are lost |
| INFO | `event = "shutdown_unsaved_summary"`, `players_in_world`, `players_not_persisted`, `players_with_dirty_ammo` | Once per shutdown. The last two count only the players not saved |

NPCs log nothing.

### Shutdown flush (TG-DB-02)

Before that report, the cell saves every in-world player through the base (`crates/cell/src/cell/service/shutdown_flush.rs`, `cell_dispatch/shutdown_flush.rs`), in parallel, for up to 10 s (D-TG15). Stop order: [service-architecture.md](service-architecture.md#rust-server-stop-order).

| Level | Row | When |
|---|---|---|
| INFO | `event = "shutdown_flush_started"`, `players_in_world`, `deadline_ms` | Once per shutdown, before the request goes to the base |
| INFO | `event = "shutdown_flush_finished"`, `players_in_world`, `saved`, `missed`, `elapsed_ms` | Once per shutdown, after the base answers or the wait ends |
| WARN | `event = "shutdown_flush_failed"`, `reason` (`base_channel_closed`, `reply_dropped`, `deadline_reached`) | The base never answered: every player counts as missed and gets the WARN above |
| WARN | `event = "shutdown_flush_task_failed"`, `reason = "task_panicked"`, `error` | A player's save task panicked; that player counts as missed |
| DEBUG | `event = "shutdown_flush_deadline_reached"`, `abandoned` | The base hit the deadline and aborted the saves still running |
| DEBUG | `event = "shutdown_flush_reply_dropped"`, `reason = "cell_stopped_waiting"` | The cell stopped waiting before the base answered |
| WARN | `event = "bandolier_slot_dropped"`, `reason = "no_item"`, identity, `slot_id` | A dirty ammo slot holds no bandolier item when the flush builds its request: there is nothing to save, so the slot is left out and its dirty marker cleared. Same row as TG-ITM-11's logout flush |

Each save writes through `PersistPosition` and `BandolierAmmoUpdate`'s writers, so a failed write also logs that writer's own WARN (below) with its reason. A player missed for any reason gets exactly one `player_state_not_persisted`.

`player_state_not_persisted` is conservative: it can over-report. A save the base aborts at the deadline may already have committed its write, and a write that lands after the cell stopped waiting still counts as missed, so a player it names may in fact be saved. It never under-reports. While the cell waits for the base's answer it does not drain base-to-cell messages; anything the base sends the cell in that window is handled (or dropped) by the shutdown that follows.

## Logout position persistence (TG-DB-04)

`PersistPosition` (`cell_dispatch/position.rs`) writes the player's last world and point when the session ends. Its rows name the player by Rule 5 (`account_id` and `account_name` from the `RETURNING account_id` of the UPDATE, `player_id` and `player_name`) and carry the position as numbers `x`, `y` and `z`. The character name falls back to the known-names cache when the session has already closed, so the disconnect row is not blank. A name that cannot be resolved is left off.

| Level | Message | Meaning |
|---|---|---|
| INFO | `PersistPosition: persisted` | The row was written (`account_id`, `account_name`, `player_id`, `player_name`, `world`, `x`, `y`, `z`) |
| WARN | `PersistPosition: no rows updated (player row missing?)` | Zero rows matched: `rows_affected = 0`, `expected = 1`, `reason = "player_row_missing"` (Pattern B counts) |
| WARN | `PersistPosition: DB write failed (next login will use the previous position)` | The UPDATE failed: `reason = "db_write_failed"`, `error = %e` |
| DEBUG | `PersistPosition: no database pool, position not stored` | No pool (test, repl or smoke mode): `reason = "no_db_pool"` |

The non-finite refusal keeps its WARN and `reason = "non_finite_position"`. Guards: `cell_dispatch::position::tests::live_db_logout_position_round_trips_with_world_id` (the account id and the cached player name on the INFO row) and `live_db_persist_no_row_is_silent_warn` (the two counts on the zero-row WARN).

## Deferred content actions dropped on teardown (TG-MIS-07)

A content chain's delayed actions (`content_actions.delay_ms > 0`) wait in `SpaceManager::pending_content_actions`. When the entity leaves before the delay elapses, `destroy_entity` or `disconnect_entity` drops the queue. A dropped `grant_item` or `advance_step` loses player progress, so a non-empty drop writes one WARN: `event = "deferred_actions_dropped"`, `reason` = `destroy_entity` or `disconnect`, Rule 5 identity (`account_id`, `account_name`, `player_id`, `player_name`), `entity_id` with `entity_name`, `dropped` (a count), `chain_ids` and `action_kinds` (via `player_journal::action_kind`). An empty queue writes nothing. Message: `deferred content actions dropped`. Written by `log_dropped_content_actions` in `cell/space_manager/entities.rs`. Guards: `destroy_entity_drops_pending_content_actions` and `disconnect_entity_drops_pending_content_actions` in `cell/space_manager/deferred_content_actions.rs` assert the WARN, `reason` and `dropped`.

## Unreadable saved missions lock (TG-DB-12, D-TG18)

World entry no longer falls back to "no saved missions" (that let a re-accept overwrite completed progress, #411). `missions/state_lock.rs` in `cimmeria-base-methods` retries the read after 200, 400 and 800 ms, each attempt cut off after 2 s (about 9.4 s at worst). If every attempt fails, the base locks the player, sends `MissionStateUnknown` to the cell before `InitPlayerState`, and re-reads every 30 s until it succeeds. The cell (`cell/missions/state_lock.rs` in `cimmeria-cell-content`) refuses every accept, abandon, step advance, objective completion and completion, drops any content chain that changes a mission whole, and saves nothing. Each player-facing refusal also sends the feedback line "Mission data unavailable, try again shortly." The base refuses any `sgw_mission` write for a locked player as a backstop. The cell also holds mission changes from `CreateEntity` until `InitPlayerState` (`reason = "saved_missions_pending"`). Refusal rows and feedback lines are throttled to one per entity per 5 s; each admitted row carries `suppressed`, the refusals swallowed since the last one (Pattern D).

| Level | Row | When |
|---|---|---|
| WARN | `event = "saved_missions_read_retry"`, `reason = "query_failed"` or `"timeout"`, `player_id`, `player_name`, `attempt`/`attempts`, `wait_ms` (world entry), `error` | One world-entry read attempt failed or ran past 2 s and will be retried, or a 30 s background read failed |
| ERROR | `event = "player_load_degraded"`, `reason = "missions_query_failed"`, `last_failure` (`query_failed` or `timeout`), `player_id`, `player_name`, `attempts`, `error` | Every world-entry attempt failed; the missions are locked |
| DEBUG | `event = "mission_state_lock_skipped"`, `reason = "no_cell_channel"` | Every attempt failed but there is no cell channel (dev); nothing is locked |
| INFO | `event = "mission_state_locked"`, `reason = "saved_missions_unreadable"`, Rule 5 identity, `entity_id`, `entity_name` | The cell locked the tracker |
| WARN | `event = "mission_state_unknown"`, `reason = "saved_missions_unreadable"` or `"saved_missions_pending"`, `site`, Rule 5 identity, `entity_id`, `entity_name`, `mission_id`, `mission_name`, `suppressed`, and `chain_id`, `chain_name` when `site = "chain"` | A mission change was refused (at most one row per entity per 5 s). `site` is `chain`, `accept`, `abandon`, `advance_step`, `complete_objective`, `complete`, `gm_missionfail` or a persist site (`accept`, `complete`, `advance_step`, `complete_objective`) |
| WARN | the same `event` and `reason`, `site = "persist"`, `player_id`, `player_name`, `mission_id`, `mission_name`, `status` | The base backstop refused an `sgw_mission` write for a locked player |
| WARN | `event = "mission_state_unknown"`, `reason = "cell_channel_closed"` | `MissionStateUnknown` could not reach the cell; the base backstop still holds |
| INFO | `event = "saved_missions_read_retried"` | A world-entry read succeeded on a retry |
| INFO | `event = "saved_missions_read_recovered"`, `attempts`, `count`, `delivered` | The background read succeeded; the base unlocked and sent `SavedMissionsRecovered` |
| INFO | `event = "mission_state_recovered"`, Rule 5 identity, `count` | The cell restored the missions, resent the journal and unlocked ("Mission data restored." to the player) |
| INFO | `event = "player_loaded_replayed"`, `entity_id`, `entity_name`, `player_id`, `player_name`, `world`, `chains` | After recovery, the `player_loaded` mission chains the lock dropped are re-fired |
| DEBUG | `event = "player_loaded_replay_skipped"`, `reason = "nothing_deferred"` | Recovery found no dropped `player_loaded` chain to replay |
| DEBUG | `event = "mission_state_retry_stopped"`, `reason = "session_gone"` | The session ended while locked; the background read stopped |
| DEBUG | `event = "mission_state_recovery_ignored"`, `reason = "not_locked"` | A duplicate recovery; nothing changed |

Guards: the live-DB test `live_db_unreadable_missions_block_reaccept` (`crates/services/src/mission_state_lock_tests.rs`) makes the read fail, re-accepts a completed mission and asserts the refusal, the feedback line, both WARNs and the unchanged row. `missions::state_lock::tests` (base) pins the retry count, the final ERROR, the persist WARN and the recovery unlock; `cell::missions::state_lock_tests` pins the cell refusal, the whole-chain drop and the recovery unlock.

## Boot effect-script registration (TG-CMB-04)

The cell's boot check of effect `script_name`s (`cell/service/startup.rs`) names only real missing registrations. `EffectScripts::unregistered` (`cell/effects/registry.rs` in `cimmeria-cell-world`) skips a blank `script_name` and the `NATIVE_EFFECT_NAMES` entries: `Reload` (effect 658) runs through the reload pipeline. Those rows are logged once at INFO, not WARN. The shipped seed has no unregistered script left, so a normal boot writes no WARN.

| Level | Row | When |
|---|---|---|
| WARN | `event = "effect_script_unregistered"`, `reason = "no_registered_script"`, `count`, `scripts` | An effect row names a script no registered script answers (once per boot) |
| INFO | `event = "effect_script_native"`, `reason = "native_or_blank"`, `count`, `scripts` (script name to effect ids) | Effect rows name a blank or native script, which needs no registered script (once per boot) |

Guards: `unregistered_skips_blank_and_native` (`cell/effects/registry.rs` in `cimmeria-cell-world`) pins the skip; `every_seeded_script_name_is_registered` (`cell/effects/registry.rs` in `cimmeria-cell-effect-scripts`) pins an empty missing list on the seed.

## UDP connection-reset throttle (TG-MER-10)

On Windows, a `send_to` to a closed port makes the next `recv_from` fail with WSAECONNRESET (10054). That error is per destination, not a socket failure, and the receive loop carries on. `log_connreset()` in `base/connect_loop/mod.rs` throttles it (Pattern D). The first reset writes at once. The resets inside the next 10 s are counted, and the next row carries them in `suppressed`. The throttle is process-wide, not keyed by entity or peer. Every reset, written or counted, bumps `udp_connreset_total`.

| Level | Row | When |
|---|---|---|
| DEBUG | message `UDP recv: WSAECONNRESET (10054) -- remote closed, continuing`, `reason = "icmp_port_unreachable"`, `suppressed` | A UDP recv returned WSAECONNRESET; at most one row per 10 s |

Guard: `connreset_burst_writes_one_row` in `base/connect_loop/mod.rs` (a burst of 50 resets writes one row; the next row after the window carries `suppressed = 49`).

## Trade cancel and refusal rows (TG-SOC-06)

Targets `cimmeria_cell_interactions::cell::trade::state` (module path) and the cell-methods trade handlers. Every `beginTrading` refusal logs `event = "trade.session_refused"` at its existing level. The INFO row `trade.session_cancelled` carries a `reason` from a closed list per call site: `user_cancel` (the player cancelled), `bad_proposal` (a proposal update was rejected), `out_of_range` (a partner walked away mid-trade), `handoff_out_of_range` (a partner was out of range at the commit handoff) and `disconnect` (a teardown from logout or entity destroy). `trade.execute_requested` marks the handoff to base. Both cancel and execute rows carry the Rule 5 identity: `account_id`, `account_name`, `player_id`, `player_name`, with `target_*` for the partner. Guard: `disconnect_cancel_logs_reason_disconnect` in `cell-methods` `trade/tests/handlers.rs`.

## Stale combat state (TG-CMB-02)

A player stuck "in combat" never regenerates and never holsters, and nothing checked for it. The vitals sample tick (`cell/service/ticks/vitals.rs`) now runs `log_combat_state_stale` (`cell-combat`, `combat/vitals.rs`) for every living player: WARN `target: "threat"`, `event = "combat_state_stale"`, identity, with `reason`:

| `reason` | Meaning |
|---|---|
| `mob_gone` | A threat entry names an entity that no longer exists |
| `mob_dead` | A threat entry names a dead entity |
| `bit_without_threat` | `BSF_InCombat` is set but the threat table is empty |
| `threat_without_bit` | The threat table has entries but `BSF_InCombat` is clear |

Pattern D: one row per (entity, reason) per 60 s, carrying `suppressed`; the throttle is forgotten when the entity is destroyed. Guards: the seven `log_combat_state_stale` tests in `combat/vitals.rs` (one per reason, a consistent-state silence case, a burst and an independence test).

## Minigame result and start seam (TG-MG-01)

`cell_dispatch/minigame.rs` in `cimmeria-base-world-entry`. A minigame result is forwarded to the cell for its victory chains; when that hand-off cannot happen the chains never run, so it is logged. The identity is the entity pair (`entity_id`, `entity_name`) and the result is named by `result_code` and `result`.

| Level | `reason` | When | Fields |
|---|---|---|---|
| ERROR | `cell_channel_closed` | `MinigameResult` reached the base, but the cell's receiver was dropped; the victory chains are lost | entity pair, `result_code`, `result`, `chain_count`, `error` |
| WARN | `no_cell_channel` | No cell channel is configured; the victory chains are lost | entity pair, `result_code`, `result`, `chain_count` |
| WARN | `minigame_server_disabled` | `StartMinigame` with no minigame registry; the player is never sent `onStartMinigame` | entity pair, `game_name` |
| WARN | `register_refused` | The session registry refused the ticket (duplicate); the player is never sent `onStartMinigame` | entity pair, `game_name` |

Guards: `cell_dispatch/tests.rs` `minigame_result_with_dropped_cell_receiver_logs_error`, `minigame_result_without_cell_channel_logs_warn` and `start_minigame_with_disabled_server_logs_warn`.

## Related

- [TESTING.md](../../TESTING.md) — Test-type picker; regression-guard rules.
- [CLAUDE.md](../../CLAUDE.md) — Doc-update map; pre-PR checklist.
- [crates/services/src/test_support.rs](../../crates/services/src/test_support.rs)
  — `LogCapture` helper used by guards.
