---
name: egui-eframe-split-version-bumps
description: Dependabot bumps egui and eframe as separate PRs; the egui-only bump is a no-op for the launcher and defers the real API breakage to the eframe PR
metadata:
  type: project
---

Dependabot raises `egui` and `eframe` as **separate** PRs even though they are
siblings in the same upstream workspace. The `egui`-only bump lands clean and
looks like the migration is done — it is not.

**Why:** `sgw-launcher` uses `eframe::egui` (the re-export), not the direct
`egui` dep. While `eframe` is still on the old minor, Cargo keeps *two* `egui`
versions in the lock (0.34.3 + 0.35.0 coexisted after PR #598), and the launcher
compiles against the old one. All the deprecation/rename breakage surfaces only
when the `eframe` PR unifies them.

**How to apply:** when reviewing an `egui`-family dependabot PR, expect the
sibling `eframe` PR to carry the actual API migration work, and check
`Cargo.lock` for duplicate `egui`/`ecolor`/`emath` entries as the tell. Also
note the launcher's only clippy coverage is the Windows job in
`.github/workflows/launcher-build.yml` — the main `ci` clippy excludes
`sgw-launcher`, so "non-Windows clippy passed" says nothing about the launcher.

Seen: PR #598 (egui 0.35.0, clean) → PR #624 (eframe 0.35.0, broke on
`CentralPanel::show_inside` → `show`).
