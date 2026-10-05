---
name: reference_desktop_launcher_windows_native
description: First native Windows run of the desktop launcher (2026-10-04) — what a dev build needs, the six Windows-only defects found, and what was and was not proven
metadata:
  type: reference
---

Learned 2026-10-04 building and running `crates/launcher/desktop` natively on Windows 11
from `launcher/desktop-client-telemetry`, on a host with no prior toolchain.

- **A dev build needs the release manifest key.** Without `LAUNCHER_MANIFEST_PUBKEY_HEX`
  at compile time a debug build verifies with the public development key, and Install
  refuses the real `content-current` manifest with "The release could not be verified."
  The key is recorded in `crates/launcher/desktop/docs/wine-validation.md`; it was checked
  against the live `manifest.json` and `manifest.json.sig` on that date. The UI gives the
  same message for a bad signature, a malformed manifest and an oversized one.
- **Play needs three staged resources** beside the exe (`windows/`): the i686 launch
  worker, the client-patches DLL and, for opted-in players, the telemetry DLL, each pinned
  by a compile-time SHA-256. A plain `cargo build` copies `shell/resources/windows/` next
  to the exe, so no bundle step is needed for a development run.
- **Verbatim paths stop the game.** `canonicalize()` returns `\\?\C:\...` on Windows.
  `SGW.exe` given that as its working directory quits with "Failed to find default engine
  .ini file". The launch worker now strips it; the Windows launcher always did
  (`crates/client-launch/src/launch.rs`).
- **`ShellExecuteExW` without `SEE_MASK_FLAG_NO_UI` blocks on a dialog** when the target
  is missing. This was the cause of the updater handoff test hanging in CI (#1194),
  observed directly: the dialog's title is the missing path, and the test passed once the
  dialog was closed.
- **Windows file locks exclude reads through a second handle.** A test that reads
  `.cimmeria-install.json` with `std::fs::read` while an `OwnerLock` is held fails with
  OS error 33. Read through the lock's own handle.
- **A closed socket with unread data resets the peer on Windows.** A loopback fixture
  that writes a response in several pieces and unwraps each write is flaky there.
- **`tools/stage-helper.py` runs on Windows** since it closes its temp file before the
  rename. Two of its unit tests still fail there because they assert POSIX mode bits;
  CI runs them on macOS only.
- **Git Bash, not `bash`.** On a Windows host with WSL installed, `bash` on PATH is the
  WSL shim. `build-native.sh` and the lane need Git Bash called by its full path.

Proven that day: fresh Install from the published release, Play, login, character select
and cooked-cache streaming. Not exercised: Repair, uninstall, game Update, launcher
self-update, adoption of an existing copy, and a packaged (bundled, signed) build.
