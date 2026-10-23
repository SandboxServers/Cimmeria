# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [adoption design](design-adopt-in-place.md), [validation](worknotes/adopt-in-place-validation.md), [UAT](../../guides/unified-uat.md#launcher-uat-fixes)

**Baseline:** Original U1-U5 (#1984 through #1988) and close-out #1989 are merged. Historical results and accepted risks remain in the ledger.

**Approvals:** macOS and Windows adopt at the selected root; explicit per-file changes, cooked normalization and backups there are permitted, bulk relocation/copy prohibited. D-LX15 approved. No new owner decisions; **a real release requires explicit owner GO**.

| Task | State at this checkpoint | Resume action |
|---|---|---|
| 1: Direct-root adoption + footer button | [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992), open/ready; setup fix `1cbfc54f5` + UI fix `ed6940107`; review 4 ship; prior review 5 ship; fresh review 6 ship for Mac fixes; next native macOS CI required | Check GitHub: if open finish CI/merge queue via ship-pr; if merged verify worktree retirement and treat implementation complete |
| 2: Repeated signed releases + applied-content logs | [PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991) merged `7be8413b0`; worktree retired 2026-10-23T03:38:43Z | Complete; Windows/macOS/package CI and merge-queue full tests passed; no actual release |

**Latest Task 1 checks:** engine 969 passed / 3 ignored (`20261023-003909-271772`), all-target engine clippy clean; six setup journeys pass, disabling comparison fails four guards. Frontend 250/footer REPL (helper unavailable included)/build pass. Post-compatibility shell 147/7 ignored, native-backed adoption UAT, workspace all-target clippy and native build pass; build 8.468s. Final fmt corrected in `6e733e12e`. [Worknote](worknotes/adopt-in-place-validation.md) preserves log IDs, prior proofs and limits.

**Safety:** shared retained-parent/owner-identity guards protect adoption, maintenance, rollback, Uninstall, receipts and tree writes. Unix uses actual descriptor-relative operations; no Darwin volfs claim. Windows junction/rename-refusal tests ran; Unix substitution execution awaits Mac CI. All interrupted schema-1 adoption, including copy, refuses recovery; metadata-only abandon preserves content. Successful legacy installs keep their layout.

**Current CI gate:** Mac run `39903893543`, job `120257546227`: 915 passed / 9 failed. Fixes under fresh review: canonicalize Unix fixture temp roots (`/var` vs `/private/var`), use supported Light in updater-busy test, retain schema-1 read-only owner/minimum when imported settings fail while keeping Play strict. New legacy guard passes; unconditional verification mutation fails. Restored full engine 969 passed / 3 ignored (`20261023-003909-271772`, 64.676s), review 6 ship; shell 147/7 ignored, workspace clippy, native build and native-backed UAT pass; next Mac CI required, no Unix-guard pass claimed. Earlier cfg/fmt fixes and independent ship reviews remain historical evidence.

**UAT limits:** fixtures are not Wine/Tauri/physical picker/packaged visuals/real macOS runtime/game/power loss/SigNoz end-to-end acceptance. Browser mock at 1280×720 verifies footer and first feedback only. Owner UAT AIP-U1..U5 and historical U1..U6 remain unattested; ask before laptop/lab use.

**Worktrees:** Task 1 `.claude/worktrees/launcher-adopt-in-place`; Task 2 retired. Preserve others' changes, no Plan fields added, keep file caps. PowerShell only; compiling Cargo through `pwsh tools/build-lane/lane.ps1 cargo ...`; ship-pr and retire on merge; never `git worktree prune`.
