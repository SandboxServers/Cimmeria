# Windows Launcher Redesign (#1153)

> Type: how-to (campaign ledger). Audience: the coordinator session, reviewers and the owner running the UAT.
> Updated: 2026-10-03. Companions: [UAT checklist 2026-10-03](uat-2026-10-03.md), [launcher guide](../../client/launcher-guide.md) (players), [sgw-launcher.md](../../client/sgw-launcher.md) (engineering reference), [unified UAT guide § Windows launcher](../../guides/unified-uat.md#windows-launcher), issue [#1153](https://github.com/SandboxServers/Cimmeria/issues/1153).

## Goal

Apply the approved dark, single-game Install/Play design to the existing
Windows `sgw-launcher` (egui), without a webview, a backend rewrite or a
macOS target. The old window was one long form: install folder, raw
configuration, four launch buttons, diagnostics and recovery tools. The
new one shows one installation surface that turns into **Play**, a
**Patch Notes** tab, a settings gear, and the diagnostics opt-in in the
footer of every view. The approved prototype is at `e70b076a9`
(`crates/launcher/prototype-macos/`).

## Scope by PR

The issue ships in three PRs. Each one is a release the owner can UAT on
its own.

| PR | Scope | Status |
|---|---|---|
| **PR 1** | Dark-only UI in layout A (gate side panel that folds away below 760 px, main column with update banner, state heading, Play / Patch Notes tabs, gear). The Play surface reducer (`app/play_state.rs`) and one primary action. Game lifecycle with or without telemetry (`GameExited`, `GameUntracked`), the process probe (`game_process.rs`), the worker's `Activity` guard and `Refused` event. Diagnostics footer (default off, saved at once, session-vs-next-launch caption, save errors shown). Patch Notes from the verified manifest only. Game settings: install folder, **Open in Explorer**, the checked folder change. Advanced holds every tool the old form had. The one-time telemetry prompt is removed. **Repair game** and **Uninstall…** are drawn but disabled. | In review |
| **PR 2** | **Repair game**: a real recovery operation that can restore a missing or damaged managed file even when the ledger says everything is installed, reusing signed downloads, hash checks and safe extraction. Progress, cancel and errors on the same surface; player settings kept. Backend guard like Install. | Not started |
| **PR 3** | **Uninstall…**: a confirmed removal of the validated game installation only, preserving per-user settings and cache, refusing unsafe or ambiguous roots and junction traversal, and reporting partial failure honestly. Backend guard like Install. | Not started |

## Decisions

| # | Decision | Source |
|---|---|---|
| D-LR1 | Production arrangement: **layout A** (split: gate panel beside the main column). The narrow layout (below 760 px) drops the gate panel, as the prototype does. | PR 1 implementation (`app/shell.rs`); #1153 required the maintainer to pick A, B or C, so confirm this before release |
| D-LR2 | Dark only, also under Windows light mode: `theme::apply` locks egui's dark theme and overwrites it with the prototype palette. | #1153 acceptance criteria |
| D-LR3 | Running state comes from lifecycle evidence, not telemetry: the launcher follows every game it starts, and a process probe finds the rest. The probe counts an unreadable `SGW.exe` as running (conservative). | #1153; `game_process.rs` |
| D-LR4 | Conflicting commands are refused in the worker as well as disabled in the UI (`worker/activity.rs`). | #1153 acceptance criteria |
| D-LR5 | Install / Update is not relabelled Repair. Repair and Uninstall stay disabled until their own PRs. | #1153 Evidence ("Repair is not an existing integrity scan") |
| D-LR6 | The telemetry prompt is replaced by the always-visible footer; `prompt_answered` is still written. | PR 1 |

## Tests and UAT

Automated coverage for PR 1 is listed in
[sgw-launcher.md § The process probe](../../client/sgw-launcher.md#the-process-probe-srcgame_processrs)
(the **Tests** paragraph): the reducer, worker guards, process-probe path
matching, Patch Notes projection, folder check, Open in Explorer refusal,
and the diagnostics save round trip.

What the automated tests do **not** cover, and the owner's UAT must:
how the window looks under both Windows themes and at other DPI scales,
keyboard focus, a real install against the published manifest, a real
`SGW.exe` launch with the client patches injected, a real login and the
Black Market window, detection of an Atera-launched game, and the
Toolhelp walk on a real process list.

**JS REPL-style logic UAT ([AGENTS.md](../../../AGENTS.md)), 2026-10-03:
13 of 13 scenarios passed** on a line-for-line Node model of
`play_state` (apply, `primary_action`, guards), the diagnostics caption
and the Patch Notes projection. The scenarios were: install to Play;
cancel and failure re-reading the ledger; a refused install returning to
Install; Play to running to exit and back to Play, with diagnostics on and
off; another pid's exit ignored and a crash flagged; a probe-found game
blocking Play and file actions; an untracked launch handing over to the
probe; the launcher-version gate; progress kept across tab and settings
changes; a stale-URL manifest ignored and a failed refresh keeping the
verified list; caption wording; the preference round trip; and the notes
projection. The model is supplementary, and the Rust tests are the proof.
Not covered by the REPL: the real filesystem, processes, egui rendering,
and the worker's async tasks.

Checklist: [uat-2026-10-03.md](uat-2026-10-03.md) (steps LR-01 to LR-44).

## Known gaps and follow-ups

- **Layout A needs the maintainer's confirmation.** The session driver
  chose it on 2026-10-03. #1153 asks the maintainer to pick the
  arrangement, so D-LR1 is confirmed only when the maintainer accepts
  this PR.
- **Fixed in PR 1 after review:** the client-state resets and Fix ASLR
  now check `Activity` (`Busy::Files`). The folder change re-checks the
  probe at the moment of the change. The writability probe no longer
  creates the install folder. "SGW.exe is missing" now gives the real
  workaround instead of Install, which skips a seed the ledger records.
- **"Changes to your client" moved under a collapsed Advanced.** It used
  to be visible before the first install; now a player sees it only by
  opening Settings › Advanced.

## Packet ledger

| Packet | What | PR | State |
|---|---|---|---|
| LR-P1 | UI redesign, lifecycle, guards, diagnostics footer, Patch Notes, settings | PR 1 | In review |
| LR-P1-UAT | Owner UAT of PR 1 on Windows ([checklist](uat-2026-10-03.md)) | — | Waiting for a launcher release |
| LR-P2 | Repair game | PR 2 | Not started |
| LR-P3 | Uninstall | PR 3 | Not started |
