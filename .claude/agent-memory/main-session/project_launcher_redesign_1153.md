---
name: project-launcher-redesign-1153
description: "#1153 Windows launcher redesign: layout A chosen 2026-10-03, phased into three PRs; PR 1 (shell, lifecycle, guards) built, Repair and Uninstall still to do. Read before touching crates/launcher/src/app or the worker lifecycle."
metadata:
  type: project
---

**Decisions (2026-10-03, session driver).** Of the approved prototype layouts at `e70b076a9` (`crates/launcher/prototype-macos/`), the session driver picked **A (split: gate panel left, main column right)**. The ticket asks the maintainer to choose, so A stands only once the maintainer accepts the PR. The work is phased: **PR 1** = dark egui shell, Play/Patch Notes tabs, gear settings with Advanced, pinned diagnostics footer, game-lifecycle events and worker guards, Explorer and the folder-change flow, with Repair/Uninstall shown but disabled. **PR 2** = real Repair, **PR 3** = confirmed Uninstall. Each needs its own tests and docs. The ticket's acceptance list is the scope.

**What PR 1 established (verified by tests in the PR).**

- A launch without telemetry used to drop the game's exit future, so nothing reported the exit. `worker/launch_sgw.rs::notify_exit` now wraps every followed launch and emits `Event::GameExited`. `run_session` `tokio::spawn`s the waiter, so the notification survives a telemetry session that ends early.
- `worker/activity.rs` is the backend guard. Install and Adopt are refused while an install runs or the game runs; a second launch is refused while one is starting or running. A refusal comes back as `Event::Refused`. The guard also asks `game_process::running_game_pids`, a Toolhelp probe for `SGW.exe` under the install folder that counts an unreadable image as running. That covers Atera-bat launches and a launcher window reopened while the game plays.
- `app/play_state.rs` is the pure reducer behind the Play surface. The main things PR 2 and PR 3 hang off are `file_action_block`, which Repair and Uninstall must also obey, and `primary_action`.
- `ManifestFetched`/`ManifestError` now carry their URL, and the UI drops a manifest fetched for a URL it no longer uses.

**Open for PR 2 and PR 3.** The ledger (`launcher-installed.json`) records patch keys and the seed hash, not a per-file inventory, and an adopted seed is unverified. Repair therefore cannot be "re-run Install". It has to re-extract the seed and patches from signed, hash-checked blobs. Uninstall needs junction-safe deletion scoped to the validated install folder.
