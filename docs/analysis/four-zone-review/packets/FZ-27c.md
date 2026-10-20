# FZ-27c: docs for the mission 1664 debrief (step 5226)

- **Status:** BlockedDependency on FZ-23 merged into the campaign branch, and on FZ-27a and FZ-27b committed with packet review `ship`. Before you start, merge the campaign branch into `four-zone-review/FZ-27-1664-debrief` (ops does this; the brief says so) so FZ-23's text is in your tree.
- **Wave:** 12, third of three sequential packets. Same worktree and branch as FZ-27a/b; one commit on top of FZ-27b, with its own packet review loop.
- **Depends on:** FZ-23 (it wrote the text this packet edits), FZ-27a, FZ-27b, and FZ-25 if FZ-25 ran (edit E6 below depends on it).
- **Fixes:** FZ-23 documents step 5226 as "not built yet (D-PU3)". Owner decision D-PU3 = B built it in FZ-27a/b; the chain reference, the FZ-U2 UAT row and both id ledgers must say so.
- **Agent:** packet-coder (docs only). About 15k working tokens.
- **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27`. No test database.
- **Branch:** `four-zone-review/FZ-27-1664-debrief`.
- **Report:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27\.claude\reports\four-zone-review\FZ-27c-coder.md`
- **Return (one line):** `FZ-27c <done|blocked|failed> <sha|-> <report path>`
- **Host:** windows

## Rules (inlined; you do not read CLAUDE.md)

1. Use PowerShell only, with absolute paths. Start every call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27;`.
2. Files under `docs/` are **CRLF**. Edit them with the Edit tool only; never rewrite a whole doc with Write. After your edits, check: `$p='<absolute path>'; ([IO.File]::ReadAllText($p) -split "`r`n" | Where-Object { $_ -match "`n" }).Count` must be 0 for each file.
3. Never hand-edit a generated block (between `<!-- gen:NAME -->` markers). If an edit point is inside one, return `blocked`.
4. Do not touch `docs/gap-analysis*` or `docs/project-status.md` (they change once, at campaign close-out).
5. Keep the tested/untested wording: every claim about 5226 is "built, replay-tested, never run on a client".
6. Line numbers below are from FZ-23's branch head `507310a5e`; find each edit by its quoted text, not its number. If a quoted text is not there (FZ-23's review changed it), make the same meaning change to whatever replaced it and say so in the report. If the whole sentence is gone, skip that edit and say so.
7. Edit only the four files below. Forbidden: `git stash`, `git worktree prune`, fetch, push, rebase, `git add -A`. Stage the four paths explicitly.
8. No compile check. Read with the Read tool, search with Grep.

## Facts to write (from FZ-27a/b; do not re-derive)

- Step 5226 "Report for debriefing", Command Center (world 68). Click Col. Marsh (template 10) and Moh'katan (template 54) in either order. Each click barks a line in chat, drops that NPC's cue and credits one objective with `complete_objective`: Marsh 6061 (chain 6018, screen 200180), Moh'katan 6062 (chain 6019, screen 200181). The second credit auto-completes 1664.
- Chains 6033 (Marsh, dsm 120302) and 6034 (Moh'katan, dsm 120303) restore the cue of whichever NPC is still unreported on Command Center login (`player_loaded 'Harset_CmdCenter'`).
- 6016 (abandon) now drops all four cues: Kaerlia, the door, Marsh, Moh'katan.
- Bark lines are PROVISIONAL (decision D-FZ15): "Report received. Good work out there." and "Your report is filed."
- Opcore chains 6511/6512 (the Praxis offer on Marsh, mission 1361) stand down while 1664 is on step 5226, so the offer dialog never buries the report. 6511 also fires on `mission_completed '1664'` and `mission_abandoned '1664'`, so a Human with 1361 not active gets Marsh's "?" back as soon as 1664 ends (D-FZ14).
- Tests: `chain_replay_tests::harset_1664_debrief` and `chain_replay_tests::mission_1664_debrief_flow` (new); `harset_1664`, `handoff`, `disjointness` extended.
- Ids: chains 6018-6019 (in the 6008-6020 block), 6033-6034 (outside it: unlisted Harset 6001-6800 capacity; 6021-6032 are HA-01a); dsm 120302-120303; dialog holder 60170; screens 200180-200181.

## Edits

**E1. `docs/content/mission-chains.md`, `#### Mission 1664: "Debriefing"` (about line 2111).** Replace the sentence `The Harset chains live in `harset_1664_chains.sql` (6008-6016; 6017-6020 reserved).` with:

- if FZ-25 did not land: ``The Harset chains live in `harset_1664_chains.sql` (6008-6016 and 6018-6019, plus the debrief cue restores 6033-6034; 6017 and 6020 reserved).``
- if FZ-25 landed (chain 6017 exists in the seed; Grep `(6017,` in `db/resources/Content/Seed/harset_1664_chains.sql`): ``The Harset chains live in `harset_1664_chains.sql` (6008-6019, plus the debrief cue restores 6033-6034; 6020 reserved).``

**E2. Same table, the 5226 row (about line 2117).** Replace `| 5226 | Report for debriefing | Not built yet (decision D-PU3) | none |` with:

```text
| 5226 | Report for debriefing | In the Command Center, click Col. Marsh (template 10) and Moh'katan (template 54) in either order: each barks a line, drops its cue and runs `complete_objective` (6061 Marsh, 6062 Moh'katan); the second credit completes 1664 | 6018, 6019; 6033 and 6034 restore the unreported NPC's cue on login |
```

**E3. Bullet `- 6016 drops both cues on abandon. Every click and login chain is gated on world 57.`** Replace with `- 6016 drops every cue on abandon (Kaerlia, the door, Marsh, Moh'katan). The 5224/5225 click and login chains are gated on world 57, the 5226 chains on world 68 (the Command Center).`

**E4. Bullet starting `- Credit uses `advance_step` only:`.** Replace the whole bullet with `- Steps 5224 and 5225 credit with `advance_step` (`complete_objective` on 6060 or 6063 would auto-complete the mission). Step 5226 credits with `complete_objective` 6061 and 6062, because the second one is meant to complete it.`

**E5. Insert a new bullet right after the E4 bullet:**

```text
- The bark lines (screens 200180 and 200181, holder dialog 60170) are provisional text (D-FZ15). While 1664 is on step 5226, the Praxis offer on Marsh (opcore chains 6511/6512, mission 1361) stands down so its dialog does not bury the report; 6511 also fires on 1664's completion and abandon, so the "?" comes back at once (D-FZ14).
```

**E6. The bullet starting `- The door control's position is a placeholder (D-PU6);`.** Only if FZ-25 landed: change `walking through the walk-in door (6006) on step 5225 does not credit the step (D-PU4)` to match what FZ-25 built (read FZ-25's chain 6017 description in the seed and say it in one clause, for example `walking through the walk-in door (6006) on step 5225 also credits the step (6017, D-PU4)`). If FZ-25 did not land, leave it.

**E7. The `- Tests:` bullet.** After `` `mission_1664_flow` `` add `` , `harset_1664_debrief`, `mission_1664_debrief_flow` `` so the list reads `` `chain_replay_tests::castle_1664`, `harset_1664`, `harset_1664_lifecycle`, `mission_1664_flow`, `harset_1664_debrief`, `mission_1664_debrief_flow`; `spawner_tests::harset::mission_1664_actors`. ``

**E8. `docs/guides/unified-uat.md`, the `| FZ-U2 |` row (about line 885).** It is one table row with three text cells (steps | expected | notes). Keep it one line.

- Steps cell: after `then find the Command Center door control and click it;` insert ` in the Command Center, click Col. Marsh, then Moh'katan (on a second run, the other order);`. After `on a second character, abandon 1664 on each step` append `, including once after reporting to only one of the two; on a Human with 1361 not active, watch Marsh before and after 1664 ends`.
- Expected cell: after `with the journal on "Report for debriefing".` insert ` Marsh and Moh'katan each show a mission cue; each click puts that NPC's line in chat and ticks one objective, either order works, and the second report completes 1664. Clicking an NPC you already reported to, before the second report, gives nothing new. A relog mid-5226 brings back the cue only on the NPC you have not reported to. A Human with 1361 not active sees Marsh's "?" offer again right after 1664 completes, and after an abandon at 5226.`. Change `After an abandon both cues are gone,` to `After an abandon every 1664 cue is gone (including the debrief cues),`.
- Notes cell: replace `Step 5226 has no credit yet.` with `The two debrief lines are provisional text (D-FZ15).`.

**E9. `docs/analysis/harset-continuation/id-allocation.md`, row 26 (`| HA-01b (provisional 1664) |`).**

- Third cell: after `dsm 120301` add `; dsm 120302-120303, dialog 60170, screens 200180-200181 (FZ-27)`.
- Fourth cell: replace `6008-6016 seeded, 6017-6020 reserved (D-PU3, D-PU4, O18-02c).` with `6008-6016 seeded; 6018-6019 seeded by FZ-27 (D-PU3 = B); 6017 and 6020 reserved (D-PU4, O18-02c). FZ-27 also claimed 6033-6034 (the 5226 cue restores) outside this block.` If FZ-25 landed, write `6017 seeded by FZ-25` and `6020 reserved (O18-02c)` instead.

**E10. `docs/analysis/four-zone-planning/id-allocation.md`.**

1. Line 110: replace `| HA-01b 1664 (FZ-20: 6008–6016 seeded, 6017–6020 reserved) | 6008–6020 |` with `| HA-01b 1664 (FZ-20: 6008–6016 seeded; FZ-27: 6018–6019 seeded, 6033–6034 claimed; 6017 and 6020 reserved) | 6008–6020, 6033–6034 |` (if FZ-25 landed: `FZ-25: 6017 seeded` and `6020 reserved`). This file uses en dashes (–) in ranges; keep them.
2. Insert this section right before the line `### Non-chain namespaces` (about line 237), with one blank line before and after:

```text
### FZ-27 claims — 2026-10-19

Individual claims for the mission 1664 step 5226 debrief (four-zone-review FZ-27a/b), made by the architect under the owner's delegation on D-PU3 (decision D-FZ14 in the [four-zone-review decisions](../four-zone-review/decisions.md)). HA-00c nominally owns Harset authored dialog allocation; these are individual claims, not a range reservation.

| Namespace | Claimed IDs | Exact use | Status |
|---|---|---|---|
| Content chain | 6018, 6019 | `interact_tag` report to Col. Marsh and to Moh'katan: bark, drop the cue, `complete_objective` 6061 / 6062 | In use (FZ-27a) |
| Content chain | 6033, 6034 | `player_loaded 'Harset_CmdCenter'` restore of the Marsh / Moh'katan debrief cue | In use (FZ-27a) |
| Dialog set map | 120302, 120303 | NULL-dialog interaction-bit cues on templates 10 and 54 | In use (FZ-27a) |
| Dialog root | 60170 | Seed-only bark holder, never displayed | In use (FZ-27a) |
| Dialog screen | 200180, 200181 | One provisional bark line each, under dialog 60170 (D-FZ15) | In use (FZ-27a) |

Collision check, 2026-10-19: a `git grep` of every local branch and worktree for these IDs as the first value of a `VALUES (` tuple (chains and triggers under `db/resources/Content`, dialog set maps, dialogs and screens under `db/resources/Dialogs`) found zero hits across 892 refs. The same patterns found known rows (chain 6012: 5 hits; screen 200160 and dialog 60160: 46 each), so the search works. Chains 6033–6034 sit in Harset's 6001–6800 block, outside every HA candidate range above (HA-01a ends at 6032). Dialog 60170 sits in the Cimmeria-authored 60100–60199 range, clear of 60150 and 60160. The screens are at 200000 or above, clear of the client PAK's screens, which run to 120383. Point-in-time.
```

## Checks, in order

1. The CRLF check from rule 2 on all four files: 0 for each.
2. `git diff --stat` shows only the four files, and `git diff` shows no line-ending churn (only the edited lines change).
3. Grep each file for the old texts (`Not built yet (decision D-PU3)`, `Step 5226 has no credit yet`, `6017-6020 reserved`, `6017–6020 reserved`): zero hits.
4. `python tools/docs-gen/regen.py --check` (record the output; do not fix drift outside your files).

## Commit

```text
docs: mission 1664 step 5226 debrief (FZ-27c)

Chain reference, FZ-U2 UAT row and both id ledgers now describe the
Marsh and Moh'katan debrief (chains 6018/6019, cue restores 6033/6034)
and the Praxis offer standing down at 5226. Built, replay-tested,
never run on a client.

Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_012n1ut2hRgmMP5C6ZXG9HEV
```

## Report

Write the report at the path above: each edit E1-E10 with done/skipped and why, the FZ-25 branch you took (landed or not), the four check results, and the commit sha.
