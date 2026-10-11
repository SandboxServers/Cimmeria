# Lab chaos mode

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main` @ `6996c9403`. Prefix `CH-`. Effort 5,
> Phase B of the [lab roadmap](../lab-roadmap/handoff.md). Depends on
> [client-limits](../client-limits/README.md) (Phase A, the levels) and
> lab-golden (effort 3, the run fingerprint and its diff). Related:
> [#1341](https://github.com/SandboxServers/Cimmeria/issues/1341). Packet
> specs: [work-packets.md](work-packets.md).
>
> **Campaign status (2026-10-11): CH-01 to CH-03 merged to `main`; CH-04 to
> CH-07 reviewed and integrated into the local feature branch
> `lab-chaos/feature` (worktree `chaos`).** A PR hold is in force: packets
> merge into the feature branch locally and the campaign opens one PR when the
> hold lifts (see Ship below). CH-08 is next (Ready). CH-09 waits for lab-golden's capture, diff and `-DiffGolden` (GD-03 to GD-07, GD-09); CH-10 and CH-11 are live and
> need the user's OK.

## Purpose

`lab uat --chaos <profile>` runs ordinary UAT rows under a controlled,
seeded stress, and reports any change against the golden fingerprint as one
line. Four profiles:

| Profile | What it stresses | Where it runs |
|---|---|---|
| `cpu` | Keeps N cores busy on the client machine | Lab daemon |
| `net` | Latency, jitter, loss, duplicates and reorder on one session's packets | Server, through a seeded transport shim (D-CH1) |
| `flush` | Delays, reorders or reshapes the first-login AoI flush | Server, lab-gated (D-CH2) |
| `burst` | Spawns K extra entities in the player's AoI | Server, through existing GM commands |

Each profile has a documented **safe** level (rows must still match the
golden) and **break** level (a known failure), taken from
`docs/client/limits/`.

**Acceptance:** the `cpu` and `flush` profiles reproduce #1341 on demand
(CH-11, live, needs the user's OK). That is the proof chaos mode finds real
bugs.

Out of scope: chaos on player sessions or the colo's public shard, a Windows
packet shaper (unless D-CH1 chooses it), and fixing #1341.

## What was found

Against `main` @ `6996c9403`.

| # | Finding | Packets |
|---|---|---|
| F1 | `cimmeria-mercury` already has a seeded `LossyTransport` (`crates/mercury/src/lossy_transport.rs`): send-side drop, latency, jitter, duplicate and reorder; receive-side drop and latency; profiles `Lan`, `Domestic`, `Transatlantic`, `Mobile`. Its config is fixed at construction and applies to every peer. | CH-06 |
| F2 | `cimmeria-base` has a `chaos-testing` feature whose only seam is `BaseService::set_transport_override`, for tests. Production builds never compile it ([network-chaos-testing.md](../../architecture/network-chaos-testing.md)). There is no runtime switch and no per-session scope. | CH-04, CH-06 |
| F3 | The lab-mcp endpoint is off unless `CIMMERIA_LAB_MCP_BIND` and a token of at least 32 bytes are both set (`crates/lab-mcp/src/config.rs`), and its tool set is fixed. A chaos control is a new tool on it, and the first one that changes server behaviour other than the console. | D-CH2, CH-05 |
| F4 | The first-login flush drains through `dispatch_deferred` in `crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs`: a phase-1 bundle of 37-byte `CREATE_ENTITY` + `UPDATE_AVATAR` records, then the phase-2 cascade. That is the one choke point a `flush` profile needs. | CH-07 |
| F5 | #1341's mechanism: a sticky per-process residue `R` (a multiple of 8) in the iterator's next-request offset; a message whose cursor equals `R` in a bundle's first packet loses the bundle's tail. Phase-1 cursors are `1 + 37j` and `12 + 37j`, so a given `R` collides only for some NPC counts, which is why it shows "about one login in four" ([receive-path finding](../../reverse-engineering/findings/client-mercury-receive-path.md#the-iterators-next-request-offset-is-never-initialized-confirmed-live-2026-09-29)). | CH-07, CH-11 |
| F6 | **A shift reproduces #1341 for any residue.** Prepending one harmless message of length `L` moves every phase-1 cursor by `L`. For a fixed `R` within the packet, exactly one `L` in 0..36 puts a cursor on `R`. Sweeping `L` over a run's logins (seeded) therefore hits the bug within 37 logins whatever `R` is; a known `R` (from the detector's `next_request_offset`) hits it at once. Which message is harmless at the head of a phase-1 bundle is for `aoi-witness-broadcast` and `bigworld-engine-advisor` to settle (CH-07). | D-CH6, CH-07 |
| F7 | If #1341's server mitigation lands first (one message per first-packet, per the finding's rule 1), a `flush` profile on today's shape cannot reproduce it. The `pack` mode rebuilds the pre-mitigation single-packet bundle on purpose, so the acceptance run proves both the bug (mitigation off) and the fix (mitigation on). | CH-07, CH-11 |
| F8 | That `cpu` load reproduces #1341 is the owner's hypothesis from "it shows under two-client load". The residue's source is not pinned: one hypothesis is our own DLL's detours at the same stack depth (CL-10 decides). CPU load could change which work item ran last at that depth. CH-11 measures the rate with and without `cpu`; if it does not rise, that is recorded, the owner is told, and acceptance rests on `flush`. | CH-11 |
| F9 | `burst` needs no new server code: `.spawnrandom <templateId> <xRange> <zRange> [count]` exists (`crates/cell-console/src/cell/console/spawn/mod.rs`), GM-gated, and reports partial delivery. Cleanup needs a despawn of exactly what the run spawned. | CH-08 |
| F10 | The lab daemon already owns process lifecycle and run cleanup (`crates/lab/src/supervisor/`), and the UAT runner already issues server console commands (`crates/lab/src/uat/runner/lab_commands.rs`). `cpu` and `burst` attach there. | CH-03, CH-08 |
| F11 | `lab uat -Json` is the output budget model: no nulls, capped groups, a few hundred characters (`tools/lab/cli/uat.ps1`, `uat-lib.ps1`, tested by `test-uat.ps1`). | CH-02, CH-09 |

## Profiles and levels

Levels are named so a spec or a command line never carries a magic number;
an explicit value (`net:latency=120ms`) is allowed for exploration. The
values below are **provisional** until client-limits fills its chaos-driven
rows (CL-08, CL-09); CH-10 replaces them with measured ones.

| Profile | Knobs | Safe (provisional) | Break (provisional) | Bound it reads |
|---|---|---|---|---|
| `cpu` | `cores`, `duty` | cores = logical cores minus 2, duty 100% | every logical core, duty 100% | `engine.md` busy cores |
| `net` | `latency`, `jitter`, `loss`, `dup`, `reorder` | 60 ms, 10 ms, 0.5% loss | 250 ms, 80 ms, 5% loss | `network.md` latency and loss rows |
| `flush` | `mode` (`delay`, `reorder`, `pack`, `shift`), `delay_ms`, `shift` | `delay` 2000 ms; `reorder` | `pack` 24 records; `shift` sweep | `aoi.md` creates in one flush |
| `burst` | `count`, `template` | 50 | the CL-09 ceiling | `aoi.md` entities held |

Every profile takes `seed=<u64>` (default: the run id's hash), recorded in
the report so a run can be replayed exactly.

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-CH1 | **DECIDED (a)**, owner, 2026-10-11 | **How `net` shapes traffic.** (a) A server-side shim: the existing `LossyTransport`, made switchable at runtime and scoped to one session's address, in a lab build only (D-CH2). (b) A Windows packet shaper on the client machine (a WinDivert-based tool). | (a) is seeded and replayable, shapes both directions, needs no driver install or admin rights on the lab machine, and works the same against a local server and a lab server. (b) shapes the real socket but is not seedable, needs a kernel driver, and affects every process on the machine. |
| D-CH2 | **DECIDED (a)**, owner, 2026-10-11 (`network-security-auth` signs off the gate in CH-04 review) | **Gating server-side chaos (`net`, `flush`).** (a) Three gates, all required: compiled only with a new `lab-chaos` cargo feature that release and colo images never enable; refused at runtime unless `DEVELOPER_MODE` is true; set only through authenticated lab-mcp tools. Every set logs a WARN with the profile, level, seed, scope and expiry. (b) Feature gate only. | Chaos that reaches a player server is a denial of service. (a) means a production binary cannot contain it, a dev binary cannot enable it without the operator's mode, and nobody without the lab token can turn it on. |
| D-CH3 | **DECIDED (a)**, owner, 2026-10-11 | **The control surface.** (a) Three lab-mcp tools: `server_chaos_set {profile, level or knobs, seed, session_entity_id, ttl_s}`, `server_chaos_clear {}` and `server_chaos_status {}`. Scope is one session (by player entity id); there is no "all sessions" scope. `ttl_s` defaults to 300 and is clamped to 1800, after which the state clears itself. (b) Console commands (`.chaos ...`) instead of tools. | Tools are authenticated by the lab token; console commands are GM-gated only, and GMs exist on shared servers. The TTL means a crashed run never leaves chaos on. |
| D-CH4 | PROPOSED (coordinator) | **`cpu` runs in the lab daemon** as N busy threads owned by the run, under a guard that stops them when the run ends, fails or is cancelled, and on daemon shutdown. Normal priority; no affinity unless `cores` asks for it. | The daemon already owns the run's lifetime (F10), so cleanup is a drop, not a separate process to hunt down. |
| D-CH5 | PROPOSED (coordinator) | **`burst` uses existing GM commands** through the runner: `.spawnrandom` around the player, then a despawn of every entity the run spawned (by the ids the server reports), at row end and on failure. A fixed default template (a humanoid with a large cascade, chosen in CL-09). | No new server code, and spawns are GM-gated already (F9). |
| D-CH6 | PROPOSED (coordinator; `aoi-witness-broadcast` confirms the filler) | **`flush` modes:** `delay` (hold the flush `delay_ms` longer); `reorder` (seeded shuffle of the introductions inside each phase); `pack` (force the pre-mitigation single-packet phase-1 bundle of up to 24 records, F7); `shift` (prepend one filler message of a seeded length, F6). With the gate off, the flush is byte-identical to today. | `pack` and `shift` make #1341 deterministic; `delay` and `reorder` test the client's tolerance without a known bug. |
| D-CH7 | PROPOSED (coordinator) | **Reporting against the golden.** A chaos run compares to the lab-golden fingerprint with lab-golden's variance model and prints the first divergence in one line, prefixed by the chaos header (`chaos=flush:pack seed=7`). A chaos run never records or re-blesses a golden. Chaos-only failures are labelled `chaos_divergence`, not row failures, in `-Json`. | Keeps the output budget (F11) and keeps goldens clean. |
| D-CH8 | PROPOSED (coordinator) | **Levels live in one committed file,** `crates/lab/src/chaos/levels.toml` (embedded with `include_str!`), each level citing the `docs/client/limits/` row it came from. CH-10 updates the numbers; the names stay. | One source for the CLI, the runner and the docs. |
| D-CH9 | **DECIDED**, owner, 2026-10-11 | **Burst cleanup: change the GM console.** `.spawnrandom` replies only after the spawned ids are known and lists them; `.despawn` takes an optional entity id. Chosen over a lab-only despawn tool and over dropping burst. Supersedes D-CH5's "no new server code". | CH-08 found `.spawnrandom` never reports ids and `.despawn` acts only on the selected target, so the runner could not despawn exactly what it spawned without moving the player's target. |
| D-CH10 | **DECIDED**, owner, 2026-10-11 | **Arming chaos for the next login: by account.** `server_chaos_set` also takes an `account` in place of an entity id: one-shot chaos held for that account's next world entry, then attached to the new session and tagged with its new player entity (same TTL cap, same three gates). Chosen over arming at character select and over relogging under chaos. Extends D-CH3's scope. | Chaos could only be aimed at an in-world entity, after that login's flush, so `flush` chaos never reached the first-login flush that #1341's acceptance (CH-11) needs. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| CH-01 | Chaos profile types, parser and levels file | packet-coder | S | 1 | none | Integrated |
| CH-02 | `lab uat --chaos` in the PowerShell CLI | packet-coder | S | 2 | CH-01 | Integrated |
| CH-03 | `cpu` stressor in the lab daemon | packet-coder | S | 1 | none | Integrated |
| CH-04 | Server chaos state and the three gates | rust-gameserver-dev, `network-security-auth` review | M | 2 | D-CH2, D-CH3 | Integrated (feature branch) |
| CH-05 | lab-mcp tools `server_chaos_set` / `clear` / `status` | packet-coder (finished by rust-gameserver-dev) | S | 3 | CH-04 | Integrated (feature branch) |
| CH-06 | `net`: switchable, per-session lossy transport | rust-gameserver-dev | M | 3 | CH-04, D-CH1 | Integrated (feature branch) |
| CH-07 | `flush`: delay, reorder, pack, shift | rust-gameserver-dev, `aoi-witness-broadcast` design and review | L | 3 | CH-04, D-CH6 | Integrated (feature branch) |
| CH-08 | `burst` and runner wiring for every profile | packet-coder | M | 4 | CH-01, CH-03, CH-05 | Integrated (burst deferred to CH-13) |
| CH-09 | Chaos report against the golden | packet-coder | S | 5 | CH-08, GD-03 to GD-07, GD-09 | Integrated |
| CH-10 | Live: calibrate safe and break levels | coordinator + Haiku lab-driver | M | 6 | CH-09, client-limits CL-08 and CL-09, user's OK | BlockedDependency |
| CH-11 | Live acceptance: reproduce #1341 with `cpu` and `flush` | coordinator + Haiku lab-driver | M | 6 | CH-10, user's OK | BlockedDependency |
| CH-13 | GM console: `.spawnrandom` reports spawned ids, `.despawn <id>` (D-CH9) | rust-gameserver-dev, `server-authority-enforcer` review | M | 4 | D-CH9 | Integrated |
| CH-14 | Server: arm chaos by account for the next world entry (D-CH10) | rust-gameserver-dev, `network-security-auth` and `server-authority-enforcer` review | M | 4 | D-CH10 | Integrated |
| CH-15 | Runner arms chaos by account before login rows (D-CH10) | rust-gameserver-dev | S | 5 | CH-08, CH-14 | Ready |
| CH-12 | Close-out | documentation-writer | S | 7 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in one wave touch disjoint files):

1. CH-01 (`crates/lab/src/chaos/` types), CH-03 (`crates/lab/src/chaos/cpu.rs`).
2. CH-02 (`tools/lab/cli/`), CH-04 (`crates/base-session/src/base/lab_chaos/`, feature plumbing).
3. CH-05 (`crates/lab-mcp/src/tools/chaos.rs`), CH-06 (`crates/base/src/base/` transport), CH-07 (`deferred_flush.rs` and a sibling).
4. CH-08 (`crates/lab/src/uat/runner/chaos.rs`, `crates/lab/src/chaos/burst.rs`).
5. CH-09.
6. CH-10, then CH-11: live, each after the user's OK.
7. CH-12.

## Dispatch rules

- **Workers.** One packet each, in its own worktree and test database:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 lab-chaos/<packet>-<slug> <worktree>`,
  then, in the new worktree, `git reset --hard lab-chaos/feature` so the
  packet builds on every integrated packet.
  `packet-coder` (Haiku) for packets marked so; `rust-gameserver-dev` for
  CH-04, CH-06 and CH-07; `documentation-writer` for CH-12. The brief
  carries the worktree path, the packet section, the contract section and
  the commit subject with the attribution lines.
- **Review.** Each packet gets a Sonnet `packet-reviewer`. CH-04, CH-05 and
  CH-06 also get `network-security-auth` (can a production build or a
  non-lab caller reach chaos?) and `server-authority-enforcer`. CH-07 also
  gets `aoi-witness-broadcast` and `bigworld-engine-advisor`. Review fixes go
  to a fresh worker or the coordinator.
- **The lab.** No packet before CH-10 touches the lab. CH-10 and CH-11
  follow the `lab-uat` skill: ask the user, take the lease, never lend a
  lease to Haiku, and verify every claim against the server log and the
  packet tap.
- **Shell.** PowerShell only: no bash, no direct `cargo`, no
  `git worktree prune`, no `git stash`. Every compiling command goes through
  `pwsh -NoProfile -File tools/build-lane/lane.ps1`.
- **Ship (PR hold, 2026-10-11).** No per-packet PRs. After review, the
  coordinator merges the packet branch into `lab-chaos/feature` (worktree
  `chaos`) with `git merge --no-ff`, reruns the touched crates' lane checks
  there, and retires the packet worktree. When the hold lifts, one PR:
  `python tools/build-lane/ship.py pr -C .claude/worktrees/chaos -m <msg>`,
  then `ship.py merge <PR> --retire chaos`. Before then, merge `origin/main`
  into the feature branch at wave boundaries to keep it current.
- **Shared files.** Only CH-12 edits `docs/architecture/network-chaos-testing.md`,
  `TESTING.md`, `docs/gap-analysis*`, `docs/project-status.md` and
  `docs/guides/unified-uat.md`; packets put their doc deltas in their
  worknote, except the rows their own section names.

## Review outcomes

Where merged code differs from the packet specs, record it here;
the code is then the reference, not the spec.

- **CH-01.** `cimmeria-lab` is a binary crate, so `mod chaos;` sits in
  `main.rs` under `#[allow(dead_code)]` until CH-08. No `thiserror`
  (`ChaosParseError`'s `Display` is hand-written with the contract's
  messages). `ChaosSpec` gains `overrides` (the knobs the text set, a flush
  mode word counting as `mode`), and `header` appends them, so the text
  after `chaos=` is itself a spec that replays the run:
  `chaos=net:break:latency=120 seed=7`. A flush mode carries only its own
  knobs, defaulted when the level lacks them (`delay` → `delay_ms=2000`,
  `pack` → `records=24`, `shift` → `shift=sweep`), and a knob for another
  mode is refused. Anything named twice, empty segments and empty values
  are `BadValue`. Review fixes by the coordinator.
- **CH-03.** `CpuStress::start` clamps `cores` and `duty_percent` itself
  (the fields are public; a duty over 100 underflowed the idle time and
  killed every thread silently). A panicked stressor thread logs a WARN on
  join. `from_knobs` refuses a `+`-prefixed core count. Two tests beyond the
  spec: clamping at start, and a refusal table for bad knob text. Review
  fixes by the coordinator.
- **CH-02.** PowerShell refuses a repeated script parameter, so several
  specs are `-Chaos cpu,flush:pack` (an array), and a multi-knob spec must
  be quoted. The server returns no header yet, so the CLI's header is
  `chaos=<spec>` per value as typed, with no resolved level or seed.
  **CH-08 owes:** return the runner's own `header()` per spec in the
  `lab_uat_run` result, and have `Set-UatChaos`'s caller replace the CLI
  header with it (not append). The plan call carries `chaos` too, so after
  CH-08 a bad spec fails there (exit 3). `Set-UatChaos` and
  `Add-UatChaosHeader` in `uat-lib.ps1` hold the wiring so it is tested.
- **CH-04 to CH-07 (wave 3, 2026-10-11, local feature branch).** Every
  packet had three reviews, and the fixes went to fresh workers.
  - **CH-04:** limits moved into the registry. `set` refuses a bad TTL or
    knob and never clamps. Session teardown clears the entry, and
    `lab_chaos_callers` holds `install` and `current` to an allowlist.
  - **CH-05:** the tools take raw JSON and refuse with `{refused}` and an
    audit row. Unknown fields, entity 0, control characters in a label and
    non-tenth percentages are refused. Clear by entity uses the registry's
    tag (`LabChaos::clear_entity`). A return to character select clears the
    session's chaos (`clear_lab_chaos_on_character_select`).
  - **CH-06:** every `set` gets a generation, so a re-set restarts the
    seeded stream. Held and delayed sends of a cleared or regenerated entry
    are discarded, never released. Each delay line holds at most 4096 sends.
    `LossyTransport` now discards its queued delays on drop.
  - **CH-07:** `Delay` reuses the cinematic AoI hold and releases from a
    spawned timer; the first version slept on the shared receive loop and
    stalled every session. Shift lengths are 0 or 2 to 38, and `Pack` is
    refused above 34 records. voiceData (0x33) is statically verified safe as
    a filler ([RE finding](../../reverse-engineering/findings/client-voicedata-and-update-frequency.md)).
    The default flush is pinned to bytes captured from the pre-change code.
  - **Coordinator:** the knob bounds moved to `lab_chaos/bounds.rs`. The
    CI `lab-chaos` step runs base-session, base, base-world-entry and
    lab-mcp.
  - **Owed:**
    - CH-10 and CH-11 must not set `Delay` near 60 s on a shared lab server
      and expect other sessions to be unaffected. Since the CH-07 fix it holds
      only the target's AoI.
    - CH-11 sends `Shift { len: 3 }` once live, as the first zero-length
      voiceData.
    - A follow-up test should pin teardown or relaunch while a chaos `Delay`
      timer is pending (the stale-token guard in `cinematic_aoi_hold`).
    - `client_ready/mod.rs` is over the 700-line cap: 736 lines before
      CH-07, 738 after. A split is owed outside this campaign.
- **CH-08 (2026-10-11).** Runner wiring for `cpu`, `net` and `flush`.
  `burst` is refused until CH-13 (D-CH9).
  - **Owner decision:** a clear that fails twice stops the run. The later
    rows are BLOCKED because chaos may still be live.
  - **Login rows:** rows that start `client_stopped` or `char_select` are
    BLOCKED under net and flush until CH-15 (arming by account, D-CH10).
    An `any` row is checked at run time.
  - **Plan time:** the plan refuses every server knob bound, and probes
    `server_chaos_status`, so a build without lab-chaos fails the plan.
  - **Cancel safety:** the set is cancel-safe. The guard is armed before the
    call.
  - **CPU and leases:** `cpu` with `-Leases` over 1 is refused.
  - **Incident:** a revert proof's child `uat.ps1` reached the live lab daemon
    and started two first-session runs. Both were stopped. CLI tests now stub
    every daemon call.
- **Coordinator note, CH-09 (2026-10-11).** lab-golden's stable `-Json` API
  gains `chaos_divergence` (the `golden` shape, only under `-Chaos`, a
  chaos-only divergence exits 0), and `lab_golden_record` refuses a chaos run
  with `cannot_record`; recorded in the
  [lab-golden contract](../lab-golden/contract.md#stable-api-for-lab-chaos-and-lab-record).
- **CH-14 (2026-10-11).** Chaos can be armed by account for that
  account's next world entry (D-CH10): `server_chaos_set {account, ...}`.
  The arm is attached in `playCharacter` before RESET_ENTITIES, so it
  reaches the first-login flush.
  - **Attach checks:** the arm attaches only while the session still owns
    the address (an owner token under the `connected` lock). The take and
    the insert are one step under one lock order.
  - **Failure after attaching:** the arm goes back to the account, unless
    the lab cleared chaos since, a newer arm exists, or the arm has expired.
  - **Adjacent fix, every build:** a `playCharacter` whose session was
    replaced or went away mid-entry no longer writes the old player's state
    into the new session or registers its reliable send there. It also
    removes the entity it created (cell `DestroyEntity`, which persists
    nothing).
  - **Scope:** an armed net profile does not shape authentication,
    character select or `playCharacter`. Arming reaches offline accounts up
    to the TTL, and only deployment keeps it off real players.
  - **Owed by CH-15:** grade a chaos login only after a `lab.chaos` flush
    row for that entity.