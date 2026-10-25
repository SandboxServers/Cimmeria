---
name: feedback-check-main-for-same-day-merges
description: Before slicing a campaign, check main for same-day merges that touch the target module
metadata:
  type: feedback
---

Before slicing a campaign, check `main` for same-day merges that touch the
target module. The architect runs no git state checks itself, so ask the
orchestrator for a `git log -- <path>` on `origin/main` via `ops`.

Why: launcher-linux-install-telemetry (2026-10-24) sliced an in-house `mscab`
cabinet reader (LI-01..LI-04) while #2020/#2023 landed the same thing,
`unpack::cab_portable`, on `main` that day. Four packets were superseded after
LI-01..LI-03 were already coded (ruling LI-02-ruling-1).

Also: the root checkout's local `main` can lag `origin/main` (it was at #2014
while `origin/main` had #2020 and #2023). Read code at `origin/main`
(`git show origin/main:<path>`) or in a fresh worktree, not in the root
checkout.
