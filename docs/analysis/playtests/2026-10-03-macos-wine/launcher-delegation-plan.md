# Delegate the remaining launcher implementation

> **Type:** How-to (execution handoff)
> **Audience:** The maintainer, Codex coordinator and a separate Claude Code session
> **Last updated:** 2026-10-04
> **Status:** Plan only. Implementation is paused; sending a worker its assignment starts that assignment.
> **Companions:** [Implementation requirements](launcher-implementation-plan.md), [ledger](launcher-implementation-ledger.md), [agent workflow](../../../agents/development-workflow.md), [desktop workspace](../../../../crates/launcher/desktop/README.md)

## Outcome and starting point

Deliver the approved dark-only, single-game launcher through complete user
journeys. Keep the full implementation plan as the specification; this document
changes execution order and ownership, not the required final scope.

The audited implementation base is
`c2e1c65d57c15dbec39c319ba529331b5c53cd38`, on
`prototype/launcher-packaging-proof`, draft PR #1164. At that base, installation,
prerequisite setup and uninstall have real implementations. Repair has substantial
backend coverage but no UI command. Game launch and the consented summary pipeline
remain incomplete. Do not infer readiness from the 294 passing engine tests.
Revalidate these dated facts before starting; documentation-only descendants can
be included in a newly recorded common base.

The root checkout has unrelated work. Use the launcher integration worktree and
new worker worktrees; never reset, stash, switch or clean the root checkout.

## What the repository guidance changes

- Isolate every writer in its own worktree. Read-only reviewers need no worktree.
- Assign one phase to each worker. Use fresh workers for review fixes; do not keep
  sending the implementation agent back through another open-ended cycle.
- Use a short explicit prompt plus relevant file pointers, not the entire chat.
- Workers report once at completion or when a decision blocks their assignment.
  No parked agents or repeated status messages. Hand off before roughly 200
  requests or 250k context tokens, per the repo guidance; those are upper bounds,
  not budgets to consume.
- Each worker delivers tests, corresponding documentation and a worknote under
  this campaign's `worknotes/`, with exact base/head commits and rerun commands.
- Use the build lane for every compiling Cargo call. Independent worktrees still
  share the machine-wide lane and disk budget. Do not change its slot count.
- Do not run the repo's automated merge/retirement path. This planning request
  authorizes no new implementation dispatch. After the user resumes implementation, the
  assignments allow implementation and draft PRs, never merging or publication.

The existing advisors are useful for review, but there is no launcher specialist
or dedicated SRE agent in the roster. Do not assume the server developer persona
knows Tauri, Wine or Effect. Consult `testing-validation-engineer` for test
validity, `network-security-auth` and `server-authority-enforcer` for ingestion
trust, and `documentation-writer` for docs. Their definitions live under
`.claude/agents/`; read the corresponding project memory too. Where Claude Code
runs these personas, preserve their declared `opus` model. In another harness,
read them as briefing material rather than claiming to have launched Claude Opus.
Some persona text describes legacy systems; current code and TESTING.md win.

## Skills to use selectively

- **codebase-design:** define small module interfaces and explicit ownership
  seams. Do not invent a new abstraction framework for existing repair code.
- **code-review:** after a worker reports, review its fixed base-to-head diff on
  separate Standards and Spec axes. Supply this plan and the implementation
  requirements; do not review the whole long-lived branch repeatedly.
- **diagnosing-bugs:** only for a reproduced launch/runtime failure.
- **research:** only for an unresolved external fact, such as runtime distribution
  rights or an ABI. Require a concrete question and a stopping condition.
- **tdd:** optional, not a blanket mandate. The installed skill requires agreed
  test seams before tests; use it only after that agreement. Repository test
  policy applies regardless, including JS logic UAT for frontend changes.
- **prototype/grilling:** not needed for this execution wave. The design is
  approved; reopen it only when a real implementation conflict requires a choice.

These skill names were reviewed in the coordinating environment. A fresh Claude
session must check availability rather than assume those skills are installed.
No skill installation is required to execute the assignments below.

## Team and waves

| Track | Owner | First complete outcome |
|---|---|---|
| L: Play | Codex coordinator | Real game launch implementation with observed lifecycle, then Play UI |
| R: Repair | One Codex implementation worker | Settings confirmation through retained repair, cancellation and explicit supported recovery |
| O: Observability | One separate Claude Code session | A consented operation summary travels through a bounded queue to local ingestion and an operator query fixture |
| M: Migration/updater | A short-lived read-only worker, then a later implementer | Evidence-based parity inventory and a bounded implementation assignment |

After the user resumes implementation, start R and O after their ownership
records and common base are written. The
coordinator works on L's engine while R owns the existing frontend integration
files. Run M briefly as capacity permits; it must not become a fourth continuing
implementation project. This Codex session has four total agent slots including
the coordinator; an external Claude session is separate, but shares host build
resources if it runs here. M must finish before review. The coordinator launches the two code-review axes
directly after an implementation worker exits; do not nest a review coordinator
that consumes the slot needed by its second reviewer.

Wave 1: R implements the Repair journey; O first delivers a bounded interface-discovery handoff, then a fresh Claude
implementation session implements local end-to-end telemetry;
L implements launch mechanics without editing R's frontend files. M reports and
exits. Wave 2: review/integrate R, release its file ownership, then L connects Play
and O's consent/operation hooks are integrated. Wave 3: finish migration/updater,
visual polish, and the final packaging gates. Stop at a human gate only for that
part of the task; continue independent authorized work.

## File ownership and interface agreement

Before dispatch, record each worker's branch, absolute worktree path (locally,
not in public docs), base SHA and exact allowed files in its prompt. Ownership
applies across the team even though worktrees prevent filesystem collisions.

- **R owns:** desktop `shell/src/host/repair/` (new), repair-focused frontend
  modules/tests, `frontend/src/install-workflow.ts`, `install-view.ts`, their
  tests/UAT, and `frontend/ui/index.html`. It may edit `shell/src/host.rs`,
  `host/install/mod.rs`, `host/install/contract.rs`, and `shell/src/main.rs`
  for Repair wiring. Corresponding `desktop/docs/repair.md` belongs to R.
- **L owns:** new launch-focused engine/helper modules and their tests. Existing
  `crates/launcher/src/worker/launch_sgw.rs`, `start32_helper.rs`,
  `crates/client-launch/` and current runtime code are reuse sources; changes
  there need a concrete compatibility reason. No parallel edits to R's files.
- **O owns:** new launcher-summary modules/tests, summary-specific server route
  modules/tests, operator query fixtures, and the telemetry architecture/ops
  documents. It first reports the exact existing server registration files it
  needs, then the coordinator records exclusive ownership. No game telemetry
  pipeline replacement or unrelated observability cleanup.
- **Coordinator reserves:** Cargo manifests/lockfiles, module registration files
  outside R's list, workflow files, desktop README, campaign ledger, docs indexes
  and the main-session memory index. Workers isolate necessary edits to these files in a separate integration commit
  with an explicit file list and SHA. The coordinator reconciles these commits
  serially and includes matching docs/index changes with integration.

A worker may edit reserved files in its isolated worktree to compile. Before
shipping, stage only those exact files and commit them separately, then use
ship.sh for the remaining owned changes. Report both SHAs in dependency order.
The coordinator integrates the complete chain once; never also apply a duplicate
patch exported from those commits. A reserved-file conflict is reconciled against
the current integration branch, never resolved by copying a worker file wholesale.
No worker may overwrite another track's contract. Workers may create uniquely named project-memory entries in their own
worktree; shared indexes are integrated serially.

Agree on operation identity/revision, terminal result semantics and the summary
submission/consent interface before O connects hooks. R reuses existing Repair
state meanings. L reports process-started/exited/unknown without claiming login.
No worker creates a second operation state machine or exports arbitrary errors.
O interface discovery is its own phase: return the proposed contract and exact
files, then stop. The coordinator records the decision; a fresh implementation
worker receives it. Later unexpected interface conflicts end with one bounded
handoff rather than a parked worker or repeated redesign loop.

## Worktree and draft-PR mechanics

The current `mk-worktree.sh` assumes Windows tools and starts from `origin/main`.
It does not prepare these Mac feature-based worktrees correctly. The coordinator
creates them with native git from an explicit common base. Example, run from the
existing launcher integration worktree after checking its status:

```bash
LAUNCHER_REPO_ROOT=$(git rev-parse --show-toplevel)
LAUNCHER_COMMON_DIR=$(git rev-parse --path-format=absolute --git-common-dir)
LAUNCHER_WORKTREE_PARENT="$(dirname "$LAUNCHER_COMMON_DIR")/.claude/worktrees"
LAUNCHER_BASE=$(git rev-parse HEAD)
git -C "$LAUNCHER_REPO_ROOT" worktree add -b launcher/repair-ui \
  "$LAUNCHER_WORKTREE_PARENT/launcher-repair-ui" "$LAUNCHER_BASE"
git -C "$LAUNCHER_REPO_ROOT" worktree add -b launcher/journey-observability \
  "$LAUNCHER_WORKTREE_PARENT/launcher-journey-observability" "$LAUNCHER_BASE"
```

Do not run this block as part of reading this plan. Check for existing branches
and worktrees before creating anything. Populate only dependencies required by
the desktop README; do not run broad Windows setup scripts on the Mac. A Claude
session on another machine fetches the exact base and uses its own worktree;
never copy local paths or secrets into its prompt.

`ship.sh pr` stages all changes and creates a PR against `main` by default. Each
worker uses an explicit title/body and `--draft`, then retargets the draft to
`prototype/launcher-packaging-proof` and verifies the base/head. This briefly
creates a broad draft; do not review that inherited diff or merge it. If repository
policy forbids that transient base, let the coordinator ship instead of changing
the tool speculatively. The coordinator reviews only `BASE..WORKER_HEAD` and
integrates approved commits into #1164 serially, respecting dependency order.
No routine rebase onto main, force push, merge, or worktree retirement. Leave
worker branches intact until integration is verified and cleanup is authorized.

## Shared prompt: prepend to every implementation assignment

```text
Implement only the assignment below in your assigned worktree.
Base: <exact SHA>. Worktree: <absolute path>. Branch: <branch>.
Allowed files and reserved integration files: <copy the ownership record>.
Read AGENTS.md, CLAUDE.md, TESTING.md, docs/agents/rules-and-gotchas.md,
docs/agents/domain.md, docs/agents/development-workflow.md,
docs/agents/doc-update-map.md, and relevant project memory.
Read docs/analysis/playtests/2026-10-03-macos-wine/launcher-delegation-plan.md
and docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md.
Inspect actual code; historical notes are
not completion evidence. Check the worktree and base before editing. Use git -C
with your absolute worktree path; stop if it disappears. Do not touch root work.
No desktop UI control/opening without explicit user permission. Leave WireGuard
off; no live observability probes, production changes, merge, deploy or release.
Every compiling Cargo call uses tools/build-lane/lane.sh. Windows artifacts build
on Windows; the standalone desktop workspace can build natively on Mac.
Keep Effect as real orchestration. Preserve consent; launcher summaries never
implicitly enable game telemetry. Final self-contained startup validation is last.
Deliver a complete assigned journey, meaningful tests, corresponding docs and JS
logic UAT where frontend behavior changes. Record what visual/native/human tests
were not run. Do not broaden the architecture or add speculative recovery modes.
Write worknotes/<track>.md under this campaign: base/head, changed files, outcome,
exact commands/results, reserved-file patches, remaining blockers and next action.
Commit/push with the repo shipping workflow and draft/base safeguards in this
plan. No merge. Report once with your final handoff, then stop. If blocked, report
the smallest decision needed and do not repeatedly retry an unchanged condition.
```

## Assignment R: copy after the shared prompt

```text
Finish Repair as a usable Settings flow using the existing engine repair code.
Read crates/launcher/desktop/docs/repair.md and
crates/launcher/desktop/engine/src/storage/repair/ first.
Wire confirmation showing the installed directory, fresh operation admission,
retained preparation-to-commit ownership, real progress and precommit cancellation.
Connect explicit supported recovery/abandonment and current-backup cleanup with
clear consequences. Unknown helper outcomes remain gated with actionable text.
Keep the old game until replacement succeeds. Missing game content must still be
repairable from saved installed identity. Do not rebuild the repair engine or
solve historical stage cleanup in this assignment. Do not enable Play based on
Repair success. Settings and consent must remain usable where safe.
Acceptance: real native command/state tests; actual Effect workflow logic UAT for
confirm/dismiss, duplicate clicks, cancellation, lost reply/reopen, recovery and
unchanged consent. Exercise native persistence, not only hand-built JS snapshots.
Document real-Wine versus fixture coverage and visual UAT still required.
```

## Assignment O: Claude discovery, then a fresh implementation session

First send this bounded discovery prompt, not the implementation prompt:

```text
Read-only discovery for launcher observability. Base: <SHA>. Worktree: <path>.
Read AGENTS.md, CLAUDE.md, docs/agents/development-workflow.md and the telemetry
section of docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md.
Inspect crates/launcher/src/telemetry/{endpoint,auth,queue,chunk,install_result}.rs,
crates/admin-api/src/routes/telemetry/ and the auth route located by scoped rg.
Use docs/architecture/dev-session-telemetry.md, docs/operations/telemetry.md,
and docs/architecture/instrumentation-discipline.md as current design references.
Verify paths before reading; do not assume historical examples are authoritative.
No edits, builds, app opening/control, WireGuard changes, live probes or push.
Return a proposed typed summary/consent interface, exact registration/hook files,
minimal auth scope, ownership list and acceptance-test seams. Cite source paths.
This is one discovery phase; report once and stop. Do not implement or park.
```

After the coordinator records that contract, send the shared prompt plus the
following assignment to a fresh Claude implementation session. Include the actual
agreed contract and file list, not merely a reference to the old conversation.

```text
Deliver focused launcher operation observability through local ingestion.
Read the implementation plan's telemetry contract and the existing launcher
telemetry, server ingestion/replay, auth and operator examples. Consult the
network-security-auth and server-authority-enforcer definitions and memories;
validate their advice against current Rust code, not legacy OpenSSL descriptions.
Implement one typed safe projection, a bounded persistent acknowledged queue,
TTL/retry/drop handling, and immediate launcher-export opt-out. Provide local
launcher-only auth/ingest validation, deduplication and operator query fixtures.
Demonstrate a setup failure reaching local ingestion without launching the game.
Use the plan's budgets as starting limits; document any justified changes.
Implement the attached agreed operation-summary/consent interface and server
registration ownership. If it is contradicted by code, return one bounded blocker
handoff; do not silently redesign or remain parked awaiting a decision. Keep export failure off the install/Play
critical path. No raw paths, logs, tokens, account names or hardware identifiers.
Test consent races, restart, TTL, duplicate requests, malformed/oversized input,
auth scope, endpoint outage and failure isolation. Never use the live stack.
Record observed opted-in cohorts honestly; no all-player funnel or login SLO.
Deliver docs/query fixtures and an integration patch for actual operation hooks.
A queue or endpoint alone is not completion: the local producer-to-query fixture
must work, and any production hook still awaiting integration must be explicit.
```

## Assignment L: coordinator's bounded implementation brief

```text
Complete native launcher game-start mechanics first, then the Play UI after R
releases shared frontend files. Reuse Windows launch preparation, suspended
process/injection/resume semantics and current managed runtime evidence.
Resolve actual D3D9/x87/prerequisite/distribution gaps from code and artifacts;
do not infer readiness from DLL loading or prerequisite tests. Track host and
guest lifetimes, duplicate launch, cancellation/early exit and uncertain outcomes.
Do not drive a visible game or desktop without explicit permission. Headless
contract tests are allowed but do not prove rendering. Deliver an exact human
launch/login/world checklist and stop at that gate, not at merely compiling.
```

## Assignment M: standalone read-only prompt

```text
Read-only audit. Base: <exact SHA>. Worktree: <absolute path>.
Read AGENTS.md, CLAUDE.md, docs/agents/development-workflow.md and
docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md,
then current desktop implementation and existing Windows configuration, identity
and self-update code. No builds, edits, push, app opening/control, WireGuard
changes, live service probes or tool installation. Produce a concise migration and
updater parity inventory with source paths, proven gaps, ownership conflicts and
acceptance tests. Do not edit runtime code, install tools or launch applications.
Propose one bounded next implementation assignment and return once. Return the report to the coordinator, who saves it as campaign
worknotes/migration-audit.md. Do not keep running after reporting.
Preserve existing identity and explicit consent; choose a single updater owner.
```

## Review, integration and anti-repeat rules

A fresh reviewer gets the exact worker base/head and assignment, not the chat.
Use the code-review skill's independent Standards and Spec reports when available;
otherwise reproduce those two review questions explicitly. Test validation asks
whether the new guard catches the stated failure, not whether the count increased.
Do not run whole-branch reviews or full suites after every small edit. Run scoped
checks during implementation, then integrated checks for the final changed surface.

The coordinator accepts a track only when its stated journey works through the
intended interface or its exact external blocker is demonstrated. A successful
command, a mock status or an unchanged screenshot is not equivalent evidence.
Fix review findings with a fresh scoped worker. Docs/index integration accompanies
code; shared project-status/generated blocks remain untouched.

At each wave boundary update one checklist: delivered user behavior, evidence,
remaining requirement, next owner. No consecutive backend-only packets without
naming the specific blocker they remove and the next user-visible acceptance test.
Do not expand a packet to cover every conceivable crash. Keep necessary safety
gates, record unsupported cases, and move to the next required journey.

Release gates remain: final Windows-native verification, human Mac graphics/login/
world UAT, migration/updater parity, visual polish, signing/notarization and clean-
machine/self-contained startup LAST. No worker may declare the full goal complete
from its own successful assignment. The coordinator performs the full requirement
audit after integration; no merger or release is authorized by this plan.
