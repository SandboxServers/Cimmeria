# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [path display design](design-path-display.md), [startup update design](design-startup-update.md), [UAT](../../guides/unified-uat.md#launcher-uat-fixes)

**Baseline:** Original U1-U5 (#1984 through #1988) and close-out #1989 are merged.

| Task | State | Next action |
|---|---|---|
| Direct-root adoption + footer | PR #1992 merged `16ff568b8`; worktree retired | Complete; owner reports adoption works in 2026.10.2302; remaining AIP-UAT rows stay pending |
| Repeated signed releases + content logs | PR #1991 merged `7be8413b0`; worktree retired | Complete; see repeat-releases validation worknote |
| Path display + startup mandatory update | One bundled PR requested; implementation in progress | Finish regression/mutation tests, frontend REPL/build, native checks, review, CI and ship-pr |

**Current worktree:** `.claude/worktrees/launcher-display-paths`, from current origin/main.
**Design:** display-only path formatting preserves saved/native spellings. Startup inspects/checks once; known updates block Play in frontend and configured native admission. Disabled builds ignore inactive offers; active/reconciliation work blocks all builds. One main-page Update and restart action prepares/applies; startup never mutates packages automatically. Saved Ready skips download. No-offer check failures permit Play with feedback; known-offer failures retain the gate.
**Evidence:** frontend 264 passed; Windows engine 973 passed / 3 ignored; standalone workspace all-target clippy passed. Native-backed JS path persistence, signed-feed startup, isolated Windows main-page Apply swap and adoption-footer logic UAT passed. Browser mock at 1280×720 shows the update action, disabled Play, clean Settings path and immediate Updating feedback. Mutation proofs: reverting display formatting fails 7 tests; frontend Play gating 4; startup check 3; check-only retry 2. Native admission/retained-offer/disabled-build mutations fail as required. Prior final Windows build and shell nextest 148/7 passed; configuration-aware final rerun and review 3/CI pending.
**Pending validation:** native mutation proofs, final shell tests/native build, fresh review and CI. These fixture/browser results do not attest actual launcher replacement/restart, packaged IPC/layout or a live game.
**Approvals:** both supported hosts adopt at selected root; explicit per-file changes/backups allowed, bulk relocation/copy prohibited. D-LX15 approved. No new owner decision identified. A real release requires explicit owner GO.
**Limits:** no new packaged picker/layout, actual restart, live game, Wine or SigNoz acceptance. PDU-U1..U5 and unattested AIP/historical rows remain pending; ask before laptop/lab use.
**Rules:** preserve others' edits; no Plan fields; file caps; PowerShell only; compiling Cargo through the lane; ship-pr and retire after merge; never git worktree prune.