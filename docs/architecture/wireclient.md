# ADR: `wireclient` — headless wire-level test client for end-to-end validation

> **Last updated**: 2026-05-24
> **Audience**: Engineers writing end-to-end tests for the SGW server emulator
> **Type**: Architecture decision record
> **Owner**: Network / test-infra
> **Tracking**: issue [#281](https://github.com/SandboxServers/Cimmeria/issues/281)

## Status

**Phase 1 accepted, Phases 2–7 pending.** This document describes the
shipped foundation (auth, handshake, trace format) and pre-commits the
architecture for the gameplay layer so contributors can pick up specific
follow-up phases without re-litigating the design.

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
flagship; new captures drop into the corpus without code changes.

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

wireclient does all three.

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
│   └── client.rs         # Top-level Client; today: auth + handshake;
│                         #   Phase 2+: entity mirror, dialog state, step driver
└── tests/
    └── auth_smoke.rs     # In-process AuthService + Phase 1/2 round trip
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

### 4. Replay model — semantic, not byte-replay

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

### 5. Combat policy at step 9

The PrisonerRetrievalUnit fight must use the **real combat path**. wireclient:

- Maintains a local mirror of equipped weapon, ammo, active bandolier slot.
- Refuses to send `useAbility` for an ability the equipped weapon doesn't grant.
- Maintains a local mirror of the NPC's position from `onPropertyUpdate`
  + movement broadcasts; refuses to fire out of range.
- Maintains a local navmesh-derived LOS oracle (v1: static "arena is open,
  LOS always true"; v2: real navmesh).
- Refuses to fire while cooldown is active.

Server-side parity work tracked separately: `use_ability.rs` currently
has range + cooldown + ammo + dead-state checks but no LOS check for
player→NPC casts. Bringing player→NPC up to parity is part of Phase 5.

### 6. Minigame policy

Castle Cellblock hits Livewire three times. The full SmartFoxServer 1.x
XML protocol is out of scope; instead a `#[cfg(test)]` force-victory hook
on `MinigameServer` synthesises the same `OnVictory` chain dispatch the
real protocol would. This is the **only sanctioned shortcut** in the
wireclient design.

### 7. Test profile

A new nextest profile `wireclient-e2e` (`.config/nextest.toml`) serialises
the suite (`threads-required = "num-test-threads"`) because each test
owns a spawned server process; parallel runs would corrupt shared state.

The auth-smoke and trace-load tests live in the default `ci` profile
because they don't spawn the BaseApp — only the in-process
`AuthService`, which `login_smoke` already proves safe to spawn many in
parallel.

## Test corpora

| Slug | Source | Events | Coverage |
|---|---|---|---|
| `castle-cellblock-full-run` | `2026-05-24_17-18.pcap` | 125,770 | World entry → mission 622 → … → mission 688 → next-world transport |

New corpora are added by:

1. Recording a session with the SGW client + `Sniffer` enabled (AES key
   logs to `Sniffer: Got AES key from auth stream`).
2. Running `python3 tools/pcap_to_session.py <pcap> <keys.txt> --out
   <slug>.jsonl --label <slug>`.
3. Adding the JSONL to the test corpus directory and a smoke entry to
   `crates/wireclient/tests/`.

## Phasing & status

| Phase | Work | Status |
|---|---|---|
| 1 | Scaffold + SOAP auth + handshake driver + JSONL trace | **Done** (this PR) |
| 1.5 | UDP send/recv loop + first encrypted round-trip against spawned BaseApp | Pending |
| 2 | `mapLoaded()` + initial entity hydration assertion | Pending |
| 3 | Entity mirror + behavior-trace module + semantic diff | Pending |
| 4 | Castle Cellblock script (steps 1–8, 10, 12–20) | Pending |
| 5 | Combat at step 9 + server-side LOS parity check | Pending |
| 6 | `#[cfg(test)]` force-victory hook | Pending |
| 7 | nextest `wireclient-e2e` profile + CI workflow | Pending |

## Risks & open questions

1. **Server-process lifecycle in tests.** Phase 1.5 must define how a
   test spawns + reaps `cimmeria-server`. Today the auth smoke runs the
   service in-process; the full server may need the same treatment or a
   `Command::spawn` fallback. Crash safety + port collisions defined
   *before* the harness lands, not after.
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
  [`crates/mercury/src/test_transport/`](../../crates/mercury/src/test_transport/)
- Tier 2: loopback Mercury harness.
  [`crates/mercury/src/test_harness/`](../../crates/mercury/src/test_harness/)
  + [ADR](mercury-loopback-harness.md)
- Server-side SOAP auth flow that wireclient drives:
  [`crates/services/src/auth/`](../../crates/services/src/auth/)
- Server-side Mercury phase-3 handshake:
  [`crates/services/src/base/login.rs`](../../crates/services/src/base/login.rs)
- Server-side ability path that Phase 5 strengthens:
  [`crates/services/src/cell/abilities/use_ability.rs`](../../crates/services/src/cell/abilities/use_ability.rs)
- Pcap → JSONL exporter:
  [`tools/pcap_to_session.py`](../../tools/pcap_to_session.py)
- Underlying Mercury dissector this builds on:
  [`tools/pcap_dissect.py`](../../tools/pcap_dissect.py)
- Test-type taxonomy this extends: [`TESTING.md`](../../TESTING.md)
- Issue: [#281](https://github.com/SandboxServers/Cimmeria/issues/281)
