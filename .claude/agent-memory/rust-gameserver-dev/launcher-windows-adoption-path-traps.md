---
name: launcher-windows-adoption-path-traps
description: Desktop launcher adoption on Windows (LX-10) - verbatim \\?\ roots, zip enclosed_name backslashes, index names, file identity, and the <root>/game layout invariant
metadata:
  type: project
---

Facts found making adoption run natively on Windows (LX-10, 2026-10-11):

- Desktop-owned installs are always `<owner root>\.cimmeria-install.json` +
  `content-ready.json` + `<root>\game\`. Play, Repair (whole-tree rename swap),
  Update and Uninstall all hard-code `destination.join("game")`. "In place"
  (D-LX12) therefore renames the release files into `<imported folder>\game\`;
  everything unlisted stays outside `game` and is never touched.
- Legacy-import paths are `canonicalize()`d, so on Windows they are verbatim
  `\\?\C:\...`. Joining `"Working/Binaries/x"` onto a verbatim root keeps `/`
  literally and fails. Join per component (`adoption::inventory::native`).
- `zip::ZipFile::enclosed_name()` returns `\` separators on Windows; the
  adoption `strict_zip` used its string and refused every nested entry as
  `InvalidArtifact`. Use `entry.name()` for the string checks.
- `inventory::scan` index names must be `/`-joined components on every host.
- `inventory::identity` was unix-only (Windows returned `UnsafeFile`); the
  Windows version uses `GetFileInformationByHandle` (volume serial + file
  index, opened with `FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT`).
- The shell test binary for `npm run uat:adoption` is the lane's
  `target/desktop/debug/deps/cimmeria_launcher_desktop-*.exe`, read from the
  "Running unittests (...)" line of the lane log.

**Why:** the macOS-only adoption tests hid all of these; the first Windows run
failed every preview with `InvalidArtifact`.

**How to apply:** any new Windows path built from an index name or plan string
goes through component joins; check `cfg(unix)` helpers before reusing adoption
code on Windows. See [[launcher-preferences-schema-change-fanout]].
