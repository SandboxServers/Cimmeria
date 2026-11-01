---
name: Tauri launcher implementation plan pointer
description: Where the desktop launcher plan and ledger live, and the build traps its first implementation hit
metadata:
  type: reference
---

The plan and its ledger are `docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md`; per-feature worknotes sit beside it. Direction: Tauri shell plus Effect workflow coordination, with Rust authoritative for install and launch mutation. This file replaces a long per-day progress log; the ledger and `git log` own the status.

Traps from the first implementation (2026-10-04):

- The desktop workspace is separate from the root workspace; root tests omit it. Its CI is `.github/workflows/launcher-desktop.yml`, so run explicit manifest checks.
- Windows shell CI needs `shell/icons/icon.ico` even for tests; a PNG-only shell passed on Mac and failed Windows resource creation.
- File-manager reveal accepts only the saved existing directory, avoiding arbitrary-path IPC.
- Launcher telemetry queues only when opted in and uploads in the next game telemetry session; failures before game launch are unseen remotely. Do not claim immediate DLL consent revocation.
- The headless UAT drives the real Rust `commands.rs` through the `state_bridge` example.

Related: [[reference_launcher_desktop_native_traps]].
