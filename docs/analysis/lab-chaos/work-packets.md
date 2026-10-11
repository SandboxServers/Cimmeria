# Lab chaos mode: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `rust-gameserver-dev` for CH-04, CH-06 and CH-07, and `packet-reviewer`
> reviewers (Sonnet). Ledger, findings (F1 to F11), profiles and levels, and
> decisions (D-CH1 to D-CH8): [README.md](README.md).
>
> PowerShell only. Every compiling command goes through the lane, from the
> worktree root, in this order:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p <crate>
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p <crate> --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p <crate>
> ```
>
> Server packets also run clippy and nextest with `--features lab-chaos` on
> the crate they change. Read the lane's summary and its failures file; do
> not rerun a build to see the output.
>
> Rust rules for every packet: no `unwrap()` outside tests; comments say
> why, not what; `#[cfg(test)] mod tests` last in a file; no new file over
> 500 lines. Test names say what they prove.

## Contents

- [Contract](#contract)
- [CH-01 Chaos profile types, parser and levels file](#ch-01-chaos-profile-types-parser-and-levels-file)
- [CH-02 lab uat --chaos](#ch-02-lab-uat---chaos)
- [CH-03 cpu stressor](#ch-03-cpu-stressor)
- [CH-04 Server chaos state and gates](#ch-04-server-chaos-state-and-gates)
- [CH-05 lab-mcp chaos tools](#ch-05-lab-mcp-chaos-tools)
- [CH-06 net: per-session lossy transport](#ch-06-net-per-session-lossy-transport)
- [CH-07 flush modes](#ch-07-flush-modes)
- [CH-08 burst and runner wiring](#ch-08-burst-and-runner-wiring)
- [CH-09 Chaos report against the golden](#ch-09-chaos-report-against-the-golden)
- [CH-10 Live: calibrate levels](#ch-10-live-calibrate-levels)
- [CH-11 Live acceptance: reproduce #1341](#ch-11-live-acceptance-reproduce-1341)
- [CH-12 Close-out](#ch-12-close-out)

## Contract

Parallel packets build against these names. A packet that needs to change
one stops and tells the coordinator. CH-04 may move the server types to a
different crate only if the dependency graph forces it, and then updates
this section before CH-05 to CH-07 dispatch.

### Lab side: `crates/lab/src/chaos/`

```text
mod.rs      CH-01: ChaosSpec, Profile, Level, parse, ChaosHeader; pub mod cpu; pub mod burst; pub mod levels;
levels.rs   CH-01: Levels, load from levels.toml (include_str!)
levels.toml CH-01: the provisional levels (README "Profiles and levels"); CH-10 updates numbers
cpu.rs      CH-03: CpuStress, CpuGuard
burst.rs    CH-08: burst commands and despawn list
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile { Cpu, Net, Flush, Burst }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level { Safe, Break, Explicit }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChaosSpec {
    pub profile: Profile,
    pub level: Level,
    /// Resolved knobs: the level's values with any explicit overrides applied.
    pub knobs: std::collections::BTreeMap<String, String>,
    pub seed: u64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ChaosParseError {
    #[error("unknown chaos profile {0:?} (cpu, net, flush, burst)")]
    UnknownProfile(String),
    #[error("unknown knob {knob:?} for {profile:?}")]
    UnknownKnob { profile: Profile, knob: String },
    #[error("bad value {value:?} for {knob}: {reason}")]
    BadValue { knob: String, value: String, reason: &'static str },
}

/// "flush:pack", "net:break:seed=7", "net:latency=120ms,loss=1%", "cpu".
/// No level means safe. `default_seed` is used when the text has no seed.
pub fn parse(text: &str, levels: &Levels, default_seed: u64) -> Result<ChaosSpec, ChaosParseError>;

/// The one-line header every chaos report starts with: "chaos=flush:pack seed=7".
pub fn header(spec: &ChaosSpec) -> String;
```

Knobs per profile (values as strings, parsed by the consumer):
`cpu`: `cores`, `duty` (percent). `net`: `latency` (ms), `jitter` (ms),
`loss` (percent, one decimal), `dup` (percent), `reorder` (buffer size).
`flush`: `mode` (`delay`, `reorder`, `pack`, `shift`), `delay_ms`, `records`
(for `pack`), `shift` (bytes, or `sweep`). `burst`: `count`, `template`.

### Server side: `crates/base-session/src/base/lab_chaos/` (feature `lab-chaos`)

```rust
/// True only in a binary built with the `lab-chaos` feature.
pub const COMPILED: bool = cfg!(feature = "lab-chaos");

#[derive(Debug, Clone, PartialEq)]
pub struct NetChaos { pub latency_ms: u32, pub jitter_ms: u32, pub loss_per_thousand: u32,
                      pub dup_per_thousand: u32, pub reorder: u32, pub seed: u64 }

#[derive(Debug, Clone, PartialEq)]
pub enum FlushChaos {
    Delay { ms: u32 },
    Reorder { seed: u64 },
    /// The pre-#1341-mitigation shape: one single-packet phase-1 bundle of up to `records`.
    Pack { records: u16 },
    /// Prepend one filler message of `len` bytes to the phase-1 bundle.
    Shift { len: u16 },
    /// Shift by a seeded length, a new one per flush.
    ShiftSweep { seed: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionChaos { pub net: Option<NetChaos>, pub flush: Option<FlushChaos>,
                          pub expires_at: std::time::Instant }

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ChaosRefused {
    #[error("lab chaos needs DEVELOPER_MODE")] NotDeveloperMode,
    #[error("no session for entity {0}")] NoSession(u32),
    #[error("ttl {0}s is over the 1800 s cap")] TtlTooLong(u32),
}

pub const DEFAULT_TTL_S: u32 = 300;
pub const MAX_TTL_S: u32 = 1800;

/// The registry. Keyed by the session's client address.
pub struct LabChaos { /* Mutex<HashMap<SocketAddr, SessionChaos>> */ }
impl LabChaos {
    pub fn new() -> Self;
    pub fn set(&self, addr: SocketAddr, chaos: SessionChaos);
    pub fn clear(&self, addr: Option<SocketAddr>);       // None clears every session
    /// Expired entries are removed here and read as None.
    pub fn net_for(&self, addr: SocketAddr, now: Instant) -> Option<NetChaos>;
    pub fn flush_for(&self, addr: SocketAddr, now: Instant) -> Option<FlushChaos>;
    pub fn status(&self, now: Instant) -> Vec<(SocketAddr, SessionChaos)>;
}

/// Installed once at startup by the server, only when DEVELOPER_MODE is true.
pub fn install(developer_mode: bool) -> Result<&'static LabChaos, ChaosRefused>;
/// None when not installed; every hook is then a no-op.
pub fn current() -> Option<&'static LabChaos>;
```

**Feature rule.** `lab-chaos` is declared on `cimmeria-base-session`
(`lab-chaos = []`) and forwarded by `cimmeria-base-world-entry`,
`cimmeria-base` (which also turns on its `chaos-testing`),
`cimmeria-services`, `cimmeria-lab-mcp` and `cimmeria-server`. **No crate
enables it in `[dependencies]` or `[dev-dependencies]`**; it is only ever
turned on from the command line (`--features lab-chaos`), so feature
unification and `cargo hakari` never fold it into an ordinary build.

### lab-mcp tools (CH-05)

| Tool | Arguments | Result (compact) |
|---|---|---|
| `server_chaos_set` | `entity_id`, `net` or `flush` object (the knobs above), `seed`, `ttl_s` | `{set: true, expires_in_s}` or `{refused: "<reason>"}` |
| `server_chaos_clear` | optional `entity_id` | `{cleared: n}` |
| `server_chaos_status` | none | `{sessions: [{entity_id, net?, flush?, expires_in_s}]}`, capped at 10 |

The tools are listed only in a binary built with `lab-chaos`.

## CH-01 Chaos profile types, parser and levels file

**Implementer:** packet-coder. **Size:** S. **Wave:** 1.

**Branch:** `lab-chaos/ch01-profiles`. **Worktree:** `ch01`.

**Change:** create `crates/lab/src/chaos/mod.rs`, `levels.rs` and
`levels.toml` per the contract; `pub mod chaos;` in `crates/lab/src/lib.rs`
(or `main.rs` if the crate has no `lib.rs`; match how `uat` is declared).
`cpu.rs` and `burst.rs` are not created here.

`levels.toml` shape, with the provisional numbers from the README:

```toml
# Each level cites the docs/client/limits/ row it came from.
# "provisional" until CH-10 measures it.
[cpu.safe]
cores = "max-2"       # logical cores minus 2
duty = "100"
source = "provisional; engine.md busy cores"
[cpu.break]
cores = "max"
duty = "100"
source = "provisional"
# net.safe, net.break, flush.safe (mode = "delay", delay_ms = "2000"),
# flush.break (mode = "pack", records = "24"), burst.safe (count = "50"),
# burst.break (count = "250") likewise.
```

**Tests** (unit, TESTING.md type 1, in `mod.rs`'s `mod tests`):

- `bare_profile_means_safe_level`: `parse("cpu", ..)` gives `Level::Safe`
  and the safe knobs.
- `explicit_knob_overrides_the_level`: `"net:break:latency=120"` keeps the
  break loss and takes latency 120.
- `unknown_profile_and_knob_are_refused`: each error variant once.
- `seed_defaults_and_overrides`: no seed gives `default_seed`; `seed=7` gives 7.
- `header_is_one_line`: `header` of `flush:pack seed=7` is exactly
  `chaos=flush:pack seed=7`.
- `levels_file_parses_and_every_level_has_a_source`: loads the embedded
  file; every level of every profile exists and has `source`. Fails if a
  level is missing.

**Lane:** fmt, clippy, nextest, each `-p cimmeria-lab`.

**Doc rows:** none (CH-12 documents the CLI).

**Commit:** `feat(lab): chaos profile types, parser and levels (CH-01)`

## CH-02 lab uat --chaos

**Implementer:** packet-coder. **Size:** S. **Wave:** 2. Depends on CH-01.

**Branch:** `lab-chaos/ch02-cli`. **Worktree:** `ch02`.

**Change** `tools/lab/cli/uat.ps1` and `uat-lib.ps1`:

1. A `-Chaos <string>` parameter (repeatable: `-Chaos cpu,flush:pack`),
   passed through unchanged as `chaos: [..]` in the `lab_uat_run` arguments
   (`crates/lab/src/server/uat.rs` accepts and ignores unknown fields until
   CH-08; confirm, and if it rejects them stop and tell the coordinator).
2. `-Json` output: when the run had chaos, a top-level `chaos` string (the
   headers joined by a semicolon and a space). No key when there was none (no nulls).
3. Human output: the header line first.

**Tests** in `tools/lab/cli/test-uat.ps1` (PowerShell CLI test, run by
`.github/workflows/lab.yml`): the argument reaches the request body; two
`-Chaos` values both arrive; `-Json` has `chaos` only when given. Each fails
if the pass-through is removed.

**Lane:** none compiles; run `pwsh -NoProfile -File tools/lab/cli/test-uat.ps1`.

**Doc rows:** `docs/guides/live-research-lab.md#commands` (one line; CH-12
writes the full section).

**Commit:** `feat(lab-cli): lab uat -Chaos pass-through (CH-02)`

## CH-03 cpu stressor

**Implementer:** packet-coder. **Size:** S. **Wave:** 1.

**Branch:** `lab-chaos/ch03-cpu`. **Worktree:** `ch03`.

**Change:** create `crates/lab/src/chaos/cpu.rs` (CH-01 adds `pub mod cpu;`;
if CH-01 has not merged, add the `mod.rs` line yourself and say so in the
commit body):

```rust
pub struct CpuStress { pub cores: usize, pub duty_percent: u8 }
pub struct CpuGuard { /* stop: Arc<AtomicBool>, handles: Vec<JoinHandle<()>> */ }
impl CpuStress {
    /// "max" and "max-N" resolve against std::thread::available_parallelism.
    pub fn from_knobs(cores: &str, duty: &str) -> Result<CpuStress, String>;
    pub fn start(&self) -> CpuGuard;
}
impl CpuGuard {
    pub fn running(&self) -> usize;
    /// Stops and joins every thread. Also called by Drop.
    pub fn stop(&mut self);
}
impl Drop for CpuGuard { fn drop(&mut self) { self.stop() } }
```

Each thread spins for `duty`% of every 10 ms slice and sleeps the rest
(100% never sleeps), checking the stop flag each slice. Threads are named
`lab-chaos-cpu-<n>`. `cores` is clamped to 1..=available.

**Tests** (unit, type 1, `mod tests` in `cpu.rs`):

- `max_minus_two_resolves_and_clamps`: `"max-2"` on the machine's count;
  `"max-999"` clamps to 1; `"0"` is refused.
- `guard_stops_every_thread_on_drop`: start 2 threads at 10% duty, drop the
  guard, then assert the threads have joined (keep a clone of a counter the
  threads hold and check its strong count falls to 1). Fails if `Drop` does
  not stop them.
- `stop_is_idempotent`.

Keep the tests at 10% duty and under 200 ms so they do not load CI.

**Lane:** fmt, clippy, nextest, each `-p cimmeria-lab`.

**Commit:** `feat(lab): cpu chaos stressor with a stopping guard (CH-03)`

## CH-04 Server chaos state and gates

**Implementer:** rust-gameserver-dev; `network-security-auth` reviews.
**Size:** M. **Wave:** 2. **Blocked on D-CH2 and D-CH3.**

**Branch:** `lab-chaos/ch04-server-gate`. **Worktree:** `ch04`.

**Change:**

1. `crates/base-session/src/base/lab_chaos/mod.rs` per the contract, the
   whole module behind `#[cfg(feature = "lab-chaos")]` except `COMPILED`.
2. The feature declarations and forwarding per the contract's feature rule,
   in each crate's `Cargo.toml`. Run `cargo hakari generate --diff` through
   the lane and confirm `lab-chaos` does not appear in the workspace-hack.
3. `crates/server/src/main.rs`: under the feature, call
   `lab_chaos::install(config.developer_mode)` after config load; on refusal
   log one INFO that lab chaos is compiled but off; on install log one WARN
   that lab chaos is enabled.
4. `LabChaos::set` logs a WARN (`target: "lab.chaos"`, fields `profile`,
   `addr_hash` (not the address), `seed`, `expires_in_s`); `clear` and
   expiry log INFO.

**Tests:**

- In `lab_chaos/mod.rs` (unit, type 1, run with `--features lab-chaos`):
  `install_refuses_without_developer_mode`; `expired_entry_reads_as_none`
  (set with an `expires_at` in the past); `clear_none_clears_every_session`;
  `scope_is_per_address` (a second address sees nothing).
- In `crates/server/src/main.rs`'s tests (or a `tests/` file),
  `#[cfg(not(feature = "lab-chaos"))] fn default_build_has_no_lab_chaos()`
  asserting `!lab_chaos::COMPILED` (reached through a `cimmeria-services`
  re-export: the server depends on `cimmeria-services`, not on
  `cimmeria-base-session` directly). This is the regression guard for the
  feature rule: it fails if any crate starts enabling `lab-chaos` through a
  dependency, because unification then compiles it in.

**Lane:** fmt; clippy and nextest `-p cimmeria-base-session` with and
without `--features lab-chaos`; then `-p cimmeria-server` without it.

**Doc rows:** `docs/architecture/network-chaos-testing.md` (a short "L4: lab
chaos" note in the worknote for CH-12); server env table in `main.rs`'s
module doc if it lists features.

**Commit:** `feat(base-session): lab-chaos state behind a feature and DEVELOPER_MODE (CH-04)`

## CH-05 lab-mcp chaos tools

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. Depends on CH-04.

**Branch:** `lab-chaos/ch05-mcp-tools`. **Worktree:** `ch05`.

**Change:** `crates/lab-mcp/src/tools/chaos.rs` with the three tools from
the contract, registered in `crates/lab-mcp/src/tools/mod.rs` the same way
`witnesses.rs` is, all under `#[cfg(feature = "lab-chaos")]`. Resolve
`entity_id` to the session address through the same path `sessions.rs`
uses. Clamp `ttl_s` (default `DEFAULT_TTL_S`, refuse over `MAX_TTL_S`).
When `lab_chaos::current()` is None, every tool answers
`{refused: "lab chaos is off (DEVELOPER_MODE)"}`. Add the names to
`names.rs` if tool names are listed there. Put tests in `chaos_tests.rs`
beside it, as the other tools do.

**Tests** (`crates/lab-mcp/src/tools/chaos_tests.rs`, type 1, with
`--features lab-chaos`): refused when not installed; `ttl_s` over the cap is
refused and the default applies when absent; set then status shows the
session and `expires_in_s <= 300`; clear returns the count. A test without
the feature asserts the tool names are not in the tool list. Each fails if
its gate is removed.

**Lane:** fmt; clippy and nextest `-p cimmeria-lab-mcp` with and without
`--features lab-chaos`.

**Doc rows:** `docs/guides/live-research-lab.md` server tool table (one row
per tool, marked "lab-chaos builds only").

**Commit:** `feat(lab-mcp): server_chaos_set, clear and status tools (CH-05)`

## CH-06 net: per-session lossy transport

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 3. **Blocked on
D-CH1** (this packet is for option (a)) and CH-04.

**Branch:** `lab-chaos/ch06-net`. **Worktree:** `ch06`.

**Change:**

1. A `ChaosTransport` in `crates/base/src/base/lab_chaos_transport.rs`
   (feature `lab-chaos`), implementing `BidirectionalTransport` over the
   real `UdpTransport`. On each send to `addr`, if
   `lab_chaos::current()?.net_for(addr, now)` is `Some`, apply the
   `NetChaos` with the same semantics as `LossyConfig` (drop, latency,
   jitter, duplicate, reorder), using a per-address `ChaCha20Rng` seeded from
   `seed`. Reuse `LossyTransport`'s filter code by extracting it into a
   function both call, rather than copying it.
2. **Receive side: drop only, by source address.** Do not add receive
   latency: a sleep in `recv` would stall every session. Send-side latency
   on the server's sends is the only latency the profile adds; document that
   the round trip grows by it once.
3. `BaseService::start`: under the feature and when `lab_chaos::current()`
   is `Some`, wrap the bound socket's transport in `ChaosTransport`; with the
   feature off, the code path is unchanged.

**Tests** (network chaos, TESTING.md type 10, under
`crates/mercury/src/test_harness/tests/chaos/` if the filter extraction
lives in mercury, otherwise in `crates/base` with the feature):

- `net_chaos_is_scoped_to_one_peer`: two peers, chaos on one at 100% loss;
  the other receives every packet. Fails if the scope check is dropped.
- `net_chaos_replays_with_the_same_seed`: the same seed drops the same
  sequence numbers twice.
- `expired_net_chaos_stops_dropping`.
- `receive_drop_never_delays_other_peers`: peer A under receive drop; peer
  B's packets arrive within the normal time.

**Lane:** fmt; clippy and nextest `-p cimmeria-mercury`, then
`-p cimmeria-base` with and without `--features lab-chaos`.

**Doc rows:** "Network-chaos primitives" (worknote for CH-12:
`network-chaos-testing.md`, TESTING.md type 10).

**Commit:** `feat(base): per-session lab-chaos net shim over the real transport (CH-06)`

## CH-07 flush modes

**Implementer:** rust-gameserver-dev; `aoi-witness-broadcast` designs the
filler and reviews; `bigworld-engine-advisor` reviews. **Size:** L.
**Wave:** 3. Depends on CH-04.

**Branch:** `lab-chaos/ch07-flush`. **Worktree:** `ch07`.

**First, the filler (design step, before code):** `aoi-witness-broadcast`
names one message that is harmless at the head of a phase-1 bundle and whose
length can be any value from 1 to 36 bytes beyond its header (README F6),
with evidence from `docs/protocol/message-dispatch-table.md` and the
receive-path finding. If no such message exists, `Shift` and `ShiftSweep`
are dropped from this packet and the ledger is updated; `Pack` alone carries
the #1341 acceptance.

**Change:** a sibling `crates/base-world-entry/src/base/world_entry/cell_dispatch/flush_chaos.rs`
(feature `lab-chaos`) with
`pub(crate) fn apply(addr: SocketAddr, phase1: &mut Vec<..>, phase2: &mut Vec<..>) -> Option<Duration>`
that reads `lab_chaos::current()?.flush_for(addr, now)` and:

- `Delay { ms }`: returns `Some(ms)`; `dispatch_deferred` sleeps that long
  before sending, logging INFO `lab.chaos` `flush_delayed`.
- `Reorder { seed }`: seeded shuffle of the introductions inside each phase,
  keeping each entity's phase-1 record whole.
- `Pack { records }`: send the first `records` phase-1 records as one
  single-packet bundle, as the server did before #1341's mitigation, even if
  the mitigation is on.
- `Shift { len }` / `ShiftSweep { seed }`: prepend the filler of `len` bytes
  (`ShiftSweep`: `len = rng % 37`, logged).

`dispatch_deferred` calls `apply` once per flush, under
`#[cfg(feature = "lab-chaos")]`; nothing else in `deferred_flush.rs`
changes.

**Tests** (fan-out byte tests, TESTING.md type 8, beside the existing
`cell_dispatch/tests.rs` cases, with `--features lab-chaos`):

- `flush_without_chaos_is_byte_identical`: the existing 28-NPC burst test's
  bytes, with the feature on and no chaos set, equal the bytes with the
  feature off. Fails if `apply` changes anything when unset.
- `pack_puts_records_in_one_packet`: 24 NPCs, `Pack { records: 24 }`; the
  phase-1 bundle is one packet of 889 bytes. Fails if the mitigation path is
  taken.
- `shift_moves_every_cursor_by_len`: `Shift { len: 5 }`; the first create's
  offset is 1 plus the filler's total length.
- `reorder_is_seeded`: the same seed gives the same order twice; a
  different seed a different order.
- `delay_returns_the_configured_wait`.

**Lane:** fmt; clippy and nextest `-p cimmeria-base-world-entry` with and
without `--features lab-chaos`.

**Doc rows:** worknote for CH-12 (`docs/architecture/network-chaos-testing.md`).

**Commit:** `feat(base-world-entry): lab-chaos flush delay, reorder, pack and shift (CH-07)`

## CH-08 burst and runner wiring

**Implementer:** packet-coder. **Size:** M. **Wave:** 4. Depends on CH-01,
CH-03, CH-05.

**Branch:** `lab-chaos/ch08-runner`. **Worktree:** `ch08`.

**Change:**

1. `crates/lab/src/chaos/burst.rs`: `pub fn spawn_command(template: u32,
   count: u32) -> String` (`.spawnrandom <template> 20 20 <count>`) and
   `pub fn despawn_commands(ids: &[u32]) -> Vec<String>`, plus a parser for
   the spawned ids from the console's reply (read the reply format in
   `crates/cell-console/src/cell/console/spawn/mod.rs`; if it does not list
   ids, stop and tell the coordinator).
2. `crates/lab/src/uat/runner/chaos.rs`: a `ChaosSession` built from the
   run's `Vec<ChaosSpec>` that, around every row, starts and stops what each
   profile needs: `cpu` a `CpuGuard` for the whole run; `net` and `flush` one
   `server_chaos_set` before the row's first step (scoped to the row's
   player entity, `ttl_s` = the row's timeout plus 60) and
   `server_chaos_clear` after its teardown; `burst` the spawn after setup and
   the despawn after teardown. On any error or cancel, everything is undone
   (a `Drop` that clears and despawns).
3. `crates/lab/src/uat/runner/mod.rs`: build the `ChaosSession` from the
   request's `chaos` field (parsed with `chaos::parse`) and hold it for the
   run. Keep the change to a few lines; `mod.rs` is near the 500-line soft
   cap.

**Tests** (unit, type 1, `crates/lab/src/uat/runner/tests.rs` or a new
`chaos_tests.rs`, with the runner's existing fake MCP):

- `flush_chaos_is_set_before_and_cleared_after_each_row`: the fake records
  `server_chaos_set` then the row's steps then `server_chaos_clear`.
- `failed_row_still_clears_and_despawns`: a step fails; clear and despawn
  are still sent.
- `burst_despawns_exactly_what_it_spawned`.
- `no_chaos_sends_no_chaos_calls`.

Each fails if its cleanup path is removed.

**Lane:** fmt, clippy, nextest, each `-p cimmeria-lab`.

**Commit:** `feat(lab): run uat rows under chaos profiles with guaranteed cleanup (CH-08)`

## CH-09 Chaos report against the golden

**Implementer:** packet-coder. **Size:** S. **Wave:** 5. Depends on CH-08
GD-04 (`diff_run` and `Divergence::line`, see
[lab-golden contract](../lab-golden/contract.md)) and GD-09 (`lab uat -DiffGolden`).

**Branch:** `lab-chaos/ch09-report`. **Worktree:** `ch09`.

**Change:** when a run has chaos, the runner diffs each row against the
golden with lab-golden's API, and the row's result gains
`chaos_divergence: "<first divergence line>"` when the row passed its own
expects but diverged, so a chaos-only change is not a row failure (D-CH7).
The run summary leads with `header(spec)`. A chaos run refuses
`lab golden record`. Use lab-golden's types as they are; do not add a
second diff.

**Tests** (unit, type 1): a row that passes and diverges reports one
`chaos_divergence` line and status pass; a row that matches has no key;
`golden record` with chaos is refused. `-Json` size for a 6-row chaos run
stays under 600 characters (assert it).

**Lane:** fmt, clippy, nextest, each `-p cimmeria-lab`; then
`tools/lab/cli/test-uat.ps1`.

**Commit:** `feat(lab): chaos runs report the first divergence from the golden (CH-09)`

## CH-10 Live: calibrate levels

**Implementer:** the coordinator, with a Haiku `lab-driver`. **Size:** M.
**Wave:** 6. **Needs the user's OK**, a local server built with
`--features lab-chaos` and `DEVELOPER_MODE=true` (colo images never have the
feature), and the lab pointed at it.

For each profile, against `first-session` FS-01 to FS-P5 and its golden:

1. Safe level, 5 runs: every row must match the golden. If one does not,
   lower the level and repeat; record the highest clean level.
2. Break level, 3 runs: record the failure each one produces (one line).
3. Update `crates/lab/src/chaos/levels.toml` with the measured numbers and a
   `source` naming the run ids and date, and the matching safe and break
   cells in `docs/client/limits/`.

**Commit:** `chore(lab): measured chaos levels (CH-10)`

## CH-11 Live acceptance: reproduce #1341

**Implementer:** the coordinator, with a Haiku `lab-driver`. **Size:** M.
**Wave:** 6. **Needs the user's OK**; same server as CH-10, with #1341's
mitigation (if merged) on.

1. Baseline: FS-01 to FS-P2 (fresh first login) 10 times, no chaos. Count
   runs with a `client.mercury.request_misparse` at the flush and Frost not
   created.
2. `-Chaos flush:pack` 5 times, then `-Chaos flush:shift=sweep` until a hit
   or 37 logins. Pass: `pack` hits in at least 4 of 5 runs, and each hit's
   report is one `chaos_divergence` line naming the missing introduction.
3. `-Chaos cpu:break` 10 times. Pass: the hit rate is clearly above the
   baseline (report both counts). If it is not, record it (README F8), tell
   the owner, and acceptance rests on `flush`.
4. With #1341's mitigation on, `flush:pack` must still hit (it rebuilds the
   old shape) and `flush:delay` and `flush:reorder` must not.

**Write:** the counts in `worknotes/CH-11.md` and a comment on #1341.

**Commit:** `docs(lab-chaos): #1341 reproduced on demand (CH-11)`

## CH-12 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 7.

- `docs/guides/live-research-lab.md`: a "Chaos mode" section (profiles,
  levels, the server build it needs, the gates, cleanup guarantees).
- `docs/guides/automated-uat.md`: running a spec under chaos.
- `docs/architecture/network-chaos-testing.md`: an L4 row for lab chaos
  (runtime, per-session, feature and `DEVELOPER_MODE` gated).
- `TESTING.md` type 10: the per-session shim tests as a pattern.
- `docs/guides/unified-uat.md`: CH-11's steps for the owner.
- `docs/project-status.md`, `docs/gap-analysis/`: one line each.
- Mark every packet Done in the ledger, retire worktrees
  (`pwsh tools/build-lane/rm-worktree.ps1 --merged`), post a board handoff.

**Commit:** `docs(lab-chaos): close-out (CH-12)`
