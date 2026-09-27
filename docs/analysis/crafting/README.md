# Crafting and Applied Science Restoration

> Type: how-to. Audience: the Claude Code coordinator, packet workers and the owner.
> Updated: 2026-09-26. Companions: [audit](audit.md), [work packets](work-packets.md), [session resume](handoffs/session-resume.md), [crafting restoration findings](../../reverse-engineering/findings/crafting-restoration.md), [gap analysis §19](../../gap-analysis.md), [documentation index](../../readme.md).

## Purpose

This campaign makes the client's existing Crafting (J) and Applied Science (Ctrl+J) windows work end to end:

- learning disciplines with applied-science points (ASP);
- the four crafting verbs: craft, research, reverse engineer and alloy;
- crafting respec;
- the login sync that restores all of it after a relog.

It builds on the Phase 1 state layer (#427) and closes #567, #723 and the crafting findings of the CAT-F security audit (#465, F-02 to F-07).

**No client patch is needed.** The 2009 client already sends methods 95-100 and handles 112 and 136-140 (audit C-30 to C-40).

Out of scope:

- crafting stations in playable worlds beyond the debug hub (a later content decision, D-CR05);
- black-market or auction trading of crafted goods;
- fixing the seed's loose discipline assignments (audit C-22). The seed is Project Giza's reconstruction, and the client validates against its own cooked blueprints, so the seed stays as it is unless a UAT shows a mismatch.

## What was found

Against `main` @ `70795027`, the [audit](audit.md) has the evidence for each row.

| Area | State on `main` | Packets |
|---|---|---|
| State and persistence | Done (#427), but never loaded at login. | CR-03 |
| Login sync | Only ASP and known crafts are sent; disciplines, paradigm levels and crafting options are not. | CR-03 |
| The six verbs | Parse-and-log stubs. | CR-04, CR-07 to CR-10 |
| Catalog | No Rust loader for disciplines, blueprints, components or crafting item attributes. | CR-01 |
| Induction timer | The client draws the bar from `onTimerUpdate` type 16, which needs an absolute expiry in the client's clock. The server's clock is inconsistent today. | CR-02, CR-06 |
| Stations and tools | The client enables a verb only when `onUpdateCraftingOptions` names a tool or a machine. No template is a station, and no item is flagged as a tool. | CR-05, CR-11 |
| Blueprint and paradigm items | 289 "Blueprint: …" items exist with no link to a blueprint; the client's "Racial Paradigm Guide" items are missing from the seed. | CR-E2, CR-15 |
| Paradigm gate | The four real root disciplines need Common paradigm level 5; every character has level 1 or none, so nothing is learnable today. | D-CR03, CR-03, CR-15 |
| Feedback | Neither the client nor the legacy server tells the player why a request failed, and three pages empty their slots on confirm. | D-CR14, every verb packet |
| Legacy reference | `Crafter.py` is complete but has nine defects (audit C-50 to C-58). | D-CR11 |

## Owner decisions

Asked on 2026-09-26 with the coordinator's recommendation beside each. All six are answered.

| ID | Status | Question | Owner answer |
|---|---|---|---|
| D-CR01 | **APPROVED** (owner, 2026-09-26) | How do players earn ASP? Even the 2009-era server granted it only by GM command (audit C-03). | 1 ASP at level 1 and +1 per level gained, granted where training points are granted (`grant_xp`), so 50 at the cap. The GM grant stays. |
| D-CR02 | **APPROVED** (owner, 2026-09-26) | What does crafting respec cost, and what does it clear? | **Free**, full reset: it clears every discipline and all expertise and refunds one ASP per learned discipline. Blueprints and paradigm levels are kept, because under D-CR04 and D-CR03 they come from items the player used, not from disciplines. |
| D-CR03 | **APPROVED** (owner, 2026-09-26) | Starting racial-paradigm levels, and how they rise (audit C-21). | A designed progression, following the client's own item text. New characters start **Common at 5** (the roots become learnable) and the other four paradigms at 1. Using a **"Racial Paradigm Guide: <paradigm>"** item raises that paradigm by 1, to a maximum of 10 (client text 28224-28234: "Permanently increases player's Racial Paradigm score by one, to a maximum of 10"). Guides are sold by the crafting-supplies vendor and can drop as loot. |
| D-CR04 | **APPROVED** (owner, 2026-09-26) | How are blueprints acquired? Python only granted them by GM command. | **Blueprint items and research.** Using a "Blueprint: …" item (289 in the seed) teaches its blueprint and consumes the item; crafting vendors sell them. A successful research of a researchable item also teaches the blueprint that makes it, when that blueprint's discipline is known. Learning a discipline grants no blueprints. |
| D-CR05 | **APPROVED** (owner, 2026-09-26) | Stations, tools, or both? | **Both.** Four per-science Crafting Stations in the stasis-room debug hub, each allowing all four verbs, and the 48 Field Crafting Tools as portable tools (rule in D-CR21). Stations in playable worlds are a later content decision. |
| D-CR06 | **APPROVED** (owner, 2026-09-26) | Reverse-engineer recovery (audit C-51, C-52). | Recovery rises with expertise: per component `floor(rand × min(1, max(exp, 1) / tc) × qty)`, where `tc` is the product's tech competency, with at least one unit of some component recovered. Reverse engineering needs no known discipline. |

The 2009 def supports the item-driven design: `SGWPlayer.def:894-909` declares `gainRacialParadigmLevels`, `gainExpertise` and `gainAppliedSciencePoints`, each "with transaction", the shape of a reward granted by item use or a mission.

## Coordinator decisions

These are PROPOSED coordinator defaults. Under the owner's autonomous-run authorization (D-CR10) they are adopted unless the owner objects. A change is recorded as a new row, never by editing an old one.

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-CR10 | **APPROVED** (owner, kickoff 2026-09-26) | Autonomous run. Each worker gets its own worktree and test database. Squash-merge each PR after green CI, re-testing after a rebase when CI predates `main`. `/release` goes on the last PR. | Owner's kickoff instructions. |
| D-CR11 | PROPOSED | Where the client and Python disagree, the **client wins**. That covers the alloy counts (Normal 10, Good 5, Great 2, Fantastic 1, counted by stack quantity; audit C-38) and the kicker rules (one per applied science, not the item's own; C-39). Python's defects C-50 to C-58 are not ported. | The client is the only 2009 artifact; `Crafter.py` is a later reconstruction. A server rule stricter or looser than the client's own check produces requests the UI cannot explain. |
| D-CR12 | PROPOSED | **Validate at request, consume at completion.** The request is validated and answered with the induction timer. At the end, one database transaction re-validates, consumes the inputs and grants the outputs. Inputs must sit in `INV_Main` (1) or `INV_Crafting` (15). | Closes CAT-F F-03 to F-05 and F-07, and fixes C-58: a crash or logout during the induction loses nothing. The vendor purchase transaction is the template (audit C-11). |
| D-CR13 | PROPOSED | **One induction at a time per player, in a FIFO queue of at most 10.** Anything past 10 is rejected with feedback. The queue lives on the base and is dropped on logout or a world change, with no consumption. | The client's reverse-engineer page sends up to 10 requests in a burst (C-34). |
| D-CR14 | PROPOSED | **Every rejection is visible.** The player gets a readable text line through the existing feedback path. Where a condition code fits (213/214, not enough ASP), `onErrorCode` is sent too. The client's slots are then corrected: an inventory resync after a craft-family rejection, and the discipline and ASP state after a spend rejection. | Project rule: every button press gets visible feedback on the first press. Three pages empty their slots on confirm (C-33). |
| D-CR15 | PROPOSED | Craft and alloy need the blueprint **and** its discipline known. Expertise: +1 per craft or alloy, +5 per successful research, capped at 100. Research chance is `100 − expertise + 5 × kickers` over the item's known disciplines with `0 < expertise < item tech_comp`, picked uniformly. | Python's gains and chance formula, which the client's research page also displays. Requiring the discipline removes C-55. |
| D-CR16 | PROPOSED | Respec is **two-step**. The first `respecCrafting` answers `onCraftingRespecPrompt(cost)` and records a pending respec for 60 s. A second one while pending executes it in one transaction, then sends `onDisciplineRespec`, the ASP property and the new known-crafts list. CR-E1 confirms or corrects the flow. | Acting on the first send would wipe the player before they confirm (C-37). |
| D-CR17 | PROPOSED | GM crafting tooling for UAT: `.allcraft` (every paradigm 7, every discipline at 100, every blueprint, and "craft anywhere"), and `.craftkit <blueprint> [count]` (grants component set 1 for `count` crafts). "Craft anywhere" sends crafting options naming the player's own entity id as the machine for all four sections, as Python's `.allcraft` did, and the server gate accepts it for that session. | Owner instruction: a proper in-game way to test. Python precedent (`cell/commands/Crafting.py:85-95`). |
| D-CR18 | PROPOSED | One shared `CraftingCatalog`, loaded once and consumed by cell and base: disciplines, blueprints with component sets, and the crafting attributes of items (flags, tier, quality, tech competency, disciplines). Researchable is `Craft_Research`, reverse-engineerable is `Craft_RevEng`, kicker is `Kicker`. `ElementaryComponent` is ignored because every item has it (C-24); alloy inputs are validated by tier and quality. | One source of truth, like `AbilityTreeCatalog` in the ability-tree campaign. |
| D-CR19 | PROPOSED | The ID blocks are templates 310-329 and spawns 410-429. The branch prefix is `craft/`. | Agreed with the debug-hub, guilds and pets sessions on 2026-09-26 (C-28). |
| D-CR20 | PROPOSED | The induction bar needs an absolute expiry (C-32). CR-02 makes the server's game clock consistent and gives one `game_time_secs()` for every timer sender. If CR-02 cannot land in time, the induction still completes on the server, and the inventory update plus a text line carry the result. | The bar is cosmetic; the craft result must not depend on it. |
| D-CR21 | PROPOSED | A **Field Crafting Tool** counts only in the crafting bag (`INV_Crafting`, 15), the only inventory bag its `container_sets` allows. Its science comes from the name prefix (BMAS = Biomedical, EAS = Electronic, PSAS = Power Systems, MAS = Materials) unless CR-E2 finds a cooked-data field. A tool enables craft, research and reverse engineering for disciplines of its science whose `tech_competency` is at most the tool's `tech_comp`. Alloying needs a station. Tools are not consumed. | Owner answer D-CR05. The tools' `tech_comp` runs 5-55 in steps of 5, which lines up with the disciplines' `tech_competency` (1-50). |
| D-CR22 | PROPOSED | Blueprint and Paradigm Guide items are used through the ordinary `useItem` path. A seed table maps each item to its blueprint or paradigm, built from the client's cooked data by CR-E2; items the client does not ship are added to the seed only if the client can render them. Using an already-known blueprint or a guide at 10 is refused with feedback, and consumes nothing. | Owner answers D-CR03 and D-CR04. Seeds are the source of truth. |
| D-CR23 | PROPOSED | **Opening a respec is server-side.** CR-E1 found that the client sends `respecCrafting` (100) only from the Yes button of the 112 prompt, and no client UI sends a first request. So a player-usable `.respeccraft` console command sends `onCraftingRespecPrompt(0)`, records a 60 s pending respec, and the following 100 executes it. A 100 with nothing pending is refused with feedback. This refines D-CR16. | [crafting-client-ui.md](../../reverse-engineering/findings/crafting-client-ui.md) Q1. |

## Coordinator launch prompt

You are the Claude Code coordinator for the crafting campaign. Implement [work-packets.md](work-packets.md) as small reviewed PRs.

1. Record `git rev-parse origin/main` and check the audit's file references still hold. Check `ListAgents` for a live peer on this campaign (`craft/*` branches). If one exists, message it and stand down.
2. **Wave 0:** dispatch CR-01, CR-E1, CR-E2 and CR-02 in parallel worktrees (`bash tools/build-lane/mk-worktree.sh craft/<packet>-<slug> <name>`). CR-01 is the bottleneck; review and merge it first.
3. **Wave 1:** once CR-01 is on `main`, dispatch CR-03, CR-04, CR-05 and CR-06. Merge in the order the contended-file list gives.
4. **Wave 2:** CR-07 to CR-12 and CR-15.
5. **Wave 3:** CR-13 close-out, with the UAT checklist and `/release` (PowerShell, or `MSYS_NO_PATHCONV=1` in Git Bash).
6. Give each worker the rules file (`%TEMP%\cimmeria-castle\CRAFT-WORKER-RULES.md`), its packet, the decisions it cites and the audit rows it cites. Each worker uses the lane, its own `sgw_<worktree>` database and the `external` junction.
7. When blocked, leave `handoffs/<packet>.md` with the exact next action, and keep `handoffs/session-resume.md` current.

## UAT milestone

The owner runs [CR-14](work-packets.md#cr-14-owner-uat-colo-after-the-release) on the colo after the `/release` deploy, from the stasis-room debug hub, as GM, and uses `.bug <note>` at each oddity. Afterwards the coordinator reads SigNoz for the `crafting` target: `request`, `rejected reason=…`, `induction_started`, `completed` and `persist_failed`.

## Where confidence is low

- The client's clock domain for `BigWorldTimeComplete` (C-13, C-32). Until CR-02 lands, the bar may not draw.
- Whether `onErrorCode` shows any text in the client, and which of the feedback paths the player actually sees (the same open question as AT-E1 Q2).
- The seed is Project Giza's reconstruction, while the client checks against its own cooked blueprints and disciplines. A mismatch shows up only in UAT, as a request the client refuses to send.
