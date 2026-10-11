# Launcher Consolidation Announcement

> Type: how-to. Audience: the maintainer who posts the announcement (Discord and the GitHub release).
> Updated: 2026-10-11. Companions: [ledger and decisions](README.md), [player guide](../../client/launcher-guide.md), [Playing on a Mac](../../guides/macos.md), [desktop launcher release](../../../crates/launcher/desktop/docs/release.md).

D-LX2 settled how players move off the old launcher: **announce only**, no bridge
build. The old `sgw-launcher.exe` stops getting updates, and players are told on
Discord, in the README and in the release notes to download the zip. This page
holds the text to post and when to post it. The README part is done (LX-22).

## Before you post

Post only after a release has refreshed `launcher-current`. Until then the stable
links below answer 404.

1. A maintainer comments `/release-launcher` on a merged PR. The run must be
   signed (the `launcher-release` environment's Minisign secrets, D-LX9) and the
   day's first release, or it does not refresh `launcher-current`
   ([release](../../../crates/launcher/desktop/docs/release.md#what-a-release-publishes)).
2. Open both stable links in a browser and check each downloads a zip:
   - <https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip>
   - <https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-macos-arm64.zip>

   If the macOS build failed, `launcher-current` has no macOS zip yet. Re-run the
   failed jobs within a day (the release guide says how), or drop the macOS
   paragraph from the Discord text until it is there.
3. Post the Discord text, then add the release-notes text to that release.

## Discord

Paste as one message in the announcements channel:

```text
**New Stargate Worlds launcher**

The old launcher (sgw-launcher.exe) is retired and will not update itself any more. Please switch to the new launcher now.

**Windows:** download https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip
Unzip it into a folder of its own (not Program Files) and run cimmeria-launcher-desktop.exe. Keep the windows and graphics folders next to it. That link always has the newest version, and the launcher updates itself from Settings > Launcher updates.

**Already installed the game with the old launcher?** You don't have to download the game again. In the new launcher open Settings (the gear), use "Select legacy folders..." under Import legacy launcher settings, then "Verify and adopt this folder..." to take over your existing install. After that, start the game from the new launcher only; shortcuts to SGW.exe stop working.

**Mac (Apple silicon):** there is now a Mac app: https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-macos-arm64.zip
Drag it to Applications, open it once, then allow it under System Settings > Privacy & Security > Open Anyway. No WoWSilicon or CrossOver needed. Mac updates are manual for now: download the zip again when we announce a new version.

The launcher isn't code-signed yet. If Windows shows "Windows protected your PC", choose More info > Run anyway.

Full guide: https://github.com/SandboxServers/Cimmeria/blob/main/docs/client/launcher-guide.md
Mac guide: https://github.com/SandboxServers/Cimmeria/blob/main/docs/guides/macos.md
```

## Release notes

Every dated `launcher-YYYYMMDD-<sha7>` release's notes already carry a short
paragraph for players of the old launcher; `tools/launcher-release/publish.sh`
writes it on every release, so nothing needs doing for later releases. For the
**first** release after the retirement, add this longer notice at the top of its
notes by hand (**Edit release** on GitHub). A re-run of that release's workflow
rewrites the notes, so add it again after a re-run.

```markdown
## The old launcher is retired

`sgw-launcher.exe` no longer receives updates, and no release builds it any more.
This release, and every one after it, ships the new Stargate Worlds Launcher instead.

- **Windows:** download [`StargateWorlds-Launcher-windows-x64.zip`](https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip),
  unzip it into a folder of its own and run `cimmeria-launcher-desktop.exe`, keeping
  the `windows` and `graphics` folders beside it. It updates itself from
  **Settings → Launcher updates**.
- **Moving from the old launcher:** **Settings → Import legacy launcher settings**,
  then **Verify and adopt this folder…** takes over the game the old launcher
  installed, without downloading it again.
- **macOS (Apple silicon):** download [`StargateWorlds-Launcher-macos-arm64.zip`](https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-macos-arm64.zip)
  and follow [Playing on a Mac](https://github.com/SandboxServers/Cimmeria/blob/main/docs/guides/macos.md).
  macOS updates are manual.

The launcher is not code-signed. If SmartScreen or Defender blocks it, see the
[launcher guide](https://github.com/SandboxServers/Cimmeria/blob/main/docs/client/launcher-guide.md#windows-defender-or-smartscreen-blocks-the-launcher).
```

## After posting

Record the release tag and the date you posted in the ledger's LX-27 close-out.
Questions from players about adoption usually come down to the rules in the
[player guide](../../client/launcher-guide.md#coming-from-the-old-launcher): the
old launcher must have finished installing every current patch, and the folder
must have the full `Working\Binaries` layout.
