# Dakara_E1 wave 1: evidence, rulings and the id block

> Type: how-to (work packets). Audience: the agent named in each packet, and the wave-1 packet-reviewer (read-only; a packet-coder applies its findings).
> Written 2026-10-18 against `main` @ `d1bba0400`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md).

Wave 1 changes no Rust and no seed. Four packets, no dependencies between them: dispatch all four at once, each in its own worktree. Every packet here carries its own rules; you do not need CLAUDE.md.

## Rules for every wave-1 packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout.
2. Any compiling `cargo` call (only DK-12a needs one) goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly. Never add `--target-dir`.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only the files your packet names. Markdown files stay under 700 lines (soft cap 500).
5. Markdown under `docs/` is CRLF. Keep it CRLF: after writing a file, check it with `[IO.File]::ReadAllText(<abs path>).Contains("`r`n")`, and convert with `[IO.File]::WriteAllText(<abs>, ([IO.File]::ReadAllText(<abs>) -replace "(?<!\r)\n", "`r`n"))` if it is not.
6. Lint every changed Markdown file: `pwsh tools/lint-md.ps1 <file>` (warnings are reported, not fatal; fix any you introduced).
7. No seed change, no `db/scripts/` file, no client patch, no new dialog row.
8. Never launch the game client, the launcher or any lab tool. The client's files on disk may be read only where the packet says so.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-<role>.md` (files changed, each check's result, the commit hash, every deviation and why). Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

These packets change no runtime behaviour, so none ships a test; the reviewer checks each claim against its cited source instead.

## DK-06a: radio dialog evidence

| Field | Value |
|---|---|
| Role | `game-archaeology-specialist` (static evidence only: docs, seeds, decompiled code, Ghidra headless if needed; no live client, no x64dbg) |
| Branch / worktree | `dakara/dk-06a-radio-evidence` / `dk-06a` |
| Commit subject | `docs(re): how a radio dialog shows with no NPC present (DK-06a)` |
| Report | `<worktree>\.claude\reports\dakara\DK-06a-game-archaeology-specialist.md` |
| Size | About 40k tokens. If the client binary needs more than one Ghidra session, stop at the server-side answer and record the client question as open. |

**Question.** Dialogs 5815 and 5816 (1570 step 4905, "use your radio to report to SGC") have screens spoken by speaker 2609 (Hammond) and speaker 0. Nobody with speaker 2609 stands on Dakara. Later radio beats have the same shape: 1647 step 4916, 1649 step 4921, 1653 step 4937 (find their dialog ids in `docs/analysis/dakara-e1-rebuild/audit.md`). How can the server show such a dialog, server-authoritative and with no client patch?

**Read first, in this order** (check the docs before any Ghidra):

1. `.claude/agent-memory/` (grep `speaker`, `DialogWin`, `radio`), and `docs/reverse-engineering/findings/dialog-controller-wire-flow.md`, `dialog-portrait-lookup.md`, `cooked-dialog-override-crash.md`.
2. The server's display path: `crates/cell-content/src/cell/content/executor/dialog/mod.rs` and its `tests.rs` (how the speaker entity is chosen: the action's own target, then the player's `last_interaction_target` pin, else abort with a WARN).
3. `docs/protocol/*-dispatch-table.md` for the dialog display method and its arguments.
4. The dialog rows: `db/resources/Dialogs/Seed/dialogs.sql`, `dialog_screens.sql` (speaker per screen) for 5815, 5816 and the later radio dialogs.

**Answer these, each with its evidence and a confidence (HIGH / MEDIUM / LOW):**

1. What the client does when the dialog message names an entity that is not the screen's speaker, or the player's own entity id, or 0. Does it show the window, the text, the portrait?
2. Whether any client-side radio, comm or "remote speaker" path exists (a UI type, a flag in `dialog_flags`, an event-set) that the server can trigger.
3. The options for the server, each classified **free** (server-only) or **client patch**: for example, display with the player as the speaker entity; display with the speaker-0 fallback; pin a hidden mission-scoped entity; anything else the evidence shows.
4. Your recommended option for D-DK10 and the exact server change it needs (file, function, the new parameter or branch), so the architect can slice DK-06b.

**Files you write:**

- `docs/reverse-engineering/findings/dialog-display-without-npc.md` (new): the question, the evidence with addresses or file:line, the options table, the recommendation. Type line `> Type: explanation.`
- `docs/reverse-engineering/findings/README.md`: one index row for the new finding.
- `docs/reverse-engineering/README.md`: one row or line where the findings index lists dialog findings.

**Checks, in order:** `pwsh tools/lint-md.ps1` on the three files; CRLF check (rule 5).

**Docs rows owed** ([doc-update map](../../../agents/doc-update-map.md)): "New RE finding or addition to `docs/reverse-engineering/` tree". List it in the report.

**Done when** the finding answers 1 to 4, and the report names the recommended option. The architect writes the D-DK10 ruling and DK-06b from it; a client-patch-only answer makes D-DK10 an owner decision.

## DK-07: register the Dakara_E1 chain block

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-07-chain-block` / `dk-07` |
| Commit subject | `docs(four-zone): reserve chain block 8001-8499 for Dakara_E1 (DK-07)` |
| Report | `<worktree>\.claude\reports\dakara\DK-07-packet-coder.md` |
| Size | Small (under 15k tokens) |

**Contract excerpt** (D-DK8): Dakara_E1 owns content chains 8001 to 8499. 8001 to 8006 are in use (`dakara_e1_space_chains.sql`). Sub-ranges: space and travel 8001-8039, mission 1570 8040-8074 and 8160-8179, mission 1645 8080-8119, mission 1646 8120-8159, later missions 8180-8499, 8075-8079 reserved for the bridge mission 63100 (D-DK19; D-DK1 F).

**Steps:**

1. Prove the block is free. Run, from the worktree root:
   `rg -n "INSERT INTO content_chains" db/resources/Content/Seed` and check that no `chain_id` (the first value after `VALUES (`) lies in 8007-8499. Then `rg -n "\b8[0-4][0-9]{2}\b" docs/analysis/four-zone-planning/id-allocation.md` to see that no other row claims it. If either finds a clash, stop and return `blocked` with the clashing file and id.
2. Edit `docs/analysis/four-zone-planning/id-allocation.md` (CRLF):
   - In the header, change `**Last updated:** 2026-10-13` to `**Last updated:** 2026-10-18`.
   - In `## Reserved blocks`, insert this row between the `6001–6800` Harset row and the `9000–9799` Tollana row (the file uses an en dash `–` in ranges; copy it):

     ```text
     | 8001–8499 | Dakara_E1 | Dakara_E1 rebuild ([ledger](../dakara-e1-rebuild/README.md)): space and travel 8001–8039 (8001–8006 in use), mission 1570 8040–8074 and 8160–8179, mission 1645 8080–8119, mission 1646 8120–8159, later missions 8180–8499, 8075–8079 reserved for the bridge mission 63100 (D-DK1 option F). Per-file ranges: [Dakara work packets, C3](../dakara-e1-rebuild/work-packets.md#c3-seed-files-and-chain-ranges-d-dk14). |
     ```

   - Change nothing else: the "in use" table already lists `dakara_e1_space_chains.sql | 6 | 8001–8006`.
3. Do not edit the Dakara ledger; the reviewer updates its row.

**Checks, in order:** `pwsh tools/lint-md.ps1 docs/analysis/four-zone-planning/id-allocation.md`; CRLF check (rule 5); `git diff --stat` shows one file, about two changed lines plus one added.

**Docs rows owed:** none beyond this page (it is the allocation table itself). Say so in the report.

**Done when** the row is in place, the date is updated, and the step-1 grep results are in the report.

## DK-08: the any-order objective ruling

| Field | Value |
|---|---|
| Role | `mission-systems-advisor` (writes a ruling note; changes no code and no seed) |
| Branch / worktree | `dakara/dk-08-any-order-ruling` / `dk-08` |
| Commit subject | `docs(dakara-e1): any-order objective pattern ruling (DK-08)` |
| Report | `<worktree>\.claude\reports\dakara\DK-08-mission-systems-advisor.md` |
| Size | About 35k tokens |

**Problem** (F7, D-DK12). In `crates/cell-content/src/cell/missions/progression.rs`, `complete_objective` completes the whole mission once every non-optional objective of the current step is complete; an objective outside the current step is a no-op with a WARN. `advance_step` force-completes the objectives of the step being left and can jump to any step. Three steps have objectives the player may finish in any order:

| Mission / step | Objectives (required unless noted) | What the player does (read the texts in `../audit.md`) |
|---|---|---|
| 1570 / 4902 | 5632, 5631 (optional), 5630 | the radio-signal and search beats; dsm 6730 (dialog 5814, all speaker 0) is the "weak radio signal" line |
| 1570 / 4901 | 5930 (hidden), 5929 | the SG-18 remains: templates 450, 451, 452, dsm 7153, 7154, 7155 (dialogs 6224, 6225, 6226), dog tags item 6778 |
| 1646 / 4908 | 5635, 5939, 5940, 5941, 5942 | five drop locations: templates 449, 458, 459, 460, 461 (D-DK16), dsm 6739-6743 (dialog 5821) |

After 4902 comes 4901, after 4901 comes 4905, after 4908 comes 4909. None of these steps is the mission's last step.

**Read first:** `progression.rs` (the `complete_objective` and `advance_step` bodies); `crates/content-engine/src/conditions/mod.rs` (the `objective_status` key `mission_{m}_obj_{o}_status` and the `counter_{name}` key); `crates/content-engine/src/loader/condition.rs` (`step_status`, `objective_status`, `counter`); `crates/content-engine/src/actions.rs` (`IncrementCounter`, `ResetCounter`, `CompleteObjective`, `AdvanceStep`); how Castle or Harset already solve a multi-objective step (grep `objective_status` and `increment_counter` in `db/resources/Content/Seed/`). Grep `.claude/agent-memory/mission-systems-advisor/` first.

**Rule on, with reasons:**

1. Which pattern each of the three steps uses. Candidates: sibling "last / not last" chains on each objective's trigger with `objective_status` conditions (disjoint by construction for two required objectives; for five it needs one chain per remaining combination, so say whether it stays readable); a counter (`increment_counter` per objective, a `counter` condition on the advancing chain) and whether counters survive a relog, and if not, how a `player_loaded` chain rebuilds them; a region-enter or item-count step; or an engine change (all-required-complete on a non-final step advances the step), with the Castle and Harset chains it would change. Prefer a seed-only answer.
2. How each objective is triggered: which template's `interact_tag` (spawn tags `Dakara_E1_SG18_Louis`, `Dakara_E1_SG18_Nguyn`, `Dakara_E1_SG18_Waters` for 450-452; `Dakara_E1_Drop1` to `Dakara_E1_Drop5` for 449, 458-461), whether 4902 needs a region (point set 2128 is reserved for it) or a bind, and what the optional and hidden objectives do.
3. The exact rows, in the C4 shape of [work-packets.md](../work-packets.md#c4-chain-row-shape), for the three steps: chain ids from 8060-8074 (4902 and 4901; D-DK19) and 8140-8159 (4908), triggers, conditions, actions, plus the `player_loaded` restore chains for any binds. Leave the step with `advance_step`, never with a last `complete_objective`. Tau'ri and Jaffa variants per D-DK15 only where a dialog differs.
4. The tests DK-10c and DK-12c must ship: for each chain, the state that fires it and the nearest state that must not, and the relog case.
5. Whether DK-09 needs a point set (2128) and what its name and radius are.

**File you write:** `docs/analysis/dakara-e1-rebuild/worknotes/DK-08.md` (new, CRLF, `> Type: reference.`): the ruling, the rows, the tests, the open questions. Nothing else.

**Checks, in order:** `pwsh tools/lint-md.ps1 docs/analysis/dakara-e1-rebuild/worknotes/DK-08.md`; CRLF check.

**Docs rows owed:** none (campaign worknote). If you find a progression behaviour that `docs/gameplay/missions.md` or `docs/content/` states wrongly, list it in the report; do not edit those files.

**Done when** each of the three steps has a pattern, exact rows and test cases, and an engine-change answer (if any) is flagged in the report as needing its own packet.

## DK-12a: five drop-location estimates

| Field | Value |
|---|---|
| Role | `movement-teleport-advisor` |
| Branch / worktree | `dakara/dk-12a-drop-locations` / `dk-12a` |
| Commit subject | `docs(dakara-e1): estimate the five mission 1646 drop locations (DK-12a)` |
| Report | `<worktree>\.claude\reports\dakara\DK-12a-movement-teleport-advisor.md` |
| Size | About 35k tokens |

**Contract excerpt** (D-DK13, D-DK3). Mission 1646 step 4908 asks the player to plant coded messages at "Drop Location #1" to "#5" for Moh'katan's scouts. No position survives. Place five points by map estimate, each on the navmesh **in component 279** (the component of the gate plaza, so reachable on foot), spread so that no two are within 25 m, none inside a tent or on a roof, and none within 15 m of an existing row (A-01 to A-16, DK-05-A-01). Each is a `MAP-GEOMETRY` or `INFERRED` estimate with a stated confidence (LOW expected). Rows are PL-DK-A-17 to PL-DK-A-21, in the order Drop Location #1 to #5.

**Read first:** the method in `docs/analysis/harset-rebuild/placements/METHOD.md`; `docs/analysis/dakara-e1-rebuild/placements/README.md` (the `No idea` list), `A-world61-story-placements.md` (row format), `C-landmark-census.md` (camps, walls, plazas), `docs/analysis/dakara-e1-rebuild/worknotes/DK-02.md` (the commands below); the 1646 texts in `../audit.md` for any hint of where the scouts are. Grep `.claude/agent-memory/movement-teleport-advisor/` first.

**Tools.** Build once, from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo build --release -p cimmeria-navmesh-extractor --bins`. The lane prints the target directory; call it `$BIN`. Then:

- `& "$BIN\nav_inspect.exe" data\spaces\dakara_e1.nav --probes <probe file> --h-tol 1.2 --v-tol 4.0 --quiet` (every row must report `ok` in component 279);
- `& "$BIN\nav_inspect.exe" data\spaces\dakara_e1.nav --max-components 800` for the component table;
- `obj_slab` needs an `extract_map` output of the client map; use it only if the owner's client copy is on disk (the DK-02 worknote has the recipe; outputs go to the scratch directory, never the repo). Without it, take floors from `nav_inspect`'s `dy` and say so in the row's "Checks run".

**Files you write:**

1. `docs/analysis/dakara-e1-rebuild/placements/A-world61-story-placements.md`: a new section `## Drop locations (A-17 to A-21)` directly above `## Counts`, a two-sentence evidence paragraph, then a table with the same twelve columns as the other sections. Update `## Counts` to 21 rows on world 61 with the new LOW rows counted.
2. `docs/analysis/dakara-e1-rebuild/placements/data/Dakara_E1_probes.txt`: one line per new row, `PL-DK-A-17 X Y Z`, in the existing format (keep that file's line endings).
3. `docs/analysis/dakara-e1-rebuild/placements/README.md`: remove the drop locations from the `No idea` list and point to the new section.

**Checks, in order:** the lane build (its final `status=` line in the report); `nav_inspect --probes` over the whole probe file, every old row still `ok` as before and every new row `ok` in component 279 (paste the five new lines in the report); `pwsh tools/lint-md.ps1` on the two Markdown files; CRLF check on both.

**Docs rows owed:** none outside the campaign's placement ledger.

**Done when** five rows exist, each passes `nav_inspect` in component 279, and the README's No idea list no longer names the drop locations. DK-09 seeds spawns 8011-8015 at these points.
