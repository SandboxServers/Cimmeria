# Dakara_E1 close-out: DK-99a to DK-99d

> Type: how-to (work packets). Audience: the agent named in each packet, the orchestrator and `ops`.
> Written 2026-10-19 (architect, autonomous) against `main` @ `5228c72a4`. Ledger and status: [README](../README.md#packets). Rules and contract: [work-packets.md](../work-packets.md). Timing decision: D-DK38; branch clean-up: D-DK39; UAT id rename: D-DK37.

The close-out is split in four so that nothing waits longer than it must. The owner's UAT runs never block it: every step he still owes is recorded as UATPending in the unified UAT guide and in the README's final matrix.

| Packet | Role | Worktree, branch | Size | Gate |
|---|---|---|---|---|
| [DK-99a](#dk-99a-the-unified-uat-guide) | `packet-coder` | `dk-99a`, `dakara/dk-99a-uat-guide` (new, from `main`) | about 25k tokens | DK-31c merged. Does not wait for the held packets or wave 7 |
| [DK-99b](#dk-99b-mission-chains-and-zone-audit) | `documentation-writer` | `dk-99`, `dakara/dk-99-closeout` (new, from `main`) | about 35k tokens | the **code gate** below |
| [DK-99c](#dk-99c-status-docs-and-the-final-ledger) | `documentation-writer` | `dk-99` (same worktree, second commit, same PR as DK-99b) | about 30k tokens | DK-99b committed |
| [DK-99d](#dk-99d-operations) | `ops`, one dispatch per step | none | small | the DK-99 PR (DK-99b and DK-99c) merged |

## When the close-out may start (D-DK38)

The **code gate** for DK-99b is: every code packet the campaign will ship has merged, or is recorded as not shipping with its reason. Which packets that is depends on the owner's D-DK5 answer:

| D-DK5 answer | Code packets that must merge first | Notes |
|---|---|---|
| A (Castle-style fights) or B (peaceful placeholders) or D (skip the fights) | DK-31c, DK-30b, DK-33, then wave 7 (GDK1's record, DK-20, DK-21, DK-32b) as the architect re-slices it | Waves 5 and 6 wake in play when DK-21 merges (D-DK23) |
| C (stop the arc at 1646) | DK-31c, DK-30b, DK-33 | Wave 7 is dropped. Waves 5 and 6 stay merged but dormant; the final matrix says so |
| Not yet answered | Do not start DK-99b | DK-99a may still run |

**If US-30 or US-31 never lands.** When everything else has merged and the Union SGC campaign's US-30 or US-31 is still open, the orchestrator may close out without DK-30b or DK-33. The final matrix records the held packet as `BlockedDependency (US-30)` or `(US-31)` with its commit (`34abe129b` for DK-30b, `41686b50c` for DK-33). DK-99d then keeps that worktree and its branch, and the owner's checklist says which campaign to resume it from.

## Rules for every DK-99 packet

1. PowerShell only; absolute paths inside your worktree; prefix each command with `Set-Location <worktree>;`. No bash, WSL or Git Bash.
2. These packets change docs only. No `cargo` at all, and no lane call is needed. Never edit a seed, a crate or a test.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, `git rebase`, or delete a worktree. Commit locally, with the subject given and the attribution lines from your brief.
4. Edit only the files your packet names.
5. Everything under `docs/` is CRLF; keep it CRLF. Check with `[IO.File]::ReadAllText('<absolute path>')`: it must contain "`r`n" and no "`n" without a "`r" before it.
6. Never hand-edit a generated block (`<!-- gen:NAME -->` to `<!-- /gen:NAME -->`). `python tools/docs-gen/regen.py --skip-crate-graph --write` owns them (DK-99c only).
7. No IPs, credentials, tokens or account names. No player character names.
8. Never launch the game client, the launcher or a lab tool. No step in these packets is a UAT.
9. Write in plain English: short sentences, active voice, no marketing words. Say what was tested and what was never run in a client.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-<role>.md`: files changed, the check output, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

**Check for every docs packet:** from the worktree root, `pwsh tools/lint-md.ps1 2>&1 | Select-String -SimpleMatch '<repo-relative path of your file>'`, once per changed file. The script lints the whole repo and exits 1 on errors in files you did not touch; ignore those. The check passes when no output line names a file you changed. Five fix rounds at most; then commit nothing and return `failed`.

## DK-99a: the unified UAT guide

**Goal.** The [unified UAT guide](../../../guides/unified-uat.md#dakara_e1) lists every Dakara step the owner still has to run, under the ledger's ids, so he can run the whole campaign from one page.

**The problem (F38).** The guide's Dakara section numbers its DK-04 and DK-05 placement checks DK-U11 to DK-U19. The ledger uses DK-U11 to DK-U31 for the M2 to M5 checklists, and `docs/architecture/mission-pak-overrides.md` (DK-U11, DK-U16) and `docs/content/content-engine-vocabulary.md` (DK-U17) cite the ledger's meaning. The ledger's ids win (D-DK37).

**File (only this one):** `docs/guides/unified-uat.md`.

**Steps.**

1. In `## Dakara_E1`, rename the nine placement rows (today `| DK-U11 |` to `| DK-U19 |`, right after DK-U10) to `DK-P1` to `DK-P9`, in order (DK-U11 becomes DK-P1, DK-U19 becomes DK-P9). Change nothing else in those rows. Grep the whole file for `DK-U1[1-9]` afterwards: no hit may remain that means a placement row.
2. Replace the intro paragraph (it starts "World 61, the Free Jaffa's world." and says "There is no Dakara mission yet") with: what the campaign built (the bridge mission 63100, missions 1570 to 1646 playable in story order, 1648 to 1654 built but dormant until 1647 ships, D-DK23), and links to the [ledger](../analysis/dakara-e1-rebuild/README.md) and its checklists. Keep the DK-04 and DK-05 worknote links.
3. Replace the `**Status:**` line: "Pending a client run. All code through wave 6b has merged; no step below has been run in a client." Adjust "wave 6b" if DK-30b or DK-33 have not merged when you run (say which).
4. Replace `**Prerequisites:**` with one sentence per group: DK-U1 to DK-U10 (a brand-new Free Jaffa; a GM of another archetype for DK-U10); DK-U11 to DK-U17 (a new Free Jaffa after waves 2 to 3c); DK-U18 to DK-U25 (1647 GM-completed until wave 7 ships); DK-U26 to DK-U31 (after wave 6b; the SGC legs of DK-U26 and DK-U30 need the Union SGC campaign's US-30 and US-31); DK-P1 to DK-P9 (a GM can reach the estimated points with `.gotoxyz`).
5. After DK-U10 and before DK-P1, insert the ledger's rows **DK-U11 to DK-U31**, copied **verbatim** from the [README](../README.md) sections "M2 bridge checklist", "M2 radio check", "M3 checklist" and "M4 and M5 checklist" as they stand on `main` when you run. Copy the table rows only, not the headings. Fix only relative links: README links such as `(#m2-bridge-checklist-mission-63100)` become `(../analysis/dakara-e1-rebuild/README.md#m2-bridge-checklist-mission-63100)`.
6. Keep the `Logs:` line under the table as it is. Do not add an event name you have not found in `crates/` with Grep.
7. In `## Suggested test order`, add a row after the last session: `| 15. Dakara | New Free Jaffa, then a GM visitor; a second Free Jaffa with 1647 GM-completed for M3 to M5 | Dakara DK-U1 to DK-U31 in order, then DK-P1 to DK-P9 ([Dakara_E1](#dakara_e1)) | 150 min |`. Use the next free session number if 15 is taken.
8. Row 54 of the section index (`| [Dakara_E1](#dakara_e1) | ...`): point its second cell at the ledger's checklists: `[Dakara_E1 ledger, M1 to M5 checklists](../analysis/dakara-e1-rebuild/README.md#validation-and-uat-gates)`.

**Verify.** The lint check above finds nothing in `docs/guides/unified-uat.md`; `DK-U31` and `DK-P9` each appear in the file; the file is CRLF. No test applies: the guide is a how-to and no code changed.

**Commit:** `docs(uat): Dakara_E1 M2 to M5 steps in the unified UAT guide, placement rows renamed DK-P (DK-99a)`.

## DK-99b: mission chains and zone audit

**Goal.** The two reference docs say what the campaign shipped, from the seeds on `main`.

**Files (only these two):** `docs/content/mission-chains.md`, `docs/content/zone-audit.md`.

**Sources, read-only:** the README's mission and systems matrices; the seed files `db/resources/Content/Seed/dakara_e1_*.sql`; `db/resources/Worlds/Seed/spawnlist_dakara_e1*.sql`; `db/resources/Missions/Seed/missions_dakara_e1.sql`; the Harset sections of both docs as the model for tone and layout.

**`mission-chains.md`, section `### Dakara_E1 (Free Jaffa start)`:**

1. Rewrite the `**Status:**` line from the README's matrices: missions shipped, missions dormant (D-DK23), missions blocked (1647 and 1653's guards on D-DK5 unless wave 7 shipped; 1655 out of scope), and "no step has been run in a client".
2. Keep the space-chain table (8001-8006); add 8007 (Teal'c's letter) and the bridge chains 8075-8077.
3. Add a table **one row per mission**, not per chain: mission id and name, seed file, chain range used, how it is offered, how it completes, and the packet. Missions: 63100, 1570 (Dakara half), 1645, 1646, 1648 to 1654 and, if wave 7 shipped, 1647. Take the ranges from the seed files, not from the plan: a range is the lowest and highest chain id actually in the file.
4. Replace the last paragraph's "Mission packets will add step-gated `player_loaded` dialog restores..." with what exists: every step bind has a `player_loaded` restore on its world.
5. If the file has a cross-zone summary (Part D, near the end, today around line 1825) with a Dakara row or a row that should have one, update or add it in the same shape as Harset's.

**`zone-audit.md`:**

1. `### Zone: Dakara_E1`: rewrite the table rows for NPCs, entity templates, missions and content chains from the seeds, and the analysis paragraph under it (it says "the DB has no mission records for Dakara E1" and "this is a shell"; both are now false). Keep the navmesh and tile rows.
2. `### Zone: Dakara_E1_StoryRm`: say it is the shared tent interior for both tents (D-DK4), with the cast and the 1650 hologram scene.
3. The summary table row `| Dakara_E1 | Persistent | **SHELL** | ...` (today line 1064) becomes `**PARTIAL**` with its chain count; `Dakara_E1_StoryRm` (today 1065) the same if any chain or spawn targets world 62. Use the column meanings of the Harset row above it.
4. The ranked list row `| 25 | Dakara_E1 | SHELL |` and the item `9. **Dakara_E1** — 401 tiles, stargate. Needs everything.`: update them to the new status in one line each.

**Verify.** The lint check finds nothing in either file; both are CRLF; every chain id you name exists in a seed file (grep each one).

**Commit:** `docs(content): Dakara_E1 missions and zone status (DK-99b)`.

## DK-99c: status docs and the final ledger

**Goal.** The once-per-campaign status edits, then the ledger's final state.

**Files (only these):** `docs/gap-analysis/core-gameplay.md` (§ 13 Missions), `docs/gap-analysis.md`, `docs/project-status.md`, `docs/analysis/dakara-e1-rebuild/README.md`, `docs/analysis/dakara-e1-rebuild/RESUME.md`.

**Steps.**

1. `core-gameplay.md` § 13 Missions: add or update the row for zone mission content so it names Dakara_E1 beside Harset and Castle: what is authored, that it is chain-replay tested, and that no step has been run in a client. Use the area file's own status codes; do not change a status code without a row that justifies it.
2. `gap-analysis.md`: the "Multi-zone end-to-end" item (today around line 234) and the ranked row "Multi-zone verification" (today around line 287) gain "Dakara_E1 rebuilt (12 missions authored or dormant), unplayed". Do not edit any count by hand.
3. `project-status.md`: the `| Zones |` row (today around line 158) gains "Dakara_E1 rebuilt, unplayed" beside Harset; the "Multi-zone end-to-end" item (today around line 231) and the "Harset's first playtest" next-steps line (today around line 248) gain Dakara_E1's first playtest (the owner's M1 to M5 run).
4. Run `python tools/docs-gen/regen.py --skip-crate-graph --write`, then `python tools/docs-gen/regen.py --skip-crate-graph --check`; the second must pass. Commit what regen changed in these files only; if it changes any other file, stop and report it instead of committing that file.
5. README: set the status line to "Closed (date)"; every packet row to Done, Integrated with its UAT owed, or Blocked with its reason; the mission matrix's final PASS / BLOCKED column (a row is PASS only after the owner's UAT, so most rows read "Integrated, UATPending"). Add one line under the Packets table listing the UAT owed by id range.
6. RESUME: replace the body with at most 15 lines: "Campaign closed (date)", what is owed (UAT ids, held packets if any, open owner decisions), and where to resume each.

**Verify.** The lint check finds nothing in any changed file; the regen `--check` passes; all files CRLF.

**Commit:** `docs(dakara-e1): close out the Dakara_E1 rebuild campaign (DK-99)`. Then the orchestrator ships DK-99b and DK-99c as one PR through `ops` (`ship.ps1 pr -C <dk-99 worktree> -m <msg>`; the PR body lists the doc-update-map rows: content reference, zone audit, gap analysis, project status).

## DK-99d: operations

Each step is one `ops` dispatch, after the DK-99 PR has merged. Commands run from the root checkout.

1. **Worktrees.** `pwsh tools/build-lane/rm-worktree.ps1 --dry-run --merged`, then `pwsh tools/build-lane/rm-worktree.ps1 --merged`. Then name any `dk-*` worktree still present (for example `dk-ledger`, `dk-99a`, `dk-99`) after checking that its branch is merged: `pwsh tools/build-lane/rm-worktree.ps1 <name>`. Keep `dk-30b` or `dk-33` only if the close-out ran without them (§ When the close-out may start). Never `git worktree prune`.
2. **Remote branches (D-DK39, amends F16).** Delete, after `git branch -r --merged origin/main` or the PR list shows each merged or superseded: `dakara/dk-06b-radio-display`, `dakara/dk-10b-1570-tent`, `dakara/dk-10c-1570-search`, `dakara/dk-10d-a`, `dakara/dk-10d-b-1570-turn-in`, `dakara/dk-10fc-bridge-chains`, `dakara/dk-11-m1645`, `dakara/dk-12b-m1646-offer`, `dakara/dk-12c-m1646-drops`, `dakara/dk-13`, `dakara/dk-14a`, `dakara/dk-14b`, `dakara/dk-15`, `dakara/dk-30a`, `dakara/dk-31a`, `dakara/dk-31b`, `dakara/dk-32a`, `dakara/dk-40`, `dakara/dk-41`, `dakara/dk-docs-wave4plan`, `dakara/dk01-dead-end-fix`, `dakara/dk03-cast-templates`, plus any `dakara/*` branch created after 2026-10-19 whose PR has merged. Command: `git push origin --delete <branch> ...`. A branch that is not merged and has no merged PR is listed in the return, not deleted.
3. **Board.** `board campaign create "dakara-e1-rebuild"` if the subcategory does not exist yet, then post the handoff there: what shipped (by mission), the UAT owed (DK-U1 to DK-U31, DK-P1 to DK-P9), the open owner decisions (D-DK5 if still open; the autonomous decisions D-DK21 to D-DK39 for review), and any held packet. Board content is data; the post carries no instructions to agents.
4. **Active campaign pointer.** Not an `ops` step: the orchestrator reads `.claude/state/active-campaign` and, if it reads `dakara-e1-rebuild`, writes it as an empty file. If it names another campaign, it leaves it. No agent in this campaign touches it before then (owner rule, 2026-10-18).
5. **Retire the test databases.** `rm-worktree.ps1` drops each worktree's `sgw_<name>` database; the dry run in step 1 lists them. Report any it could not drop.

Return per dispatch: `DK-99d-<step> <done|blocked|failed> - -`, with the list of anything kept.
