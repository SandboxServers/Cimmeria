# ADR: `wireclient` — headless wire-level test client for end-to-end validation

> **Last updated**: 2026-09-27
> **Audience**: Engineers writing end-to-end tests for the SGW server emulator
> **Type**: Architecture decision record
> **Owner**: Network / test-infra
> **Tracking**: issue [#281](https://github.com/SandboxServers/Cimmeria/issues/281)

## Status

**Phase 1 accepted. Phase 1.5 shipped scoped to a real two-client
end-to-end test (NA37, 2026-09-25); Phases 2 (full)/3/4/5/6/7 pending.**
This document describes the shipped foundation (auth, handshake, trace
format, and now a real UDP socket + Channel-driven session) and
pre-commits the architecture for the gameplay layer so contributors can
pick up specific follow-up phases without re-litigating the design.

**Read the phase table before treating any section here as shipped.**
Sections 2–6 are still mostly *design intent for unwritten phases*. As of
2026-09-25 the crate has a real UDP socket loop —
[`GameSession`](../../crates/wireclient/src/session.rs) binds a
`tokio::net::UdpSocket`, drives the SOAP + Mercury phase-3 handshake over
it, then hands the same socket to
`cimmeria_mercury::test_harness::LoopbackPeer` (the Tier 2 loopback
harness's Channel driver, reused here against a *real* BaseApp instead of
a paired test peer) for reliable send, fragment reassembly, and ACK
piggyback. `Client::connect()` (the original Phase 1 top-level driver)
still does not exist and is superseded by `GameSession` for anything past
the handshake. There is still no replay engine: `Trace::c2s()` /
`Trace::s2c()` are iterator filters with no consumer, and there is no
semantic behavior-trace decoder (Phase 3) or Castle Cellblock script
driver (Phase 4). What `GameSession` does cover — because
[`crates/wireclient/tests/it/two_client_castle_visibility.rs`](../../crates/wireclient/tests/it/two_client_castle_visibility.rs)
needed it — is auth → character select → world entry
(`ENABLE_ENTITIES`/`playCharacter`/`mapLoaded`/`onClientReady`) and enough
client→server builders (movement, disconnect) to drive a player around
after entry. [`bundle.rs`](../../crates/wireclient/src/bundle.rs) adds a
structural (not semantic) decoder for server→client bundles — msg_id,
entity_id, class_id, and method index — a small slice of Phase 3 pulled
forward because the visibility test needed to assert "did entity X's
create/appearance/leave reach this witness" against real wire bytes.

Two generic builders let a test call any SGWPlayer method without a
dedicated wrapper (added for the organizations campaign's two-client tests,
ORG-01). Both return framed message bytes for `send_bundle`, and both take
`args` already serialized in `.def` order:

- `GameSession::cell_method(method_index, entity_id, args)` encodes an
  exposed cell method the way the server's decoder
  (`crates/base/src/base/connect_loop/cell_arms.rs`) reads it: index 0-60
  as msg_id `0x80 | index`, index 61 and above as `0xBD` with the sub-slot
  byte `index - 61` after the entity id. Indices come from
  [cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md).
- `GameSession::base_method(msg_id, args)` frames a base method by its wire
  id (`0xC0 + index`,
  [sgwplayer-base-method-dispatch-table.md](../protocol/sgwplayer-base-method-dispatch-table.md)),
  with no entity id prefix.

`cell_method(25, id, &[])` and `base_method(0xD8, &[])` produce the same
bytes as `map_loaded(id)` and `on_client_ready()`; the unit tests in
`session.rs` pin both encodings and the 60/61 boundary.

`GameSession::enter_world(player_id, timeout)` (SS-U2,
[`world_entry.rs`](../../crates/wireclient/src/world_entry.rs)) is the
character-select and world-entry sequence as a library call, returning an
error instead of panicking. The integration tests' `enter_castle` wraps
it, and the crate's one binary, [`sparbot`](#sparbot-a-duel-partner-for-solo-testing),
uses it to put a second player in the world.

## TL;DR

`cimmeria-wireclient` is the **Tier 3** end-to-end test surface above
`crates/mercury/`'s Tier 1 byte-fan-out fake (`test_transport`) and Tier 2
paired-channel loopback (`test_harness`). Where those tiers exercise the
wire layer in isolation, wireclient drives the *full* protocol the way the
original Flash client would have — SOAP auth, Mercury handshake, encrypted
gameplay traffic — and asserts the server reacts the way a recorded
reference session did.

Validation is **hybrid**:

- **Byte-exact** for the handshake and static base messages (msg_ids
  `0x00`–`0x7F`). These are deterministic; any drift is a regression.
- **Behavioral** for entity-method messages (msg_ids `0x80`–`0xFE`) and the
  observable gameplay layer (entities spawned, missions advanced, chains
  fired, dialogs opened, kismets played). The diff happens at the
  semantic layer; runtime-allocated entity IDs, seq numbers, and
  timestamps drift freely without failing tests.

The reference baseline is **any** decrypted `.pcap` + AES `keys.txt`
captured from a live SGW session. The Castle Cellblock corpus is the
intended flagship; new captures drop into the corpus without code changes.
Note that only a 5-event head fixture is checked in today — see
[Test corpora](#test-corpora).

## Context

Issue #281 spells out the gap this layer closes: ~half of the integration
bugs caught in PR reviews #131+ live in the slice between *"server
accepts a call"* and *"a real client could have actually sent that call."*
Every existing test type — unit / wire-format / live-DB / smoke /
concurrency / chain-replay / legacy reference / fan-out byte / Mercury
session — injects events at or below the dispatcher. None proves the
wire path leading up to the event would have fired.

A pure server-side test cannot:

- Refuse to send `useAbility` for an ability the equipped weapon doesn't
  grant (the chain-replay test fakes the trigger; the real client gates
  on `MaxAmmoCount`, `BSF_InCombat`, range, LOS).
- Detect that the server's `onCharacterList` shape drifted from what the
  Flash client would parse.
- Diff the observable behavior of a full Castle Cellblock playthrough
  against a known-good baseline.

wireclient is designed to do all three. None of the three works yet — see
Status above and the phase table; Phase 1 delivered the auth, handshake,
and trace-format foundation they will be built on.

## Decision

### 1. Crate layout

`crates/wireclient/` is a library crate (not a binary) so tests can pull
it in as a normal `[dev-dependencies]` entry:

```text
crates/wireclient/
├── Cargo.toml
├── src/
│   ├── lib.rs            # Module decls, public re-exports, design doc
│   ├── error.rs          # Single Error enum spanning SOAP + handshake + replay
│   ├── auth.rs           # SOAP Phase 1+2 driver (mirrors login_smoke)
│   ├── handshake.rs      # baseAppLogin builder + connect_reply/time_sync parser
│   ├── session_trace.rs  # JSONL trace loader + ComparisonPolicy trait
│   ├── session.rs        # GameSession: real UDP socket + LoopbackPeer-driven
│   │                     #   Channel, world-entry builders (Phase 1.5 + a
│   │                     #   slice of Phase 2/4 — auth/char-select/world-entry
│   │                     #   only, no entity mirror or script driver yet)
│   ├── world_entry.rs    # GameSession::enter_world: char select + world
│   │                     #   entry as one call (SS-U2)
│   ├── sparbot.rs        # The duel test partner: duel builders, the
│   │                     #   Sparbot state machine, the keep-alive run loop
│   ├── bin/sparbot.rs    # The `sparbot` binary (SS-U2)
│   ├── bundle.rs         # decode_bundle(): structural (msg_id/entity_id/
│   │                     #   class_id/method_index) server->client bundle
│   │                     #   decoder -- NOT the Phase 3 semantic decoder,
│   │                     #   just the slice two_client_castle_visibility.rs needs
│   └── client.rs         # Original Phase 1 top-level Client; today: login_only
│                         #   + byte builders only (no socket) -- GameSession is
│                         #   the driver for anything past the handshake now
└── tests/
    ├── it/                                    # ONE integration-test binary
    │   ├── main.rs                            # Declares the modules below
    │   ├── auth_smoke.rs                      # In-process AuthService + Phase 1/2 round trip
    │   ├── trace_load.rs                      # Loads the checked-in head fixture
    │   ├── two_client_castle_visibility.rs    # Live-DB: two real GameSessions,
    │   │                                      #   one shared Castle world, both
    │   │                                      #   arrival orders (NA37)
    │   ├── two_client_castle_visibility_chaos.rs  # Live-DB: the same scenario
    │   │                                      #   under injected loss/latency
    │   ├── sparbot_duel.rs                    # sparbot vs cimmeria-wire, and a
    │   │                                      #   live-DB duel accept (SS-U2)
    │   ├── two_client_squad.rs                # Live-DB: /squadinvite, accept,
    │   │                                      #   both join, leave (ORG-03)
    │   ├── two_client_command_invite.rs       # Live-DB: a Command invite by
    │   │                                      #   type, accepted via the base (ORG-07)
    │   └── support/mod.rs                     # Shared server bring-up + world-entry driver
    └── fixtures/
        └── castle_cellblock_head.jsonl         # 1 header + 5 events
```

### 2. Trace format — generic across captures

`session_trace::Trace` is a JSONL document with one header line plus one
event per line. **Capture-agnostic**: any decrypted pcap + AES key can be
turned into a trace via `tools/pcap_to_session.py`. New regression
captures drop into the corpus without touching wireclient code.

```text
{"header": {"label", "source_pcap", "session_key_hex",
            "client_addr", "server_addr", "packet_count", "schema_version"}}
{"event": {"t_seconds", "packet_no", "direction" ("c2s" | "s2c"),
           "seq", "flags", "acks", "messages": [
               {"msg_id", "name?", "body_hex"}, ...]}}
```

The producer (`tools/pcap_to_session.py`) reuses `tools/pcap_dissect.py`'s
decoder so the wire-format truth source stays singular. Schema bumps
require a producer change and a `schema_version` increment.

### 3. Comparison policy

`session_trace::ComparisonPolicy::compare(observed, recorded) -> Diff`
classifies every observed message against the recorded baseline:

- `Diff::Exact` — bytes match.
- `Diff::Drift(reason)` — bytes differ in a non-load-bearing way; logged,
  not failed.
- `Diff::Regression(reason)` — bytes differ in a load-bearing way; fails
  the test.

The default policy is byte-exact for static msg_ids (`0x00`–`0x7F`),
length-and-msg_id for entity-method msg_ids (`0x80`–`0xFE`). Tests
needing stricter / looser comparison swap in a custom impl.

`0xFF` (`BASEMSG_REPLY_MESSAGE`) is deliberately excluded from the drift
band and compared byte-exactly alongside `0x00`–`0x7F`.

**Consequence worth stating plainly:** because `DefaultPolicy` compares
`0x80`–`0xFE` bodies by **length only**
([`src/session_trace.rs:292-328`](../../crates/wireclient/src/session_trace.rs))
— equal length yields `Diff::Drift`, unequal yields `Diff::Regression` —
a trace diff under the default policy **cannot validate gameplay content**.
It catches a body that changed size, not a body that changed meaning. The
semantic decoder that would close this gap is Phase 3.

### 4. Replay model — semantic, not byte-replay (Phases 1.5–3 — not implemented)

Pure byte-replay would fail almost every assertion because the server
emits *different bytes* than the recorded server: entity IDs are
runtime-allocated, timestamps drift, random rolls (combat QR, loot
tables) won't match. The right model is:

1. **Extract intent from C2S events.** The recorded client-to-server
   stream is the player's intent: `useAbility(ability=X, target=…)`,
   `attemptInteract(entity_id=…)`, `dialogChoice(dialog=X, choice=Y)`.
2. **Wireclient replays intent against a fresh server.** New entity IDs
   are mapped through wireclient's local entity mirror; the wireclient's
   own ability/ammo/range/LOS guards refuse client-impossible sends.
3. **Compare observables.** The recorded S2C stream is the *expected
   behavior*: entities spawned by type, mission state transitions,
   chains fired, dialogs opened, sequences played. The replay's S2C
   stream is the *observed* behavior. The diff happens at the semantic
   layer (a behavior-trace module — Phase 3).

Phase 1 ships the byte layer (handshake + trace format). Phase 3 adds
the behavior layer on top. Both flavors of diff produce structured
output the test harness can fail-fast on.

### 5. Combat policy at step 9 (Phase 5 — not implemented)

The PrisonerRetrievalUnit fight must use the **real combat path**. None of
the mirrors or guards below exist in the crate yet; this is the Phase 5
specification — the target behaviour, not shipped behaviour. wireclient:

- Maintains a local mirror of equipped weapon, ammo, active bandolier slot.
- Refuses to send `useAbility` for an ability the equipped weapon doesn't grant.
- Maintains a local mirror of the NPC's position from `onPropertyUpdate`
  + movement broadcasts; refuses to fire out of range.
- Maintains a local navmesh-derived LOS oracle (v1: static "arena is open,
  LOS always true"; v2: real navmesh).
- Refuses to fire while cooldown is active.

Server-side parity work tracked separately: `crates/cell-combat/src/cell/abilities/use_ability/`
currently has range + cooldown + ammo + dead-state checks but no LOS check
for player→NPC casts. Bringing player→NPC up to parity is part of Phase 5.

### 6. Minigame policy (Phase 6 — not implemented)

Castle Cellblock hits Livewire three times. The full SmartFoxServer 1.x
XML protocol is out of scope; instead a `#[cfg(test)]` force-victory hook
on `MinigameServer` synthesises the same `OnVictory` chain dispatch the
real protocol would. This is the **only sanctioned shortcut** in the
wireclient design.

### 7. Test profile (Phase 7 — not implemented)

`.config/nextest.toml` today defines exactly two profiles, `ci` and
`ci-live-db`. **There is no `wireclient-e2e` profile.** All 30 wireclient
tests run under the default `ci` profile, which is correct for them: they
don't spawn the BaseApp, only the in-process `AuthService`, which
`login_smoke` already proves safe to spawn many of in parallel.

When Phase 1.5+ lands tests that each own a spawned server process, Phase 7
adds a `wireclient-e2e` profile serialising the suite
(`threads-required = "num-test-threads"`), because parallel runs would
corrupt shared state.

## Test corpora

### Checked into the repo

| Path | Events | Coverage |
|---|---:|---|
| [`crates/wireclient/tests/fixtures/castle_cellblock_head.jsonl`](../../crates/wireclient/tests/fixtures/castle_cellblock_head.jsonl) | 5 (+1 header line) | Head of a Castle Cellblock capture — enough to pin the JSONL loader in `tests/it/trace_load.rs`. |

### Planned, not in the repo

| Slug | Source | Events | Coverage |
|---|---|---:|---|
| `castle-cellblock-full-run` | `2026-05-24_17-18.pcap` | 125,770 | World entry → mission 622 → … → mission 688 → next-world transport |

The full-run corpus is **not committed** — the pcap and the derived JSONL
live outside the repo, so any test depending on it must be produced locally
by the recipe below (or skip-not-fail when the fixture is absent, the same
discipline the pcap-replay chaos tests use). Don't write a test that assumes
it is present.

New corpora are added by:

1. Recording a session with the SGW client + `Sniffer` enabled (AES key
   logs to `Sniffer: Got AES key from auth stream`).
2. Running `python3 tools/pcap_to_session.py <pcap> <keys.txt> --out
   <slug>.jsonl --label <slug>`.
3. Adding the JSONL to the test corpus directory and a smoke module to
   `crates/wireclient/tests/it/` (declared in its `main.rs`).

## sparbot: a duel partner for solo testing

`sparbot` (SS-U2) is a small binary in this crate that logs a second
account into the world and acts as a duel opponent, so one tester can
exercise duels alone. It accepts every duel challenge addressed to it,
stands still, and forfeits a set time after accepting. It is a test tool:
it has no invariant enforcement, does not move, and ignores everything
except duel challenges and the lines the server sends it.

### What it does on the wire

| Step | Message |
|---|---|
| Log in and enter the world | SOAP Phase 1 and 2, `baseAppLogin`, then [`GameSession::enter_world`](../../crates/wireclient/src/world_entry.rs) |
| Keep the session alive | `AUTHENTICATE` (0x01), unreliable, every 250 ms. The server drops a client it has not heard from for 60 s (`base-session` `tick_sync.rs`), and its reliable sends wait for acks the bot only sends piggybacked on an outbound packet. The real client sends `AUTHENTICATE` on every tick while idle, so this is what an idle client looks like |
| A challenge arrives | `onDuelChallenge` (client method 143) on the bot's own entity |
| Accept | `sendDuelResponse(1)` (cell method 102) |
| Forfeit, `--forfeit-after` seconds after the accept | `duelForfeit()` (cell method 103). Until SS-D3 the server only logs `UNIMPLEMENTED: duelForfeit`; the bot logs the send either way. If the server sends "Duel aborted" first, the bot does not forfeit |
| Stop | After `--run-for`, on Ctrl-C, or when the server has been silent for 15 s. On the way out it sends `DISCONNECT`, so the character leaves the world at once |

The indices and texts are pinned against the server's own `cimmeria-wire`
constants and decoders by `tests/it/sparbot_duel.rs::sparbot_wire_matches_the_server`.

### How to duel yourself on a local server

1. **Pick a second account and character.** Any account other than the
   one you play works; the local seed accounts in
   `db/sgw/Accounts/Seed/account.sql` all use the password `test`. The
   bot does not need GM rights. Give that account a character with the
   real client once, and leave it in the world you will test in: the bot
   enters wherever the character last was. Find its id with:

   ```sql
   SELECT p.player_id, p.player_name, p.world_location
   FROM sgw_player p JOIN account a USING (account_id)
   WHERE a.account_name = '<second account>';
   ```

2. **Start the bot** with the local server running:

   ```bash
   SPARBOT_USER=<second account> SPARBOT_PASSWORD=<password> \
     cargo run -p cimmeria-wireclient --bin sparbot -- --player-id <id>
   ```

   It prints `in the world; waiting for duel challenges` with its entity
   id.

3. **Bring it to you.** As a GM in the same world, `.summon <bot name>`
   moves it next to you. The challenge range is 20 units.

4. **Challenge it** with `/duel <bot name>` (the client sends
   `sendDuelChallenge`). The bot accepts at once; you both see "Duel accepted. The duel
   starts in 5 seconds." Check a duel's state with `.duel_status <bot
   name>`, and end it from the GM side with `.duel_end <bot name>`.

5. **Stop the bot** with Ctrl-C. It logs out and prints how many
   challenges it accepted and forfeits it sent.

Options (every one can also come from an environment variable,
`SPARBOT_<OPTION>`, such as `SPARBOT_FORFEIT_AFTER`; a flag wins):

| Flag | Default | Meaning |
|---|---|---|
| `--user`, `--password` | required | The bot's account. Never built in; prefer the environment variables so the password stays out of your shell history |
| `--player-id` | required | The character to play (`sgw_player.player_id`) |
| `--auth-url` | `http://127.0.0.1:8081` | The auth server |
| `--shard` | `Test` | The shard name sent at server selection |
| `--forfeit-after` | `30` | Seconds from the accept to the forfeit; `0` never forfeits |
| `--run-for` | `0` | Seconds to stay in the world; `0` runs until Ctrl-C |

`RUST_LOG` sets the log filter (default `sparbot=info,warn`). The bot's
own events are `sparbot.in_world`, `sparbot.challenge_accepted`,
`sparbot.forfeit_sent`, `sparbot.server_line` (every feedback line, such as
"Duel aborted") and `sparbot.session_lost` (`reason = server_silent |
send_failed`). The server's side of the same duel is in SigNoz under
`scope_name = 'duel'`, correlated by `duel_id`.

### Limits

- **Stand-in behaviour until SS-D2 and SS-D3.** Today a duel's countdown
  ends in "Duel aborted" (SS-D2 engages it), and the forfeit has no effect
  (SS-D3 implements it). The bot needs no change for either: it already
  sends the forfeit and reports every line it gets.
- **Same-space `.summon` only.** A same-space `.summon` is a position snap
  the bot needs nothing for. A cross-world `.summon` or a gate trip starts
  the world-entry handshake again, which the bot does not answer, so the
  character is left loading. Log the bot's character into your world
  instead.
- **Against the colo** the bot needs an account the owner provides for
  it, and the colo's auth URL (`--auth-url`). The UDP socket binds every
  interface when the BaseApp address is not loopback, but the bot has only
  been run against local servers.
- **Keep-alive proof.** `sparbot_session_outlives_the_inactivity_reap`
  holds a session for 70 s, past the server's 60 s reap, then has the bot
  accept a challenge. It is `#[ignore]`d because of its length; run it
  with `--ignored` after touching the run loop. With the heartbeat
  disabled it fails: the session is reaped and the bot stops with
  `ServerSilent`.

## Phasing & status

| Phase | Work | Status |
|---|---|---|
| 1 | Scaffold + SOAP auth + handshake driver + JSONL trace | **Done** — 30 tests: `src/auth.rs` (6), `src/handshake.rs` (10), `src/session_trace.rs` (10), `tests/it/auth_smoke.rs` (3), `tests/it/trace_load.rs` (1) |
| 1.5 | UDP send/recv loop + first encrypted round-trip against spawned BaseApp | **Done** (2026-09-25, NA37) — `GameSession::connect`/`from_auth_session` in `src/session.rs`, reusing `cimmeria_mercury::test_harness::LoopbackPeer` as the client-side Channel driver against a real `BaseService` UDP socket instead of building a second reliable-delivery implementation |
| 2 | `mapLoaded()` + initial entity hydration assertion | **Partial** (NA37) — `GameSession` drives `ENABLE_ENTITIES`/`playCharacter`/`mapLoaded`/`onClientReady` through a real spawned `Orchestrator` and asserts `CREATE_ENTITY`/`BEING_APPEARANCE` hydration for a *second* real client's avatar (`tests/it/two_client_castle_visibility.rs`). No entity mirror, no single-player Castle Cellblock assertion yet |
| 3 | Entity mirror + behavior-trace module + semantic diff | Pending — `bundle.rs`'s `decode_bundle` is a structural decoder (msg_id/entity_id/class_id/method index) pulled forward for NA37, not the semantic per-method-argument decoder this phase specifies |
| 4 | Castle Cellblock script (steps 1–8, 10, 12–20) | Pending |
| 5 | Combat at step 9 + server-side LOS parity check | Pending |
| 6 | `#[cfg(test)]` force-victory hook | Pending |
| SS-U2 | `sparbot` binary + `GameSession::enter_world` | **Done** (2026-09-27) — see [sparbot](#sparbot-a-duel-partner-for-solo-testing); `tests/it/sparbot_duel.rs` (one wire pin, one live-DB accept, one ignored 70 s keep-alive run) |
| 7 | nextest `wireclient-e2e` profile + CI workflow | Pending — `two_client_castle_visibility.rs` is live-DB-gated (skips without `DATABASE_URL`) and is **not** wired into `.github/workflows/test.yml`'s `ci-live-db` job yet (that job runs the lib tests of the crates in `tools/test-live-db.sh` only); run it manually per the header comment in the test file until this phase lands |

## Risks & open questions

1. **Server-process lifecycle in tests.** ~~Phase 1.5 must define how a
   test spawns + reaps `cimmeria-server`.~~ Resolved by NA37: the full
   `Orchestrator` (auth + base + cell + minigame) is spun up **in-process**
   on ephemeral ports, the same TOCTOU-tolerant bind-and-drop pattern
   `login_smoke`/`tls_smoke` already use, just repeated per service port.
   No `Command::spawn` of a separate `cimmeria-server.exe` was needed or
   built — see `start_server` in `tests/it/support/mod.rs`.
2. **Dissector handshake quirk.** The Python dissector splits the
   unencrypted `baseAppLogin` and the encrypted `BASEMSG_REPLY_MESSAGE`
   bodies into spurious sub-messages because the message walker treats
   embedded ASCII ticket bytes as message boundaries. wireclient handles
   the handshake via its own byte-exact builders/parsers, so the trace
   artifacts don't affect Phase 1; Phase 3 must mask these on load.
3. **Behavior-trace fidelity.** Phase 3 needs a semantic decoder for the
   ~50 entity-method msg_ids Castle Cellblock touches. The dissector
   names them; the wireclient must decode their bodies to extract
   observable behavior. This is the bulk of Phase 3's work.
4. **Archetype-property race.** Archetype-branched dialogs (steps 3, 14,
   19 in the Castle Cellblock script) read the NPC's archetype from its
   property bag. The wireclient driver must wait for the property to
   land before sending the dialog choice. Solved by gating step
   transitions on entity-mirror state, not on time.
5. **LOS oracle source.** Server uses navmesh-derived LOS. Wireclient v1
   uses a static stub; v2 replicates the navmesh (recast-detour-rs is
   already evaluated for the server). Negative test for spoofed-LOS
   sends ships with Phase 5.

## Cross-references

- Tier 1: `test_transport` — byte-exact fan-out fake.
  [`crates/mercury/src/test_transport.rs`](../../crates/mercury/src/test_transport.rs)
- Tier 2: loopback Mercury harness.
  [`crates/mercury/src/test_harness/`](../../crates/mercury/src/test_harness/)
  + [ADR](mercury-loopback-harness.md)
- Server-side SOAP auth flow that wireclient drives:
  [`crates/auth/src/auth/`](../../crates/auth/src/auth/)
- Server-side Mercury phase-3 handshake:
  [`crates/base/src/base/login/`](../../crates/base/src/base/login/)
- Server-side ability path that Phase 5 strengthens:
  [`crates/cell-combat/src/cell/abilities/use_ability/`](../../crates/cell-combat/src/cell/abilities/use_ability/)
- Two-client Castle visibility end-to-end test (NA37) and the AoI
  introduction cascade it validates over the wire:
  [player-ghost-aoi-cascade.md](player-ghost-aoi-cascade.md)
- Pcap → JSONL exporter:
  [`tools/pcap_to_session.py`](../../tools/pcap_to_session.py)
- Underlying Mercury dissector this builds on:
  [`tools/pcap_dissect.py`](../../tools/pcap_dissect.py)
- Test-type taxonomy this extends: [`TESTING.md`](../../TESTING.md)
- Issue: [#281](https://github.com/SandboxServers/Cimmeria/issues/281)
