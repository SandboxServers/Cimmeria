# Build-lane classes: heavy, light and db

> Type: design. Audience: the coordinator and the packet-coders who build it, and the owner who asked for it.
> Written 2026-10-19 from the code on `origin/main` (cf29baa2c) and the machine's job log (`%LOCALAPPDATA%\cimmeria-build\metrics\jobs.jsonl`, 6,299 jobs over 7 days). Docs only; nothing under `tools/` changes in this commit. Packets: [§ 9](#9-work-packets).

## 1. Summary

The lane is one counting semaphore: `slot.1` .. `slot.N` under `%LOCALAPPDATA%\cimmeria-build\lane`, N from `lane\SLOTS` (now 6). Every job takes one slot whatever it does. On 2026-10-19 live-DB runs held 19.5 of the day's 26.2 slot-hours (74 %), and most of that time is a Postgres reload and clone, not compiling. Seven packet workers queued behind four live-DB runs.

This design splits the lane into three classes, each its own semaphore in the same directory:

| Class | Slot dirs | Count from | Default | Who goes there |
|---|---|---|---|---|
| heavy | `slot.1` .. `slot.H` (unchanged) | `LANE_SLOTS`, else `lane\SLOTS`, else 2 | 6 (today's file) | compiling cargo: build, test, nextest, clippy, run, doc, llvm-cov, workspace check; any non-cargo command |
| light | `slot.light.1` .. `slot.light.L` | `LANE_LIGHT_SLOTS`, else `lane\LIGHT_SLOTS`, else 4 | 4 | `cargo fmt`, `rustfmt`, `cargo metadata/tree/version/...`, `cargo check -p <crate>` |
| db | `slot.db.1` .. `slot.db.D` | `LANE_DB_SLOTS`, else `lane\DB_SLOTS`, else 3 | 3 | phase 2 of `live-db-test.ps1`/`.sh` (reload, clone, run); only by explicit `--class db` |

Callers change nothing: the lane classifies the command itself, and `live-db-test.ps1`/`.sh` split into a heavy compile phase and a db run phase. A count of 0 sends that class back to heavy, which is the kill switch. Light and db jobs run with `CARGO_BUILD_JOBS=4` so a mis-sorted job can't take the machine. A lane started inside a lane job runs in its parent's slot, so a light job that starts a heavy one can't deadlock.

Projection from today's log: moving the reload/clone/run part of live-DB runs (about 85 % of their time) and fmt out of the heavy class cuts heavy load from about 26 to about 10 slot-hours a day, with the same 6 heavy slots.

## 2. The lane today (verified)

- **Slots.** `lane.ps1` l.33-35 and `lane.sh` read `LANE_SLOTS`, else `lane\SLOTS`, else 2. A slot is a directory made atomically: `lane-slots.ps1` stages a dir with `winpid`, `winstart` and `what` (`HH:MM:SS <worktree> :: <command>`) and renames it into place; `lane.sh` `mkdir`s it and writes `pid` and `what` (`try_slot`, l.239-249). Either lane breaks a slot whose holder is dead; `lane.sh` asks `powershell.exe` about a `winpid` holder (`WINPID_PROBE`, l.225). Both lanes share one protocol and one set of slots.
- **Taking a slot.** Non-exclusive: try `slot.1..N` in order, take the first free one, else sleep 5 s and retry, printing the busy slots every 60 s (`lane.ps1` l.196-220, `lane.sh` l.253-271). No queue and no fairness: whoever polls first after a release wins.
- **`--exclusive`** (only as the first argument) tries every slot and, unless it got all of them, releases what it got and retries. It can starve: one workspace build waited 695 s today.
- **After taking a slot** the lane checks `lane\retiring.<worktree>` and exits 75 if `rm-worktree` is retiring it.
- **Jobs.** `CARGO_BUILD_JOBS` defaults to `max(4, cores / SLOTS)` unless the caller set it (`lane.ps1` l.127-129); 32 logical CPUs / 6 = 5.
- **Job log** (`lane.ps1` l.407-419, same fields in `lane.sh`): `v=1, t, start, worktree, branch, commit, cmd, exit, wait_s, run_s, exclusive, slots, slots_total, busy_at_start, jobs, incremental, dev_drive, target, sccache, sccache_hits, sccache_misses, min_free_mb, mem_total_mb, disk_free_gb, pruned_mb, quiet, log`. `busy_at_start` counts every `slot.*` dir other than the job's own. `min_free_mb` is the lowest `FreePhysicalMemory` sampled every 2 s.
- **Live-DB entry.** `live-db-test.ps1` runs `lane.ps1 pwsh -File live-db-test.ps1 --in-lane <args>`: one slot for the whole run. The in-lane half runs `reload-db.ps1` first, then `tools/test-live-db.ps1 <args>`, which clones the template into `<db>_0..7` and runs `cargo nextest run --profile=ci-live-db -p <29 crates> --lib <args>`. `live-db-test.sh` does the same through `lane.sh bash -c`. So a compile error is found only after a 30-70 s reload. `test-live-db.ps1 --build-only` (and `--wireclient --build-only`) compiles the same binaries with `--no-run` and needs no database; CI already builds that way before it runs.
- **Who reads slot dirs.** Everything globs `slot.*`: `Get-SlotDirs` (`lane-slots.ps1` l.148), the busy message, the prune guard, `rm-worktree.ps1` `Test-Building` (l.119-125), `rm-worktree.sh` `building()` (l.129-134), `build-hygiene/sweep.ps1` l.83, and the lab supervisor's `crates/lab/src/supervisor/lane_idle.rs` l.62 (`starts_with("slot.")`). Class dirs named `slot.light.N` and `slot.db.N` are seen by all of them with no change.
- **`lane_summary.py`** drops every line that starts with `[lane] ` from the command output (l.196), so new `[lane]` messages never reach a summary.
- **CI** (`build-lane-scripts.yml`): the Windows job runs `tools/build-lane/test-*.ps1`; Linux runs `bash -n`, `shellcheck -S error tools/build-lane/*.sh` and `python3 -m unittest discover -s tools/build-lane -p "test_*.py"`. The launcher workflows call `bash tools/build-lane/lane.sh cargo ...`; those stay heavy.
- **Postgres** (bundled, `:5433`): PostgreSQL 17.9, `max_connections` 100, 3 reserved for superusers, `w-testing` is not a superuser (97 usable), `shared_buffers` 128 MB. Each live-DB run uses up to 8 clones (`.config/nextest.toml` `live-db = { max-threads = 8 }`) with a test pool of `max_connections(4)` (`crates/test-support/src/live_db_gate.rs` l.76): about 34 connections at worst. The server holds 657 databases (20 GB); 531 are leftover `_N` clones.

## 3. What the job log says

Last 7 days, 6,299 jobs:

| Kind | Jobs | Run p50 / p90 | Slot-hours | Share | Waits > 60 s |
|---|---|---|---|---|---|
| live-DB | 1,832 | 69 s / 164 s | 44.2 | 58 % | 85 |
| clippy `--all-targets` | 1,232 | 12 s / 79 s | 10.5 | 14 % | 46 |
| nextest `-p` | 922 | 14 s / 79 s | 7.2 | 9 % | 66 |
| cargo test | 556 | 22 s / 82 s | 4.7 | 6 % | |
| cargo build | 125 | 33 s / 360 s | 3.3 | 4 % | |
| fmt | 1,341 | 1.2 s / 13 s | 2.0 | 3 % | 63 |
| nextest `--workspace` | 12 | 159 s / 294 s | 1.9 | 2 % | |
| cargo check | 59 | 39 s / 103 s | 0.8 | 1 % | 5 |

- 63 fmt jobs (a 1-second job) waited more than a minute for a slot.
- 705 of 1,832 live-DB runs exited non-zero; many of those are compile errors found after a reload.
- **RAM is not what binds.** Lowest free RAM per job: p1 6.7 GB, p5 9.7 GB, p50 28 GB, and the p50 stays at 26-29 GB whatever `busy_at_start` was. CPU is the shared resource the heavy class protects.
- **Live-DB phases** (412 job logs parsed): reload p50 40 s (p10 29, p90 69); cargo's `Finished` line p50 1.1 s; the test run itself p50 0.4 s; the rest (clone 8 databases, nextest listing 29 binaries) p50 21 s, p90 112 s. Only 139 of 412 compiled anything (p50 6 units); a few were cold (one compiled 308 crates in a 279 s run).
- **Postgres saturates at 3-4 concurrent runs.** Reload time against the number of other live-DB runs going at the time: 0 others p50 32 s; 1: 39 s; 2: 50 s; 3: 61 s; 4: 58 s. Three runs finish three reloads in 50 s, four finish four in 61 s: the fourth adds almost nothing.
- The retained Postgres logs show no `too many clients` even with five concurrent runs.

## 4. Design

### 4.1 Classes and counts

Three classes, each a separate semaphore with the existing directory protocol (§ 2). Counts:

- `H` = `LANE_SLOTS` env, else `lane\SLOTS`, else 2. Unchanged; the file stays 6.
- `L` = `LANE_LIGHT_SLOTS` env, else `lane\LIGHT_SLOTS`, else 4.
- `D` = `LANE_DB_SLOTS` env, else `lane\DB_SLOTS`, else 3.

A value that does not parse as a non-negative integer counts as unset. A light or db count of 0 makes that class take a heavy slot (the job is logged `class: heavy`, `class_req: light`). No borrowing between classes in v1: a light job waits for a light slot even when heavy slots are free. Borrowing would let a burst of fmt jobs fill the heavy slots and is the first thing to try if light waits show up in the log.

### 4.2 Choosing the class

`lane.ps1|lane.sh [--exclusive] [--class heavy|light|db] <command> [args...]`. Flags come before the command, in either order; `--class=X` also works. An unknown class exits 2 with a usage line; `--exclusive` with `--class light` or `db` exits 2.

Without `--class`, the lane classifies from argv only, so no caller changes:

1. Take `argv[0]`'s base name, lower-cased, without `.exe`.
2. `rustfmt` → light.
3. `cargo`: skip a `+toolchain` word, then skip global options (`--color`, `--config`, `-Z`, `-C` with their value; any other word starting with `-`). The first remaining word is the subcommand; `c`, `b`, `t`, `r` are `check`, `build`, `test`, `run`.
   - light: `fmt`, `metadata`, `tree`, `version`, `locate-project`, `pkgid`, `verify-project`, `search`, `help`, `update`, `generate-lockfile`, `fetch`, `hakari`, `machete`, and no subcommand (`cargo --version`).
   - `check` is light only when it names a package (`-p X`, `--package X`, `--package=X`) and has no `--workspace` or `--all`; otherwise heavy.
   - everything else (`build`, `test`, `nextest`, `clippy`, `run`, `bench`, `doc`, `llvm-cov`, unknown) → heavy.
4. Any other command → heavy (wrapper scripts such as `pwsh -File test-live-db.ps1` are opaque).

`clippy -p` stays heavy even without `--all-targets`: it is 14 % of slot-hours and compiles test targets in most uses. db is never automatic; only `live-db-test` asks for it.

Both lanes test the same table: `tools/build-lane/fixtures/lane-classes.tsv`, one `<class>\t<command line>` per row (words split on spaces), at least 30 rows covering every rule above.

### 4.3 Light and db jobs that compile

A light job never upgrades to a heavy slot when it turns out to compile. An upgrade means holding one slot while waiting for another (hold-and-wait), which can deadlock when two classes are full of upgraders. Instead light and db jobs get `CARGO_BUILD_JOBS` = `LANE_LIGHT_JOBS` (default 4) unless the caller set `CARGO_BUILD_JOBS`. Heavy keeps `max(4, cores / H)`. Worst case on this machine: 6 × 5 + 4 × 4 + 3 × 4 = 58 rustc jobs on 32 threads, which only happens if every light and db job compiles at once; in the log, light jobs compile almost never and db phase 2 compiles nothing when phase 1 passed.

### 4.4 Live-DB in two phases

`live-db-test.ps1 <args>` becomes two lane calls, one after the other (never nested):

1. **Build**, heavy: `lane.ps1 --class heavy pwsh -NoProfile -File <tier> [--wireclient] --build-only`. Only a leading `--wireclient` (and `--llvm-cov`) is passed on; the filter and extra args are not, because `--no-run` builds the same binaries without them (CI's pattern). A non-zero exit ends the script with that code; no reload is spent.
2. **Run**, db: `lane.ps1 --class db pwsh -NoProfile -File live-db-test.ps1 --in-lane <args>`, unchanged from today: reload, clone, `nextest run` (a no-op build), tests.

`live-db-test.sh` does the same with `lane.sh`. Two test hooks, read by both twins: `LIVE_DB_TIER_SCRIPT` (default `tools/test-live-db.ps1`/`.sh`) and `LIVE_DB_RELOAD_SCRIPT` (default `reload-db.ps1`/`.sh`).

Trade-off accepted: phase 1 needs a heavy slot even when the build is warm (cargo's fingerprint check takes 1-3 s). With live-DB out of the heavy class, heavy waits should be short. If phase 2 does compile (extra args that change features), it compiles at 4 jobs in a db slot and the log shows it.

### 4.5 Postgres limits

D = 3 by default. Postgres throughput stops growing at 3-4 concurrent runs (§ 3), so a fourth db slot only adds connections. Three runs at the theoretical worst of about 34 connections each would be 102 against 97 usable, but the logs show nothing near that: tests rarely hold four pool connections at once. If `too many clients` or `remaining connection slots are reserved` ever shows in a lane log, lower `lane\DB_SLOTS` to 2 first. Raising D above 3 needs `max_connections` raised on the shared Postgres, which means a restart under every worker (owner call, § 8).

### 4.6 A lane inside a lane

The lane exports two variables to its command: `CIMMERIA_LANE_SLOT` (the held slot's path; the first slot for an exclusive job) and `CIMMERIA_LANE_TOKEN` (a fresh random token, also written to a `token` file in that slot dir after the slot is taken). A lane call that finds both variables set, the dir present and its `token` equal to the variable:

- takes no slot and prints `[lane] nested in <slot dir name>; not taking a slot` to stderr;
- still checks the retiring mark;
- keeps the parent's `CARGO_BUILD_JOBS` (it is set in its environment);
- never deletes the parent's slot on exit (it holds nothing);
- logs `nested: true`, `slots: 0`, and the class it would have taken.

Anything else (no variables, no dir, a different token) takes a slot as usual. This fixes the light-to-heavy deadlock (a light job whose command starts `lane.ps1 cargo build` while every heavy slot waits on a light one) and the same-class case (with `LANE_SLOTS=1`, a heavy job that calls the lane today waits on itself forever). The token, not just the path, stops a detached grandchild of a finished job from riding a slot that someone else now holds.

### 4.7 `--exclusive`

`--exclusive` takes every heavy slot and nothing else. Light and db jobs keep running beside an exclusive job: fmt and Postgres work don't skew a timing run much, and holding them all would make the starvation in § 2 worse.

### 4.8 Job log

Same `v=1`; new fields are additive, so old readers keep working:

| Field | Meaning |
|---|---|
| `class` | the class whose slot the job took (`heavy`, `light`, `db`) |
| `class_req` | the class asked for or classified, before a 0-count fallback |
| `class_slots_total` | the count of `class` (H, L or D) |
| `busy_class_at_start` | other held slots of the same class when it started |
| `nested` | true when it ran inside its parent's slot |

`slots_total` stays H and `busy_at_start` keeps counting every `slot.*` dir, so old charts mean what they did. `lane_stats.py` gains a `class` column, a `--class` filter, and a `live-db-build` kind for `test-live-db --build-only`.

### 4.9 PowerShell and bash lanes together

Both lanes use the same dir names, holder files and `token` file, so a `lane.sh` job and a `lane.ps1` job never share a slot. The class rules live in one helper per lane (`lane-class.ps1`, `lane-class.sh`) and are held equal by the shared fixture. `lane.sh` must keep LF endings (`*.sh text eol=lf`), pass `bash -n` and `shellcheck -S error`, and keep working for the launcher workflows' `bash tools/build-lane/lane.sh cargo ...` (heavy, unchanged).

Each worktree runs its own checkout's lane. Until every worktree is on the new commit, old lanes take `slot.1..H` for every job and new lanes take heavy slots only for heavy jobs. That mix is safe: heavy concurrency never exceeds H, and the extra light and db jobs are bounded by L + D.

## 5. Failure modes

| # | Failure | What happens | Guard |
|---|---|---|---|
| F1 | A light or db job starts a lane job | it would wait on a full class while holding a slot | § 4.6: the inner call runs in the parent's slot |
| F2 | A job is put in the wrong class | a compile runs in a light slot | capped at 4 jobs (§ 4.3); `class` in the log shows it; fix the fixture row |
| F3 | Phase 2 compiles | a db slot compiles at 4 jobs | expected only when extra args change features; visible in the log |
| F4 | Two jobs in one worktree wait on cargo's target-dir lock | one slot idles while the other builds | exists today; a light and a heavy job in one worktree now overlap more often. Accepted; the waiting one uses no CPU |
| F5 | A class slot's holder dies | its dir stays | the same stale-holder code breaks it (both lanes), keyed on `winpid`/`pid` as today |
| F6 | `--exclusive` starves | waits while others come and go | unchanged and pre-existing; now fewer jobs compete for heavy slots. Fairness is a follow-up (§ 10) |
| F7 | H is lowered while `slot.5`/`slot.6` are held | those jobs finish; nobody takes 5 or 6 again | safe; the dirs go when their holders exit |
| F8 | A detached child inherits the lane variables | after its parent ends, the token no longer matches | it takes a slot normally (§ 4.6) |
| F9 | Postgres runs out of connections | tests fail with `too many clients` | D = 3; lower to 2 if seen; see § 4.5 |
| F10 | The lab supervisor sees a light or db slot as busy | a lab launch waits up to 90 s for a fmt or live-DB job | conservative and accepted; teaching `lane_idle.rs` to skip `slot.light.*` is a follow-up |
| F11 | A `LIGHT_SLOTS` file holds junk | parse fails | counts as unset: the default |

## 6. Rollout while workers are using the lane

1. The packets' tests use a scratch `LANE_ROOT`; nothing touches the real lane before merge.
2. Merge the wave PR. Running workers keep their own lane copy until their worktree is created or rebased on the new `main`, so the change spreads one worktree at a time.
3. Canary: one fresh worktree runs `pwsh tools/build-lane/lane.ps1 cargo fmt --check` and one `live-db-test.ps1` filter. Check that `slot.light.1` appears during the fmt job and `slot.<N>` then `slot.db.1` during the live-DB job and that their `jobs.jsonl` lines carry `class`.
4. Watch `python tools/build-lane/lane_stats.py --days 1 --class heavy` and `--class db` for a day: heavy waits, db reload times, any `too many clients`.
5. Kill switch, no code change: write `0` to `lane\LIGHT_SLOTS` or `lane\DB_SLOTS`; every new-lane job of that class goes back to heavy at its next start.
6. Tune with the files. Keep `SLOTS=6` until a day of class data says otherwise; with live-DB out of heavy, 4 heavy slots at 8 jobs each may build faster (owner call, § 8).

## 7. Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LC1 | Three classes (heavy, light, db), each its own semaphore in the same lane dir; heavy keeps `slot.N` | Decided | the existing protocol, holder breaking and every `slot.*` reader work unchanged |
| D-LC2 | Defaults H = SLOTS file (6), L = 4, D = 3; env, then file, then default; 0 sends the class to heavy | Decided | D = 3 matches Postgres saturation (§ 3); 0 is the kill switch |
| D-LC3 | Automatic class from argv[0] and the cargo subcommand; `--class` overrides; db only by `--class db` | Decided | no caller changes; wrappers stay heavy unless they ask |
| D-LC4 | `cargo check -p` light; workspace check, clippy, build, test, nextest heavy | Decided | owner named check as light; clippy is 14 % of heavy time |
| D-LC5 | No upgrade to heavy when a light job compiles; light and db cap `CARGO_BUILD_JOBS` at 4 | Decided | an upgrade is hold-and-wait and can deadlock |
| D-LC6 | Live-DB in two sequential phases: heavy `--build-only`, then db reload and run | Decided | the reload stops costing a heavy slot, and a compile error stops costing a reload |
| D-LC7 | Nested lane calls run in the parent's slot, matched by path and token | Decided | removes cross-class and same-class deadlock |
| D-LC8 | `--exclusive` takes heavy slots only | Decided | fmt and Postgres work don't skew timings much; holding them all worsens starvation |
| D-LC9 | Job log stays `v=1`; class fields are additive | Decided | `lane_stats.py` and other readers keep working |
| D-LC10 | On by default at merge; `LIGHT_SLOTS`/`DB_SLOTS` = 0 is the kill switch | Decided | worktrees adopt it gradually anyway; the switch needs no deploy |
| D-LC11 | One wave, one integration PR, PowerShell and bash in the same PR | Decided | the twins share slots and must change together |

## 8. Owner calls (non-blocking)

None of these block the packets.

- **Postgres `max_connections`** above 100 (and D above 3): needs a restart of the shared test Postgres under every worker. Not needed at D = 3.
- **Dropping clone databases after each run:** 531 leftover `_N` clones hold about 17 of the server's 20 GB. Today they go only at the next run or at `rm-worktree`.
- **Heavy slots 6 vs 4** once a day of class data is in.

## 9. Work packets

One wave of three packets, all `Host: windows`, about 40k working tokens each. They ship as **one PR** from an integration worktree, because the two lanes share slots and must not drift: `ops` makes `lane-classes/integration` (worktree `lane-classes-int`) from `origin/main`, then merges each packet branch into it with `merge-noff.ps1`. **Merge order: LC-01, then LC-02 and LC-03** (either order). LC-01 starts first; LC-02 and LC-03 branch from LC-01's commit once it is reviewed (`ship`) and run in parallel.

| Packet | Scope | Size | Depends on | Status |
|---|---|---|---|---|
| LC-01 | PowerShell lane: classes, classifier, jobs cap, nesting, log fields, fixture, test | S (~40k) | - | Ready |
| LC-02 | bash lane twin and `live-db-test.sh` two-phase, Python test | S (~40k) | LC-01 | BlockedDependency |
| LC-03 | `live-db-test.ps1` two-phase, `lane_stats.py`, docs | S (~40k) | LC-01 | BlockedDependency |

### Rules every packet carries

1. Windows, PowerShell 7 only; never Bash (`Host: windows`). Absolute paths; start each shell call with `Set-Location <worktree>;`.
2. No direct `cargo`. These packets compile no Rust; if a check ever needs cargo, it goes through `pwsh tools/build-lane/lane.ps1 cargo ...`.
3. Tests must set `LANE_ROOT` to a fresh temp dir and `LANE_METRICS` as the test needs; never touch the real `%LOCALAPPDATA%\cimmeria-build\lane`. Copy `test-lane-slots.ps1`'s setup: `LANE_MIN_FREE_GB=0`, `LANE_VERBOSE=1`, `LANE_PRUNE=0`, `RUSTC_WRAPPER='none'`.
4. Test type: TESTING.md § 7 (Python script tests) for `.py` tests; standalone `tools/build-lane/test-*.ps1` with a `Check` helper and non-zero exit on failure for PowerShell, which CI's Windows job already runs. Every new test must fail when the change it covers is reverted; say how in the report.
5. Files: soft cap 500 lines, hard cap 700. `lane.ps1` (435) and `lane.sh` (456) must stay under 500; the class logic goes in the new helper files.
6. `*.sh` files are LF and must pass `bash -n` and `shellcheck -S error` if shellcheck is installed. `docs/**/*.md` are CRLF: write, then check with `[IO.File]::ReadAllText` that every `\n` is `\r\n`.
7. Edit only the files the packet names. No `git stash`, no push, no `git worktree prune`, no amend.
8. Commit locally with explicit paths (`git add <paths>`), subject as given, ending with the attribution lines from the brief.
9. Report to `<worktree>\.claude\reports\lane-classes\<packet>-coder.md` and return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

### Contract (all packets build against this)

- Flags: `[--exclusive] [--class heavy|light|db | --class=X]` before the command, in any order. Unknown class or `--exclusive` with light/db: stderr `usage: lane.ps1 [--exclusive] [--class heavy|light|db] <command> [args...]`, exit 2.
- Counts: heavy `LANE_SLOTS` > `lane\SLOTS` > 2; light `LANE_LIGHT_SLOTS` > `lane\LIGHT_SLOTS` > 4; db `LANE_DB_SLOTS` > `lane\DB_SLOTS` > 3. Junk counts as unset. Light or db count 0 → the job takes a heavy slot; `class=heavy`, `class_req` keeps the asked class.
- Slot dir names: heavy `slot.<i>`, light `slot.light.<i>`, db `slot.db.<i>`, i from 1. Holder files as today plus `token`.
- Jobs: unless the caller set `CARGO_BUILD_JOBS`, heavy `max(4, floor(cores / H))`, light and db `LANE_LIGHT_JOBS` else 4.
- Nesting: export `CIMMERIA_LANE_SLOT` (full path of the first held slot) and `CIMMERIA_LANE_TOKEN` (32 hex chars). Nested iff both set, the dir exists and its `token` file's trimmed text equals the token. Nested: no slot, stderr `[lane] nested in <dir name>; not taking a slot`, retiring check still runs, nothing released on exit.
- Busy message: `[lane] all <n> <class> slots busy: [what] ... <s>s` listing only that class's dirs.
- Job log: add `class`, `class_req`, `class_slots_total`, `busy_class_at_start`, `nested` (JSON bool) after `slots_total`; `slots` is 0 when nested.
- Fixture: `tools/build-lane/fixtures/lane-classes.tsv`, LF, `#` comments, `<class>\t<command line>`.
- Live-DB hooks: `LIVE_DB_TIER_SCRIPT`, `LIVE_DB_RELOAD_SCRIPT` (paths; the `.ps1` twin runs them with `pwsh -NoProfile -File`, the `.sh` twin with `bash`).

### LC-01: PowerShell lane classes

- **Host:** windows. **Branch:** `lane-classes/lc-01-ps-lane`, worktree `lc-01`. **Commit:** `LC-01: lane classes (heavy, light, db) in lane.ps1`.
- **New `tools/build-lane/lane-class.ps1`** (dot-sourced by `lane.ps1`, like `lane-slots.ps1`), about 120 lines:
  - `Get-LaneClass([string[]]$Cmd) -> 'heavy'|'light'` per § 4.2 (never returns db).
  - `Get-LaneClassCount([string]$LockDir, [string]$Class) -> int` per the contract counts.
  - `Get-LaneSlotPath([string]$LockDir, [string]$Class, [int]$I)`.
  - `Get-LaneClassJobs([string]$Class, [int]$Heavy) -> int`.
  - `Test-LaneNested -> [string]` (the parent slot dir or `$null`).
- **`lane.ps1`:**
  - Parse `--exclusive` and `--class` in a loop at l.37-40; update the usage line.
  - Work out `$ClassReq` (flag or `Get-LaneClass`), then `$Class` (fall back to heavy when the class count is 0) and `$ClassSlots`.
  - `CARGO_BUILD_JOBS` at l.127-129 from `Get-LaneClassJobs`.
  - The acquire loop at l.196-220 uses `Get-LaneSlotPath` for `$Class`; exclusive stays heavy-only. The busy message lists that class's dirs.
  - Before the loop: if `Test-LaneNested`, skip acquiring, print the nested line, and leave `$Held` empty.
  - After acquiring: write `token`, set both env vars.
  - Job log fields per the contract.
  - Keep `lane.ps1` under 500 lines.
- **New `tools/build-lane/fixtures/lane-classes.tsv`:** at least 30 rows: every light subcommand, `cargo +nightly fmt`, `cargo --color never fmt --check`, `cargo.exe fmt`, `rustfmt x.rs`, `cargo check -p a` and `--package=a` (light), `cargo check --workspace -p a`, `cargo check`, `cargo c --all` (heavy), `cargo clippy -p a` (heavy), build/test/nextest/run/doc/llvm-cov (heavy), `pwsh -File x.ps1` (heavy).
- **New `tools/build-lane/test-lane-classes.ps1`** (scratch `LANE_ROOT`, the command is a tiny `pwsh -NoProfile -Command` that sleeps or writes a file). Cases:
  1. Every fixture row matches `Get-LaneClass`.
  2. With `LANE_SLOTS=1` and a background holder in `slot.1`, a `--class light` job finishes within 15 s, and `slot.light.1` existed during its run.
  3. With `slot.light.1` held and `LANE_LIGHT_SLOTS=1`, a heavy job still runs.
  4. With `LANE_LIGHT_SLOTS=0`, a `--class light` job takes `slot.1` and logs `class=heavy`, `class_req=light`.
  5. With `LANE_SLOTS=1`, a lane job whose command calls `lane.ps1` again finishes within 30 s and passes the inner exit code (e.g. 7) out. Reverted, it hangs: use a job timeout, never an unbounded wait.
  6. `--class bogus` and `--exclusive --class light` exit 2.
  7. With `LANE_METRICS=1`, the jobs.jsonl line carries `class`, `class_req`, `class_slots_total`, `busy_class_at_start` and `nested`.
  8. `CARGO_BUILD_JOBS` seen by a light job is 4.
- **Checks, in order:** `pwsh -NoProfile -File tools/build-lane/test-lane-classes.ps1`, then `pwsh -NoProfile -File tools/build-lane/test-lane-slots.ps1` (must still pass). Both exit 0.
- **Docs:** none in this packet (LC-03 owns them).

### LC-02: bash lane twin

- **Host:** windows (runs the bash twin through Python's `find_bash`, as `test_lane_stats.py` does; no Bash tool). **Branch:** `lane-classes/lc-02-sh-lane` from LC-01's commit, worktree `lc-02`. **Commit:** `LC-02: lane classes in lane.sh and two-phase live-db-test.sh`.
- **Read first:** LC-01's `lane-class.ps1`, its `lane.ps1` diff (`git -C <wt> show <LC-01 sha>`), and the fixture.
- **New `tools/build-lane/lane-class.sh`** (sourced by `lane.sh`, LF): `lane_class "$@"` (echoes heavy or light), `lane_class_count <class>`, `lane_slot_path <class> <i>`, `lane_class_jobs <class>`, `lane_nested` (echoes the parent dir or nothing). Same rules as the contract; the token comes from `/dev/urandom` (`od -An -tx1 -N16 | tr -d ' \n'`).
- **`lane.sh`:** the same changes as LC-01 made to `lane.ps1`, at the flag parse, `CARGO_BUILD_JOBS`, `try_slot` callers at l.253-271 (pass the class path), nesting before the loop (`held` stays empty so the `release` trap removes nothing), `token` and env vars after acquiring, and the job-log fields. Stay under 500 lines.
- **`live-db-test.sh`:** two phases per § 4.4: `"$HERE/lane.sh" --class heavy bash "$tier" $lead --build-only || exit $?`, then `exec "$HERE/lane.sh" --class db bash -c '...'` with the reload and tier paths from `LIVE_DB_RELOAD_SCRIPT` / `LIVE_DB_TIER_SCRIPT`. `$lead` is a leading `--wireclient` and/or `--llvm-cov`.
- **New `tools/build-lane/test_lane_classes_sh.py`** (unittest; reuse `find_bash` from `test_lane_stats.py`; skip with a reason when no bash is found). Cases: every fixture row through `lane_class`; a light job runs while `slot.1` is held and `LANE_SLOTS=1`; nesting with `LANE_SLOTS=1` finishes within 30 s and passes exit 7; the job-log line has the class fields; `live-db-test.sh` with stub tier and reload scripts that append their argv to a file: on success two jobs.jsonl lines (`class` heavy, `cmd` containing `--build-only`; then `class` db) and the tier stub saw the filter only in phase 2; with the build stub exiting 3, the script exits 3, one log line, and the reload stub never ran.
- **Checks, in order:** `python -m unittest discover -s tools/build-lane -p "test_lane_classes_sh.py" -v`, then `python -m unittest discover -s tools/build-lane -p "test_*.py"`, then `pwsh -NoProfile -File tools/build-lane/test-lane-slots.ps1` (the `WINPID_PROBE` line is shared). Run `shellcheck -S error tools/build-lane/*.sh` if `shellcheck` is on PATH; else say so in the report.
- **Docs:** none (LC-03).

### LC-03: two-phase live-db-test.ps1, lane_stats and docs

- **Host:** windows. **Branch:** `lane-classes/lc-03-livedb-docs` from LC-01's commit, worktree `lc-03`. **Commit:** `LC-03: two-phase live-db-test.ps1, lane_stats classes, docs`.
- **`tools/build-lane/live-db-test.ps1`:** keep the `--in-lane` half, but take the reload and tier paths from `LIVE_DB_RELOAD_SCRIPT` / `LIVE_DB_TIER_SCRIPT` (defaults as today). The outer half: collect leading `--wireclient`/`--llvm-cov` into `$lead`; run `lane.ps1 --class heavy pwsh -NoProfile -File $tier @lead --build-only`; on non-zero exit, exit with that code; then `lane.ps1 --class db pwsh -NoProfile -File $PSCommandPath --in-lane @args`. Update the header comment.
- **New `tools/build-lane/test-live-db-phases.ps1`:** scratch `LANE_ROOT`, `LANE_METRICS=1`, stub tier and reload `.ps1` files that append their args to a file; the reload stub prints `DATABASE_URL=postgres://stub`. Cases: success gives two jobs.jsonl lines, heavy with `--build-only` then db, and the tier stub saw the filter only on its second call; tier exit 3 under `--build-only` gives exit 3, one line, and no reload call. Reverted to one phase, the first case sees one line.
- **`tools/build-lane/lane_stats.py`:** add `class`, `class_req`, `nested` to `FIELDS` (CSV); a `class` column in `--recent`; `--class` filter in `select()`; `classify()` returns kind `live-db-build` for a command with `test-live-db` and `--build-only`; a by-class slot-hours line in the report. Tests in `test_lane_stats.py` for each.
- **Docs (CRLF):**
  - `docs/architecture/build-system.md` § 3: replace the Slots and Jobs bullets (l.60-61) with the three classes, the classifier and the jobs rule; add the class fields to the Job log bullet; set `last_updated: 2026-10-19`.
  - `docs/agents/development-workflow.md` l.233: the lane has three classes, counts in `SLOTS`, `LIGHT_SLOTS`, `DB_SLOTS`; drop "currently 4". l.258: "builds in a heavy slot, then reloads and runs in a db slot".
  - `.claude/skills/lane-build/SKILL.md` l.43-44, l.91-92, l.119 the same way, then run `python tools/agent-skills/sync.py`.
  - `tools/build-lane/README.md` l.7 and l.11 (`--class`, two phases), and add rows for `lane-class.ps1`/`.sh` and the fixture.
  - `tools/README.md` l.315: `lane.ps1 [--exclusive] [--class heavy|light|db] cargo …` and `lane-class.ps1`. Never edit inside `<!-- gen:tools-catalog -->` (l.11-241).
  - List the doc-update-map rows touched in the report.
- **Checks, in order:** `pwsh -NoProfile -File tools/build-lane/test-live-db-phases.ps1`; `python -m unittest discover -s tools/build-lane -p "test_lane_stats.py" -v`; `python tools/docs-gen/regen.py --check`; if it lists a stale block, run `python tools/docs-gen/regen.py` (never hand-edit a `<!-- gen:... -->` block), commit what it rewrote, and rerun `--check`.

### Wave review and ship

When all three have `ship`, `ops` merges LC-01, LC-02, LC-03 into `lane-classes-int` with `merge-noff.ps1`. One wave reviewer checks the integration tree for drift between the twins (fixture parity, identical job-log field order, the same messages), both lanes taking the same dir names, and doc text matching the code. Then one PR, `ship.ps1 pr -C <lane-classes-int>`, and after merge the canary in § 6. Retire `lc-01..03` with `rm-worktree.ps1 --merged-into <PR> lc-01 lc-02 lc-03`.

## 10. Follow-ups (not in this wave)

- A FIFO ticket queue per class, which also fixes exclusive starvation (F6).
- `lane_idle.rs` in the lab supervisor could skip `slot.light.*` (F10).
- Measure the clone step: PostgreSQL 17 `CREATE DATABASE ... STRATEGY` (default `WAL_LOG`) against `FILE_COPY` for the 8 clones, which are most of the 21 s "rest" of a live-DB run.
- Drop a run's clone databases when it ends (owner call, § 8).
- Clear stale retiring marks: `lane\retiring..claude` and `lane\retiring.cb-livewire-solver` exist today and make those names exit 75.
