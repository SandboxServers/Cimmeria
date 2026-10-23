# Launcher UAT fixes: follow-up resume card

> **Type:** Reference · **Audience:** Resuming engineers and owner · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [adoption design](design-adopt-in-place.md), [validation](worknotes/adopt-in-place-validation.md), [UAT](../../guides/unified-uat.md#launcher-uat-fixes)

**Baseline:** Original U1-U5 (#1984 through #1988) and close-out #1989 are merged. Historical results and accepted risks remain in the ledger.

**Approvals:** macOS and Windows adopt at the selected root; explicit per-file changes, cooked normalization and backups there are permitted, bulk relocation/copy prohibited. D-LX15 approved. No new owner decisions; **a real release requires explicit owner GO**.

| Task | State at this checkpoint | Resume action |
|---|---|---|
| 1: Direct-root adoption + footer button | [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992), draft; setup fix `1cbfc54f5` + UI fix `ed6940107`; fresh review 4 running; native macOS CI pending | Check GitHub: if open finish review/CI/ship-pr; if merged verify worktree retirement and treat implementation complete |
| 2: Repeated signed releases + applied-content logs | [PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991) merged `7be8413b0`; worktree retired 2026-10-23T03:38:43Z | Complete; Windows/macOS/package CI and merge-queue full tests passed; no actual release |

**Latest Task 1 checks:** engine 968 passed / 3 ignored (`20261022-232513-157888`), all-target engine clippy clean; six setup journeys pass, disabling comparison fails four guards. Frontend 250/footer REPL (helper unavailable included)/build pass. Earlier shell 147/7 ignored, native-host UAT, workspace clippy and production build passed before setup fix; their final repeats pending. [Worknote](worknotes/adopt-in-place-validation.md) preserves log IDs, prior proofs and limits.

**Safety:** shared retained-parent/owner-identity guards protect adoption, maintenance, rollback, Uninstall, receipts and tree writes. Unix uses actual descriptor-relative operations; no Darwin volfs claim. Windows junction/rename-refusal tests ran; Unix substitution execution awaits Mac CI. All interrupted schema-1 adoption, including copy, refuses recovery; metadata-only abandon preserves content. Successful legacy installs keep their layout.

**CI caveat:** initial Mac fixture-import failure fixed by conditional gating `094f8e904`; latest Mac CI still pending. Fresh review 4 not yet accepted; setup original/approved expectations are plan-bound and reject edits before recovery or rollback.

**UAT limits:** fixtures are not Wine/Tauri/physical picker/packaged visuals/real macOS runtime/game/power loss/SigNoz end-to-end acceptance. Browser mock at 1280×720 verifies footer and first feedback only. Owner UAT AIP-U1..U5 and historical U1..U6 remain unattested; ask before laptop/lab use.

**Worktrees:** Task 1 `.claude/worktrees/launcher-adopt-in-place`; Task 2 retired. Preserve others' changes, no Plan fields added, keep file caps. PowerShell only; compiling Cargo through `pwsh tools/build-lane/lane.ps1 cargo ...`; ship-pr and retire on merge; never `git worktree prune`.
