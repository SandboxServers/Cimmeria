# Deterministic Session Bootstrap (Stage 0)

> **Last updated**: 2026-07-26
> **Audience**: Engineers writing integration tests that need a running server
> **Type**: Architecture decision + how-to
> **Owner**: Test infrastructure
> **Related**: [wireclient ADR](wireclient.md) (risk 1, "server-process
> lifecycle in tests"), [TESTING.md](../../TESTING.md)

## Problem

Verifying a server change requires a human launching the game client and
clicking through login and character select. That makes every check manual,
slow, and unrepeatable.

Stage 0 removes the human from the **server-controlled** part of that path:
cold start to in-world, scripted. It is deliberately *half* the problem —
everything here works with no game client running. Driving the client is a
later stage and is explicitly out of scope.

Two pieces are needed, and the [wireclient ADR](wireclient.md) blocks on the
second:

> **Server-process lifecycle in tests.** Phase 1.5 must define how a test
> spawns + reaps `cimmeria-server`. […] Crash safety + port collisions
> defined *before* the harness lands, not after.

## Part 1 — Autoplay

### What it does

When armed, the server auto-selects a character once the character list has
been delivered, instead of waiting for the client's `playCharacter` (`0xC4`).

The hook lives at the end of the char-list branch of `handle_enable_entities`
and calls **the same `handle_play_character`** that a real `0xC4` dispatches
to, with the same arguments. There is no parallel world-entry flow: from
`RESET_ENTITIES` onward the sequence is identical to a human-driven entry.
That is the entire point of solving this server-side.

### What it does not do

Autoplay skips **client input only**. It is not an authentication bypass:

- It runs off the `ConnectedClientState` the normal login flow creates, so the
  session has already passed credential validation and Mercury session
  establishment. Autoplay cannot manufacture a session; it can only act on one
  that already authenticated.
- The configured character id is **validated against the authenticated
  account's own character list** before use. The list comes from
  `query_character_list(db_pool, account_id)`, so membership in it is proof of
  ownership. A configured id belonging to another account is refused. Without
  this check a config value would decide which entity a session enters the
  world as — a privilege-escalation shape.
- No server-side gate is relaxed anywhere. If a future change to autoplay
  cannot work without weakening one, that is a signal the design is wrong, not
  a licence to weaken it.

### Configuration

| Variable | Default | Meaning |
|---|---|---|
| `CIMMERIA_AUTOPLAY_PLAYER_ID` | unset | Character id to auto-select. Unset ⇒ autoplay disabled. |
| `CIMMERIA_AUTOPLAY_WORLD` | unset | Optional world guard, e.g. `Castle_CellBlock`. |
| `DEVELOPER_MODE` | `false` | Autoplay's mandatory second gate. |

**Two independent gates.** `AutoplaySettings::is_armed` requires *both* an
explicit character id and `DEVELOPER_MODE`. A single environment variable
leaking into a production process cannot arm auto-world-entry. When autoplay
does arm, startup emits a WARN naming it, so an operator who armed it by
accident sees it in the first screen of logs rather than inferring it from
behaviour.

**The world setting is a guard, not a teleport.** It refuses to auto-enter the
wrong map; it will not relocate a character into the named one. Spawn position
still comes from the character's persisted location exactly as it does for a
human-driven `playCharacter`. This keeps autoplay from silently diverging from
the flow it is supposed to be exercising — a character that is not already in
Castle Cellblock is a seeding problem, and autoplay says so instead of
papering over it.

### Decision table

`base::autoplay::decide` is a pure function over settings + the account's
character list, evaluated in this order:

| Condition | Outcome |
|---|---|
| No `player_id` configured | `Disabled` |
| `player_id` set, `developer_mode` off | `Refused { developer_mode_required }` |
| Character not in the authenticated account's list | `Refused { character_not_owned_by_account }` |
| World guard set and `world_location` differs | `Refused { world_location_mismatch }` |
| Otherwise | `Engage { player_id }` |

Refusals log a WARN with a stable `reason` and fall back to normal
client-driven character select. A misconfiguration degrades to the ordinary
flow; it never half-enters the world.

Keeping `decide` pure is what makes the gates testable without a socket, a
database, or a live session. The process-global in `autoplay::settings` only
supplies the configured values at the call site, and reports "disabled"
whenever `init` was never called — which is the case for the entire unit-test
suite and every library consumer that is not the server binary.

## Part 2 — The spawn/reap harness

`crates/server-harness/` (`cimmeria-server-harness`) starts a real
`cimmeria-server`, waits for a definite readiness signal, and guarantees the
process dies afterwards.

### Readiness contract

**Signal: `GET /api/config/status` on the admin port returns HTTP 200 with
`services.auth`, `services.base` and `services.cell` all `true`.**

This is the only observable that is causally downstream of every bind:

- `Orchestrator::start_all` starts auth, base and cell strictly sequentially,
  `await`ing each and propagating errors. Each service sets `is_running = true`
  only *after* its own listener bind returned `Ok` — for base and cell that is
  `UdpSocket::bind(...).await?`. A `true` flag therefore means the socket is
  genuinely bound, not merely that a task was spawned.
- The admin listener binds *after* `start_all` returns, so the endpoint being
  reachable at all already implies the three services started.
- The route carries no authentication middleware, so probing needs no
  credentials.

Reading the flags rather than stopping at "the admin port accepts TCP" costs
one request and removes the assumption that startup ordering never changes.

**`database` is deliberately not required by default.** The orchestrator
treats a DB connection failure as non-fatal and starts anyway, so requiring it
would hang against a server that is running fine by its own definition.
Callers that need a database opt in via `ReadinessCheck::require_database`.

**Failure modes are distinguished, and fail closed.** The poll loop reports
`Unreachable` (nothing listening yet) separately from `NotReady` (answered,
still starting), and checks `try_exit_status` each iteration so a server that
dies during startup fails immediately instead of burning the full timeout. A
missing or renamed flag is treated as *not ready*, so an endpoint change
surfaces as a loud timeout rather than a harness that declares every server
ready without checking anything.

#### Alternatives rejected

| Alternative | Why not |
|---|---|
| A fixed sleep | Fails under load, wastes time when fast. The explicit non-goal. |
| Scraping stdout for `Server ready` | Couples tests to a log string and the tracing format, and needs pipe draining to avoid the child blocking on a full stdout buffer. |
| TCP-connect to the base port | Base is UDP; connect proves nothing. |
| Bare TCP-connect to the admin port | Works today, but silently assumes admin binds last. Reading the flags does not. |

### Port allocation

The server binds six ports. Every one is reserved by the harness and passed by
environment, because any port left at its compiled-in default collides with a
second instance.

`minigame_port` was previously **not overridable by environment** — it always
bound 30000, which alone made concurrent servers impossible. `MINIGAME_PORT`
was added as part of this work.

Reservation asks the OS for a free port (bind port 0, read the assignment,
close). The known race between close and the server's bind is accepted rather
than solved by passing inherited sockets, which would mean changing the
server's bind path purely to serve tests. Three things narrow it:

1. **Probes bind `0.0.0.0`**, matching the server's default bind address. A
   port free on `127.0.0.1` is not necessarily free on `0.0.0.0`.
2. **Probes match protocol** — UDP ports probed with a UDP socket, TCP with a
   TCP listener. The two port spaces are independent.
3. **A process-wide reservation set** so two harnesses in one test binary can
   never be handed the same number even if the OS recycles it.

### Reaping: three layers

No single mechanism covers every way a parent can die, so `ChildGuard` stacks
three:

| Layer | Mechanism | Covers |
|---|---|---|
| 1 | `Drop` → `kill()` + blocking `wait()` | normal return, early `?`, **panic unwind** |
| 2 | console / process group | Ctrl-C at an interactive terminal |
| 3 | Job Object (Windows) / `PR_SET_PDEATHSIG` (Linux) | parent hard-killed, `panic = "abort"`, OOM kill |

**Layer 1** is the workhorse. `Drop` runs during unwind, so a panicking test
body still reaps its server. The `wait()` is blocking on purpose: returning
before the child is reaped would let the next test race this one for ports.

**Layer 2** needs no code. A child spawned with inherited stdio joins the
parent's console (Windows) / foreground process group (Unix), so Ctrl-C
reaches it directly. For `cimmeria-server` this is the *preferred* path — it
has a real Ctrl-C handler and shuts down gracefully, which the hard `kill()`
in layer 1 does not give it. Inheriting stdio is therefore a deliberate
default, not merely a convenience.

**Layer 3** closes the orphan hole. Layers 1 and 2 both assume the parent gets
to run code or that a signal is delivered; neither holds when the parent is
`taskkill /F`'d, aborts, or is OOM-killed. A Windows Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` makes the kernel kill the child when the
last job handle closes — which happens during process teardown however violent.

> **Testing note.** Layers 1 and 3 are redundant by design, and that
> redundancy defeats a naive test: with the job object active, deleting the
> entire `Drop` impl still leaves every child correctly reaped. This was
> verified by experiment, not assumed. Tests that assert on `Drop` therefore
> spawn with `OrphanProtection::DropOnly`, which is the only configuration in
> which a broken `Drop` is observable. `OrphanProtection` is a real runtime
> option, not a test-only affordance — job assignment can legitimately be
> refused by a restrictive container.

### Usage

```rust
use cimmeria_server_harness::{HarnessConfig, HarnessError, ServerHarness};

let config = HarnessConfig::default().with_autoplay(4242, Some("Castle_CellBlock"));

let server = match ServerHarness::start(config) {
    Ok(server) => server,
    // Skip rather than fail when the binary hasn't been built — the same
    // discipline the pcap-replay fixtures use.
    Err(HarnessError::ServerBinaryNotFound { .. }) => return,
    Err(e) => panic!("harness failed: {e}"),
};

// server.logon_addr() / server.base_addr() / server.admin_addr()

drop(server); // reaped here, including if the body above panicked
```

`with_autoplay` sets **both** required gates. A helper that set only the
character id would produce a server that refuses autoplay with
`developer_mode_required`, which looks like a broken harness rather than a
config error.

The server executable is resolved from `CIMMERIA_SERVER_EXE`, then the
directory holding the running test binary and its parent
(`target/<profile>/deps/` → `target/<profile>/`), then the repo-root copy the
documented build step produces. A missing binary yields
`HarnessError::ServerBinaryNotFound`, which callers should treat as *skip*,
not *fail* — a developer running `cargo test -p cimmeria-services` has not
necessarily built the server.

## What is still manual

Stage 0 stops at the server boundary. Explicitly **not** solved here:

- **Driving the game client.** No CEGUI automation, no injection. A real
  client is still required to see anything rendered.
- **The client half of world entry.** Autoplay sends `RESET_ENTITIES` and then
  waits for `ENABLE_ENTITIES` from a connected peer, exactly as the normal
  flow does. Autoplay removes the *character-select* wait, not the client. A
  headless peer that answers those packets is `cimmeria-wireclient` Phase 1.5,
  which this harness now unblocks.
- **Seeding a character into Castle Cellblock.** Autoplay guards the world, it
  does not set it. The test character must already be persisted there.
- **Asserting on in-world state.** The harness proves the server is up; it
  makes no gameplay assertions.

## Cross-references

- [wireclient ADR](wireclient.md) — risk 1 is what this resolves; Phase 1.5 is
  the intended first consumer.
- [TESTING.md](../../TESTING.md) — test-type taxonomy.
- `crates/server-harness/` — the harness.
- `crates/services/src/base/autoplay.rs` — the decision logic.
- `crates/services/src/base/world_entry/play_character.rs` — the shared
  world-entry path autoplay routes through.
