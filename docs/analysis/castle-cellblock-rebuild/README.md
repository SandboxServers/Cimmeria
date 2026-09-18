# Castle Cellblock Rebuild Handoff

> Type: how-to. Audience: Claude Code coordinator and implementing engineers.
> Updated: 2026-09-17. Companions: [audit.md](audit.md), [work-packets.md](work-packets.md), [parity campaign protocol](../legacy-command-parity/README.md), [documentation index](../../readme.md).

## Purpose And Evidence Boundary

Use this document as the coordinator launch prompt for bringing the Castle_CellBlock tutorial up to the `SGW_Castle_CellBlock_Rebuild_Spec.xlsx` specification. The [audit](audit.md) compares that spreadsheet, the original Python server scripts under `deprecated/python/cell/`, and the live Rust chain seed. It is a static comparison: no build, test, live-DB run or client session accompanies this handoff, and nothing here is recovered binary evidence or a Bible chapter.

The single most important finding: the spec assumes the 2009 server scripts are lost and must be inferred. They are not. Every Cellblock script exists in this repository and is already ported to content chains 1001-1111. Of the spec's 25 rebuild rows, 15 are already done, 5 are spec-only content the original server never triggered (new authoring), 2 are restorations of Python behavior Cimmeria dropped, and 3 are Castle-side or design questions. The audit also found five live defects in the seed, the largest being an auto-exported duplicate chain file that cross-binds Human and Jaffa dialogs.

Source baseline: `01ab54b0` on `fix/gm-teleport-navmesh-loop` (tree identical to `main` for every path cited), inspected 2026-09-17. Unrelated untracked `crates/services/logs/` is excluded.

This handoff adds documentation only. It does not authorize commits, branches, worktrees, builds, runtime edits or new agents. The execution protocol is for a subsequent, user-authorized implementation session.

## Coordinator Launch Prompt

You are the Claude Code coordinator for the Castle Cellblock rebuild. Work in this repository's root checkout. Implement the approved packets in [work-packets.md](work-packets.md) through small, reviewed changes, not one monolithic content rewrite. Do not broaden into other zones (beyond the single Castle handoff packet), the XP system, NPC AI, or an engine redesign.

1. Run `git rev-parse HEAD` and `git status --short --branch`; record them in the ledger. Compare with the baseline above and inspect only changed paths under `db/resources/Content/Seed/`, `crates/services/src/cell/content/` and `crates/content-engine/` before dispatching affected packets.
2. Read [AGENTS.md](../../../AGENTS.md), [CLAUDE.md](../../../CLAUDE.md), [.github/instructions/content-chains.instructions.md](../../../.github/instructions/content-chains.instructions.md) and [TESTING.md](../../../TESTING.md) (chain-replay is test type 6). Read [content-engine.md](../../content/content-engine.md) sections 3, 8 and 9 once; give workers only the sections their packet cites.
3. Read the decisions below. Every **PROPOSED** decision needs the user's answer before its packet leaves BlockedDecision. Record answers as new decision rows; do not rewrite existing ones.
4. Confirm implementation-session authorization. Use `rust-gameserver-dev` on Sonnet as the writer, with the advisors named per packet; discover agents from `.claude/agents/`, and do not create new ones.
5. Dispatch C01 first (it removes the double-fire baseline every later test would otherwise encode). C08a may run in parallel with disjoint ownership. Then follow the dependency order in the ledger's closeout section.
6. For GC1/GC2/GC3, collect evidence, propose the child manifests, get a decision id, then dispatch children individually.
7. Integrate one packet at a time under the single machine-wide Cargo lane, re-run the packet's replay tests plus `cargo nextest run --profile=ci-live-db -p cimmeria-services --lib chain_replay`, and pause for the user's milestone UAT.
8. When blocked by a decision, evidence or context budget, leave a durable handoff under `handoffs/<packet-id>.md` with the exact next action. Do not substitute a placeholder chain for missing content.

## Approved And Proposed Decisions

Rows marked **PROPOSED** are the coordinator's recommended defaults with the reasoning and confidence behind them. They are questions for the user, not approvals. Confidence describes the evidence for the recommendation, not the difficulty of the work.

| ID | Status | Contract and reason | Confidence |
|---|---|---|---|
| D-CB01 | PROPOSED | Evidence precedence: original Python scripts, then the curated Rust seed, then the spec. Spec rows marked INFERRED/UNRESOLVED are proposals. The spec is used for content the Python never wired and for the acceptance test list. | HIGH: the scripts are in the tree and the chains cite them line by line. |
| D-CB02 | PROPOSED | Purge auto-exported chains 5000-5029, keeping only the Region8 guard aggro (re-keyed to `Castle_CellBlock.Region8`, Python values 1000 threat plus aggression 1). | HIGH that the rows duplicate curated chains and cross-bind Jaffa/Human dialogs; MEDIUM on the exact in-client symptom, since no client run was done. |
| D-CB03 | PROPOSED | Accept mission 1360 (Frost's Letter) when Frost's letter is granted; leave step 4038 for the Castle. The original never accepted it; this is new content the spec calls optional. | HIGH on mechanics (one chain). The value is a product call. |
| D-CB04 | PROPOSED | Apply Stasis Sickness (ability 1372) on zone load while 639 is not completed, via a new `LaunchAbility` executor arm. No Stage 2 timer. | HIGH that the Python launched 1372 on every load and Cimmeria never does; MEDIUM that effect 1634 shows anything (no effect script; may be icon-only) and that the cure ability 1374 already fires from the client's item binding. |
| D-CB05 | PROPOSED | Cover objective 2484 gates step 2144 (both cover and drone kill required, as the spec and objective rows say). Flank objectives 2725/2731 are tracked but do not gate mission completion. Reason: gating the flank risks a soft-lock if the player kills guards from range before any flank fires, and the 2009 script did not gate on it. | MEDIUM: the engine has the triggers; whether a cover set covers the med-bay desk is unverified (candidate id 425). |
| D-CB06 | PROPOSED | Keep the Python two-branch Aftermath rewards (Humans get the Commando stealth set + knife with dialog 3942; Jaffa 3943). Per-class variants 2517/4408/4409 stay a design group because their item ids are unknown. | HIGH that this is what shipped; the spec's per-class mapping is inference. |
| D-CB07 | PROPOSED | Straegis scene: play the StraegisAttack camera Matinee (sequence 1751) on 686 completion, despawn Marsh, show 2516 after 10.1 s. No creature, blood or data disc. Requires honoring `delay_ms` (C08a). | MEDIUM: the sequence id and duration come from the spec's map read; whether `play_sequence` drives a Director-track Matinee correctly is untested. |
| D-CB08 | PROPOSED | Show mission-prompt blurbs 2305/4000/2308/2518 on accept, after one UAT check that the client does not already show an accept prompt. Blurb 2298 already displays this way. | MEDIUM: precedent exists; client behavior unverified. |
| D-CB09 | PROPOSED | Include one Castle-side packet: port the Gerschon handoff slice of `Castle.py` (dialog 2573, accept 701) plus a Jaffa 5861 branch. Stop at 701 accept. | HIGH that no Castle chains exist today and spawn 112 is inert; 5861 is spec-only. |
| D-CB10 | PROPOSED | Mission completion XP stays out of scope (design group GC3). The observed 52 XP is not in the DB and the engine has no XP action. | HIGH on the facts; the formula is unknown. |
| D-CB11 | PROPOSED | Reuse the parity campaign protocol wholesale: isolated worktrees, disjoint ownership, one Cargo lane, serialized live-DB tests, worknotes and handoffs in the repo, milestone UAT. | HIGH: same team, same repo rules. |
| D-CB12 | PROPOSED | Symbiote Loss (1926/2480) is out. No script references it and the ability has a placeholder name. | HIGH. |
| D-CB13 | PROPOSED | Escape escort and lockdown (GC1) is a design gate, not a packet. Marsh following the player, the unresolved 5019 gating and the missing energy-field actor each need a decision or evidence. | LOW on what the original designers intended; the 2009 script had none of it. |

## Open Decisions

All seven questions below were answered by the user on 2026-09-17. Kept for traceability; see [Decision Answers](#decision-answers) for the resolutions and what each unblocks.

1. D-CB03: accept Frost's Letter (1360) in the Cellblock, yes or no?
2. D-CB05: should the Mess Hall and Hallway05 flank objectives gate mission completion (spec) or only be tracked (recommended)?
3. D-CB06: keep the shipped two-branch Aftermath rewards, or invest in the per-class split (which needs item research first)?
4. D-CB07: is a camera-only Straegis scene with Marsh vanishing acceptable, or is it better left out until a creature/VFX plan exists?
5. D-CB08: does the current client already show a prompt when a mission is accepted? If yes, C07 is unnecessary.
6. D-CB09: is the Castle-side Gerschon handoff in scope for this campaign, or does it belong to a Castle campaign?
7. D-CB13 (GC1): do you want Marsh to follow the player at all? The cheapest faithful option is dialogs only.

## Decision Answers

Answers are recorded as new rows, not as edits to the PROPOSED rows above (per this document's own rule). Date: 2026-09-17.

| ID | Answer | Unblocks | Notes |
|---|---|---|---|
| D-CB03 | Accept 1360 (took the recommended default). | C01 → C04 | No change to C04's scope as written. |
| D-CB05 | Track only, do not gate (took the recommended default). | C01 → C05 → C06 | No change to C06's scope as written. |
| D-CB06 | **Diverged from the recommended default.** Invest in the per-class Aftermath split (Soldier 2517 / Commando 3942 / Scientist and Archeologist 4408 or 4409), not the shipped two-branch mapping. | GC2 | GC2 stays BlockedDesign. Evidence pass (2026-09-17, `items-systems-advisor`) found dialogs 2517/4408/4409 are real unused rows in `dialog_set_maps.sql` (`dialog_set_id = 628`, `topic_text = 'Aftermath'`, alongside the two the Python fires) with verbatim reward text in `dialog_screens.sql`, but **zero DB wiring** from any of the three to an item (`dialogs.sql` shows `event_set_id = NULL` for all three; no `items_event_sets.sql` row references them) and no dead/commented branch in `Aftermath.py` to recover a selection rule from. Item-name matching: 2517 ("heavy weapon" + chest armor) has no candidate at all; 4408 ("deployment belt") matches too many tiers (4444, 7243, 7242/7244-7256) to pick one; 4409 ("Asgard hologram emitter") has one clean match (item 6843) but nothing ties it to mission 687. **Verdict: not scriptable from repository data alone.** Next step is a `game-archaeology-specialist` RE pass (Ghidra trace of the dialog-selection logic, or PAK/Lua reward-table extraction) to find concrete item ids and the archetype→dialog rule before any chain authoring — see [work-packets.md](work-packets.md#gc2). |
| D-CB06 (RE follow-up, 2026-09-17) | **Overturns the premise behind the diverged answer above.** `game-archaeology-specialist` found the raw Atrea node-graph source (`deprecated/data-scripts/scripts/missions/Castle_CellBlock/Aftermath.script`) the shipped Python was compiled from: a complete, unbroken graph (every node 2-25 present and enabled, no dead branches), with the original designer's own comments on the two branch nodes reading `"Human"` and `"JAffa"`. **This was always a deliberate two-way species split, not truncated cut content** — there is no lost per-archetype branch to recover, and dialogs 2517/4408/4409 were never wired to this graph at any revision we have evidence of (2517's low id suggests an early superseded draft). Client binary string search (Ghidra) found zero hits for any of this content, confirming BigWorld keeps it server-side with no client-side reward table to recover either. The one real, structurally-justified gap: `archetype<5 OR archetype==8` excludes archetype 6 (Goa'uld), and Goa'uld IS a Praxis-reachable starting archetype (`crates/services/src/base/chardef.rs`) — Asgard(5) and Sholva(7) are not reachable here at all, so they're moot. **A Goa'uld branch would be new design, not restoration** — no evidence exists anywhere for what it should contain. | GC2 | Checked whether that gap is even live in this repo: it isn't — Cimmeria's chains 1098/1099 already gate on `archetype neq 8`/`eq 8` (not the Python's `< 5`/`== 8`), the same pattern used everywhere else in the Cellblock seed, so Goa'uld already gets the Human-branch reward today. Presented to the user 2026-09-17; **decision reversed to close GC2 with no packet**, keeping the shipped two-branch mapping. See [work-packets.md GC2](work-packets.md#gc2) for the full resolution record. |
| D-CB07 | Camera-only Straegis scene (took the recommended default). | C08a → C08b | No change to C08b's scope as written. |
| D-CB08 | Resolved 2026-09-18 via RE precheck (live UAT unavailable): safe to implement. | C01 → C07 | `game-archaeology-specialist` decompiled the client's mission-accept path — `onMissionUpdate` unconditionally fires a generic, content-free HUD toast (token 0x1393, no mission-specific text) on every accept, already coexisting with the 2298 precedent's `display_dialog`. No double-prompt risk for 640/641/680/688. C07 moved to Ready; see [work-packets.md C07](work-packets.md#c07) for the full citation. |
| D-CB09 | Include the Castle-side Gerschon handoff now, as C09 (took the recommended default). | C01, C04 → C09 | **Superseded 2026-09-17:** the Gerschon handoff authoring itself was reassigned to a sibling Castle-zone campaign (as their CA01) to avoid two sessions both creating `castle_chains.sql`. This ledger keeps only a UAT check that the arrival lands correctly once CA01 ships — see [work-packets.md C09](work-packets.md#c09). |
| D-CB13 | **Diverged from the recommended default.** Full escort: Marsh follows the player and rings with them through the Escape sequence, not dialogs-only. | GC1 | GC1 stays BlockedDesign. This is the harder of GC1's three required children (b): `set_follow_target` wiring, a ring-hop teleport for the NPC, and a navmesh check on the topside floor — none of which the original Python had. The lockdown VFX child (c) remains BlockedEvidence regardless of this answer; no energy-field actor or Kismet event id has been recovered. See the GC1 evidence-gathering task in [work-packets.md](work-packets.md#gc1). |

## New Decision From The v3 Spec (2026-09-17)

A collaborator supplied a deeper spec, `SGW_Castle_CellBlock_Dev_Master_v3.xlsx`, which surfaced a genuinely new mechanic this repository's audit missed: every Castle_CellBlock player is forced to start wearing "Prison Boots" (item 3438) that lock movement (ability 1597/effect 1939) until a minigame clears them (ability 1598/effects 3081+1942) — confirmed independently against this repository's own DB seed, not just the spreadsheet's word. See [audit.md's new-evidence section](audit.md#new-evidence-from-sgw_castle_cellblock_dev_master_v3xlsx-2026-09-17) for the full citation chain. New packet **C00** in [work-packets.md](work-packets.md#c00).

| ID | Status | Contract and reason | Confidence |
|---|---|---|---|
| D-CB14 | PROPOSED | What minigame implements the Prison Boot removal. No recovered script names one — item/ability/effect/char-creation data confirms the gate exists, but the minigame type is unrecovered. Recommend reusing an existing SGW minigame (this repo already has Livewire wired for the cell-door hack at mission 638, `crates/services/src/minigame/`) rather than inventing new minigame infrastructure for a single tutorial beat. | MEDIUM: the gate's existence is HIGH confidence; which minigame implements it is a guess informed only by "reuse what exists," not evidence. |

Open question for the user: **does reusing Livewire (or another already-implemented SGW minigame) for the Prison Boot removal work for you, or do you want a different/simpler mechanism (e.g. a timed server-authoritative QTE) for this one-time tutorial beat?**

## Where Confidence Is Low Or A Guess

- Whether the duplicated 5xxx chains produce visible double dialogs in the client or whether the last `display_dialog` wins. The seed rows resolve; the UI outcome was not observed.
- What effect 1634 (Stasis Sickness) does when applied. It has no `script_name`; it may be a debuff icon and nothing else. Acceptable for a tutorial cue, but do not expect gameplay pressure.
- Cover set coverage for the med-station desk and the Mess Hall long table. The catalog has `_CA-CellBlock_Int00-15-15`; per-prop sets are unverified.
- Whether dialog 5019 was meant for Jaffa, for a later revision, or both. The spec could not resolve it and neither can the scripts.
- The 52 XP observation and any XP formula.
- Ring-hop handling for an escorting NPC if GC1 is approved: the ring FSM moves players only.
- Straegis Matinee playback semantics through `onSequence` (`viewType` for a camera Director track).

## Architecture Guardrails

Content lives in `resources.content_*` rows loaded at boot; the executor in [executor/mod.rs](../../../crates/services/src/cell/content/executor/mod.rs) is the only place side effects happen. A seed verb with no executor arm silently no-ops; check the catalog in [content-engine.md](../../content/content-engine.md#3-the-vocabulary) before authoring. Region keys are case-sensitive string matches against `point_sets.name`. Interaction bits are not persisted; every chain that sets one needs a `player_loaded` restore chain gated on the active step. Counter completion conditions read the pre-increment value (`gte target-1`) and increment chains run at priority 1. `accept_mission` refuses re-accept of an active mission, but dialog-set binding and `display_dialog` have no such guard.

Keep the 500/700 line caps. `castle_cellblock_chains.sql` is 1706 lines and is exempt as data, but new mission families go in their own seed file (`castle_chains.sql` for the Castle side). Chain-replay tests are one file per mission.

Mission steps that do not exist in the client PAK need a `MissionOverride` (see [mission-pak-overrides.md](../../architecture/mission-pak-overrides.md)); none of the planned packets adds a step.

## Agent Selection

| Area | Advisor roles |
|---|---|
| Chain authoring, mission state, dialogs | `mission-systems-advisor` |
| Cover objectives, NPC aggro, escort | `npc-ai-spawn-advisor`, `combat-systems-advisor` |
| Ability launch, effects | `combat-systems-advisor` |
| Despawn fan-out, sequences to witnesses | `aoi-witness-broadcast` |
| Cross-world arrival | `movement-teleport-advisor` |
| Regression strategy | `testing-validation-engineer` |
| Docs | `documentation-writer` |

Persona files may carry stale claims about the content engine; the executor arm list in the audit is current as of the baseline.

## Validation And UAT Gates

Tests must fail when the seed rows or executor arm are removed. Chain-replay tests assert exact resolved action lists for both the matching and the adjacent non-matching state. Executor arms need a unit test on the side effect. Live-DB tests use `require_db_or_skip!` and serialized execution.

| Milestone | User-assisted in-client acceptance; all pending |
|---|---|
| M1 Baseline repair (C01-C03) | Jaffa and Human each see exactly one Prisoner 329 topic dialog and one Marsh briefing; hallway controllers accept once; the pistol guard aggros on Region8 entry; the Stasis Sickness icon appears on load and disappears on cure; relog at each step. |
| M2 Tutorial objectives (C04-C06) | Frost's Letter appears in the log; cover indicator shows on vial pickup and hides on taking cover; step 2144 needs both objectives; flank objectives track. |
| M3 Narrative beats (C07, C08) | One prompt per accept; Straegis camera plays once, control returns, Marsh is gone, 2516 shows once; relog after the scene does not replay it. |
| M4 Boundary (C09, UAT-only) | Arrive in Castle near Gerschon with mission 1360 (Frost's Letter) still active. The dialog/mission-701 acceptance itself is the sibling Castle campaign's CA01 — this ledger only confirms the handoff lands correctly once CA01 ships. |

Pause at each milestone for the user. Record skipped scenarios explicitly.

## Handoff Validation Record

| Check | Recorded outcome |
|---|---|
| Spreadsheet read | All 21 sheets exported via openpyxl and read in full. |
| Repository evidence | Python space and mission scripts, both chain seed files, executor arm list, loader, chain-replay test listing, seeds for missions/abilities/effects/items_event_sets/char_creation/worlds/spawnlist/ring regions/point sets/cover sets, mission and dialog overrides, all read directly. |
| `tools/lint-md.ps1 --no-globs` on the three new files plus `docs/readme.md` | Exit 1: zero issues in the three new documents; the same five pre-existing MD012 blank-line warnings in the index that the parity handoff recorded. Index diff is the single added row. |
| Runtime, build, live DB, client UAT | Not run; documentation-only session. No commits, branches or runtime edits. |
