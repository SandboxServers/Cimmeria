# Four-zone campaign: Castle and Harset work that can run offline before UAT

> **Scope:** Castle and Harset work that can run now: no lab, no owner answer, and before the owner's end UAT. Checked against the combined branch `campaign/four-praxis-zone-restoration` at `040ef6394` (FZ-01 to FZ-05 merged; FZ-06 to FZ-10 in flight in `fz-06` to `fz-10`).
> **Author:** architect, 2026-10-19. Nothing was built, run or committed. Companions: [README](README.md), [decisions](decisions.md), [packets/](packets/).

## Survey corrections (verified)

1. `content/castle-campaign` (`12af405fa`) and `content/harset-campaign` (`1f935687c`) are both ancestors of the tip (`merge-base --is-ancestor` exits 0). The Harset RESUME line 2b ("do NOT merge ... Not merged") is stale on the combined branch.
2. **CS-07b G-REVIEW round 3 has already run.** `zone-castle\.claude\reports\castle-continuation\CS-07b3-review.md` gives the verdict **ship**, with 0 blockers, 0 majors, 3 minors and 3 nits. No ledger records it yet: Castle RESUME line 18 still says "Next is the round-3 check". What's left is to apply its minors and nits (PU-01, PU-02), not a new round.
3. **The Harset reds in `letter_turn_in_route.rs` are fixed.** FZ-03a (`5c71457bb`) cleared the Rule 6 fields and FZ-04 (`33d925849`) added the OTEL row. `wave2-review-1.md` says every gate is green. FZ-11 re-proves both. No packet.
4. **`reliable_fit.rs` was partly reviewed.** `CS-07b2-review.md` line 52 checked the owner-check enforcement (`RecipientSnapshot::matches`, `ReliableRegistrationOutcome`). Nobody has checked the merge as a whole against both parents: `0f1af91d4` (Castle) and `b28b4fa11` (campaign) into `0a851a410`. The file is 677 lines (soft cap 500, hard cap 700).
5. **D-CA18 is already met.** No seed under `db/resources/Content` names chain 1398 or mission 1664 (Grep: 0 hits). CS-06a/b were never built, so nothing is stranded and there is nothing to revert. HA-01b does not block this PR.
6. **The CD-11 / HA-D04 gate is now D-FZ1.** The Castle 2698 grant (chain 1397) and the cross-world replay (`mission_567_castle_lifecycle*`) exist offline. Enabling shipped as FZ-12 (D-FZ1 = enable); the owner's lab pass (FZ-U1) is still owed. The only offline work left is ledger text: round-3 m-3 goes into the enable recipe, and the HA-D04 row gets refreshed.

## Candidates

| Candidate | Still needed? | Overlap with FZ-06..10 or another PU | Gate | This PR or later | Packet |
|---|---|---|---|---|---|
| CS-07b round 3 code nits: m-1 map floor, n-2 rewrap, m-2 test comment | Yes | **FZ-07**: `seed_id_uniqueness.rs` and `mission_567.rs`. FZ-12 (blocked): `mission_567_castle_lifecycle.rs` | none; after FZ-07 merges | This PR | PU-01 |
| CS-07b round 3 doc minors (m-2, n-1), the m-3 recipe caveat, the round-3 record, D-CA18 met | Yes | **FZ-07**: `docs/content/mission-chains.md` (line 1640 there; PU-02 edits the 1632 heading) | none; after FZ-07 merges | This PR | PU-02 |
| `reliable_fit.rs` merge review | Yes (send-path authority) | none (FZ-10 edits `cell_handoff.rs` and only reads `letter_turn_in_send.rs`) | none | This PR | PU-03 |
| Recheck CS-04c1c2 (`13a994b3e`) and CS-04c2c2 (`bb7eb8296`) | Yes | none (`player/castle_return_range_tests.rs` is not FZ-06's `interaction/mod.rs`) | none | This PR | PU-04a |
| Recheck CS-DOC4 (`a41feb202`, `f3fe7f1b0`) | Yes | read-only; any fix touches `mission-chains.md` and `end-uat.md`, so it joins PU-02 | none | This PR | PU-04b |
| `letter_turn_in_route.rs` reds | **No**: FZ-03a and FZ-04 | none | none | none | none |
| HA-01b graph seal, 1664 (docs only) | Yes, but nothing in the PR needs it | none (new worknote only) | Abandon path without 1399: owner, after the design. Template 260, spawn 360 and chains 6008-6020: coordinator collision check (HA-D09) | **Now (D-PU1 decided 2026-10-19: in this PR).** The build is FZ-18 to FZ-23 (waves 9-11); ids allocated in the FZ packets | PU-05 |
| H05 specialist contract (HA-05, HA-09): minigame competency 1, abilities 0 in `cell_dispatch/minigame.rs` | Yes, before HA-09 | none for the docs. A later implementation touches the minigame files that FZ-03c and FZ-08 touched, so it waits for wave 3 | none for the contract. HA-05 and HA-09 stay BlockedEvidence (Market waves, Replitech anchors; HA-01a needs a client look) | Later | PU-06 |
| HA-15g trainer trees (draft input for HA-15c) | Yes | none (new worknote) | Trees: none (O18-05 allows AI-derived drafts). Roster needs about 21 templates against 9 candidates (261-269): coordinator under O14-04 | Later | PU-07 |
| HA-15b stock lists and price proposal | Partly: a draft only | none (new worknote) | **Owner prices inside O14-12 (D-PU2)**; vendor faction gating is a maintainer call; placement probes are still open | Later | PU-08 (draft) |
| HA-15c trainer seeds via `tools/ability_trees/generate_seed.py` | Yes, after PU-07 | none | PU-07 reviewed, a template allocation, sealed placements (HA-15g §8 items 1-4) | Later PR | not sliced |
| HA-15g placement probes (bank 314 component, `find_path`, clearance) | Yes for HA-15b/c | none | none (offline navmesh, as HA-02 did) | Later | PU-07 can carry it if capacity allows; otherwise not sliced |
| Harset ledger refresh: drop line 2b, HA-D04 row, rows for PU-05..08 | Yes | PU-05..08 (runs after them) | none | This PR (docs) | PU-09 |

## Packets

Worktrees follow the FZ convention. `ops` runs `mk-worktree.ps1 four-zone-review/PU-NN-<slug> pu-NN` and fast-forwards the new branch to the campaign tip recorded at dispatch. On a `ship` verdict, `ops` merges it `--no-ff` into the campaign branch in `four-zone-campaign` and retires `pu-NN`. Each worktree has its own test DB, `sgw_pu_NN`. Reviews follow #1865: a read-only `packet-reviewer`, at most 3 passes, and a `fix` verdict goes to a fresh `packet-coder`.

| Id | Wave | Title | Agent | Worktree / branch | Files | ~Tokens |
|---|---|---|---|---|---|---|
| PU-03 | A (now) | Review the `reliable_fit.rs` merge against both parents | packet-reviewer, plus `network-security-auth` as a parallel read-only pass | none; pinned read with `git show 040ef6394:<path>` in `four-zone-campaign` | none edited | 40k |
| PU-04a | A (now) | Recheck the CS-04c1c2 and CS-04c2c2 test fixes: revert proofs, theatre tests | packet-reviewer | `pu-04a` / `four-zone-review/PU-04a-castle-return-recheck` (lane runs only) | none edited | 40k |
| PU-04b | A (now) | Recheck the CS-DOC4 rows against seeds and test names | packet-reviewer | none; pinned read | none edited | 35k |
| PU-05 | H (now) | HA-01b graph seal: 1398 grant (frozen in `castle-continuation/evidence/1664.md`), Kaerlia (HA-01b2), 5225 door (HA-01b1), abandon-path proposal, ID collision check | mission-systems-advisor, then packet-reviewer | `pu-05` / `four-zone-review/PU-05-ha01b-seal` | new `docs/analysis/harset-continuation/worknotes/HA-01b-graph-seal.md` | 60k (advisor) |
| PU-06 | H (now) | H05 minigame competency and abilities contract | minigame-systems-advisor, then packet-reviewer | `pu-06` / `four-zone-review/PU-06-h05-contract` | new `worknotes/H05-minigame-competency-contract.md` | 40k |
| PU-07 | H (now) | HA-15g trainer-tree drafts from the generator workbook; reconcile the seeded lists 1 and 350 | combat-systems-advisor, then packet-reviewer | `pu-07` / `four-zone-review/PU-07-ha15g-trees` | new `worknotes/HA-15g-trainer-trees.md` | 50k |
| PU-08 | H (now) | HA-15b stock-list draft: one stock per (faction, role, tier), the price ladder left as a proposal | items-systems-advisor, then packet-reviewer | `pu-08` / `four-zone-review/PU-08-ha15b-draft` | new `worknotes/HA-15b-stock-draft.md` | 40k |
| PU-01 | B (after FZ wave 3 and PU-04a) | Round-3 code nits plus any PU-04a fix notes | packet-coder | `pu-01` / `four-zone-review/PU-01-cs07b3-code` | `seed_id_uniqueness.rs`, `mission_567.rs`, `mission_567_castle_lifecycle.rs`; the PU-04a fix files if any | 20-35k |
| PU-02 | B (after FZ wave 3 and PU-04b) | Castle ledger: record round 3, m-2, n-1, the m-3 recipe caveat, D-CA18 met; plus any PU-04b fix notes | packet-coder | `pu-02` / `four-zone-review/PU-02-castle-ledger` | `docs/analysis/castle-continuation/{README,RESUME}.md`, `docs/content/mission-chains.md` heading 1632, `uat/end-uat.md` line 210 anchor | 25-35k |
| PU-09 | B (after PU-05..08) | Harset ledger: drop line 2b, refresh HA-D04, add rows for PU-05..08 | packet-coder | `pu-09` / `four-zone-review/PU-09-harset-ledger` | `docs/analysis/harset-continuation/{RESUME,README,work-packets}.md` | 25k |

PU-01 detail, from `CS-07b3-review.md`:

- m-1: assert that ids 120001, 120002 and 120101 are among the `dialog_set_maps` ids read. A floor of 4684 is the fallback. The revert proof drops the `VALUES (` branch, and the new assert must fail.
- n-2: rewrap the 138-character `///` line above `live_db_castle_567_chains_ship_gated`.
- m-2: `mission_567_castle_lifecycle.rs:68` should read "The eight gated Castle chains ship disabled (CD-11; 1400 ships on); force both sets on."

**Folding option:** if FZ-07's review returns `fix`, append m-1 and n-2 to that fix pass, and PU-01 shrinks to the one comment.

PU-02 detail:

- m-2 lines: RESUME 18 and 33, README 126.
- n-1: name `12af405fa` for CS-07bF5.
- If the 1632 heading changes, change `end-uat.md:210` with it.
- Record round 3 as ship. Add m-3 ("a GM-forced step 2000 gets no cabinet feedback; do not widen 1400") to the CD-11 enable recipe.
- D-CA18: met by absence (finding 5).

## Waves and order against the FZ plan

1. **Wave A, now, parallel to FZ wave 3:** PU-03, PU-04a, PU-04b. All are read-only and write nothing to the shared tree.
   - A PU-03 `fix` verdict goes to the architect for a fix packet, because this is the authority path; it is never fixed in place.
   - PU-04a and PU-04b `fix` notes are folded into PU-01 and PU-02.
2. **Wave H, now, independent:** PU-05 to PU-08. Each writes one new worknote, so they don't overlap each other or any FZ packet. Each is followed by one packet-reviewer pass.
3. **Wave B, after FZ-06 to FZ-10 merge:** PU-01, PU-02 and PU-09, on disjoint files. That makes three small packets for one wave review.
4. **Then FZ-11** verifies the integrated branch, PU merges included.
   - FZ-12 (if D-FZ1 is "enable") runs after PU-01 and PU-02, because it edits `mission_567*.rs` and `mission-chains.md`.
5. **Deferred to a later PR:** CS-06a/b + the HA-01b build, HA-15c seeds, HA-15b vendors, the H05 implementation, HA-05, HA-09.

## Rules every PU brief must carry inline (packet-coder skips CLAUDE.md)

1. PowerShell only, with absolute paths. Start every call with `Set-Location <worktree>;`.
2. Run cargo only through `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>` on `sgw_pu_NN`; never touch `sgw`.
3. Edit only the named files. `docs/**/*.md` is CRLF: edit with the Edit tool.
4. Files have a soft cap of 500 lines and a hard cap of 700; `mission_567_castle.rs` is already at 504, so don't grow it.
5. Seeds live in `db/resources/` only; no `db/scripts` migration. Change no `enabled` flag (D-FZ1).
6. Never hand-edit a `gen:` block, and commit no regenerated one (CS-07bF5).
7. These are forbidden: `git stash`, `git worktree prune`, fetch, push, rebase and `git add -A`. A test fix needs a revert proof.
8. No lab and no client. Advisors label every design PROVISIONAL (O18-13) and allocate no ID without the collision check (HA-D09).

## Owner and maintainer items (the architect asks nobody)

| Id | Status | Question | Recommendation | Blocks |
|---|---|---|---|---|
| D-PU1 | Decided (owner, 2026-10-19) | Ship mission 1664 (Castle CS-06a chain 1398 + the Harset HA-01b receiver) in this PR? | **In this PR** (the owner overrode the "follow-up PR" recommendation). | FZ-18 to FZ-23 |
| D-PU2 | BlockedDecision | HA-15b vendor prices inside O14-12 (tier 1 at 1.0x, tier 2 at 2.5x proposed) | Review the PU-08 draft | finalizing HA-15b |
| (open) | Owner | All decided (see [decisions.md](decisions.md)) except D-PU6 (door position, FZ-U2 lab pin) | See [decisions.md](decisions.md); the build assumes P0 and needs none of them | none |
| (open) | Maintainer | Vendor faction gating (HA-15g §6) | Keep it flavour-only for the first seal | HA-15b build |
| (open) | Owner | HA-15a treat item; Castle OD-4 hub medic; the two uncommitted files in `harset-ha-1360a2b-refusal` (inspect or discard before its retire) | unchanged | as before |
| D-FZ1 | Decided (owner, 2026-10-19): enable | Enable 567 | Shipped by FZ-12 | none |
