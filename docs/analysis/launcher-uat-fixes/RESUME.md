# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [adoption design](design-adopt-in-place.md), [validation](worknotes/adopt-in-place-validation.md), [UAT](../../guides/unified-uat.md#launcher-uat-fixes)

**Baseline:** Original U1-U5 (#1984 through #1988) and close-out #1989 are merged. Historical results and accepted risks remain in the ledger.

**Approvals:** macOS and Windows adopt at the selected root; explicit per-file changes, cooked normalization and backups there are permitted, bulk relocation/copy prohibited. D-LX15 approved. No new owner decisions; **a real release requires explicit owner GO**.

| Task | State at this checkpoint | Resume action |
|---|---|---|
| 1: Direct-root adoption + footer button | [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992), open/ready; setup fix `1cbfc54f5` + UI fix `ed6940107`; review 4 ship; prior review 5 ship; review 6 ship; review 7 ship for shell compatibility; next native macOS CI required | Check GitHub: if open finish CI/merge queue via ship-pr; if merged verify worktree retirement and treat implementation complete |
| 2: Repeated signed releases + applied-content logs | [PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991) merged `7be8413b0`; worktree retired 2026-10-23T03:38:43Z | Complete; Windows/macOS/package CI and merge-queue full tests passed; no actual release |

**Latest Task 1 checks:** engine 969 passed / 3 ignored (`20261023-011056-248396`), all-target engine clippy clean; six setup journeys pass, disabling comparison fails four guards. Frontend 250/footer REPL (helper unavailable included)/build pass. Final shell 148/7 ignored, native-backed adoption UAT, workspace all-target clippy and native build pass; build 11.821s. Final fmt corrected in `6e733e12e`. [Worknote](worknotes/adopt-in-place-validation.md) preserves log IDs, prior proofs and limits.

**Safety:** shared retained-parent/owner-identity guards protect adoption, maintenance, rollback, Uninstall, receipts and tree writes. Unix uses actual descriptor-relative operations; no Darwin volfs claim. Windows junction/rename-refusal tests ran; All six Unix substitution guards passed in Mac CI; the shell job still fails. All interrupted schema-1 adoption, including copy, refuses recovery; metadata-only abandon preserves content. Successful legacy installs keep their layout.

**Current CI gate:** run `39907274967` Mac engine 925 passed / 24 ignored, including all six Unix guards, legacy minimum and updater Light Busy; frontend 250/REPL/build pass. Mac shell job `120266139979`: 157 passed / 3 failed. Fixes accepted by review 7 restore authenticated schema-1 display/provenance compatibility while keeping schema 2 and pre-Play settings strict, and replace obsolete legacy-copy Recover expectation with shared recovery-refusal/content-preservation coverage. Review 7 ship; strict-plan mutation fails the legacy guard, restored source. Final engine 969/3 ignored, shell 148/7 ignored, clippy/build/fmt and native-backed UAT pass; next Mac CI required. Whole Mac job is not green; next CI required.

**UAT limits:** fixtures are not Wine/Tauri/physical picker/packaged visuals/real macOS runtime/game/power loss/SigNoz end-to-end acceptance. Browser mock at 1280×720 verifies footer and first feedback only. Owner UAT AIP-U1..U5 and historical U1..U6 remain unattested; ask before laptop/lab use.

**Worktrees:** Task 1 `.claude/worktrees/launcher-adopt-in-place`; Task 2 retired. Preserve others' changes, no Plan fields added, keep file caps. PowerShell only; compiling Cargo through `pwsh tools/build-lane/lane.ps1 cargo ...`; ship-pr and retire on merge; never `git worktree prune`.
