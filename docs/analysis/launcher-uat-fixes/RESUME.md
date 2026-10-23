# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [adoption design](design-adopt-in-place.md), [validation](worknotes/adopt-in-place-validation.md), [UAT](../../guides/unified-uat.md#launcher-uat-fixes)

**Baseline:** Original U1-U5 (#1984 through #1988) and close-out #1989 are merged. Historical results and accepted risks remain in the ledger.

**Approvals:** macOS and Windows adopt at the selected root; explicit per-file changes, cooked normalization and backups there are permitted, bulk relocation/copy prohibited. D-LX15 approved. No new owner decisions; **a real release requires explicit owner GO**.

| Task | State at this checkpoint | Resume action |
|---|---|---|
| 1: Direct-root adoption + footer button | [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992), open/ready; setup fix `1cbfc54f5` + UI fix `ed6940107`; review 4 ship; prior review 5 ship; review 6 ship; review 7 ship; narrow production-cfg review 8 ship; next Mac clippy/executable build required | Check GitHub: if open finish CI/merge queue via ship-pr; if merged verify worktree retirement and treat implementation complete |
| 2: Repeated signed releases + applied-content logs | [PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991) merged `7be8413b0`; worktree retired 2026-10-23T03:38:43Z | Complete; Windows/macOS/package CI and merge-queue full tests passed; no actual release |

**Latest Task 1 checks:** engine 969 passed / 3 ignored (`20261023-011056-248396`), all-target engine clippy clean; six setup journeys pass, disabling comparison fails four guards. Frontend 250/footer REPL (helper unavailable included)/build pass. Final shell 148/7 ignored, native-backed adoption UAT, workspace all-target clippy and native build pass; build 11.821s. Final fmt corrected in `6e733e12e`. [Worknote](worknotes/adopt-in-place-validation.md) preserves log IDs, prior proofs and limits.

**Safety:** shared retained-parent/owner-identity guards protect adoption, maintenance, rollback, Uninstall, receipts and tree writes. Unix uses actual descriptor-relative operations; no Darwin volfs claim. Windows junction/rename-refusal tests ran; All six Unix substitution guards and corrected shell cases pass in Mac CI; production clippy remains gated. All interrupted schema-1 adoption, including copy, refuses recovery; metadata-only abandon preserves content. Successful legacy installs keep their layout.

**Current CI gate:** Mac run `39910259596`, job `120276089926`: engine 925 passed / 24 ignored; shell 160 passed / 17 ignored, including all three previously failing cases; frontend 250/persistence REPL/UI build pass. Production clippy flags two legacy helpers and retained IPC ImportRequired as unused on Mac. Narrow cfg/expect fix preserves runtime/wire behavior; review 8 ship; local shell 148/7 ignored, workspace clippy, native build and fmt pass. Corrected next-head Mac clippy still required; overall CI is not green.

**UAT limits:** fixtures are not Wine/Tauri/physical picker/packaged visuals/real macOS runtime/game/power loss/SigNoz end-to-end acceptance. Browser mock at 1280×720 verifies footer and first feedback only. Owner UAT AIP-U1..U5 and historical U1..U6 remain unattested; ask before laptop/lab use.

**Worktrees:** Task 1 `.claude/worktrees/launcher-adopt-in-place`; Task 2 retired. Preserve others' changes, no Plan fields added, keep file caps. PowerShell only; compiling Cargo through `pwsh tools/build-lane/lane.ps1 cargo ...`; ship-pr and retire on merge; never `git worktree prune`.
