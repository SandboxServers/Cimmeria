# Client limits and bounds: work packets

> Type: work packets. Audience: `game-archaeology-specialist` for the RE
> packets, `packet-coder` workers (Haiku) for the code packets,
> `documentation-writer`, and `packet-reviewer` reviewers (Sonnet). Ledger,
> findings (F1 to F18), the bounds table skeleton and decisions (D-CL1 to
> D-CL8): [README.md](README.md).
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
> Read the lane's summary and its failures file; do not rerun a build to see
> the output. Docs-only packets run `pwsh -NoProfile -File tools/lint-md.ps1`
> on the files they touch (warn-only, but fix what it reports).
>
> Every value a packet writes follows D-CL2: a grade (static, config,
> measured, inferred) and a source. A packet that cannot find a value writes
> "not measured" and the reason, never a guess.

## Contents

- [Contract](#contract)
- [CL-01 Bounds docs skeleton](#cl-01-bounds-docs-skeleton)
- [CL-02 RE: client receive and resend limits](#cl-02-re-client-receive-and-resend-limits)
- [CL-03 RE: client entity capacity and interpolation](#cl-03-re-client-entity-capacity-and-interpolation)
- [CL-04 Static: PE flags, engine ini chain, background throttle](#cl-04-static-pe-flags-engine-ini-chain-background-throttle)
- [CL-05 SigNoz mining](#cl-05-signoz-mining)
- [CL-06 DLL: frame-time percentiles](#cl-06-dll-frame-time-percentiles)
- [CL-07 Measurement spec](#cl-07-measurement-spec)
- [CL-08 Live: per-world memory and frame time](#cl-08-live-per-world-memory-and-frame-time)
- [CL-09 Live: create-burst ceiling](#cl-09-live-create-burst-ceiling)
- [CL-10 Live: stock-client residue probe](#cl-10-live-stock-client-residue-probe)
- [CL-11 Timing from captures](#cl-11-timing-from-captures)
- [CL-12 Raise candidate: Large Address Aware lab trial](#cl-12-raise-candidate-large-address-aware-lab-trial)
- [CL-13 Raise candidate: lab ini overrides](#cl-13-raise-candidate-lab-ini-overrides)
- [CL-14 Raise candidate: server send window](#cl-14-raise-candidate-server-send-window)
- [CL-15 Raise candidate: lab low-resolution window](#cl-15-raise-candidate-lab-low-resolution-window)
- [CL-16 Close-out](#cl-16-close-out)

## Contract

### `docs/client/limits/` layout (CL-01 creates it)

```text
README.md     the summary table (one row per bound, linking its area file), grade key, how to update
network.md    datagrams, windows, resend, fragments, bundles, the #1341 residue
aoi.md        radius, entity count, create bursts, the deferred buffer, the first-login hold
process.md    address space, working sets, cache archives
engine.md     frame time, texture pool, smoothing, background throttle, display loss
timing.md     server tick, interpolation, world entry, teleport to AoI
lab.md        clients per daemon, bridge start, window size
```

Each area file has one table with exactly these columns, so lab-chaos can
read it:

```markdown
| Bound | Value | Grade | Source | Safe level | Break level | Notes |
```

- **Value**: the number with its unit, or `not measured`.
- **Grade**: `static`, `config`, `measured` or `inferred`.
- **Source**: a Ghidra address, a repo path and constant, an ini file and
  key, or a measurement (`SigNoz, 2026-10-12, 14 sessions`). No IPs, account
  names or local paths.
- **Safe level / Break level**: filled only for the bounds a lab-chaos
  profile drives (marked in CL-01 below); `n/a` otherwise.

### Chaos-driven bounds

Lab-chaos reads these rows; each must end with a safe and a break level
before CH-10 runs:

| Area file | Bound | Chaos profile |
|---|---|---|
| `aoi.md` | Creates in one flush the client absorbs | `burst`, `flush` |
| `aoi.md` | Entities the client holds at once | `burst` |
| `network.md` | Server-side added latency and jitter the session survives | `net` |
| `network.md` | Server-side loss rate the session survives | `net` |
| `engine.md` | Busy cores at which frame time passes the hitch threshold | `cpu` |
| `network.md` | Flush delay after the hold before the client times anything out | `flush` |

## CL-01 Bounds docs skeleton

**Implementer:** documentation-writer. **Size:** S. **Wave:** 1.

**Branch:** `client-limits/cl01-bounds-skeleton`. **Worktree:** `cl01`.

**Do:**

1. Create the seven files in the contract layout. Move every row of the
   ledger's bounds table skeleton into its area file in the contract's
   column format, keeping the value, grade and source. Rows the ledger marks
   "none" to measure keep their value; the others say `not measured` with the
   packet that will fill them in Notes.
2. Add the six chaos-driven rows from the contract, `not measured`, with
   their profile in Notes.
3. `README.md`: purpose (two sentences), the grade key, the summary table
   (bound, value, area file), and "how to update" (change the area file, keep
   the grade and source, update the summary row in the same PR).
4. Link `docs/client/limits/README.md` from `docs/client/README.md` (or
   `docs/client-tools.md` if `docs/client/` has no README), and from
   `docs/readme.md`.

**Test:** docs-only. `tools/lint-md.ps1` clean on the new files; every
relative link resolves (open each one).

**Doc rows:** "Client-side analysis" (`docs/client/`); "Adding or renaming
a doc" (`docs/readme.md`).

**Commit:** `docs(client): bounds table skeleton under docs/client/limits (CL-01)`

## CL-02 RE: client receive and resend limits

**Implementer:** game-archaeology-specialist. **Size:** M. **Wave:** 1.

**Branch:** `client-limits/cl02-receive-limits`. **Worktree:** `cl02`.

**Questions,** each answered with an address or "not found, searched X":

1. **Assembled bundle size.** Does `Bundle::iterator::data()` (`0x01579a50`)
   allocate its temp buffer with a fixed size or per message? What is the
   largest message body it can copy? Is there any cap on the number of
   packets in a chain between `Nub::processPacket` (`0x0157fd20`) and
   `processOrderedPacket` (`0x0157c820`)?
2. **The client's own resend.** For reliable packets the client sends: the
   RTO, the retry cap and what happens at the cap (the channel class around
   `Channel::send`, `0x01576f90`, and `UnAckedHandler`).
3. **Socket buffer.** Any `setsockopt` with `SO_RCVBUF` or `SO_SNDBUF` on the
   game socket (the IAT thunk for `setsockopt`; the socket created near
   `FUN_0158a200`). If none, say the Windows default applies.
4. **Per-tick drain.** Does the game thread drain the whole
   `ClientIncomingMessage` queue each tick (the loop that runs
   `FUN_0158d7d0`'s work items), or a fixed number?

**Start from:** `docs/reverse-engineering/findings/client-mercury-receive-path.md`,
`docs/protocol/mercury-wire-format.md`, `docs/drafts/spec/mercury-wire-format.md`.
Check them before Ghidra (the `re-lookup` skill's order).

**Write:** the answers as rows in `docs/client/limits/network.md` (or
`worknotes/CL-02.md` if CL-01 has not merged), and the addresses in a new
section of `client-mercury-receive-path.md`, "Receive limits".

**Test:** none (docs). A `bigworld-engine-advisor` read of the claims about
the BigWorld channel.

**Doc rows:** "New RE finding" (`docs/reverse-engineering/findings/README.md`
if a finding file is added; a new section needs no index row).

**Commit:** `docs(re): client receive, resend and drain limits (CL-02)`

## CL-03 RE: client entity capacity and interpolation

**Implementer:** game-archaeology-specialist; `bigworld-engine-advisor`
reviews. **Size:** M. **Wave:** 1.

**Branch:** `client-limits/cl03-entity-capacity`. **Worktree:** `cl03`.

**Questions:**

1. **Entity table.** Where the client keeps entities by id (the BigWorld
   entity manager's map, reached from the `CREATE_ENTITY` handler, message
   `0x09` in `docs/protocol/message-dispatch-table.md`). Fixed array or
   growable? Any count check, and what happens past it?
2. **Actors.** Does each entity spawn a UE3 actor (`UWorld::SpawnActor`,
   `0x00876970`), and is there an actor-count limit in this build?
3. **Interpolation.** The avatar filter's time constants (how far behind
   the server the client renders, extrapolation limit). Check
   `docs/drafts/spec/position-updates.md` first.
4. **AoI radius.** Confirm the client has no radius of its own (it creates
   whatever the server introduces) or name where it culls.

**Write:** rows in `docs/client/limits/aoi.md` and the interpolation rows in
`docs/client/limits/timing.md` (worknote if CL-01 has not merged); addresses
in `docs/reverse-engineering/findings/` (extend the closest existing finding,
or add `client-entity-capacity.md` and its index row).

**Test:** none (docs).

**Commit:** `docs(re): client entity capacity and interpolation bounds (CL-03)`

## CL-04 Static: PE flags, engine ini chain, background throttle

**Implementer:** game-archaeology-specialist. **Size:** S. **Wave:** 1.

**Branch:** `client-limits/cl04-static-config`. **Worktree:** `cl04`.

Reading the client install's files is not lab use; do not start, stop or
attach to any `SGW.exe`.

1. **PE header.** Read `SGW.exe`'s file header `Characteristics` with
   PowerShell (`[IO.File]::ReadAllBytes`, `e_lfanew` at `0x3c`, then
   `Characteristics` at `e_lfanew + 0x16`). Record whether
   `IMAGE_FILE_LARGE_ADDRESS_AWARE` (`0x0020`) is set, and the file offset of
   that byte, next to the ASLR byte (`0x186`) in `docs/client/sgw-launcher.md`.
2. **Ini chain.** From the client's `Config/` and `Engine/Config/` folders:
   the effective value and the defining file of `[TextureStreaming]`
   `PoolSize`, `bSmoothFrameRate`, `MinSmoothedFrameRate`,
   `MaxSmoothedFrameRate`, and any key naming background, focus or idle
   (for example `bLowerPriorityWhenInBackground`). Follow `BasedOn=` lines.
   Note whether the user-folder `SGWEngine.ini` overrides each.
3. **Background throttle.** The `Sleep(5)` call site inside or under
   `FEngineLoop::Tick` (the DLL hooks it; find the address in
   `crates/client-telemetry/src/hooks/`), the condition that guards it, and
   whether an ini key or command-line switch reaches that condition.

**Write:** rows in `docs/client/limits/process.md` and `engine.md`
(worknote if CL-01 has not merged); the PE byte in `sgw-launcher.md`.

**Test:** none (docs).

**Commit:** `docs(client): PE flags, engine ini pools and background throttle (CL-04)`

## CL-05 SigNoz mining

**Implementer:** the coordinator, with the `telemetry-triage` skill. **Size:**
S. **Wave:** 1. No lab use.

**Queries** (filter `service.name` for the client service, last 30 days,
condense large results; group by `cimmeria.session_kind` so lab and player
sessions are counted apart):

1. `client.engine.memory`: distinct `total_virtual_mb` values (the Large
   Address Aware answer, F12); per world, max `peak_working_set_mb` and min
   `avail_virtual_mb`; count of `warn` samples.
2. `client.engine.hitch`: per world, count and p95 `gap_ms`.
3. `client.mercury.request_misparse`: distinct `next_request_offset` per
   session and the cursor it hit (extends F6's table).
4. Server: `aoi.create_emit` `bytes` and `packets` per `phase` (largest
   seen); `event = "oversize_fragmented"`; `tx_hole_stall` count; any
   `deferred` buffer-full WARN from `push_deferred`.
5. World entry: `playCharacter` to `onClientReady` per world.

**Write:** `worknotes/CL-05.md` with each query, its result condensed, the
date and the sample sizes; then the values into the area files (or hand them
to CL-01's author if it has not merged). Session ids as 8-character prefixes;
no addresses or names.

**Decides:** D-CL4's input. If every world's minimum free address space is
above 512 MB, record that D-CL4's trial is not needed.

**Commit:** `docs(client-limits): SigNoz memory, hitch and flush bounds (CL-05)`

## CL-06 DLL: frame-time percentiles

**Implementer:** packet-coder. **Size:** S. **Wave:** 1.

**Branch:** `client-limits/cl06-frame-percentiles`. **Worktree:** `cl06`.

**Why:** nothing reports ordinary frame times (F12); the hitch event fires
only past 200 ms.

**Change** `crates/client-telemetry/src/hooks/seams/frame_health.rs` only:

1. Add a fixed-bucket histogram of tick-to-tick gaps, no allocation on the
   tick path:

   ```rust
   /// Upper bounds in ms of the frame-gap buckets; the last is open-ended.
   pub const FRAME_BUCKETS_MS: [u32; 12] = [8, 12, 17, 25, 33, 50, 67, 100, 150, 200, 500, u32::MAX];
   pub struct FrameGaps { counts: [u32; 12], max_ms: f64, frames: u32 }
   impl FrameGaps {
       pub const fn new() -> Self;
       pub fn record(&mut self, gap_ms: f64);
       /// Bucket upper bound at or above the percentile `p` (0.0..=1.0); None when empty.
       pub fn percentile_ms(&self, p: f64) -> Option<u32>;
       pub fn reset(&mut self);
   }
   ```

   Keep it behind the existing tick state (a `Mutex<FrameGaps>` or atomics
   per bucket, matching how `LAST_TICK` is held); record every gap the tick
   hook already computes.
2. In `memory_fields`, add `frames`, `frame_p50_ms`, `frame_p95_ms`,
   `frame_max_ms` from the histogram, then reset it, so each 30 s sample
   covers its own window. Omit the fields when no frame was recorded (no
   nulls).
3. Update the `client.engine.memory` row in
   `docs/architecture/client-telemetry.md` § Subsystem seams.

**Tests** (unit, TESTING.md type 1, in the file's `mod tests`):

- `frame_percentiles_pick_the_bucket_bound`: record 90 gaps of 16 ms and 10
  of 120 ms; p50 is 17, p95 is 150, max is 120.0. Fails if the percentile
  walks the wrong way or uses the lower bound.
- `memory_sample_resets_the_frame_window`: two `memory_fields` calls; the
  second has no frame fields. Fails if the reset is dropped.
- Extend `a_memory_sample_reports_every_number` for the new keys.

**Lane:** fmt, clippy, nextest, each `-p cimmeria-client-telemetry`.

**Doc rows:** client telemetry events (`docs/architecture/client-telemetry.md`).

**Commit:** `feat(client-telemetry): frame-time percentiles on client.engine.memory (CL-06)`

## CL-07 Measurement spec

**Implementer:** packet-coder. **Size:** S. **Wave:** 2. Depends on CL-06
(merged and the DLL rebuilt for the lab, which the coordinator does).

**Branch:** `client-limits/cl07-measurement-spec`. **Worktree:** `cl07`.

**Add** `docs/guides/uat-specs/client-limits.toml`, modelled on
`first-session.toml` and the schema in `crates/lab/src/uat/spec.rs`
(authoring guide: `docs/guides/automated-uat.md`):

- Section `client-limits`, a GM lab character, `players = 1`.
- One row per world in this order: Castle_CellBlock, Castle, Harset,
  Agnos, Ihpet_Crater_Light (world 1300), Dakara_E1, Tollana. Each row:
  `chat = ".gotospace <world>"`, `wait_ms = 90000` (three memory samples),
  then an `expect` clause that at least three `client.engine.memory` events
  arrived in the row's window, and `evidence` that captures them.
- A final row `CL-M5` with `players = 5` that stands all five clients in
  Castle for 90 s.
- The header records what each row measures and that the numbers go to
  `docs/client/limits/process.md` and `engine.md`.

Use only actions and clause sources that already exist; if a needed clause
source is missing, stop and tell the coordinator.

**Test:** the lab workflow's spec validation (`.github/workflows/lab.yml`)
must pass; run it locally with the command `docs/guides/automated-uat.md`
gives for validating a spec. Fails if a row uses an unknown tool or clause.

**Doc rows:** "UAT steps" (`docs/guides/automated-uat.md` spec list).

**Commit:** `docs(uat): client-limits measurement spec (CL-07)`

## CL-08 Live: per-world memory and frame time

**Implementer:** the coordinator, driving a Haiku `lab-driver` through
`lab uat`. **Size:** M. **Wave:** 3. **Needs the user's OK** and a lab with
five free instances.

1. Ask the user; wait until the lab is free.
2. `lab uat client-limits -Leases 1 -RunsPerLease 3 -Json`, then the `CL-M5`
   row with `-Leases 5`.
3. Read the memory and frame fields from SigNoz for the run's sessions
   (`cimmeria.session_kind = lab`).
4. Write per world: max `peak_working_set_mb`, min `avail_virtual_mb`,
   p95 frame time; and for five clients, the same plus the machine's CPU and
   memory load from `machine_load_percent`. Fill the `engine.md`
   chaos-driven `cpu` row's safe level from the five-client p95 (a busy-core
   count is CH-10's job).

**Write:** `docs/client/limits/process.md`, `engine.md`, `lab.md`, graded
`measured` with the date and run count.

**Commit:** `docs(client-limits): measured per-world memory and frame time (CL-08)`

## CL-09 Live: create-burst ceiling

**Implementer:** the coordinator, with a Haiku `lab-driver`. **Size:** M.
**Wave:** 3. **Needs the user's OK.**

1. One lab client in Debug Area (world 1300), GM character, standing on
   paving (see the Ihpet render-gaps note in the shared memory index).
2. For K in 25, 50, 100, 161, 250: despawn the previous lineup, then
   `.spawnrandom <template> 20 20 <K>` with one fixed humanoid template
   (pick one with a large cascade, such as 221, so each introduction is the
   worst case). Wait 30 s. Read `server_witnesses` for the player and
   `client_entity_table`; record introduced against created, and any
   `client.mercury.*` fault or `tx_hole_stall`.
3. Repeat K = 100 and 250 while the player relogs into the space, so the
   introductions go through the deferred flush (the `flush` path, F7).
4. The ceiling is the smallest K with any introduced-but-not-created entity
   in two of three tries.

**Write:** `docs/client/limits/aoi.md` rows "Creates in one flush" and
"Entities the client holds at once" with the safe level (largest K clean in
three of three) and break level, graded `measured`.

**Commit:** `docs(client-limits): measured create-burst ceiling (CL-09)`

## CL-10 Live: stock-client residue probe

**Implementer:** game-archaeology-specialist. **Size:** M. **Wave:** 3.
**Blocked on D-CL8 and the user's OK.**

The probe the finding describes (F6): x64dbg on a lab client the user
frees, a **non-freezing** breakpoint (condition `0`, log text, fast resume
off) at `0x0157c9dc` logging `[esp+0x64]` as a 16-bit value on every
`processOrderedPacket` call.

1. Client A launched with `CIMMERIA_CLIENT_HOOKS_DISABLE=all` (or no DLL):
   log in, play to the first-login flush, record the logged values.
2. Client B, default hooks: the same.
3. Compare: if A logs only 0 or small clean values, the residue is the DLL's
   (a sibling detour at the same stack depth), and the fix is to stop
   hooking that depth or patch `0x01578e90`; if A also logs aligned values,
   it is stock and the server mitigation is right.

**Write:** the result in `client-mercury-receive-path.md` § "Where the
residue comes from" (replace "Decisive probe (not run)" with the result),
the `network.md` residue row, and a comment on #1341 with the result
(no addresses beyond the client's).

**Commit:** `docs(re): stock-client iterator residue probe result (CL-10)`

## CL-11 Timing from captures

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. No lab use.

**Branch:** `client-limits/cl11-capture-timing`. **Worktree:** `cl11`.

From `crates/wireclient/tests/fixtures/praxis_start_tap.json` (records carry
`ts_ms` and `dir`), compute with a PowerShell one-off (not committed):

- each `gmGotoXYZ` (`dir = in`, method 163) to the next `CREATE_ENTITY` or
  `onStaticMeshNameUpdate` sent to the client: the teleport-to-AoI latency;
- `onClientReady` to the first introduction: the hold as observed;
- the largest gap between two server-to-client records.

**Write:** rows in `docs/client/limits/timing.md`, graded `measured`
(source: the fixture path and its capture date, n = the count of teleports).

**Test:** none (docs).

**Commit:** `docs(client-limits): teleport and hold timing from the Praxis capture (CL-11)`

## CL-12 Raise candidate: Large Address Aware lab trial

**Implementer:** packet-coder. **Size:** S. **Wave:** 4. **Blocked on
D-CL4**, and only if CL-05 shows a world under 512 MB of free address space.

**Risk:** engine or middleware code that treats pointers above 2 GB as
negative (PhysX, Bink, FMOD, the client-patches DLL). Symptoms: crashes at
load, or corrupt heaps under memory pressure. **Rollback:** clear the bit
(one byte), or restore the lab install's backup exe.

**Change:** a lab-only setup step in `tools/lab/cli/setup.ps1` (or the
script CL-04 names) that sets `0x0020` in the lab install's `SGW.exe` file
header, after backing up the exe, and refuses to touch any install the lab
does not own. Mirror the checks in `crates/launcher/src/client_setup/aslr.rs`
(read, verify, write one byte, re-read).

**Test:** a PowerShell test in `tools/lab/cli/` (run by `lab.yml`) against
a fixture PE header: sets the bit, leaves every other byte equal, is a no-op
the second time, and refuses a file without the `MZ`/`PE` signatures. Fails
if the write lands on the wrong offset.

**Then live (user's OK):** CL-08's spec once with the flag; compare
`total_virtual_mb` and free address space.

**Commit:** `feat(lab): optional Large Address Aware flag on the lab install (CL-12)`

## CL-13 Raise candidate: lab ini overrides

**Implementer:** packet-coder. **Size:** M. **Wave:** 4. **Blocked on
D-CL5** and CL-04 (the keys and their files).

**Risk:** a larger texture pool uses address space the client may not have
(CL-05); frame smoothing changes timing that calibrated specs depend on.
**Rollback:** remove the override file; reseed the instance profile.

**Change** `crates/lab/src/supervisor/instance_profile.rs`: after seeding,
apply overrides from an optional committed file
`tools/lab/ini-overrides.toml` (`[[override]] file = "SGWEngine.ini",
section = "TextureStreaming", key = "PoolSize", value = "..."`), rewriting
only those keys, keeping CRLF, and logging each applied override. The
overrides apply at every launch, not only at the first seed (the seed never
reruns, per #1312).

**Tests** (unit, type 1, in `instance_profile.rs`'s tests): an override
replaces an existing key in its section only; adds a missing key; leaves a
same-named key in another section alone; keeps CRLF; applies again after a
reseed. Each fails if the section match is dropped.

**Lane:** fmt, clippy, nextest, each `-p cimmeria-lab`.

**Doc rows:** `docs/guides/live-research-lab.md` (parallel clients section).

**Commit:** `feat(lab): ini overrides for lab instance profiles (CL-13)`

## CL-14 Raise candidate: server send window

**Implementer:** rust-gameserver-dev; `bigworld-engine-advisor` and
`aoi-witness-broadcast` review. **Size:** M. **Wave:** 4. Depends on CL-09.

**Why:** the server's `TX_WINDOW_SIZE` is 32 with no client reason; the
client's window is 512 (F3). A wider window lets a dense flush drain in fewer
round trips.

**Risk:** a burst that outruns the client's socket buffer (CL-02's answer)
turns into loss and retransmits. **Rollback:** the constant.

**Change:** make the window a `ServerConfig` value (default 32, unchanged),
read once into the channel; the lab sets 64 or 128. Then CL-09's burst at
its break level with each window, live (user's OK).

**Tests:** a type 9 Mercury session test that 128 packets in flight are
accepted and acked with the window at 128 and held at 32 with the default;
a type 10 chaos test that 1% loss at window 128 still delivers every packet.
Fails if the config is not read.

**Lane:** fmt, clippy, nextest, each `-p cimmeria-mercury`, then
`-p cimmeria-base`.

**Doc rows:** "Mercury protocol-layer behavior"
(`docs/architecture/mercury-loopback-harness.md`, TESTING.md type 9);
`docs/protocol/mercury-wire-format.md`.

**Commit:** `feat(mercury): configurable server send window (CL-14)`

## CL-15 Raise candidate: lab low-resolution window

**Implementer:** packet-coder. **Size:** S. **Wave:** 4. Depends on CL-08
(the cost per client to compare against).

**Risk:** calibrated specs use screen points and camera counts measured at
1280x720 (`first-session.toml` header); a different size moves them.
**Rollback:** unset the variable.

**Change** `crates/lab/src/supervisor/process.rs`: a per-instance window
size from `CIMMERIA_LAB_WINDOW_<LABEL>` (for example `640x360`), falling back
to `DEFAULT_WINDOW`; `-windowed ResX= ResY=` as today. A spec row records
the window it was calibrated at; the runner refuses a calibrated row on a
different window unless the spec says it does not depend on it.

**Tests** (unit, type 1): the variable parses, bad values fall back with a
warning, and the launch arguments carry the size. The refusal is tested in
`crates/lab/src/uat/runner/tests.rs`. Each fails if the fallback is removed.

**Lane:** fmt, clippy, nextest, each `-p cimmeria-lab`.

**Doc rows:** `docs/guides/live-research-lab.md`; `docs/guides/automated-uat.md`.

**Commit:** `feat(lab): per-instance window size for low-resolution clients (CL-15)`

## CL-16 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 5.

- Every row in `docs/client/limits/` has a value or `not measured` with a
  reason; the six chaos-driven rows have safe and break levels.
- Fold worknotes into the area files; mark each packet Done or the reason
  it was dropped in the ledger.
- `docs/guides/unified-uat.md`: any owner checks still pending (CL-12's
  live comparison, if run).
- `docs/project-status.md` and `docs/gap-analysis/` (the client-tooling
  area): one line each pointing at the bounds table.
- Tell lab-chaos's coordinator the levels are final; post a handoff on the
  board.

**Commit:** `docs(client-limits): close-out (CL-16)`
