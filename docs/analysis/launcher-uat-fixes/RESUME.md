# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [adoption design](design-adopt-in-place.md), [validation](worknotes/adopt-in-place-validation.md), [UAT](../../guides/unified-uat.md#launcher-uat-fixes)

**Baseline:** Original U1-U5 (#1984 through #1988) and close-out #1989 are merged. Historical packet results and accepted risks remain in the ledger. These two follow-ups are separate tasks, not a resumption of pending original waves.

**Owner approvals (2026-10-23):** macOS adopts directly at the selected root too; explicit per-file setup/patch changes, cooked normalization and replacement backups there are allowed; bulk copy/relocation is prohibited. D-LX15 stable versioning is approved. **No release is authorized as part of these tasks.**

| Task | Worktree / state | Next action |
|---|---|---|
| 1: Direct-root adoption and main-page button | `.claude/worktrees/launcher-adopt-in-place`; [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992); review-1 fixes N1-N6 complete; fresh review 2 pending; native CI pending | Finish fresh review and CI, then ship via ship-pr |
| 2: Several signed releases per day + applied-content identity logs | `.claude/worktrees/launcher-repeat-releases`; [PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991); fresh review 3 ship, required CI green, awaiting native CI; unmerged | Wait for native CI before merge; no real release |

**Task 1 checks:** post-fix engine 939 passed / 3 ignored; all-target clippy clean; safety adoption 257 passed; restored provenance 2 passed. Prior shell adoption 24 passed / 1 ignored, frontend 250 passed. Native-backed `uat:adoption` now passes; footer/Light REPL and frontend build pass again. Restoring bulk relocation fails the invariant, then restored fix passes; frontend mutations fail 5/6 targeted checks, restored fix passes 6/6. N1-N5 mutations also fail their guards after reverting fixes. Log IDs and attribution: [validation worknote](worknotes/adopt-in-place-validation.md).

**Native UAT boundary:** production Effect/native host, isolated disk and signed loopback fixtures cover replacement consent, hard-link refusal, single confirmation, original executable path and selective Uninstall preserving extras; not Wine/Tauri/physical picker/real game.

**Visual boundary:** 1280 × 720 browser mock shows footer, Settings opening, immediate pending feedback and duplicate disabling. Incomplete unrelated mocks mean this is not native acceptance.

**Not tested:** real macOS runtime, physical folder picker, packaged native visuals, game launch, physical power loss, SigNoz end to end. Native macOS CI pending. Owner UAT AIP-U1..U5 and historical U1..U6 remain unattested; ask before laptop/lab use.

**Layout invariant:** schema-2 Plan and InstallIntent select direct-root after ownership/provenance/journal checks; no new Plan fields. Successful schema-1 installs keep root/game. Legacy interrupted movers refuse recovery; metadata-only abandon leaves bytes/originals/backups for manual inspection. Schema-2 abandon undoes approved per-file changes. Repair/Update commit per file; selective Uninstall retains root/extras.

**Rules:** PowerShell only; compiling Cargo via `pwsh tools/build-lane/lane.ps1 cargo ...`; one task per worktree from origin/main. Preserve file caps, especially in_place/inventory/session/log-upload seams. Ship with ship-pr, retire on merge, never `git worktree prune`.
