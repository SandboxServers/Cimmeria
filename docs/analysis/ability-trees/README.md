# Ability Trees (FINAL v2, Level 1-50)

> Type: how-to. Audience: Claude Code coordinator and implementing engineers.
> Updated: 2026-09-25. Companions: [compatibility audit](audit.md), [work packets](work-packets.md), [source data](source/README.md), [Phase 0 gap report](../sgw-handoff-pack-v1.2/phase0-gap-report.md), [documentation index](../../readme.md).

## Purpose

This campaign replaces the two-class, level-1 stub ability tree with the owner's PROJECT FINAL progression: 7 archetypes, 21 branches, 439 purchasable nodes and a level-50 cap. It adapts the existing trainer path (`trainAbility` → cell gates → atomic base debit → `AbilityGranted`) rather than building a second skill-tree system.

The source of truth is the owner's **EMULATOR FINAL v2** workbook ([source/](source/README.md)). The FINAL v1 Claude handoff that came with it supplies the packet shape and the import-safety checklist. Where the two disagree, v2 wins.

Out of scope:

- ability mechanics (each broken ability is its own ticket);
- per-world trainer placement;
- Asgard and Goa'uld gameplay UAT (their data is seeded, but their maps are outside the showcase);
- the 280-ability reserve pool.

## What was found

Against `main` @ `acbcc22e`, the [audit](audit.md) has the evidence for each row.

| Area | State on `main` | Packets |
|---|---|---|
| Trainer loop | Exists end to end and is replay-safe for a single ability. | (reused) |
| Tree data | Stub: Soldier and Commando only, level 1, no prerequisites. A different hand-copied tree lives in Rust as a fallback. | AT-01, AT-02, AT-05 |
| Branch-point gate | Not representable. **As written in v1 it cannot be met per branch**: only the 21 roots are reachable. v2 makes it archetype-wide, which reaches all 439. | AT-03 |
| Per-node cost, provenance | Missing. The base debits a hard-coded 1 point, and trained abilities are indistinguishable from starter grants. | AT-01, AT-03 |
| Trainer authority | A forged `trainAbility` trains from anywhere. | AT-04 |
| Feedback | Every rejection is silent, and the point counter is stale after a purchase. | AT-04 |
| Client UI | The Trainer window is disabled in its `.toc`, but the enabled **Ability window** handles `TrainerOpen` and purchases. No client patch is needed. At most 30 buttons fit per tab; the largest branch has 25. | AT-E1 |
| Level cap | 20, with two XP tables and a DB check at 20. v2 sets 50 with a defined 21-50 curve. | AT-07 |
| Respec | The button is live in the client, but the server handler is a stub. | AT-08 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-AT01 | **APPROVED** (owner, v2 workbook, 2026-09-25) | `required_branch_points` counts trainer points spent **across the archetype**. Prerequisites alone keep a node on its branch path. | Sheets `13_Progression_Rules` and `16_Emulator_Decisions`. Per-branch counting deadlocks every branch after its root (audit A-20). |
| D-AT02 | **APPROVED** (owner, v2) | Level cap 50. XP 1-20 unchanged; 21-50 per sheet `15_Emulator_Level_1_50`. Training points: 1 at level 1, +1 per level, 50 in total. New characters start with 1. | v2 `16_Emulator_Decisions`. This supersedes the v1 handoff's "do not invent a 21-50 curve", because the owner has now supplied one. |
| D-AT03 | **APPROVED** (owner, v2) | Only trainer purchases count as spend. Respec removes and refunds trainer-purchased nodes only, resets spend, and is atomic and replay-safe. The naquadah price stays at today's policy. | v2 `13_Progression_Rules`. |
| D-AT04 | **APPROVED** (owner, v1 handoff and v2) | The Free Jaffa / Shol'va tree (Heritage, Tau'ri, Tactics) maps to `ARCHETYPE_Sholva` (7) only. `ARCHETYPE_Jaffa` (8) gets no rows. | Both handoffs. Phase 0's "fourth Tau'ri branch" is superseded. |
| D-AT05 | **APPROVED** (owner, 2026-09-25) | Autonomous run: workers in isolated worktrees, squash-merge each PR after green CI and review, and `/release` on the last PR. | Owner answer to the launch question. |
| D-AT06 | PROPOSED (coordinator default) | Provenance is stored on `sgw_player` as `trained_abilities integer[]` plus `tree_points_spent integer`. Both change in the same `UPDATE` as the debit. | One row and one statement make it atomic. A character's archetype never changes, so one counter is enough. v2 asks to "persist/infer trainer provenance". |
| D-AT07 | PROPOSED | For UAT, the Interaction Debug NPC (template 25, list 1) offers the full tree of every archetype. Binding real trainer NPCs waits until the system is proven. | v1 handoff §6. Locked nodes must be offered with `trainable = 0` to appear greyed rather than hidden (audit A-23). |
| D-AT08 | PROPOSED | A rejected purchase sends `onErrorCode` (`ERRORCODE_SYSTEM_Ability`, the ability id, and the closest `EConditionHandlerFeedback` value per reason, confirmed by AT-E1), then re-sends `onTrainerOpen`. A duplicate purchase stays silent. | Project rule: every button press gets visible feedback on the first press. |
| D-AT09 | PROPOSED | Starter ability 1646 (also a Goa'uld Servant Lord node) is left as it is and listed as the seed guard's one allowed exception, until Goa'uld UAT is scheduled. | v2 says to reconcile it before Goa'uld UAT, which is outside the showcase scope. |
| D-AT10 | PROPOSED | The respec price stays at 1000 naquadah (today's `onTrainerOpen` value). | v2: "currency price remains separate/current-server policy until sourced". |
| D-AT11 | PROPOSED | Existing characters keep every ability they know. `trained_abilities` starts empty, so their spend starts at 0. Nothing is revoked or backfilled. | Both handoffs forbid revoking. The colo database reloads from the seed on each deploy, so only local databases carry old characters. |

Under the autonomous-run authorization, PROPOSED rows are adopted at these defaults unless the owner objects. A change is recorded as a new row, never by editing an old one.

## Coordinator launch prompt

You are the Claude Code coordinator for the ability-tree campaign. Implement [work-packets.md](work-packets.md) as small reviewed PRs.

1. Record `git rev-parse origin/main` and check the audit's file references still hold. Check `~/.claude/sessions/*.json` for a live peer on this campaign (`trees/*` branches). If one exists, message it and stand down.
2. **Wave 0:** dispatch AT-01, AT-E1 and AT-05a in parallel worktrees with disjoint ownership. AT-01 is the only bottleneck, so review and merge it first.
3. **Wave 1:** once AT-01 is on `main`, dispatch AT-02, AT-03, AT-04 and AT-07 in parallel, and rebase AT-05a into AT-05b. Merge in the order the contended-file list gives.
4. **Wave 2:** AT-08, then AT-09 with `/release`.
5. Give each worker only its packet, the contract section and the audit rows it cites. Every worker prompt names the lane wrapper, its `sgw_<worktree>` database and the junction step.
6. When blocked, leave `handoffs/<packet>.md` with the exact next action.

## UAT milestone

A single milestone: the owner runs [AT-06](work-packets.md#at-06-owner-uat-colo-after-the-release) on the colo after the `/release` deploy, as GM, and uses `.bug <note>` at each oddity. The coordinator reads SigNoz afterwards for the `abilities` target: `train_requested`, `train_rejected reason=…`, `granted` and `train_raw_cost_zero`.

## Where confidence is low

- How native `getTrainableList` orders and filters nodes when the trainer list and `onAbilityTreeInfo` differ (AT-E1, question 1).
- Which feedback codes the client renders as readable text for a training failure (AT-E1, question 2).
- Whether any client-side level table caps the XP bar at 20 (AT-E1, question 4).
- Everything the workbook marks as reconstruction: Commando's Stealth and Precision branch membership, Robotics, Archaeology, the whole Free Jaffa and Goa'uld membership and order, and every unlock breakpoint, cost and spend gate. These are project values, not recovered retail data, and the docs must say so.
