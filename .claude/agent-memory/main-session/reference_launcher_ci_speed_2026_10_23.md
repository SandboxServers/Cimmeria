---
title: Launcher CI build time and signing isolation
type: reference
audience: launcher maintainers and agent operators
last_updated: 2026-10-23
companions:
  - ../../../docs/analysis/launcher-uat-fixes/design-ci-speed.md
  - ../../../docs/architecture/build-system.md
---

# Launcher CI build time and signing isolation

The 2026-10-23 baseline identified uncached native validation and duplicate
release-tool dependency builds. PR run 39947687629 took 21m27s for native
Windows and 11m for native macOS. Release run 39916978171 spent 431s compiling
the isolated feed tool and 312s compiling the overlay CLI. These timings do
not establish savings after this change.

Native desktop jobs restore a standalone-graph Rust cache and save only on
main; superseded PR runs cancel while main finishes. CI sets dev/test debug
to zero without changing local settings or validation coverage. Release feed
and overlay CLIs use `release-tools` (inherits release, opt-level 0, 256 codegen
units, no LTO); the overlay CLI shares the desktop graph and target directory.
The app and shipped helpers keep normal release optimization.

The signing runner remains clean: no restored cache, node or third-party action.
Its feed-tool build precedes loading secrets, and only the signing step receives
the key/password from the `launcher-release` environment. Do not trade this
boundary for cache reuse. No real release is authorized by a CI dry run.

`tools/launcher-release/test_workflow_policy.py` uses PyYAML 6.0.3 to check
workflow semantics and preserved boundaries. The design is linked from the
campaign README; the release contract and pre-PR checks carry operating guidance.
