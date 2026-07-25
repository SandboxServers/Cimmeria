---
title: docs/technical/ — early-project research archive
type: reference
audience: contributors browsing historical context
last_updated: 2026-07-25
companion_docs:
  - ../readme.md
  - ../reverse-engineering/README.md
---

# `docs/technical/` — early-project research archive

This directory holds the **early-project RE analysis** that drove the initial server-emulator feasibility decisions and laid the foundation for the per-system findings now in [`docs/reverse-engineering/`](../reverse-engineering/). The content is **historical** — it predates the reorganised `docs/` tree and the Rust rewrite, and is kept only for context.

**Do not use these files as the current reference.** Every doc here has a forward link to the canonical replacement; if a banner is missing, treat the page as out of date and check [`docs/readme.md`](../readme.md) for the current owner.

## What's in here

Per the file naming:

| Topic | Historical doc here | Current canonical replacement |
|---|---|---|
| Build process (C++ Boost 1.55, OpenSSL 1.0.1e, MSVC) | [`building.md`](building.md) | [`../building.md`](../building.md) (Rust how-to) |
| Game systems survey from the binary | [`game-systems.md`](game-systems.md) | [`../game-systems.md`](../game-systems.md), [`../gameplay/`](../gameplay/) |
| Network message catalog (raw) | [`network-messages.md`](network-messages.md) | [`../network-messages.md`](../network-messages.md), [`../protocol/message-catalog.md`](../protocol/message-catalog.md) |
| Login + authentication flow | [`login-auth-flow.md`](login-auth-flow.md) | [`../connection-flow.md`](../connection-flow.md), [`../protocol/login-handshake.md`](../protocol/login-handshake.md) |
| What happens post-auth | [`post-auth-sequence.md`](post-auth-sequence.md) | [`../connection-flow.md`](../connection-flow.md), [`../protocol/world-entry-phases.md`](../protocol/world-entry-phases.md) |
| BigWorld version identification | [`bigworld-version-analysis.md`](bigworld-version-analysis.md) | [`../engine/bigworld-architecture.md`](../engine/bigworld-architecture.md) |
| Mercury protocol overview | [`mercury-protocol.md`](mercury-protocol.md) | [`../protocol/mercury-wire-format.md`](../protocol/mercury-wire-format.md), [`../drafts/spec/mercury-wire-format.md`](../drafts/spec/mercury-wire-format.md) (in-progress bible) |
| Mercury audit (server vs. BW reference) | [`mercury-audit.md`](mercury-audit.md) | [`../protocol/mercury-wire-format.md`](../protocol/mercury-wire-format.md) |
| Server emulator feasibility (early) | [`server-feasibility.md`](server-feasibility.md) | Settled — the server exists. See [`../project-status.md`](../project-status.md). |
| Source-code reconstruction feasibility (early) | [`source-reconstruction-feasibility.md`](source-reconstruction-feasibility.md) | Settled — see [`../reverse-engineering/STATUS.md`](../reverse-engineering/STATUS.md). |
| Game data analysis | [`game-data-analysis.md`](game-data-analysis.md) | [`../game-data.md`](../game-data.md), [`../content/`](../content/) |
| sgw.exe binary overview | [`sgw-binary-overview.md`](sgw-binary-overview.md) | [`../reverse-engineering/STATUS.md`](../reverse-engineering/STATUS.md), [`../reverse-engineering/address-map.md`](../reverse-engineering/address-map.md) |
| Slash commands | [`slash-commands.md`](slash-commands.md) | [`../commands.md`](../commands.md) |
| Launcher binary analysis | *(moved out — no historical copy remains here)* | [`../reverse-engineering/binaries/launcher-exe.md`](../reverse-engineering/binaries/launcher-exe.md) (canonical) |
| AtreaLoader analysis | [`atrealoader-exe.md`](atrealoader-exe.md), [`atrealoader-config.md`](atrealoader-config.md), [`atrearl-loader.md`](atrearl-loader.md) | **Not superseded** — these three remain the reference for the injector, patch table and runtime DLL. Orientation entry point for the toolchain as a whole (and the only coverage of the in-game UnrealEd editor) is [`../reverse-engineering/findings/atrea-editor.md`](../reverse-engineering/findings/atrea-editor.md). |

## Why this directory exists

When the project started, the `docs/` tree was flat and everything that wasn't a README went here. As the work matured, the canonical docs migrated to system-specific directories (`protocol/`, `engine/`, `gameplay/`, `reverse-engineering/`, etc.) and a curated index in [`docs/readme.md`](../readme.md). The files left behind are kept rather than deleted because:

- They contain early reasoning that informed later decisions and is occasionally useful for context.
- Some are still cited from external places (academic write-ups, forum posts) and shouldn't 404.
- The cost of keeping them with a forward-link banner is lower than the risk of losing a piece of project history.

## Eventual fate

When a historical doc has been fully absorbed into its canonical replacement, it can be removed. Until then, the banner-and-leave approach is what we use. If you find content here that **isn't** reflected in the canonical replacement, please open an issue (or a PR) — it means the migration hasn't fully landed yet.
