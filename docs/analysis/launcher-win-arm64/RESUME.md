# launcher-win-arm64 resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-24
> **Companions:** [Ledger](README.md), [work packets](work-packets.md)

**Goal:** every launcher release ships a native Windows arm64 launcher (`StargateWorlds-Launcher-windows-arm64.zip`, feed key `windows-aarch64`) beside x64, from one signed `latest.json`.

**State: closed out 2026-10-24.** Wave 1 (WA-01..WA-06) merged as PR #2002; WA-07 (status docs, unified UAT, this card) ships as its own docs PR. Nothing is in flight.

**Verified vs not:** the launcher app itself was confirmed running on the owner's arm64 Windows laptop. The release pipeline (hosted `windows-11-arm` build, two-zip feed, `windows-aarch64` updater pick) has not been proved by a real release.

**Owed (owner, no lab use):** WA-U1 (the PR's arm64 dry-run zip on the Arm laptop), WA-U2 (first release: install and Play on Arm), WA-U3 (second release: self-update on arm64 and on x64-on-Arm). Steps: [unified UAT § Launcher on Windows on Arm](../../guides/unified-uat.md#launcher-on-windows-on-arm).

**Next action:**

1. After the WA-07 PR merges, `ops` retires `lwa-01`..`lwa-07` and `lwa-integration` with `rm-worktree.ps1 --merged-into <PR> <names>` and posts the board handoff.
2. The first release after the merge is the orchestrator's call under the standing launcher-release instruction, if it covers this campaign; otherwise the owner's. If that release's `sign` or `publish` fails on the arm64 job, recovery is "Re-run failed jobs" (D-WA5).

**Follow-ups:** WA-F1 (bootstrap and `setup.ps1` pick arm64, after WA-U2), WA-F2 (split the over-cap implementation ledger). Optional owner call: make the dry runs required checks (D-WA7).

**Rules:** PowerShell only; compiling cargo through the lane; `.sh`/`.yml` LF, docs CRLF; never `git worktree prune`; retire worktrees on merge.
