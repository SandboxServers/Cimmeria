# launcher-win-arm64 resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-24
> **Companions:** [Ledger](README.md), [work packets](work-packets.md)

**Goal:** every launcher release ships a native Windows arm64 launcher (`StargateWorlds-Launcher-windows-arm64.zip`, feed key `windows-aarch64`) beside x64, from one signed `latest.json`.

**Baseline:** designed against `main` `2019922fe`. No code written, no worktree created, nothing in flight.

| Wave | Packets | Ships as | State |
|---|---|---|---|
| 1 | WA-01..WA-06 | one PR from `lwa-integration` (branch `launcher-win-arm64/integration`) | Ready; WA-02 waits on WA-01's review `ship` |
| close-out | WA-07 | own docs PR | after the wave PR merges |
| owner UAT | WA-U1..U3 | none | after CI, first and second release |

**Next action:**

1. `ops`: create `lwa-integration` and `lwa-01`, `lwa-03`, `lwa-04`, `lwa-05`, `lwa-06` from main (`mk-worktree.ps1`).
2. Dispatch WA-01, WA-03, WA-04, WA-05 (packet-coder) and WA-06 (documentation-writer) in the background, each with its § from work-packets.md.
3. After WA-01's review returns `ship`: `ops` creates `lwa-02` at WA-01's reviewed head; dispatch WA-02.
4. Merge into the integration branch in order WA-01, 02, 03, 04, 05, 06; run the wave review; open the PR.
5. Hold the merge until the four dry-run and release-scripts checks are green (work-packets § Dispatch rules). That run is the first real arm64 compile; a failure gets a fix packet in `lwa-integration`.

**Decisions:** D-WA1..D-WA11 decided (ledger). Hosted `windows-11-arm` (free, public repo); compile-time key pick; one feed with both keys; arm64 on the release critical path.

**Open owner decisions:** none blocking. Optional: make the dry runs required checks (D-WA7).

**Follow-ups:** WA-F1 (bootstrap and `setup.ps1` pick arm64, after WA-U2), WA-F2 (split the over-cap implementation ledger).

**Rules:** PowerShell only; compiling cargo through the lane; `.sh`/`.yml` LF, docs CRLF; never `git worktree prune`; retire worktrees on merge.
