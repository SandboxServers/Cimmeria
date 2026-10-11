# Launcher Consolidation Acceptance

> Type: reference. Audience: the coordinator, packet workers and the owner running LX-26.
> Updated: 2026-10-10. Companions: [ledger and decisions](README.md), [work packets](work-packets.md), [desktop launcher acceptance checklist (macOS/Wine)](../playtests/2026-10-03-macos-wine/launcher-acceptance.md), [desktop launcher ledger README](../playtests/2026-10-03-macos-wine/README.md), [unified UAT guide](../../guides/unified-uat.md).

This file owns two things: the desktop launcher's open Windows and platform-neutral acceptance gates, adopted from its macOS/Wine ledger by LX-25, and the result of each LX-26 UAT step. The macOS-only gates stay in [launcher-acceptance.md](../playtests/2026-10-03-macos-wine/launcher-acceptance.md).

A gate closes when its closing packet merges with the guard the packet names *and* its LX-26 step records a pass. Change a row's Status here, not in the desktop ledger.

## Adopted gates

Each row quotes the "Remaining verification" cell of the [requirement and evidence checklist](../playtests/2026-10-03-macos-wine/launcher-acceptance.md#requirement-and-evidence-checklist) (or names the section it came from), trimmed to the Windows and platform-neutral part. "Step N" means LX-26 step N below.

| Gate | Source | Open verification (Windows / platform-neutral) | Closed by | Status | Notes |
|---|---|---|---|---|---|
| G-01 | Checklist: Single-game dark-only Tauri interface | Keyboard and focus, responsive layout, and the visuals of the integrated features | Steps 1 and 6 | Open | Step 6 includes the keyboard-only pass over every new Settings view (LX-03 to LX-07). |
| G-02 | Checklist: Real Effect orchestration and persistent settings | Integrated native-persistence UAT; failed-save and reconnect behaviour | Step 6 | Open | Step 6 forces a failed save with a read-only `preferences.json` and restarts the launcher. The LX-02 stale-revision test guards a different case (a stale `expected_revision`), not a failed write. |
| G-03 | Checklist: Installation and prerequisites | Graphics and Play after install; install failures and cancellation | LX-12; steps 2 and 3 | Open | Step 2 cancels the install once and restarts it. |
| G-04 | Checklist: Play with lifecycle observation | Authentication and world entry; current native parity | Step 2 | Open | Step 2 ends in the world. "Actual SGW computer use" is the macOS Wine-identity gate and stays in the desktop ledger. |
| G-05 | Checklist: Repair | Native visual and original-client Repair UAT | Step 12 | Open | |
| G-06 | Checklist: Confirmed uninstall | Integrated confirmation, dismiss, removal and recovery UAT | LX-10 (uninstall of an adopted folder); step 12 | Open | Run Uninstall on a fresh install and on an adopted one (step 4's folder): the adopted one must leave unlisted user files. |
| G-07 | Checklist: Signed GitHub manifest patch notes | Refresh and reconnect after the integrated changes | Steps 2 and 11 | Open | Step 2 opens patch notes after install; step 11 checks they refresh after a self-update relaunch. |
| G-08 | Checklist: Default-off optional summary consent | Native Windows CI result; consent copy and frontend UAT | None in this campaign | Open | Turning summaries on is [out of scope](README.md#out-of-scope); the later, separately decided summaries rollout closes it. Only the Windows CI result can land here, through `launcher-desktop.yml` (LX-29). |
| G-09 | Checklist: Focused observability | Production endpoint, fixtures checked against a live SigNoz | None in this campaign | Open | Same rollout as G-08. Game telemetry (LX-08) is a different pipeline; step 9 does not close this gate. |
| G-10 | Checklist: Migration and identity/consent preservation | Adopted Update and Windows adoption | LX-10, LX-03; steps 4, 5 and 12 | Open | D-LX12 approved 2026-10-10 (adopt in place). Step 12 runs a game Update on the step 4 folder. The macOS verified-copy adoption and adopted-Play items stay in the desktop ledger. |
| G-11 | Checklist: Single updater owner and version/asset mapping | Real packaged Apply, restart and rollback; download resume; production configuration; Windows replacement parity | LX-13, LX-15, LX-17; step 11 | Open | Production configuration needs D-LX9. Step 11 interrupts the network once during the download. |
| G-12 | Checklist: Tests, docs and project memory | Per-packet guards, fresh bounded review, matching docs and indexes | Every packet's review; LX-23, LX-24, LX-27 | Open | Closed at close-out, not by a UAT step. |
| G-13 | Checklist: Windows native parity | Corrected native tests, FDI preflight, latest helper, updater and adoption parity | LX-10, LX-13, LX-29; steps 2, 4 and 11 | Open | Step 2's RAR install exercises the FDI cabinet preflight and the helper. LX-29 owns the native proofs from the desktop ledger ("Earlier integration wave" and its worknotes): a green native Windows `launcher-desktop.yml` run that executes tests; the fail-on-revert proof that swapping `read_open` for `read` fails `native_preparation_reads_locked_owner_and_retains_exclusion` on Windows ([lock-fix worknote](../playtests/2026-10-03-macos-wine/worknotes/launcher-launch-lock-fix.md)); and revalidation of the `native_uat` import guard ([updater hand-off worknote](../playtests/2026-10-03-macos-wine/worknotes/updater-handoff-fix.md)), since run 37223140800 executed no tests. |
| G-14 | Checklist: Final self-contained packages (LAST) | Offline first open, bundled dependencies, clean-machine checks, per-OS artifacts | LX-11, LX-12, LX-15; step 1 | Open | Windows only here. Code signing is deferred (D-LX10); notarization and the macOS artifact stay in the desktop ledger. That ledger makes this the last gate: if any packet lands after step 1 passes, repeat step 1 on the final zip. |
| G-15 | Section: Windows troubleshooting handoff | The updater ShellExecute test stall, [#1194](https://github.com/SandboxServers/Cimmeria/issues/1194), and its unapplied candidate patch | LX-13 | Open | LX-13 replaces the installer hand-off with a zip swap. If no ShellExecute path remains, close #1194 in the LX-13 PR as obsolete; otherwise apply and prove the candidate there. |

Left in the desktop ledger as macOS only: "Actual Mac graphics/login/world", and the macOS parts of G-04, G-10 and G-14 named in their Notes.

## LX-26 results

The owner runs steps 1-12 on a Windows machine with no launcher state, and step 13 on a Mac, each from the `launcher-current` download. Record `Pass` or `Fail`, the date, the release tag, and for a fail the first error and a log path. The step text is the canonical list in [work-packets.md](work-packets.md#lx-26-windows-uat-of-the-released-zip).

| Step | What to check | Packets | Gates | Result | Date and tag | Notes |
|---|---|---|---|---|---|---|
| 1 | Download and unzip from the stable link; first run with WebView2 present, an offline reopen, and once on a machine or VM without WebView2 | LX-11, LX-15 | G-01, G-14 | | | |
| 2 | Fresh install from the archive.org RAR, cancelled once and restarted; patch notes open; Play enters the world | LX-15 | G-03, G-04, G-07, G-13 | | | |
| 3 | PhysX missing: setup offers and installs it | LX-12 | G-03 | | | |
| 4 | Adopt an existing egui install in place; Play works without a reinstall | LX-10 | G-10, G-13 | | | |
| 5 | Import egui settings; the server list carries over | LX-03 | G-10 | | | |
| 6 | Server list edit, client-patches toggle off and on, both visible in "Changes to your client"; keyboard-only pass; forced failed save and restart | LX-03, LX-04, LX-07 | G-01, G-02 | | | |
| 7 | Reset cache; reset all with confirm | LX-06 | | | | |
| 8 | Upload logs twice; the second says nothing new | LX-05 | | | | |
| 9 | Telemetry opted in: a session shows in SigNoz with tailed log lines and exit code | LX-08a, LX-08b, LX-08c | | | | |
| 10 | Second launch focuses the first | LX-09 | | | | |
| 11 | Self-update from release N-1 to N with one network drop during the download; patch notes refresh; then a forced failed update rolls back | LX-13, LX-15, LX-17 | G-07, G-11, G-13 | | | |
| 12 | Repair, game Update and Uninstall, on a fresh install and on the step 4 adopted folder | LX-10 | G-05, G-06, G-10 | | | |
| 13 | macOS: a Mac that has never run the app downloads the zip, uses Open Anyway once, installs and reaches character select | LX-28 | | | | |
