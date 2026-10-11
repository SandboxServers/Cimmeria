# Launcher Consolidation Acceptance

> Type: reference. Audience: the coordinator, packet workers and the owner running LX-26.
> Updated: 2026-10-10. Companions: [ledger and decisions](README.md), [work packets](work-packets.md), [desktop launcher acceptance checklist (macOS/Wine)](../playtests/2026-10-03-macos-wine/launcher-acceptance.md), [desktop launcher ledger README](../playtests/2026-10-03-macos-wine/README.md), [unified UAT guide](../../guides/unified-uat.md).

This file owns two things: the desktop launcher's open Windows and platform-neutral acceptance gates, adopted from its macOS/Wine ledger by LX-25, and the result of each LX-26 UAT step. The macOS-only gates stay in [launcher-acceptance.md](../playtests/2026-10-03-macos-wine/launcher-acceptance.md).

A gate closes when its closing packet merges with the guard the packet names *and* its LX-26 step records a pass. Change a row's Status here, not in the desktop ledger.

## Adopted gates

Each row quotes the "Remaining verification" cell of the [requirement and evidence checklist](../playtests/2026-10-03-macos-wine/launcher-acceptance.md#requirement-and-evidence-checklist) (or names the section it came from), trimmed to the Windows and platform-neutral part. "Step N" means LX-26 step N below.

| Gate | Source | Open verification (Windows / platform-neutral) | Closed by | Status | Notes |
|---|---|---|---|---|---|
| G-01 | Checklist: Single-game dark-only Tauri interface | Keyboard and focus, responsive layout, and the visuals of the integrated features | Steps 1 and 6 | Open | No step names keyboard and focus. Check them by hand during step 6, on every new Settings view (LX-03 to LX-07). |
| G-02 | Checklist: Real Effect orchestration and persistent settings | Integrated native-persistence UAT; failed-save and reconnect behaviour | LX-02 (stale-revision refusal test); step 6 | Open | Step 6 proves a save persists. No step forces a failed save; the LX-02 engine test is the only guard for it. |
| G-03 | Checklist: Installation and prerequisites | Graphics and Play after install; install failures and cancellation | LX-12; steps 2 and 3 | Open | Cancelling a fresh install is not a step. Cancel once during step 2 and restart it. |
| G-04 | Checklist: Play with lifecycle observation | Authentication and world entry; current native parity | Step 2 | Open | Step 2 stops at character select. Enter the world once to close this gate. "Actual SGW computer use" is the macOS Wine-identity gate and stays in the desktop ledger. |
| G-05 | Checklist: Repair | Native visual and original-client Repair UAT | Step 12 | Open | |
| G-06 | Checklist: Confirmed uninstall | Integrated confirmation, dismiss, removal and recovery UAT | LX-10 (uninstall of an adopted folder); step 12 | Open | Run Uninstall on a fresh install and on an adopted one (step 4's folder): the adopted one must leave unlisted user files. |
| G-07 | Checklist: Signed GitHub manifest patch notes | Refresh and reconnect after the integrated changes | Step 2 | Open | No step names it. Open the patch-notes tab during step 2, then again after step 11's self-update. |
| G-08 | Checklist: Default-off optional summary consent | Native Windows CI result; consent copy and frontend UAT | None in this campaign | Open | Turning summaries on is [out of scope](README.md#out-of-scope); the rollout packet in the summaries ledger closes it. Only the Windows CI result can land here, through `launcher-desktop.yml`. |
| G-09 | Checklist: Focused observability | Production endpoint, fixtures checked against a live SigNoz | None in this campaign | Open | Same rollout as G-08. Game telemetry (LX-08) is a different pipeline; step 9 does not close this gate. |
| G-10 | Checklist: Migration and identity/consent preservation | Adopted Update and Windows adoption | LX-10, LX-03; steps 4 and 5 | Open | Blocked on D-LX12. Run a game Update on the step 4 folder during step 12. The macOS verified-copy adoption and adopted-Play items stay in the desktop ledger. |
| G-11 | Checklist: Single updater owner and version/asset mapping | Real packaged Apply, restart and rollback; download resume; production configuration; Windows replacement parity | LX-13, LX-15, LX-17; step 11 | Open | Production configuration needs D-LX9. Download resume is not a step: interrupt the network once during step 11's download. |
| G-12 | Checklist: Tests, docs and project memory | Per-packet guards, fresh bounded review, matching docs and indexes | Every packet's review; LX-23, LX-24, LX-27 | Open | Closed at close-out, not by a UAT step. |
| G-13 | Checklist: Windows native parity | Corrected native tests, FDI preflight, latest helper, updater and adoption parity | LX-10, LX-13; steps 2, 4 and 11 | Open | Step 2's RAR install exercises the FDI cabinet preflight and the helper. The corrected test-only filesystem snapshots and the owner-lock unlock fix (desktop ledger, "Earlier integration wave") need a green native Windows `launcher-desktop.yml` run; no packet owns that. |
| G-14 | Checklist: Final self-contained packages (LAST) | Offline first open, bundled dependencies, clean-machine checks, per-OS artifacts | LX-11, LX-12, LX-15; step 1 | Open | Windows only here. Code signing is deferred (D-LX10); notarization and the macOS artifact stay in the desktop ledger. That ledger makes this the last gate: if any packet lands after step 1 passes, repeat step 1 on the final zip. |
| G-15 | Section: Windows troubleshooting handoff | The updater ShellExecute test stall, [#1194](https://github.com/SandboxServers/Cimmeria/issues/1194), and its unapplied candidate patch | LX-13 | Open | LX-13 replaces the installer hand-off with a zip swap. If no ShellExecute path remains, close #1194 in the LX-13 PR as obsolete; otherwise apply and prove the candidate there. |

Left in the desktop ledger as macOS only: "Actual Mac graphics/login/world", and the macOS parts of G-04, G-10 and G-14 named in their Notes.

## LX-26 results

The owner runs these on a Windows machine with no launcher state, from the `launcher-current` download. Record `Pass` or `Fail`, the date, the release tag, and for a fail the first error and a log path. The step text is the canonical list in [work-packets.md](work-packets.md#lx-26-windows-uat-of-the-released-zip).

| Step | What to check | Packets | Gates | Result | Date and tag | Notes |
|---|---|---|---|---|---|---|
| 1 | Download and unzip from the stable link; first run with WebView2 present, and once on a machine or VM without it | LX-11, LX-15 | G-01, G-14 | | | |
| 2 | Fresh install from the archive.org RAR; Play reaches character select | LX-15 | G-03, G-04, G-07, G-13 | | | |
| 3 | PhysX missing: setup offers and installs it | LX-12 | G-03 | | | |
| 4 | Adopt an existing egui install in place; Play works without a reinstall | LX-10 | G-10, G-13 | | | |
| 5 | Import egui settings; the server list carries over | LX-03 | G-10 | | | |
| 6 | Server list edit, client-patches toggle off and on, both visible in "Changes to your client" | LX-03, LX-04, LX-07 | G-01, G-02 | | | |
| 7 | Reset cache; reset all with confirm | LX-06 | | | | |
| 8 | Upload logs twice; the second says nothing new | LX-05 | | | | |
| 9 | Telemetry opted in: a session shows in SigNoz with tailed log lines and exit code | LX-08a, LX-08b, LX-08c | | | | |
| 10 | Second launch focuses the first | LX-09 | | | | |
| 11 | Self-update from release N-1 to N; then a forced failed update rolls back | LX-13, LX-15, LX-17 | G-07, G-11, G-13 | | | |
| 12 | Repair, game Update and Uninstall on Windows | | G-05, G-06, G-10 | | | |
