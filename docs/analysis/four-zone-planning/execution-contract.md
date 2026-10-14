# Four-zone Haiku campaign execution contract

> **Type:** How-to
> **Audience:** Campaign coordinators, implementation coders and reviewers
> **Last updated:** 2026-10-14
> **Companions:** [planning index](README.md), [Cellblock packets](cellblock-work-packets.md), [CB-N05a seal](cb-n05a-seal.md)

Status: planning contract only. This document governs four future campaigns; it does not dispatch them. The owner has chosen a campaign feature branch with wave integrations, local CI-equivalent checks and review before one remote PR per complete campaign. The `campaign-packet` skill's rule is one PR per packet; this contract deliberately overrides that rule for these four campaigns. The single exception is CB-N05a, which ships as its own early PR to `main` (owner decision, 2026-10-13; see [the catalog prerequisite](#client-mission-catalog-prerequisite)). Repository merge-queue and lab-lease rules still apply.

## Ledgers

Owner decision, 2026-10-13: each campaign's live ledger is a new directory created at dispatch, `docs/analysis/cellblock-continuation/`, `docs/analysis/castle-continuation/`, `docs/analysis/harset-continuation/` or `docs/analysis/tollana-continuation/`. Its `README.md` holds the packet status table, `worknotes/<packet>.md` holds each packet's handoff, and `RESUME.md` holds the wave-boundary snapshot. The older `docs/analysis/castle-rebuild/` and `docs/analysis/harset-rebuild/` ledgers stay as historical sources; the continuation ledger cites them and does not rewrite their history. No per-zone board category is required. Chain IDs come from the shared [ID allocation table](id-allocation.md).

## Budget and worker brief

- Give a fresh `packet-coder` one packet, one registered worktree, one test database and one bounded file allowlist. A hook caps Haiku workers at **90k tokens of total context**, about 35–40k of which is fixed startup overhead, so target **about 40k working tokens** for the coder's implementation, test and final report (`docs/analysis/campaign-metrics/README.md`, "Packet sizing vs the 90k Haiku cap"; `findings.md`). Owner decision, 2026-10-13: the coordinator checks each packet against this budget **at seal time**, when its allowlist, IDs, tests and source excerpts are frozen. A sealed contract that would not fit splits into children before dispatch; a fix round that would not fit becomes a new correction packet rather than a resumed worker. The coordinator checks token-profile data after each stage. Three review rounds remain a quality gate; if their cost cannot fit, split the remaining correction into a new packet rather than declaring an issue resolved. Never resume an expired worker to save a report; use its commit and worknote in a fresh session.
- The brief names its exact section in the zone `*-work-packets.md`, base SHA, source evidence, branch/worktree, owning files, preconditions, behavior, negative cases, test type, revert proof, telemetry events, lane commands, docs, commit subject and stop conditions. Copy the 5–10 relevant repo rules into the brief for a Haiku coder that omits `CLAUDE.md`; do not ask it to discover the architecture anew.
- The frozen manifest assigns **each quest beat and encounter** a provenance class from [the index](README.md): `RESTORED_SOURCE`, `CIMMERIA_RECONSTRUCTION`, or `CIMMERIA_AUTHORING`. It records the exact source file/ID/build if known, the reconstructed or invented portion, and the mission/lore advisor's approval. A historic ID with a newly invented step is mixed provenance; mark that step reconstruction or authoring explicitly. Client text and public docs must not present either as a verbatim 2009 restoration.
- The worker writes `worknotes/<packet>.md` in its zone's continuation ledger (what changed, evidence, tests, telemetry events, decisions, risk, commit SHA) and returns one line. A coordinator reviews the result and tracks actual token use. A scope that needs a new primitive, client patch, map design, unreserved ID, incompatible historical revision or more than about 40k working tokens is split or returned to an architect/domain advisor; Haiku does not decide that design in flight.
- Never hand a small model a research gate as if it were an implementation packet. `game-archaeology-specialist`, domain advisors, `documentation-writer` and the architect own evidence, prose and decisions. Packet-coder receives a frozen contract only after those outputs are incorporated.

## Measured pilot before larger waves

Before dispatching a 2–4 packet implementation wave, complete one evidence/contract packet and one small, sealed implementation packet through integration and fresh review. Pick the implementation packet from an already supported route or recovered story beat; do not make a new authored quest or repeatable reset the first measurement. In their continuation-ledger worknotes, record the scoped estimate and actual working/context tokens, elapsed implementation time, lane/test time, review and fix rounds, and any evidence or UAT dependency that prevented completion. Compare those measurements with the 40k target and revise packet slices and wave size before dispatching the next implementation wave. The 65 families are a scope map, not a throughput or delivery-time estimate. This pilot uses the normal packet, test and review gates; a required client check remains `UATPending` until the owner approves a lab lease and the check runs.

## Common completion contract

Every packet in every zone meets these in addition to its own acceptance:

1. A runtime change carries a regression test that fails when the change is reverted, picked with [TESTING.md](../../../TESTING.md).
2. **Telemetry (owner decision, 2026-10-13).** Every chain, mission or encounter packet emits progress telemetry sufficient to debug the content from SigNoz alone: the chain that fired, each step advance, each objective credit, and every refusal with its reason (wrong state, wrong faction, wrong world, already completed, missing item). Reuse the existing content-engine events where they already cover a case. The packet's acceptance names the telemetry events a reviewer should see, and its worknote records them.
3. Seed changes go under `db/resources/`, reload the worktree's own database, and carry a `live_db` guard where a join or persistence rule changed.
4. User-visible docs change in the same packet; status docs do not (see [Campaign completion](#campaign-completion-and-remote-pr)).

## Branch and wave topology

For each zone `<zone>`, the coordinator creates a dedicated integration worktree on `content/<zone>-campaign` from current `origin/main`, after auditing current worktrees, branches, board and old ledger. Independent packet workers branch from the integration branch at a recorded SHA: `<zone>/<packet>-<slug>` in `.claude/worktrees/<zone>-<packet>`. One worker owns a file at a time. Each worker commits locally and stops; no worker pushes, opens a PR, merges into main or edits another packet's worktree. The integration coordinator merges reviewed packet commits into `content/<zone>-campaign` in dependency order and records the integrated SHA in the ledger.

`tools/build-lane/mk-worktree.ps1` always creates its new branch at `origin/main`, regardless of the campaign SHA. After creating a packet worktree, verify its branch is the requested new branch, its worktree is clean, and its HEAD equals the recorded `origin/main` SHA. Then run `git -C <packet-worktree> rebase --onto <recorded-feature-sha> origin/main <packet-branch>` on that empty branch. Verify `git -C <packet-worktree> rev-parse HEAD` equals the recorded feature SHA and `git -C <packet-worktree> merge-base HEAD <feature-branch>` equals that SHA before dispatch. Stop if any check differs; never use a destructive reset to repair an unexpected worktree. Record the SHA and command in the ledger. Repeat the base check for every later wave, since the helper never inherits the campaign tip automatically.

Packet worktrees are never pushed, so `rm-worktree --merged` does not see them as merged and will not retire them. Retire each packet worktree with `pwsh tools/build-lane/rm-worktree.ps1 --force <name>` as soon as its commit is merged into the campaign branch; that also drops its test database. The merge queue squashes the final campaign PR into one commit on `main`, so per-packet history survives only in the worknotes. Write them before retiring the worktree.

Packet waves are small enough for a reviewer to understand in one fresh context, generally **2–4 implementation packets**, fewer when they share seed tables or content-engine dispatch. Finish the evidence/contract wave before runtime waves. Keep one feature branch for the campaign across waves, and do not publish a remote wave PR. At every wave boundary write `RESUME.md` in the zone's continuation ledger (at most 40 lines: base SHA, integrated packets, worktrees, current checks, next action, blockers), update the packet status table, and compact/restart agents rather than carrying a large stale context.

Status vocabulary:

| Status | Meaning |
|---|---|
| Ready | Contract sealed, dependencies integrated, budget checked; can be dispatched. |
| BlockedDependency | Waits on another packet or campaign (named) to integrate or merge. |
| BlockedDecision | Waits on a named owner decision. |
| BlockedEvidence | Waits on a named source fact (ID, coordinate, build lineage, client behavior) that a specialist must establish; a coder may not guess it. |
| Writing | A worker holds the packet. |
| Review | Commit made; packet or wave review in progress. |
| Integrated | Merged into the campaign branch and passed the wave gate. |
| UATPending | Integrated; client UAT not yet run or not yet approved. |
| Done | Merged to `main` with required UAT recorded honestly. |

## Review and local CI gate for each wave

1. Inspect each packet's commit against its contract. Confirm its revert-proof regression guard, telemetry events, docs and ID/coordinate ownership. Give any client-data mutation to `server-authority-enforcer`, movement to the movement advisor, mission/content to the mission advisor, item paths to the items advisor, and other domains to the relevant advisor. Reviewers are fresh and read-only.
2. Integrate the packet commits on the campaign feature branch. Reload **that worktree's** seed into its isolated `sgw_<worktree>` database using `seed-change` procedures. Run local CI-equivalent checks in order: `cargo fmt --all -- --check` (non-compiling), `pwsh tools/build-lane/lane.ps1 cargo clippy ... -- -D warnings` for changed crates, targeted nextest through the lane, full workspace nextest and live-DB tier through the lane before a wave is marked Integrated. Use the exact current pre-PR checklist and exclude syntax from `CLAUDE.md`; never call compiling cargo directly. Run seed lints, doc/Markdown checks and client override validation that the touched files require. Capture commands, exit codes, run IDs and counts in the ledger. A green packet worktree is not a substitute for the merged feature branch check.
3. Give the **whole integrated wave diff** and packet specs to a fresh read-only adversarial reviewer. Verify findings, then assign every accepted fix to a **separate fixer agent** working on the feature branch (or a fix worktree merged back before retest). The original coder does not fix review findings. Rerun affected targeted checks and the wave gate. Send the resulting full diff to a **new fresh reviewer**. Repeat for at most **three review rounds** total; stop early when a reviewer finds no actionable issues. A disputed or still-open finding after round 3 blocks the wave and goes to the architect/owner with evidence; it does not become an automatic pass. Record reviewer identity/session, reviewed SHA range, findings, fixer commit and recheck results for every round.
4. After the wave passes, mark its rows Integrated or UATPending. Never mark a player-facing story Done solely from seed tests. Client UAT requires the owner's explicit approval before any lab lease or client/server change; list approved and pending rows separately. Do not claim local CI covers client appearance or dialog/journal presentation.

## Campaign completion and remote PR

After **all planned active waves** are integrated, run the full local pre-PR suite and seed reload again on the final feature-branch tip. Have a fresh campaign-level reviewer inspect the entire diff, cross-zone handoffs, test inventory, provenance labels, telemetry events, doc-update map, memory and close-out rows. Apply the same fixer → fresh reviewer loop, at most three rounds, and block on unresolved issues. UAT rows that cannot run without a lab lease remain explicitly UATPending; the PR states that limitation. The zone close-out packet (CB-N08, CS-07, HA-19, T-14) updates **only its own continuation ledger**, the content docs its packets changed, and `docs/guides/unified-uat.md`; it does not touch the shared status docs.

Only then use `tools/build-lane/ship.py pr` from the registered campaign worktree to push and open **one campaign PR** against `main`, with its attribution footer, evidence, checks and UAT limits. Attach the PR to the task. Wait for required PR checks and merge via `ship.py merge` and the merge queue; do not bypass CI or use an administrative merge. Fix any remote CI/review failures on the same feature branch, repeat local checks and fresh review of the changed range, and push again. Retire the integration worktree after merge (packet worktrees are already retired), preserve worknotes and final handoff, and mark Done only when the remote merge and required UAT state are honestly recorded.

## FZ-99 — shared four-zone close-out

Owner decision, 2026-10-13: one final packet, **FZ-99**, updates the once-per-campaign status docs for all four zones together. It is not a zone family and is not counted in the 65.

- **Owner:** `documentation-writer` with `testing-validation-engineer`; no code.
- **When:** after the last of the four zone campaign PRs merges, or earlier for any zone the owner closes separately (FZ-99 then runs once per closed zone set, never per packet).
- **Files:** `docs/gap-analysis.md` and the affected `docs/gap-analysis/` area files, `docs/project-status.md`, and `docs/content/zone-audit.md`. Its Tollana section (from about line 591) is stale: it says Tollana has no `.nav` file and zero missions, but `data/spaces/tollana.nav` exists and the 51-record census is seeded. Rewrite each zone's section from its continuation ledger. Recount the [ID allocation table](id-allocation.md).
- **Acceptance:** every status claim cites a zone ledger row; UATPending rows stay pending; `git diff --check` and `pwsh tools/lint-md.ps1` (warn-only) pass. It ships as its own docs PR.

## Cross-campaign order and ownership

Cellblock escape and Castle share one story boundary; Castle may return while Cellblock stays one-time. Harset and Tollana share mission handoffs. Freeze a single owner for each cross-zone chain and let the receiving zone test the handoff without duplicating it. The four campaigns may plan in parallel, but implementation waves that depend on another campaign's unmerged contract wait. No new authored quest is labeled as shipped 2009 content. Side stories and repeatables follow restoration UAT, use the [story source guide](story-research.md), and begin with useful supplies and modest XP/currency.

Single owners fixed by the owner on 2026-10-13:

| Cross-zone item | Owner | Receiving side |
|---|---|---|
| Chain 1109, Cellblock→Castle ring arrival | Ring-transport campaign (`docs/analysis/ring-transport-cellblock-castle/`) | Cellblock and Castle list it as an external dependency; the current `cross_world_teleport` stays as fallback. |
| Missions 1360 and 567, Castle legs | Castle CS-08 | Harset owns its own legs; without CS-08 only 31 of Harset's 33 active records are completable. |
| Mission 1664 | Owner: Castle the opening/exit beat, Harset the debrief. Proposed mapping (CS-00/HA-00b confirm): Castle CS-06 grants/accepts 1664 at the Castle exit; Harset HA-01b owns steps 5224, 5225 and 5226 | Each tests the other's handoff. |
| Tollana turn-ins 603, 611, 618, 621, 1626, 1632, 1643 | Tollana, both ends, chains in 9800–9999 | Harset HA-00b signs off the NPC tags. |
| Missions 1364, 1366 | Tollana T-05a/T-05b | Harset records the interface in HA-00b. |
| CB-N05b Castle turn-in of the Cellblock story | Cellblock | Castle adds no duplicate recipient. |

### Client mission catalog prerequisite

Current `crates/resources/src/base/resources/apply_overrides.rs::apply_mission_overrides` skips IDs absent from `CookedDataMissions.pak`; a server seed and `MissionOverride` alone cannot make a net-new quest visible. **CB-N05a** must insert and serialize new mission XML through the **existing ResourceCache full-category resync path** (category metadata, complement-version opening reply, every entry via `resourceFragment`, served-version closing reply). Both replies use empty `InvalidKeys` and `RequiredUpdates=0`; the override-ID list does not select entries. The [CB-N05a seal](cb-n05a-seal.md) freezes the complete-entry API and test-only fixture. Owner decision, 2026-10-13: CB-N05a ships as an **early standalone infrastructure PR to `main`**, branched from `origin/main` rather than the Cellblock campaign branch and merged before the rest of the Cellblock campaign. Once it merges, Castle, Harset and Tollana authored-quest packets may be sealed against it; until then they are `BlockedDependency`. Implement the first authored quest as a bounded client-catalog proof; hold additional authored-quest implementation until that quest's journal, dialog, step/objective advance and relog behavior pass live-client UAT. The lab lease requires the owner's approval. If approval is unavailable, record `UATPending` and continue independent restoration work. No authored quest is Done until the proof exists. Preserve existing mission overrides and resource version behavior; no zone worker invents a second distribution path.
