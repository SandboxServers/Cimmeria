# Legacy Command Parity Handoff

> Type: how-to. Audience: Claude Code coordinator and implementing engineers.
> Updated: 2026-09-16. Companions: [audit.md](audit.md), [work-packets.md](work-packets.md), [documentation index](../../readme.md).

## Purpose And Evidence Boundary

Use this document as the complete coordinator launch prompt for restoring the legacy emulator's dot commands. You do not need the originating chat. The [audit](audit.md) is a static source comparison, not recovered 2009 binary evidence, a client compatibility certification, or a Bible chapter. No runtime implementation, build, test, or in-client UAT accompanies this handoff. Larger subsystem designs remain pending user approval.

The source baseline is `6279fcfb53a7325ec9f00ff705e027b6e0d39ce0`, inspected on 2026-09-16 on `main`. Tracked files were clean before this documentation change; unrelated untracked `crates/services/logs/` existed and is excluded. The baseline comprises all 116 registrations in [ConsoleCommands.py](../../../deprecated/python/cell/ConsoleCommands.py), following their imported implementations. Rust registers 71 dots: 57 overlapping legacy names, 59 missing legacy names, and 14 Rust-only names. These are registration counts, not counts of working commands.

This handoff adds documentation only. It does not authorize commits, branches, worktree creation, builds, runtime edits, new agents, or subsystem implementation in the current documentation session. The execution protocol below is for the subsequent, user-authorized Claude Code implementation session. Obtain that authorization before provisioning worktrees or changing runtime code; ask separately before commits or branch publication.

## Coordinator Launch Prompt

You are the Claude Code coordinator for Cimmeria's legacy command restoration. Work in `C:\Users\Steve\source\projects\Cimmeria`. Preserve current working changes and the Rust-only seed/patrol commands. Implement the approved contracts below through small, reviewed packets, not one monolithic port. Do not broaden into all native slash commands, offline-player administration, cluster routing, or an engine rewrite.

1. First run `git rev-parse HEAD` and `git status --short --branch`. Record the revision, branch, tracked changes and unrelated untracked paths in the packet ledger. Compare the revision with this baseline; inspect only relevant changed paths before dispatching affected packets. Do not automatically repeat the entire audit.
2. Read [AGENTS.md](../../../AGENTS.md), [CLAUDE.md](../../../CLAUDE.md), [.github/copilot-instructions.md](../../../.github/copilot-instructions.md) and the applicable path instructions. Before writing tests, read [TESTING.md](../../../TESTING.md). The existing CLAUDE documentation-map rows for game systems, architecture, public command behavior and developer guides cover this work; no review-policy change is proposed here.
3. Read the decisions here and the [packet ledger](work-packets.md). Read audit rows only for the packets being scheduled. Keep the audit frozen as the point-in-time baseline; record implementation progress in the ledger, not by retroactively changing baseline statuses.
4. Confirm implementation-session authorization. Discover available Claude agents from [.claude/agents/](../../../.claude/agents/), not from VS Code's catalog. Read the relevant persona before using it. Existing persona metadata may select Opus and may contain stale source claims: select Sonnet explicitly where supported, or use an equivalent bounded role prompt on Sonnet. Do not create new agent files.
5. Launch the first Ready packet, P01, after assigning exact file ownership and its test window. Then launch newly unblocked packets in dependency order, using isolated worktrees and disjoint owned paths. Dependencies override the priority queue: quick wins, world authoring, missions, player administration, maintenance, then crafting/network diagnostics.
6. For every design gate, collect evidence and propose the named child contracts in [work-packets.md](work-packets.md). Record approval as a new decision ID before implementation. A design group is not an implementation packet. Approve and enumerate bounded children first, then dispatch them individually.
7. Integrate one packet at a time. Review code, tests, documentation, notes and handoff together, then validate the integrated result in the single build lane. Pause for the user's milestone in-client UAT. A green source check, successful enqueue, or registration entry is not command parity.
8. Continue with the next Ready packet only within the authorized scope. When blocked by policy, evidence, UAT or context budget, leave a durable handoff with the exact next action. Do not replace missing functionality with success text or delete a command to shrink the denominator.

## Approved Decisions

These IDs record the three completed user interviews. Append future decisions; do not silently rewrite these contracts. A superseding decision must name the old ID, rationale, approval and affected packets.

| ID | Approved contract and reason |
|---|---|
| D01 | Audit all 116 Python dot registrations and imported bodies. Native GM methods are reuse evidence only, not a 266-command slash campaign. Preserve all 14 Rust-only seed/patrol names. |
| D02 | Restore exact legacy dot spellings, argument order/defaults and selected-target intent. Correct legacy bugs rather than reproducing them. Keep the current Rust GM threshold, bounds and authoring model. |
| D03 | Use shared typed operations. Carry invoking GM identity separately from the subject entity/player, with caller feedback and subject state/UI. Never fabricate incoming wire packets or impersonate the GM by substituting subject IDs into caller-oriented native handlers. |
| D04 | Work priority is quick wins, world authoring, missions, player admin, maintenance, crafting/network. Required dependencies take precedence. Repair registered partial/no-op commands as well as missing names. |
| D05 | Named-player travel covers online players across loaded spaces on this service, joining the destination player's actual instance. Preserve failure, disconnect, spatial-index and area-of-interest (AoI) handling. Offline lookup and cluster-wide routing need separate approval. |
| D06 | Account for every command, including new functionality. Larger designs stay in the backlog until approved and split into executable child packets. Stubs are not parity. |
| D07 | `.removeitem designId quantity` removes the exact quantity across matching stacks atomically. Insufficient aggregate stock leaves every stack unchanged. This deliberately fixes the Python wrapper's instance-ID bug and differs from native instance-ID removal. |
| D08 | `.missioncomplete` marks completion without automatic catalog payout. `.missionrewards` provides guarded actionable selection/claim, not text preview. Eligibility, exactly-once, rollback, choice groups and completion ordering require design approval first. |
| D09 | Include selected-player `.kill`, in-place `.revive` and real `.god [enabled]`. Reuse canonical lifecycle and cover all relevant damage paths; health-only patches are insufficient. |
| D10 | Parallel writers use isolated worktrees with disjoint file ownership. One coordinator integrates one packet at a time. Shared registry, dispatch, message enums and shared docs are coordinator-owned or explicitly serialized. |
| D11 | One machine-wide Cargo/rustc/build lane across every worktree. Serialize live-DB tests too. Separate worktrees do not make concurrent Cargo builds safe. |
| D12 | Automated regression tests and milestone in-client UAT with the user are mandatory. Pause for UAT; current source-implemented commands are not runtime-proven. |
| D13 | Store decisions, worknotes, evidence, exact validation results, review and handoffs in the repository. Each worktree brings these artifacts to integration with its code. Chat and personal agent memory are not the handoff. |
| D14 | Give Sonnet one bounded behavior contract, a 4-8-file starting set and explicit stop conditions. Split expanding work, persist a handoff before the budget runs out, and never promise that compaction cannot occur. |

## Architecture Guardrails

The active dot path is [chat.rs](../../../crates/services/src/cell/chat.rs) through [console/registry.rs](../../../crates/services/src/cell/console/registry.rs) and [console/dispatch.rs](../../../crates/services/src/cell/console/dispatch.rs). The generic [commands registry](../../../crates/commands/src/registry.rs) is not the active services dot roster. Do not restore commands there. [gm/mod.rs](../../../crates/services/src/cell/cell_methods/gm/mod.rs) is a separate native method dispatch surface; method names are not proof of typed slash spellings.

Extract the smallest reusable checked operation from working code. Preserve validation and transaction boundaries, authority, event order, persistence and post-commit client synchronization. An operation that accepts a request is not necessarily one that has completed: propagate failure to the invoking GM and report real outcomes. Internal mission status is not automatically the client's wire status. State-field bit indices and combatant masks are different domains.

For client methods, verify the [dispatch table](../../protocol/client-method-dispatch-table.md) and [entity definitions](../../../entities/defs/). Use the transport abstraction required by repo instructions. An authoritative player snap uses `BASEMSG_FORCED_POSITION` through `build_forced_position`; `onPlayerTeleport` is a streaming hint, not the move itself. Use actual world IDs in wire data, not space/instance IDs.

For authoring, [seed.rs](../../../crates/services/src/cell/console/seed.rs) enqueues live SQL, writes a log and buffers the entry immediately. `seedconfirm` emits grouped SQL; it is not prewrite authorization. `seedcancel` clears the pending buffer; it does not undo the live DB, runtime state or log. Preserve this model while making correlated spawn results truthful. Re-read active schema and applicable seed instructions before future DB edits. Do not infer a new migration policy from old comments or force typed spawn-ID results through a rowcount-only SQL channel.

Keep the source caps: 500 lines soft, 700 hard. The console registry is already about 599 lines and entity handler about 499 at baseline. Use natural handler-family splits and preserve exports; do not grow another giant file. Read current sizes before allocating ownership. No unrelated cleanup, broad framework replacement or invented compatibility layer.

## Agent Selection

Use `rust-gameserver-dev` as default writer. The following existing Claude personas provide framing; availability and model override still need verification in the execution session. Do not spawn advisors merely to obtain a glossary or reread the audit.

| Area | Advisor roles |
|---|---|
| Items and durable state | `items-systems-advisor`, `database-persistence` |
| Mission transitions and rewards | `mission-systems-advisor`, `server-authority-enforcer` |
| Travel and visibility | `movement-teleport-advisor`, `aoi-witness-broadcast` |
| Death, invulnerability and threat | `combat-systems-advisor` |
| World authoring and pathing | `npc-ai-spawn-advisor` |
| Minigame debug adapter | `minigame-systems-advisor` |
| Privilege, subscriptions and logs | `network-security-auth`, `server-authority-enforcer` |
| Review and documentation | `testing-validation-engineer`, `documentation-writer` |
| Ambiguous wire evidence only | `game-archaeology-specialist`, with the relevant system advisor |

Persona files are not current-code evidence. In particular, old mission UPSERT warnings and broad claims that reload or minigames do not exist must be checked against the audit's concrete sinks. This campaign does not create new RE findings or Bible chapters. If a future implementation raises a canonical-spec conflict, record it and seek scope approval instead of silently expanding this audit into Bible work.

## Worktree And Build Protocol

The coordinator owns registry/dispatch integration, module exports, shared message enums, common test helpers and shared documentation by default. Workers may read those files but deliver a precise integration request in their handoff rather than editing them concurrently. Alternatively, grant a worker exclusive ownership for a serialized window and record the transfer in the ledger. File ownership, not subsystem labels, decides whether writers can overlap.

Provision worktrees only after implementation authorization. Record packet ID, worktree path, source base and exact owned paths before launch. Keep unrelated untracked logs out of every packet. Do not run cleanup commands against another writer's work or kill unknown processes. If Cargo/rustc is already active, identify its owner and wait for the lane to be released; do not start a second build.

Examples after P01 integration: P03's stat readouts and P05's progression adapter may overlap if their manifests exclude shared dispatch/tests; P09's spawn persistence and P19's mission readouts may overlap after checking base-message ownership. P13/P14/P15 must serialize while they share the entity handler; P38/P39/P40 must serialize while they share the net handler. Split ownership only along approved natural file boundaries. Designs can be read/reviewed while the build lane is occupied.

## Durable Artifact Protocol

Maintain the live status ledger in [work-packets.md](work-packets.md). Suggested future artifact paths are `worknotes/<packet-id>.md` and `handoffs/<packet-id>.md` within this folder. Do not create empty placeholders or dead links now. Once an artifact exists, link it from its ledger row and link back to the packet, decisions and evidence. Each packet has one bounded handoff report; append dated entries to its worknotes rather than scattering reports across chats.

Before implementation, each worker records:

- Packet ID, approved behavior contract, dependencies and applicable decision IDs.
- Source revision/base, worktree, exact read set and owned paths; shared-file integration requests are separate.
- Local hypothesis, evidence links, smallest discriminating check and scope exclusions.
- Planned tests with intended names/filters and the lane reservation; these are plans, not results.

During work, record decisions and discoveries before they become assumptions. Before context exhaustion, a blocked dependency, a handoff or integration, update the notes and produce the handoff with:

- Actual changed paths and source revision/base; behavior implemented versus remaining gaps.
- Exact validation commands, environment, exit codes, selected test counts and skipped tests with reasons. Mark unrun commands `Not run`, never `passed`.
- Regression proof: how the test fails when the fix is reverted, preferably the recorded controlled negative run. Do not revert other work to demonstrate it.
- State/persistence/client-recipient evidence, failure and rollback evidence, and unresolved wire questions.
- Reviewer identity/role, findings, resolutions and required documentation links.
- Client UAT status, scenario results or `Pending user session`; remaining blockers and the precise next action.

The coordinator imports notes and handoff with the implementation, updates status and decision links, and records integrated validation separately from worker validation. A design report includes alternatives, risks, approval question, and child packet manifests; it grants no implementation permission itself.

## Validation And UAT Gates

Tests must fail when the fix is reverted. Assert exact final state, IDs, recipient counts and bytes, not merely feedback presence or successful dispatch. Use the [TESTING.md picker](../../../TESTING.md): unit/command tests for parsing, live-DB for persistence and rollback, wire-format plus fan-out for client-visible changes, lifecycle/relog tests for durable state. Use existing fixtures and relationship assertions, not hardcoded seed IDs. Live-DB tests use `require_db_or_skip!`, exact `i32` sentinel cleanup, and serialized execution. The wireclient has only the capabilities currently implemented; do not assume full replay support from a planned API.

Future iteration uses `cargo check -p cimmeria-services` on Windows in the reserved lane. Run packet-specific tests, then the current CLAUDE pre-PR gates when preparing a PR. Workspace commands exclude `cimmeria-app`, `cimmeria-content-editor`, `cimmeria-scene-editor`, `sgw-launcher` and `cimmeria-client-telemetry`. Full server builds target Windows and copy the server executable to the root per CLAUDE. None of those runtime commands are part of this documentation session. Lint only touched Markdown with `tools/lint-md.ps1 --no-globs <paths>`; explicit paths alone still include the repository configuration's additive glob.

Each implementation packet updates the corresponding behavior/public-surface docs selected by CLAUDE, and preserves existing documentation indices. New frontend behavior also needs the repository's JS REPL-style logic UAT plus visual verification. Client wire or gameplay UAT is not a substitute for that rule when actual frontend code changes.

| Milestone | User-assisted in-client acceptance; all pending |
|---|---|
| M1 Quick wins | GM operates on a distinct selected player; target state/UI changes, caller receives feedback, observer sees only intended updates, non-GM is rejected. Exercise grant failure and readout accuracy. |
| M2 World authoring | Spawn, save twice, move/save, delete persistence while entity survives, despawn separately, autosave new entity, random interior placement, visibility off/on and observer re-entry. Check DB failure, server reload and stable spawn identity. |
| M3 Missions | Inspect hidden and visible active/history records; accept/advance/reset/fail/abandon/clear then relog. Complete without payout; approved reward offer/claim handles duplicates, choices and full inventory. Verify unrelated player isolation. |
| M4 Player administration | Player/NPC death and in-place revival, god mode across damage paths, abilities/TP/addresses/respawner durability. Travel within a space and across worlds, especially same-world different-instance destination, with transfer failure/disconnect recovery. |
| M5 Maintenance and diagnostics | Approved save/reload/log/debug behavior, subscription cleanup, inventory reconciliation and permission denial. Old state remains usable on failed reload. |
| M6 Crafting and network | Craft membership/paradigms/ASP after relog; bounded allcraft effects; timers, map markers, sequences, speech, dialog, DHD, time-of-day and debug minigame session/results on the intended clients. |

Pause at each milestone for the user's session. Record skipped/unavailable scenarios explicitly; do not mark the milestone complete without the required evidence. UAT requires a GM, distinct target, observer and non-GM where relevant. These are planned gates, not claims that a client has been launched.

## Completion And Remaining Decisions

Completion requires all 116 baseline rows resolved by implementation and verification, or explicitly retained as blocked work with user-approved disposition. Registration coverage alone cannot close the campaign. Preserve the baseline matrix and record progress separately so future reviewers can distinguish what existed from what was restored.

Open approvals are the G01-G14 design groups in the packet ledger: full respawn reload, privileged interaction policy, durable mission representation, rewards, cross-instance transfer, invulnerability, hidden addresses, respawner unlocks, maintenance reloads, logging, inventory rehydration, debug instrumentation, allcraft batching and minigame debug parameters. Packet-level numeric validation and lifecycle questions must be resolved against source and current safety bounds; escalate a changed user contract rather than guessing.

Documentation debt to revisit with the owning implementation: no-op maintenance explanations, spawn persistence promises, crafting membership semantics and mission durability comments currently overstate what the sinks do. Correct those alongside the relevant code, not through unrelated edits in this handoff.

## Handoff Validation Record

Documentation-session checks on 2026-09-16, against the source baseline above:

| Check | Recorded outcome |
|---|---|
| In-memory PowerShell source/matrix comparison | Exit 0: 116 exact unique legacy rows; 71 Rust registrations, 57 overlap, 59 absent and 14 Rust-only. Every row's Yes/No matches source and its packet explicitly names that command. |
| In-memory PowerShell relative-link, heading-anchor, ASCII and size check | Exit 0: all new-document links/anchors resolve; all three files ASCII and under 500 lines. No validation script was written to the repository. |
| `tools/lint-md.ps1 --no-globs docs/analysis/legacy-command-parity/README.md docs/analysis/legacy-command-parity/audit.md docs/analysis/legacy-command-parity/work-packets.md docs/readme.md` | Exit 1: zero issues in the three new documents; five pre-existing MD012 blank-line warnings in the index. The index diff contains only the requested added row; unrelated formatting is preserved. |
| `git diff --check` | Exit 0. |
| Runtime/build/live-DB/client UAT | Not run; documentation-only session. No commits, branches, new agents or runtime edits. |

The first exact-path lint invocation also loaded the configuration's repository-wide glob and exited 1 on existing findings. Subsequent invocations use `--no-globs` to select exactly the touched files; no unrelated files were fixed. These checks establish documentation integrity, not behavioral parity.
