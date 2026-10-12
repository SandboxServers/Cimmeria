# LH work packets

Ledger: [README.md](README.md). Packet specs live in four files, one per area; each packet section is self-contained (files, exact code, test, lane checks, docs, live smoke, commit subject):

| File | Packets |
|---|---|
| [packets-ui.md](packets-ui.md) | LH-01 .. LH-04 (drag, tabs, item use, vendor buy) |
| [packets-camera.md](packets-camera.md) | LH-05 .. LH-08 (mouse x/y, pitch clamp, own-avatar occlusion, interact range) |
| [packets-events-move.md](packets-events-move.md) | LH-09 .. LH-12 (player stats, move_to, event keeper, click ring) |
| [packets-supervisor.md](packets-supervisor.md) | LH-13 .. LH-17 (log tail, startup grace, lane wait, keep-awake, server pick, lab-driver) |

Live probe results that change a packet are in [probes.md](probes.md). Where a probe result contradicts a packet's code, `probes.md` wins and says which step changes.

## Dispatch rules

- **Integration branch:** `lab-harness/integration`. Every packet branches from its current tip: `pwsh tools/build-lane/mk-worktree.ps1 lab-harness/<lh-nn>-<slug> lh<nn>` (from the main checkout; it branches from `origin/main`), then `git -C <wt> merge --ff-only origin/lab-harness/integration`. After review the coordinator merges the packet branch into integration with `--no-ff` and pushes. One PR, integration to `main`, at the end.
- **Parallel lanes** (packets in one lane run in order; lanes run in parallel, at most four coders at once, one lane slot each):
  - UI: LH-01, LH-02, LH-03, LH-04
  - Camera: LH-05, LH-06, LH-07, LH-08
  - Events and movement: LH-09, LH-11, LH-12, then LH-10 once LH-06 is integrated (`steer.rs`)
  - Supervisor: LH-13, LH-14, LH-15, LH-16
  - Last: LH-17, then LH-18 close-out.
- **Implementer:** `packet-coder` (Haiku). Brief: worktree path, packet file + section, commit message with attribution lines. If a coder runs out of context and the partial work is good, a fresh `packet-coder` continues from the uncommitted diff and `worknotes/<LH-nn>.md`. At most three Haiku sessions per packet; then a Sonnet worker finishes it.
- **Review:** every Haiku commit, including follow-ups and merge resolutions, gets a Sonnet `packet-review-fixer` pass (reviews, fixes, reruns the lane checks, commits). Before the PR, one fresh Sonnet whole-branch adversarial review, plus `server-authority-enforcer` on LH-13 (product crate).
- **Workers:** PowerShell and `lane.ps1` only, run lane jobs in the foreground, never `git push`, `git stash`, `git worktree prune`, `gh pr`. Workers never drive the lab: every daemon call in a test is stubbed by the fake bridge / fake CEGUI.
- **Lab:** only the coordinator (through probe runners and the LH-18 close-out) uses the live lab, with the user's approval for this campaign, and only while the build lane is idle (item 13).
- **Docs:** each packet adds its own rows to `docs/guides/live-research-lab.md` / `automated-uat.md` as its spec says. `docs/gap-analysis*` and `docs/project-status.md` change only in LH-18.

## Shared contract

Crate: `cimmeria-lab` (`crates/lab`), plus `cimmeria-lab-mcp` (`crates/lab-mcp`) and `cimmeria-admin-api` (`crates/admin-api`) for LH-13. Lane checks, from the packet worktree:

```powershell
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab <filter>
```

New tool arguments are optional with today's behavior as the default, except where a decision in the README changes a default (D-LH1, D-LH5, D-LH6). New tools (`client_vendor_buy`) are registered in `crates/lab/src/server/ui/mod.rs`, added to the lease policy if they drive the client, and listed in `.claude/agents/lab-driver.md` by LH-17.
