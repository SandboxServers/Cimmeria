---
title: Launcher CI and release build time
type: explanation
audience: launcher maintainers and CI engineers
last_updated: 2026-10-23
companions:
  - ../../architecture/build-system.md
  - ../../../crates/launcher/desktop/docs/release.md
---

# Launcher CI and release build time

The owner requested shorter launcher PR checks and release workflows on
2026-10-23. This build-only change has its own worktree and PR, separate from
the bundled player-facing PR #1993. No release is authorized.

## Measured baseline

[PR run 39947687629](https://github.com/SandboxServers/Cimmeria/actions/runs/39947687629)
took 21m27s for native Windows validation and 11m for native macOS validation.
The Windows engine check took 3m51s, engine tests 7m17s and shell tests 4m45s.
Neither native job restored Rust artifacts. Windows packaging took 8m23s,
followed by serial Windows workers for macOS (7m56s) and Mac packaging (4m1s).
Workflow-wide `cancel-in-progress: false` kept newer PR commits waiting behind
obsolete packaging work.

[Release run 39916978171](https://github.com/SandboxServers/Cimmeria/actions/runs/39916978171)
on 2026-10-23 took 20.3 minutes for the Windows build, 8.2 minutes for signing,
8.7 minutes for macOS workers and 3.4 minutes for the Mac app. Dominant steps
were launcher compilation (670s), the root-workspace overlay pack (312s) and
isolated feed-tool compilation (431s). These are baseline measurements, not
promised savings.

## Selected changes

1. Hosted native jobs restore a pinned rust-cache step for the standalone
   graph and save caches only from main, following the existing repository
   10 GB cache-budget policy. CI consistently disables dev/test debug info.
   Native tests, clippy, executable builds and JS persistence UAT remain.
2. Superseded PR runs cancel; main runs finish to save their caches. Real
   release concurrency and signing/publication ordering remain unchanged.
3. Release-only feed and overlay CLIs use a dedicated `release-tools` profile.
   It inherits release semantics and debug-assertion settings, disables LLVM
   optimization and LTO, and uses 256 codegen units. Player executables and
   helpers keep their normal release profile. The overlay CLI builds from the
   standalone desktop graph and target directory, avoiding a second build of
   the root-workspace dependency graph.

The signing job keeps a clean hosted runner with no restored cache, node or
third-party action. Only its signing step receives the private key and password
from the `launcher-release` environment. The CLI still compiles before secrets
are present and signs/verifies through the same engine code. This change does
not introduce trusted executables or caches into the signing job or move keys.

## Validation and limits

Semantic workflow guards protect validation coverage, main-only cache writes,
PR cancellation, tool-profile use and signing isolation. Artifact checks built the real tools, verified a feed with a throwaway key,
and checked overlay content, hash and deterministic output. All five workflow
guards have mutation proofs; see the [validation worknote](worknotes/ci-speed-validation.md). Existing
shell stubs, shellcheck and full package dry runs execute in CI.

The workflow-policy suite is
`python tools/launcher-release/test_workflow_policy.py` (PyYAML 6.0.3, installed
in CI's test virtual environment). Release identity tests remain separate.
Record compile/job timings without claiming savings until a changed run has
finished. A larger job split to start Mac helpers earlier remains deferred.
See the [release contract](../../../crates/launcher/desktop/docs/release.md#build-time-and-signing-isolation)
and [pre-PR checks](../../agents/pre-pr-checks.md#desktop-launcher-ci-policy-checks)
for operating guidance.

The doc-update-map walk covers build profiles, CI gating and desktop launcher
packaging/release. Corresponding updates belong in the build-system explanation,
pre-PR reference, desktop README/release contract, tools-catalog prose and project
memory. Generated blocks and player instructions do not change.
