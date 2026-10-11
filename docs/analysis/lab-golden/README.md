# Lab golden runs and run diffing

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main` @ `6996c9403`. Prefix `GD-`. Source brief:
> [lab-roadmap handoff § 3](../lab-roadmap/handoff.md#3-golden-runs-and-run-diffing).
> Packet specs: [work-packets.md](work-packets.md) (GD-01 to GD-07) and [work-packets-2.md](work-packets-2.md) (GD-08 to GD-11); shared names: [contract.md](contract.md).
>
> **Campaign status (2026-10-10): planned, nothing built.** One owner
> decision (D-GD6) gates the live acceptance packet GD-10. GD-01 and GD-02
> can start now.

## Purpose

Make a lab run comparable with other lab runs without a model reading it.
Each run gets a normalised **fingerprint**; a fingerprint becomes **golden**
only when five fresh runs agree on it (owner rule: no single run is golden);
and any later run can be diffed against the golden, with the first
divergence reported in one line.

The fingerprint is made of:

- the ordered client-to-server calls and server-to-client messages from the
  server packet tap, by name and key arguments;
- the mission, step and objective transitions;
- the windows opened and closed;
- each row's verdict and its deterministic clause verdicts.

Runtime noise is stripped: entity ids become labels (`self`, a seed tag, a
spawn or template id), and timestamps, item instance ids, session ids and the
run id never reach it.

Consumers outside this campaign: `lab-record` drafts become real specs only
after `lab golden record` agrees five times, and `lab-chaos` reports each
chaos run against the golden through the stable API in the
[contract](contract.md#stable-api-for-lab-chaos-and-lab-record).

Out of scope: a fingerprint of the second player (`p2`) on two-player rows,
inbound key arguments for client calls other than the five in the contract,
client telemetry (`client.mercury.*`) in the fingerprint, and gating `ship.py`
or CI on a golden.

## What was found

Against `main` @ `6996c9403`. "Fixture #n" is record n (0-based) of
`crates/wireclient/tests/fixtures/praxis_start_tap.json`, the packet tap of one
new Praxis character on 2026-10-10.

| # | Finding | Packets |
|---|---|---|
| F1 | The runner starts a packet tap only for a row with a `source = "packet"` clause (`crates/lab/src/uat/runner/mod.rs:412`). No row in `first-session.toml` has one, so no run of that section has ever captured a tap. A fingerprint needs one on every in-world row. | GD-06 |
| F2 | A tap can start only on a connected in-world player (`crates/lab-mcp/src/tools/packet_tap.rs:26`). FS-01, FS-02 and FS-P1 never reach the world, and FS-P2 enters it inside its own steps (`first-session.toml:199`). So a tap bound at row start misses FS-P2's world entry entirely. | GD-06, GD-03 |
| F3 | A tap read drains the ring (`crates/wire-log/src/wire_log/tap.rs:43`), and `lab_timeline`, an open tool, reads the same tap (`crates/lab/src/timeline/packet_tap.rs`). An observer calling it during a golden run takes rows from the fingerprint. A second `tap_start` on the entity replaces the ring (`replaced_existing`). | D-GD8, GD-06, GD-03 |
| F4 | The outbound half of the tap records only entity-method fan-out (`tap.rs:9`), so entity creation is not in it and an entity id cannot be named from the tap alone. `server_entity_query` returns `tag`, `template_id` and `spawn_id` per entity (`crates/wire/src/cell/messages/lab.rs:131`), capped at 256 (`lab.rs:38`). The seed tags Frost and the Guard `ArmYourself_FrostBody` and `ArmYourself_GuardBody` (`db/resources/Worlds/Seed/spawnlist.sql:86` and `:68`). | D-GD3, GD-02, GD-06 |
| F5 | Inbound cell calls have `decoded: null`. Their `args_hex` keeps the 4-byte entity-id prefix and, for `msg_id` `0xBD` (189), the sub-slot byte. `interact`'s one `INT32` is the target entity id: fixture #19 is `0x0001898f` = 100751, Frost. `dialogButtonChoice` (fixture #1) is dialog 2982, button -1. | GD-02 |
| F6 | Outbound records name entities in decoded fields (`onDialogDisplay.entity_id`, `onSequence.source_id` and `target_id`) and in `target_entity_id`. The player is entity 8 in the fixture, a per-run value. | GD-02 |
| F7 | 39 of the fixture's 84 records are noise for a fingerprint: 29 `avatarUpdateExplicit`, 5 `perfStats`, 5 `requestEntityUpdate`. Their count depends on time spent, not on what the server answered. | D-GD5, GD-03 |
| F8 | `args_hex` holds at most the first 256 payload bytes (`HEX_DUMP_CAP`, `crates/wire-log/src/wire_log/mod.rs:65`). Key arguments come from `decoded` or from the first bytes only. | GD-02 |
| F9 | The client has no window open or close event. `client_ui_state` lists the visible children of `Root` (`crates/lab/src/supervisor/flows/ui_state.rs:140`), and it is an open tool (`crates/lab/src/lease/policy.rs:66`). Windows have to be sampled. The tutorial in FS-P4 opens "sometimes seconds later" (`first-session.toml:389`), so samples per action are not stable but the row's net set is. | D-GD5, GD-06 |
| F10 | `RowEvidence` already holds the row result and every clause with its source and verdict (`crates/lab/src/uat/evidence.rs:155`). Timing, SigNoz and human clauses are not deterministic. | GD-03 |
| F11 | Mission, step and objective changes are in the tap as decoded `onMissionUpdate`, `onStepUpdate` and `onObjectiveUpdate` (fixture #23 to #30). Transitions need no database snapshot. | GD-03 |
| F12 | The run id is letters from the clock (`runner/mod.rs:107`), the rows name characters `Px${run_id}` (`first-session.toml:182`), and every row's `vars` carries `run_id` and `character` (`runner/mod.rs:522`). Any text that echoes a name varies per run. | GD-03 |
| F13 | `cimmeria-lab` is a binary crate (`crates/lab/src/main.rs:63` declares `mod uat;`). A `pub` item nothing calls yet is dead code, and clippy runs with `-D warnings`. `policy.rs:76` uses `#[cfg_attr(not(test), allow(dead_code))]` for the same case. | contract, GD-07 |
| F14 | `runner/mod.rs` is 547 lines and `drive_steps` (`mod.rs:434`) is where per-action hooks go. New capture code needs its own file. | GD-06 |
| F15 | The spec loader reads only `*.toml` in the spec folder's root (`crates/lab/src/uat/mod.rs:66`), so a `golden/` subfolder is invisible to it. `lab.yml` path-filters on `docs/guides/uat-specs/**`, so a golden change runs the lab tests. | GD-05 |
| F16 | Each batch line in `batch.jsonl` carries `RunDir` (`tools/lab/cli/uat.ps1:188`). `-Json` gives only the `batch` folder (`uat-lib.ps1:300`), so a caller reads the run dirs from `batch.jsonl`. | GD-08 |
| F17 | The server build is optional: `-ServerVersion` (`uat.ps1:51`), else the manifest says `"source": "unknown"` (`runner/mod.rs:184`). | GD-08 |
| F18 | #1341 (the dropped first-login flush tail) shows up under two-client load, and `uat-lib.ps1:25` already tags FS-P2 failures with it. Parallel lanes make disagreement more likely. | D-GD6 |
| F19 | Every routed tool must be classified for the lease (`policy.rs:24`); anything not in `OPEN` needs the lease. | GD-07 |
| F20 | The wireclient campaign writes its own typed decoders independently of `cimmeria-wire-log` (D-WC4), so an oracle that shares the server's decoder cannot hide a server decoder bug. A run-to-run diff has no such risk: both sides go through the same decoder, and a diff only asks whether two runs differ. | D-GD2 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-GD1 | PROPOSED (coordinator) | **The fingerprint is computed offline from the evidence bundle by pure code.** In fingerprint mode the runner only captures raw data into the bundle: the tap on every in-world row, entity snapshots and window samples. `uat::golden` turns a run directory into a fingerprint. | Pure code is unit-testable from fixture bundles, needs no lab, and lets a fingerprint be recomputed from an old bundle when the normaliser changes. |
| D-GD2 | PROPOSED (coordinator) | **Names come from the tap, not from the wireclient decoders.** Events use the tap's `msg_name` (the `cimmeria-wire-log` names from the dispatch tables) and its `decoded` JSON, through a fixed key-argument allowlist. The lab crate does not depend on `cimmeria-wireclient`. | F20. The tap is the only data a lab run has. The allowlist keeps a golden stable when wire-log gains a decoder or a field. If the wireclient campaign later publishes a canonical name table, a fingerprint schema bump can adopt it. |
| D-GD3 | PROPOSED (coordinator) | **Entity labels.** The lab character is `self`, the `p2` character `p2`, any other player `player`. An NPC is `tag:<tag>`, else `spawn:<spawn_id>`, else `tpl:<template_id>`, from `server_entity_query { space_id }` taken when the tap binds and again before it is read. An id in neither snapshot is `?<n>`, numbered by first appearance in the row. | F4, F6. Tags and spawn ids are seed values, the same in every run. Two snapshots catch NPCs introduced late (the FS-P2 flush) and NPCs that despawn. |
| D-GD4 | PROPOSED (coordinator) | **Key arguments are an allowlist.** A message in the table keeps only its listed fields, entity-valued ones as labels. A message outside the table is compared by name and target only. A spec can add fields per message with `key_args`. Item instance ids are never listed. | F5, F8. Stable under decoder growth, small, and it strips per-run ids by not selecting them. |
| D-GD5 | PROPOSED (coordinator) | **The variance model** lives in the spec under `[section.golden]` and `[row.golden]` (the row extends the section): `ignore` (message globs dropped), `allow_unordered` (globs moved out of the ordered stream into a per-row sorted multiset), `collapse` (consecutive identical events become one), `ignore_windows`, `key_args`, `late_bind_settle_ms`, and row-only `skip`. Built-in defaults always apply: `ignore = ["avatarUpdate*", "perfStats", "requestEntityUpdate"]`. Windows are compared as the row's net opened and closed sets. The golden file stores the resolved variance per row, and a diff applies the golden's variance, not the spec's. | F7, F9. A spec edit to the variance must not silently change what an existing golden accepts: it takes a re-bless (five fresh runs). |
| D-GD6 | **BlockedDecision** (owner). Recommended: **(a)** | **What may become golden.** (a) Five or more fresh runs launched by `lab golden record` itself, run one after another on one instance by default (`-Leases` up to the run count is allowed), **every row PASS in every run**, the same spec SHA, server build and SGW.exe hash, and all fingerprints exactly equal under the variance model. (b) The same, but agreeing failing runs may also be recorded, as a "known-bad" golden. | The owner rule is "golden only when 5 runs agree". (a) adds that the baseline is a good run and that lanes are serial by default, because #1341 shows under two-client load (F18). (b) would make a deterministic failure diffable, but a golden that encodes a bug is easy to mistake for a good one. |
| D-GD7 | PROPOSED (coordinator) | **What a divergence does.** `lab uat -DiffGolden` keeps every row's verdict as it is, adds a `golden` field per run, and exits 5 when every run passed but one diverged. Nothing gates `ship.py` or CI on it. | A divergence is a signal to look, not a failure of the row. Exit 5 lets a script tell it apart from a failed row (1) or a lane brake (4). |
| D-GD8 | PROPOSED (coordinator) | **Tap integrity.** A row's fingerprint records whether its tap dropped messages, was replaced, or failed to stop. `lab golden record` refuses runs with any of these, and with any in-world row whose tap is `unavailable` (no lab-mcp). The record guide says not to call `lab_timeline` on the lab character while recording. | F3. A drained or replaced tap gives a fingerprint that looks deterministic but is missing rows. |
| D-GD9 | PROPOSED (coordinator) | **Tool surface.** Two open (no lease) MCP tools on the daemon, `lab_golden_record` and `lab_golden_diff`, next to `lab_uat_report`. They read run bundles; `lab_golden_record` also writes `docs/guides/uat-specs/golden/<section>.json` under the `specs_dir` it is given. The `lab golden` and `lab uat -DiffGolden` commands call them. | The lab CLI already reaches everything through the daemon's MCP endpoint, so agents and `lab-chaos` get the same tools. Neither touches the client, so neither needs the lease (F19). |
| D-GD10 | PROPOSED (coordinator) | **The golden file** is pretty-printed JSON, at most 128 KiB, with at most 1 500 ordered events per row; events are one-line strings. It names its five evidence runs by run id and folder name (local evidence, not committed). A committed-golden test checks every file parses, fits, and names a section and rows that exist. | Reviewable in a PR diff and cheap to load into an agent's context. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| GD-01 | Spec contract: `GoldenSpec` on the section and the row, validation | packet-coder | S | 1 | none | Ready |
| GD-02 | Fingerprint types, entity labels, key-argument tables | packet-coder | M | 1 | none | Ready |
| GD-03 | Normaliser: variance, transitions, windows, row and run fingerprints | packet-coder | M | 2 | GD-01, GD-02 | BlockedDependency |
| GD-04 | Agreement and first-divergence diff, with the test-only acceptance | packet-coder | M | 3 | GD-03 | BlockedDependency |
| GD-05 | Golden file format, size cap, committed-golden guard | packet-coder | S | 2 | GD-02 | BlockedDependency |
| GD-06 | Runner capture in fingerprint mode: tap every row, late bind, entities, windows | rust-gameserver-dev | M | 2 | GD-01 | BlockedDependency |
| GD-07 | MCP tools `lab_golden_record` and `lab_golden_diff` | packet-coder | M | 4 | GD-04, GD-05, GD-06 | BlockedDependency |
| GD-08 | `lab golden record` command | packet-coder | M | 5 | GD-07 | BlockedDependency |
| GD-09 | `lab uat -DiffGolden` | packet-coder | S | 6 | GD-08 | BlockedDependency |
| GD-10 | Live acceptance: golden FS-01 to FS-P5 from five runs | coordinator (needs the user's OK for the lab) | S | 7 | GD-09, D-GD6 | BlockedDecision |
| GD-11 | Close-out: guides, tools README, unified UAT, status docs | documentation-writer | S | 8 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in one wave touch disjoint files and run in parallel):

1. GD-01 (`uat/spec.rs`, `uat/spec_validate.rs`), GD-02 (`uat/golden/` skeleton, `event.rs`, `labels.rs`, `key_args.rs`).
2. GD-03 (`golden/normalise.rs`, `golden/build.rs`), GD-05 (`golden/file.rs`), GD-06 (`uat/runner/fingerprint.rs`, small hooks in `runner/mod.rs` and `runner/packet.rs`, `server/uat.rs` args).
3. GD-04 (`golden/agree.rs`, `golden/diff.rs`).
4. GD-07 (`server/golden.rs`, `lease/policy.rs`).
5. GD-08 (`tools/lab/cli/golden.ps1`, `golden-lib.ps1`, `test-golden.ps1`, one switch in `uat.ps1`).
6. GD-09 (`uat.ps1`, `uat-lib.ps1`, `test-uat.ps1`).
7. GD-10 (`first-session.toml` variance, `golden/first-session.json`).
8. GD-11.

## Dispatch rules

- **Workers.** One packet each, in its own worktree:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 lab-golden/<packet>-<slug> <worktree>`.
  `packet-coder` (Haiku) for packets marked so; `rust-gameserver-dev` for
  GD-06; `documentation-writer` for GD-11. The brief carries the worktree
  path, the packet's section of [work-packets.md](work-packets.md), the
  contract ([contract.md](contract.md)), and the commit subject with the attribution lines. No
  packet needs a test database.
- **Review.** Each finished packet gets a Sonnet `packet-reviewer` on its
  commit range. GD-04 and GD-06 also get `testing-validation-engineer` (does
  the acceptance test fail when the normaliser is wrong; does fingerprint mode
  leave the grade unchanged). Review fixes go to a fresh worker or the
  coordinator, never back to the original implementer.
- **Shell.** PowerShell only: no bash, WSL or Git Bash, no direct `cargo`, no
  `git worktree prune`, no `git stash`. Every compiling command goes through
  `pwsh -NoProfile -File tools/build-lane/lane.ps1`. The `cimmeria-lab` crate
  builds on Windows only, which is the workstation.
- **The lab.** No packet but GD-10 touches the lab. GD-10 runs only after the
  user says the lab is free and approves the run (five full first-session
  runs, about 25 minutes on one instance).
- **Ship.** `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`, then
  `python tools/build-lane/ship.py merge <PR> --retire <worktree>` once the
  `lab` workflow's jobs pass. Update this table and write
  `worknotes/<packet>.md` when anything is left over.
- **Shared files.** Only GD-11 edits `docs/guides/unified-uat.md`,
  `docs/gap-analysis*` and `docs/project-status.md`. GD-01 owns the golden
  subsection of `docs/guides/automated-uat.md`; GD-07, GD-08 and GD-09 each
  add their own lines to `docs/guides/live-research-lab.md` and
  `tools/README.md`, in separate waves so they never conflict.

## Reconcile with the parallel lab campaigns

- **lab-spec-vocab (SV-).** SV adds step vocabulary to `ActionSpec` and new
  tools; GD-01 adds a `golden` field to `SectionMeta` and `RowSpec`. Both
  structs are `deny_unknown_fields`, so the two PRs touch neighbouring lines in
  `spec.rs`: merge the second one on top of the first, no design conflict.
  SV resolves tag to id; GD labels id to tag from its own snapshot, with no
  shared helper.
- **lab-fixtures (FX-).** If FX establishes a row's fixture as its own phase
  before setup, the fingerprint should not include it (fixture commands are
  not what the row tests). GD-06 taps from the start of setup today; when FX
  lands, its fixture phase must run before `tap_start`, or the FX packet that
  adds the phase moves the call. Record it in that packet.
- **lab-record.** Uses `lab golden record` as its last step. Nothing to build
  here.
- **lab-chaos.** Uses the stable API in the contract (`fingerprint_run`,
  `diff_run`, `Divergence::line`) or the `lab_golden_diff` tool. A chaos run
  is diffed with `lab uat -DiffGolden`.

## Review outcomes

None yet. Where merged code differs from the packet specs, record it here;
the code is then the reference, not the spec.
