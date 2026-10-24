---
title: "Launcher Guide"
type: how-to
audience: players, operators
last_updated: 2026-10-23
companion_docs:
  - ../guides/macos.md
  - ../../crates/launcher/desktop/README.md
  - launcher-self-update.md
  - launcher-distribution-setup.md
---

# Launcher Guide

How to get Cimmeria Launcher, install the game with it and keep
both up to date, and how operators publish the game patches it installs.

There is one launcher: **Cimmeria Launcher**, a desktop app for
Windows and for Apple silicon Macs. It replaced the old single-file launcher,
`sgw-launcher.exe`, which was retired on 2026-10-11 and no longer updates
itself. If you still use that one, read
[Use an existing game folder](#use-an-existing-game-folder).

> **Audience split:**
> [Part 1 — Players](#part-1--for-players) is for anyone installing and
> playing the game. [Part 2 — Operators](#part-2--for-operators-publishing-patches)
> is for whoever publishes the game patches that every launcher installs.
> Skip the one that isn't you. On a Mac, follow
> [Playing on a Mac](../guides/macos.md) instead of the Windows steps below.

How the launcher is built and released is in the
[desktop launcher README](../../crates/launcher/desktop/README.md) and its
[release guide](../../crates/launcher/desktop/docs/release.md). The one-time
operator setup (content releases and the two signing keys) is
[launcher-distribution-setup.md](launcher-distribution-setup.md).

---

## Contents

- [Part 1 — For players](#part-1--for-players)
  - [Download the launcher](#download-the-launcher)
  - [First start on Windows](#first-start-on-windows)
  - [First start on Linux and Steam Deck](#first-start-on-linux-and-steam-deck)
  - [Install the game](#install-the-game)
  - [Install PhysX](#install-physx)
  - [Play](#play)
  - [Use an existing game folder](#use-an-existing-game-folder)
  - [Use a game archive you already downloaded](#use-a-game-archive-you-already-downloaded)
  - [Main-page preferences and changes](#main-page-preferences-and-changes)
  - [Settings](#settings)
  - [Launcher updates](#launcher-updates)
  - [Where your files are](#where-your-files-are)
  - [How the client finds its server](#how-the-client-finds-its-server)
  - [Install layout](#install-layout)
- [Part 2 — For operators (publishing patches)](#part-2--for-operators-publishing-patches)
  - [Step 1 — Build the seed](#step-1--build-the-seed)
  - [Step 2 — Build a patch zip](#step-2--build-a-patch-zip)
  - [Step 3 — Write the manifest](#step-3--write-the-manifest)
    - [Schema versioning policy](#schema-versioning-policy)
    - [Minimum launcher version (`min_launcher`)](#minimum-launcher-version-min_launcher)
  - [Step 4 — Sign + publish to GitHub Releases](#step-4--sign--publish-to-github-releases)
  - [Append-only invariants](#append-only-invariants)
  - [Future automation](#future-automation)
- [Troubleshooting](#troubleshooting)

---

## Part 1 — For players

### Download the launcher

Download the newest launcher from its stable link. The link never changes and
always holds the newest release:

- **Windows (64-bit):** [`StargateWorlds-Launcher-windows-x64.zip`](https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip)
- **Windows on Arm:** [`StargateWorlds-Launcher-windows-arm64.zip`](https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-arm64.zip).
  On an Arm PC, download this one. The x64 zip also runs there, emulated, but it
  stays x64 and does not move to arm64 on its own. The game itself runs under
  Windows' x86 emulation either way.
- **macOS (Apple silicon):** [`StargateWorlds-Launcher-macos-arm64.zip`](https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-macos-arm64.zip),
  set up as [Playing on a Mac](../guides/macos.md) describes.
- **Linux and Steam Deck (x86-64):** [`StargateWorlds-Launcher-linux-x86_64.AppImage`](https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-linux-x86_64.AppImage).
  It installs the game; Play is not available on Linux yet. See
  [First start on Linux and Steam Deck](#first-start-on-linux-and-steam-deck).

Each download has a `.sha256` file beside it on the same release if you want to
check your download.

Unzip the Windows zip into a folder of its own that you can write to, such as
`%LOCALAPPDATA%\Stargate Worlds Launcher`. Not `C:\Program Files`: the
launcher updates itself by replacing its own files, which needs a folder it
can write without admin rights. New ZIPs hold just:

```text
cimmeria-launcher-desktop.exe   the launcher; start this one
```

The launcher carries its helper and DLLs inside the EXE. On startup it checks
and prepares its private cache automatically; you do not need companion
`windows` or `graphics` folders. Older loose files can remain after an update;
new builds do not use them. The download stays a ZIP. On Mac, the ZIP already
contains one **Cimmeria Launcher.app** bundle with its helper resources inside.
WebView2, the game download and the Mac Wine runtime remain separate.

### First start on Windows

The launcher is not code-signed (that is deferred), so Windows may stop the
first start:

- **SmartScreen, "Windows protected your PC":** choose **More info → Run
  anyway**. Only do this for a launcher you downloaded from the stable link
  above.
- **"The Microsoft Edge WebView2 Runtime is needed to run the launcher":**
  the launcher draws its window with WebView2. Windows 11 has it, and Windows
  10 usually does. Answer **Yes** to open Microsoft's download page, install
  the runtime, then start the launcher again.
- **Defender quarantines a file:** see
  [Windows Defender or SmartScreen blocks the launcher](#windows-defender-or-smartscreen-blocks-the-launcher).

The **Cimmeria Launcher** window opens on **Play**. The gear button
(⚙, "Game settings") opens **Settings** for folders, maintenance and recovery.
The main page scrolls when the window is small. Below **Play**, you can toggle
client patches, launcher diagnostics and game diagnostics. **Changes to your
client** shows a summary; expand **View all changes** for the full list. There
is no Patch Notes tab.

### First start on Linux and Steam Deck

The Linux launcher is one AppImage file. On a Steam Deck, use Desktop Mode.

1. Download `StargateWorlds-Launcher-linux-x86_64.AppImage` from the stable link
   above. Keep it somewhere you can find again, such as `~/Applications`.
2. Make it executable: in the file manager, right-click it, choose
   **Properties → Permissions** and tick **Is executable**. Or in a terminal:
   `chmod +x StargateWorlds-Launcher-linux-x86_64.AppImage`.
3. Double-click it, or run `./StargateWorlds-Launcher-linux-x86_64.AppImage`.

What works on Linux today:

- **Install** downloads and installs the game the same way as on Windows, and
  **Continue** resumes an interrupted install. The default game folder is
  `~/.local/share/app.cimmeria.launcher.desktop/Stargate Worlds`.
- **Use a game archive you already downloaded** works as on Windows.
- **Play, Repair, Update and Install PhysX are not available on Linux yet.**
  Running the game needs Proton, which a later launcher release will set up.
- The launcher does not update itself on Linux. Download each new release from
  the same link.

### Install the game

The launcher installs the original 2009 client (build 0.8348.1.4046) from
archive.org, then applies Cimmeria's patches to it. You don't need the old
`SetupQA.exe` installer.

1. Open **Settings** (⚙). **Local install directory** shows where the game
   goes. On first start it is a `Stargate Worlds` folder under
   `%LOCALAPPDATA%\app.cimmeria.launcher.desktop`. To use another place,
   click **Choose folder…** and pick an empty folder, or a new folder inside
   one that exists. The launcher refuses a folder that already has files in
   it.
2. Click **Install Cimmeria**. A progress bar shows the percent and
   size, plus the speed and time left for large downloads, and a percent
   while unpacking. The download is 4.1 GB, and the install drive needs about
   14 GB free while it unpacks.
3. When it finishes, the game is in a `game` folder inside the folder you
   chose.

**Cancel installation** stops the attempt. To start again, click **Remove
partial installation…**, confirm with **Remove partial files**, and install
again; that also deletes what was downloaded so far. If the launcher or the PC
stopped part-way instead, the launcher offers **Resume installation**, which
continues from where it stopped. Nothing is ever retried without you asking.

Once native inspection confirms setup is complete, **Recheck Play status** and
**Use a downloaded game archive…** disappear from the main page and stay hidden
while the game runs or a launcher update is offered. Settings keeps archive
selection and a separate **Check Play status** button; pressing it immediately
shows feedback, followed by the inspection result beside it. A launcher update can still require
**Update and restart** before Play. A previously completed adoption does not
need compatibility setup just because you reopened the launcher.

### Install PhysX

The game needs the legacy PhysX 2.6.3 core to start its physics engine. A newer
PhysX installation can lack that core. On Windows, the main page offers
**Install legacy PhysX…** whenever the required core is missing, even if you
adopted your game without its original installer.

1. Click **Install legacy PhysX…**. The launcher shows progress while it
   downloads and verifies NVIDIA's legacy 9.13.0604 installer.
2. Accept the Windows permission prompt, read the vendor license and finish
   the installer. Nothing installs automatically when you open the launcher.
3. If it says "Restart Windows before you play", restart.

The launcher checks for the core again after installation and whenever you
reopen it. The offer disappears when the core is found. A cancelled or failed
attempt shows a result and lets you retry. Missing PhysX warns that the game
may not start; the warning itself does not block Play.

**Settings → PhysX System Software → Install PhysX** still runs the verified
installer retained from the game archive, when present. If that older installer
is unavailable or another version blocks it, use the main-page legacy offer.
macOS continues to prepare PhysX inside Wine during compatibility setup.

### Play

Wait for the startup launcher-update check to finish. If **Update and restart**
appears on the main page, click it before playing; see [Launcher updates](#launcher-updates).
Then click **Play**. The launcher writes the login server list into the client,
loads the client patches (and, if you turned it on, the diagnostics module),
and starts `SGW.exe`. The line under the button then says "Game process
started. Login and world entry are not verified": the launcher sees the game
start and exit, not whether you logged in.

On the login screen, pick the server in the list and log in with your account.
Accounts are created by the server owner; ask for one.

If the line says "Update the launcher before playing this release", the
server's operator requires a newer launcher. See
[Launcher updates](#launcher-updates).

### Use an existing game folder

If the game is already on this PC, you don't have to download it again. Point
the launcher at its `SGW.exe`. You don't need the old launcher for this.

1. On the main page, click **Adopt an install** beside the other game actions.
   It appears only while no valid managed or adopted install exists. A valid
   install remains valid when Play is blocked by a runtime prerequisite, a
   launcher update requirement, or another operation. An invalid registered
   install must be repaired or uninstalled before another folder can be adopted.
   You can also open **Settings → Use an existing game folder → Choose SGW.exe…**.
2. The first press opens the adoption panel and immediately shows
   **Checking your game folder…**, then opens **Choose SGW.exe in your existing
   install**. Adoption uses
   your selected installation where it is on Windows, Linux and macOS; it never
   copies or relocates it into a launcher-managed game subfolder.
3. Select the existing client's `Working\Binaries\SGW.exe`. The picker
   filters for `.exe` files, but the launcher accepts only `SGW.exe` and
   resolves the installation root from it. Choosing another executable is
   refused. Cancelling changes nothing; selecting the file opens the review
   before any installation changes.
4. Read the review screen. It lists:
   - files that will be updated to the current release;
   - modified or missing files. The launcher replaces these only if you tick
     the box. Your originals go to a folder named
     `cimmeria-adoption-backup-<id>` in the game folder. The launcher never
     deletes that folder, not even when you uninstall.
   - what will be downloaded.

   A **Use it where it is** review reads differently: it says the game was
   found in your folder, how many patches are already applied and how many will
   be applied (with the download size), and, if there are any, which patch
   changes can't be applied (see below). It has no tick box for replacing
   files, because that mode never replaces or removes your files. The button
   reads **Use this folder**.
5. Tick the confirmations and click the confirm button on the screen.

The selected folder must be writable. If a protected location cannot be used,
choose an accessible game installation. Adoption does not relocate it for you.

#### What "can't be applied" means

A patch changes specific files from a known starting version. If one of your
files differs from that starting version (a modded file, or a different build),
the launcher can't apply that change to it. It skips the change, leaves your
file exactly as it is, and still lets you use the folder. The review lists those
files. The rest of the patch is applied. Nothing about a skipped change stops
the game from starting, but that part of the patch is missing until you repair.

After the adoption, the main view tells you how many patch changes were skipped.
**Repair** checks the signed release and repairs its managed files. For a new
adopted installation, repairs and updates apply individual changes at the
selected root and preserve unrelated files; they never swap or relocate the
whole folder. An Update of a stock folder adopted with **Use this folder**
downloads only the new patches and keeps your other files.

Nothing here has been tried by a player yet: this describes how the launcher is
built and tested, not a verified walkthrough.

Before you use a folder where it is, know what changes:

- The installation stays at its selected root. Start it through this launcher
  afterwards and do not run the old launcher on the same folder.
- Added files, including PAKs in `SourceCache.en-us`, remain in place and stay
  available to the game. Explicit reviewed content changes are allowed here:
  patching individual files, normalizing the stock cooked-data directory name,
  and keeping replaced originals in the displayed backup directory.
- Your game settings and the client cache live in `Documents` and are not
  affected.
- An adopted folder plays without the game diagnostics module for now, even
  with **Game diagnostics** on. A fresh install has it.
- With game diagnostics on, each step of the adoption is sent as it happens;
  offline, nothing is sent, and the adoption works the same.
- A folder that is behind the current release is fine. The review lists the
  files that will be updated, and the launcher downloads only the patches they
  need.

#### If the launcher refuses the folder

Adoption checks the whole game folder before it changes anything. If it finds
something it can't adopt safely, it lists up to five files or folders, each
with its path inside the game folder and what to do, and says how many more it
found. Nothing was copied, moved or removed.

| Reason | What to do |
| --- | --- |
| A link or junction | Put the real folder or file there. |
| A OneDrive or other cloud-synced file | Move the whole game folder out of the synced folder, for example to `C:\Games`. "Always keep on this device" is not enough, because the files stay cloud files. |
| A hard link | Replace it with a plain copy, or remove it. |
| Not a plain file or folder | Remove it or move it out. Files compressed with `compact /EXE` count here: run `compact /U /EXE /S` in the game folder to decompress them. |
| A name the launcher cannot use | Control characters, `:` or `\`, or a name that is too long: rename or move it. |
| Two names that differ only in case | Remove or rename one of them. |

The check covers the whole game folder, including folders you added such as
mods or backups. If the launcher shows the general "could not be checked
safely" message without a list, a file changed while it was checked: close the
game and any sync or backup program and try again.

#### Optional: bring settings from the old launcher

The old launcher (`sgw-launcher.exe`) no longer receives updates. You can bring
its settings over, so the folder you pick next plays with your login servers
and your client-patches setting. This is optional. Nothing in either folder
changes.

1. Close the old launcher. While it runs, the import is refused.
2. In **Settings → Import legacy launcher settings**, click
   **Select legacy folders…**. Two folder dialogs open one after the other:
   1. the folder that holds `sgw-launcher.exe` (its `launcher-config.json` is
      there);
   2. the game folder the old launcher installed into (by default
      `%LOCALAPPDATA%\Stargate Worlds`; its `launcher-installed.json` is there).
3. Check the review and click **Confirm settings import**. The launcher keeps a
   copy of the old launcher's settings. Import them before you pick the folder,
   and that folder plays with them.

If adoption stops part-way, use only the recovery actions the panel offers.
Recovery finishes the reviewed changes at the selected root. Abandoning leaves
content in place; inspect any setup changes or backups before trying again.
An interrupted adoption from an older launcher that had begun moving or copying
files must be abandoned without relocating those files. Inspect that partial
installation manually before adopting again. The full rules are in the
[migration contract](../../crates/launcher/desktop/docs/migration.md#adopting-an-existing-folder).

After adoption, the folder is the launcher's own: **Play**, **Repair game**,
game updates and **Uninstall…** work on it as on a fresh install. Uninstall
removes only what the launcher manages there and leaves your own files.

#### Why adoption downloaded the game archive

Adoption checks your files against the published file index for the release.
When that index is available and verified, the launcher compares your files
with it and downloads only the files it must replace. If the index is missing
or cannot be used, the launcher has no file list to compare against, so it
downloads the whole signed game archive instead, which is about 4.1 GB for the
current release, and builds the list from it. The launcher names the reason:

- This release has no published file index.
- The published file index is malformed.
- The published file index could not be downloaded.
- The published file index has the wrong size.
- The published file index does not match its signed digest.
- The published file index could not be read.
- The published file index does not belong to this release.
- The published file index could not be read from disk.

To skip the download, use a game archive you already have. See
[Use a game archive you already downloaded](#use-a-game-archive-you-already-downloaded).

#### Reading an adoption refusal

When the launcher refuses an adoption, it shows one line with up to four parts:

- **Check** names the test that failed, in plain words.
- **File** names the file the test was about, when one applies.
- **Expected** is the value the signed release requires, such as a size or a digest.
- **Found** is the value the launcher saw in your files.

Expected and Found appear only when the test compares two values. If this build
does not know a test, the line reads `Check:` followed by its internal name.

### Use a game archive you already downloaded

If you already downloaded the game archive, you can use that file instead of
downloading it again. You don't unpack it first.

1. Click **Use a downloaded game archive…**. It is in **Settings** and on the
   Play tab.
2. Pick the archive file.

The file must be the archive.org release
`Stargate Worlds (0.8348.1.4046) (2009-06-30) (beta).rar`, which is
4,135,724,034 bytes (about 4.1 GB). The launcher checks the size and the
fingerprint before it uses the file. A file that doesn't match is not used.

When no game installation is present and setup can begin, choosing a verified
archive starts installation immediately, without a second **Install** press.
The same applies when you accept the Downloads offer, from the main page or
Settings. You see verification progress first, then installation progress.

Cancelling the picker or verification does not start installation, even if a
previous archive is remembered. Reopening the launcher also does not start it.
An existing installation, unfinished operation or recovery state keeps its
normal controls; choosing an archive does not replace that installation or
bypass those checks.

- The launcher copies game files from the archive when it needs them. It never
  moves, changes or deletes your file.
- If that exact file is in the top level of your Downloads folder, the launcher
  offers it: "Found … in Downloads. Use it?" Nothing is read until you say Yes.
- **Forget** stops the launcher using the archive.
- The launcher remembers the archive only while it matches the current release.
  After a release change, pick the file again.
- The project never hosts the game archive. It comes from archive.org or from
  your own copy.

#### If this build cannot verify released game content

Use the [published launcher ZIP](#download-the-launcher). A development build
can carry a test verification key that cannot verify the published game
release. In that case the launcher stops before checking your archive and
leaves it unchanged; downloading the RAR again does not fix the build's key.

If you build the launcher yourself, compile it with the approved release
public verification key using the [developer instructions](../../crates/launcher/desktop/README.md#build-against-released-game-content).
You need the public key, not the private signing key. The launcher still
verifies both the released manifest and your archive. A connection failure
and a rejected release signature have separate messages and do not by
themselves prove that your archive is damaged.

### Main-page preferences and changes

These controls are below **Play**. Each save shows feedback and restores the
confirmed choice if saving fails. Diagnostics are optional and off by default;
expand **About diagnostics and privacy** to read what they share.
It lists the detailed data categories, local logs, automatic game-log ZIPs,
manual-upload behavior and stopping limits. Game diagnostics can include chat,
names, paths and persistent identifiers; they are not anonymous. The
[source-backed privacy inventory](../../crates/launcher/desktop/docs/privacy-inventory.md)
records exact collectors and the different redaction rules for each stream.

| Control | What it does |
|---|---|
| **Launcher diagnostics** | Off by default. When it is on, setup diagnostics are sent over https to your first login server, if that address is https. With the default server list (plain http) they are saved but not sent yet, and the box says so. Nothing is sent while it is off. The built-in fallback address is not set yet. |
| **Game diagnostics** | Off by default. When on, Play loads a small diagnostics module into the game, which sends game events and logs to your login server, so the developers can see what went wrong. It changes nothing in the game. When it is on, the launcher also sends its game-folder events (the steps of **Use an existing game folder**, and why a folder was refused) to your login server as they happen, without Play. They name files only by their place inside the game folder, never your user name or home folder. |
| **Client patches** | **Load client patches** is on by default. The patches fix parts of the 2009 client; the Black Market window, for one, needs them. Turn it off only to rule them out when something misbehaves. A directly adopted folder follows the saved choice on the next Play; a legacy-bound adopted client keeps its reviewed setting until Repair. |
| **Changes to your client** | Read-only. Every change the launcher makes to the stock client: the setup steps, each applied patch, the DLLs Play loads, and what the server sends while you play. |

Direct-folder adoption follows the current saved client-patches preference on
the next Play. A legacy-bound adopted client keeps the patch setting reviewed
with its imported settings until Repair rebuilds it; changing the checkbox still
saves your preference, and the adjacent status explains that binding.

Game diagnostics shows a compact status. Its detailed status stays collapsed
while idle and opens automatically when saving or reporting an error. You can
also expand it yourself; the full privacy explanation is under **About
diagnostics and privacy**.

### Settings

Everything below is in **Settings** (⚙). Every button shows a status line as
soon as you press it.

| Section | What it does |
|---|---|
| **Local install directory** | **Choose folder…** picks where a new install goes. **Show folder** opens the saved folder. Windows paths display without the internal `\\?\` prefix; the saved location stays the same. |
| **Check Play status** | Inspects native Play availability and shows the result beside the button, with feedback on the first press. Available here even when the main-page recheck is hidden after setup. |
| **Repair game** | Downloads the signed release again and replaces modified or missing game files in the installed folder. Confirm in the panel that opens. **Cancel** works until replacement starts. Your settings are kept. The [repair contract](../../crates/launcher/desktop/docs/repair.md) has the recovery buttons. |
| **Uninstall…** | Removes the game files and modifications from the installed folder after you confirm with **Uninstall game**. Launcher settings stay. |
| **Login servers** | The servers the game's login screen lists, one `Name = http://host:port` per line. **Save servers** saves them; a line the launcher cannot read is shown under the box and nothing is saved. Empty means the default Cimmeria server. |
| **Debug logs** | **Upload debug logs** sends the launcher's own log lines and the game's `SGWDebugLog` and session log lines to the project's diagnostics server, in a few paced batches. It sends only lines it has not sent before, at most 20,000 launcher lines and 20,000 game lines per press, and shows "Sent N log lines. Reference `<id>`." when it is done. Quote the reference when you report a problem. File paths and addresses are removed first. Game logs can include chat and character names. Pressing the button sends the logs even when **Game diagnostics** is off. |
| **Client cache and settings** | **Reset client cache** clears the game's downloaded data cache in `Documents\My Games\Firesky\SGWGame\Cache.en-US`; the game refills it from the server at your next login. **Reset all client state…** deletes the whole `Documents\My Games\Firesky` folder (your game settings, key bindings and cache) after you confirm with **Delete all client state**. Your characters on the server are not affected. Both refuse while the game runs. |
| **PhysX System Software** | Windows only. See [Install PhysX](#install-physx). |
| **Game updates** | **Check for game updates** looks for a newer signed release of the game patches; **Update game…** applies one after you confirm. The update is built beside your install and swapped in, so a failed update leaves the old one working. |
| **Launcher updates** | See [Launcher updates](#launcher-updates). |
| **Use an existing game folder**, **Import legacy launcher settings** and **Use a downloaded game archive** | See [Use an existing game folder](#use-an-existing-game-folder) and [Use a game archive you already downloaded](#use-a-game-archive-you-already-downloaded). |

The **Settings** heading also shows the launcher's version and release tag.

### Launcher updates

The Windows launcher checks the signed release feed when it starts. **Play**
waits while the check runs. When a newer launcher is available, the main page
shows **Update and restart**, and you must update before playing.

Click **Update and restart** once. The launcher downloads the zip, verifies its
signature and version, puts the files in place and restarts. It shows progress
from the first press; leave it open while this runs. A previously downloaded and
verified package is reused. The old files remain as `<name>.old` until the new
launcher starts and confirms its version, then they are deleted. Checking at
startup does not download or install anything without your click.

If the check fails before any update is known, the launcher shows the failure
and a retry button, and you can play after the check finishes. If it already
knows about an update, a failed recheck keeps **Play** blocked. Settings retains
**Check for updates**, **Download update**, **Apply and restart** and
**Recheck update status** for troubleshooting.

If anything fails before the new launcher starts, the old files are put back
and the section says so, for example "The update could not start. The original
launcher was restored when its files could be verified." or, for a package
that fails its check, "The package signature or signed release version could
not be verified. Nothing was installed." Update between game sessions: the launcher
refuses to update while the game, an install, a repair or a game update is
running.

macOS updates are manual: download the new zip from the stable link and
replace the app. There, and in any launcher you built yourself, the **Launcher
updates** section says launcher updates are unavailable, and **Check for
updates** is greyed out.

How the update is verified and swapped is in
[launcher-self-update.md](launcher-self-update.md).

### Where your files are

| What | Where (Windows) |
|---|---|
| The launcher | The folder you unzipped it to |
| Verified helper/DLL cache | `%LOCALAPPDATA%\app.cimmeria.launcher.desktop\payloads\<content-digest>\windows` (managed by the launcher) |
| Launcher settings and state | `%APPDATA%\app.cimmeria.launcher.desktop\state` |
| The game, by default | `%LOCALAPPDATA%\app.cimmeria.launcher.desktop\Stargate Worlds\game` |
| Your game settings and the client cache | `Documents\My Games\Firesky\SGWGame` |
| Game logs | `game\Working\Binaries\SGWDebugLog.log` and `game\Working\Binaries\sessions\` |

Uninstalling the game or deleting the launcher folder leaves the launcher
settings. Delete the state folder too if you want the launcher to start fresh.

### How the client finds its server

The client's login screen lists the servers `LoginInternal.lua` defines. The
stock file points at the original developers' long-dead servers, so the
launcher writes the **Login servers** list into it before every Play. Edit the
list in Settings, not the file: Play rewrites it. A Cimmeria login server
listens on `http://<host>:8081`.

Play also switches off ASLR in `SGW.exe` (one byte in its header), because the
client patches and the project's reverse-engineering notes assume `SGW.exe`
loads at `0x00400000`. The stock exe with that byte cleared is byte-identical
to a known-good QA client's.

### Install layout

A full install has the original installer's layout, inside the `game` folder:

```text
<install>\game\Common\...
<install>\game\Resources\...
<install>\game\Working\Binaries\SGW.exe   (plus its DLLs, SGWDebugLog.log, sessions\)
<install>\game\Working\SGWGame\...
```

The client reads cooked data (`Cooked*.pak`, `TextStrings.pak`, …) from
two tiers, set in `Working\Engine\Config\GameplayEngine.ini`:

| Tier | INI key | Where it is | Who writes it |
|---|---|---|---|
| Writable | `CachePath=..\SGWGame\Cache` | `Documents\My Games\Firesky\SGWGame\Cache.en-US` | The server's version push, on every connect |
| Bundled, read-only | `SourceCachePath=..\SGWGame\SourceCache` | `Working\SGWGame\SourceCache.en-us` | Nothing at runtime. The client copies a PAK from it over the writable one when its version is newer (`client.cooked_cache.write` with `tier = source_cache`) |

The 2009 cabinets put the bundled PAKs in `Working\SGWGame\Cache.en-US`,
which is neither path, and a stock install logs a
`Non-existent source archive directory` warning for each cooked-data category.
So after unpacking, the installer renames that folder to `SourceCache.en-us`.
It is also why a PAK placed by hand in `SourceCache.en-us` stays put, while
one placed in the Documents `Cache.en-US` is rewritten the next time the client
connects. **Reset client cache** wipes only the Documents tier.

---

## Part 2 — For operators (publishing patches)

The launcher doesn't generate patches — it consumes them. Operators own
the publishing pipeline. There's no automation yet; this section
describes the manual procedure. The manifest code every launcher uses is
[`crates/launcher-core/src/manifest.rs`](../../crates/launcher-core/src/manifest.rs)
(crate `cimmeria-launcher-core`).

### Step 1 — Build the seed

The seed is the whole game as a fresh install should look. There are
two ways to provide it.

**Use the archive.org client RAR as-is (recommended).** The 2009 beta
client, build 0.8348.1.4046, is archived at archive.org item
`StargateWorlds_0.8348.1.4046`. It is a single stored RAR holding the
original installer: `SetupQA.exe` plus a MakeCAB set, `Data\DATA.INF`
and `Data\DATA1.CAB`..`DATA4.CAB` (5,983 files). The launcher expands
the cabinets straight into the installed layout, so the installer never
runs. Point the manifest's seed at the archive.org URL (an absolute
`blob` URL is used as-is), and nothing has to be rehosted. archive.org
serves `Range` requests, so an interrupted download resumes.

| Field | Value |
|---|---|
| `blob` | `https://archive.org/download/StargateWorlds_0.8348.1.4046/Stargate%20Worlds%20%280.8348.1.4046%29%20%282009-06-30%29%20%28beta%29.rar` |
| `size` | `4135724034` |
| `sha256` | `7ba97ed2cb94f86edaba17a513824ae08d0f19920583d2a3da242faf1e034f07` |

archive.org publishes only MD5 and SHA-1 for the file
(SHA-1 `1ad25c4dbd4b8717447b0de7145f5eb52d34ab06`); the SHA-256 above was
computed from a copy whose SHA-1 matches.

**Or roll your own zip** containing the installed layout
(`Common\`, `Resources\`, `Working\...`), for example a curated snapshot
that bundles content fixes you don't want to ship as separate patches.

Either way the seed contains the stock `SGW.exe` and stock
`LoginInternal.lua`: the launcher's client setup (login servers, ASLR)
runs after all manifest patches, and again before every Play.

Every extracted file keeps the modified time the archive records for it (the
cabinets date the client 2009-06-30), as the original installer does. See
["Your ini file is outdated"](#your-ini-sgwgameconfigsgweditorini-file-is-outdated)
for why that matters.

Compute the seed file's SHA-256 — it becomes `manifest.seed.sha256`.

### Step 2 — Build a patch zip

There are two kinds of patch zip, and the launcher tells them apart by
whether the zip carries a `cimmeria-patch.json` recipe.

**Patch sets (for anything derived from CME files).** The project never
hosts CME bytes. A change to a stock file (a UI Lua file, a map, a cooked
PAK) ships as a bsdiff delta that the launcher applies to the player's own
stock copy, so the zip holds only the bytes we authored. Specs and built
zips live in [`data/client-patches/`](../../data/client-patches/README.md),
built with the `cimmeria-patchset` tool:

```bash
cargo run -p cimmeria-patchset -- build data/client-patches/<id>/patch.json \
  --stock <stock client, seed unpacked, SourceCache renamed> \
  --patched <a client with the change> \
  --out data/client-patches/<id>.zip \
  --blob-url https://raw.githubusercontent.com/SandboxServers/Cimmeria/<commit>/data/client-patches/<id>.zip
```

Give the spec a `title` and a one- or two-sentence `description` of
what the patch changes, in words a player understands; the tool copies
both into the manifest entry, and the launcher shows them in its
**Changes to your client** list. Add the same text to `builtin_description`
in [`crates/launcher-core/src/client_changes.rs`](../../crates/launcher-core/src/client_changes.rs):
a test checks the two match.

It prints the manifest entry. Each recipe op pins the SHA-256 of every
source and of the result; the launcher refuses a source that isn't stock,
computes every op before writing anything, and skips ops whose target
already has the result. Maps our `upk_patch` tool wrote are diffed against
the stock map *normalized* (decompressed and rewritten by the same
patcher), which is what keeps a 4 MB map's delta under 2 KB. Files that are
entirely ours ship whole in the same zip (`"files"` in the spec).

**Plain overlay zips (for content that is entirely ours).** A zip
without a recipe contains **only the files that changed**, laid out
exactly as they go into the install directory:

```text
002-mercury-config.zip
└── Binaries/
    └── res/
        └── server/
            └── mercury.xml    ← the file you changed
```

The launcher extracts the patch zip over the install directory.
Existing files at the same paths are **overwritten**; files not present
in the patch are left alone.

Three properties fall out of this simple model:

- **Add files**: include them at their destination path.
- **Modify files**: include the new version at the same path.
- **Delete files**: not supported. If you need to remove a file, either
  (a) ship a new seed and bump `manifest.seed.sha256` to force a fresh
  install for everyone, or (b) include a stub/empty replacement at that
  path.

Every patch gets a stable `id` — convention is `NNN-short-slug` so the
manifest array stays sortable when read by humans. Compute the patch
zip's SHA-256.

### Step 3 — Write the manifest

`manifest.json` lives at the root of the container, alongside the
`seed/` and `patches/` sub-prefixes:

```json
{
  "schema": 1,
  "seed": {
    "blob": "seed/sgw-0.8348.1.4046.zip",
    "size": 5234567890,
    "sha256": "abc123..."
  },
  "patches": [
    {
      "id": "001-base",
      "blob": "patches/001-base.zip",
      "size": 123456,
      "sha256": "def456...",
      "after": null
    },
    {
      "id": "002-mercury-config",
      "blob": "patches/002-mercury-config.zip",
      "size": 23456,
      "sha256": "789abc...",
      "after": "001-base"
    }
  ]
}
```

Rules the launcher enforces (`Manifest::validate` in
[`crates/launcher-core/src/manifest.rs`](../../crates/launcher-core/src/manifest.rs)):

- `schema` must be `1`. Bumping it invalidates every launcher binary
  built before the bump — serve a legacy manifest at the old URL during
  any transition. See [Schema versioning policy](#schema-versioning-policy)
  below before considering a bump.
- Every `after` reference must point to a patch declared **earlier** in
  the array. Forward references and unknown ids fail validation.
- Patch ids must be unique.
- Array order **is** the application order. `after` is documentation /
  sanity — the launcher applies patches in their order in `patches[]`.

#### Schema versioning policy

The `schema` field is an integer version number. Today only `1` is
supported.

**When to bump:**

- Adding a **required** field to `seed`, `patches[*]`, or top-level →
  bump.
- Removing any existing field → bump.
- Changing field semantics (e.g. reinterpreting `size` as bytes vs
  blocks) → bump.
- Adding an **optional** field with a backwards-compatible default →
  **no bump** needed; older launchers will ignore the new field via
  `#[serde(default)]`.

**Compatibility model:**

The launcher bails on any schema that isn't `1` — the check is a literal
`if self.schema != 1` in `Manifest::validate`, not a named constant.
There is no multi-schema support today. Bumping the schema is therefore a
hard cutover that requires every player to be on a launcher build that
understands the new schema, and the bump means editing that literal.

The recommended bump procedure:

1. Cut a new launcher release that accepts *both* schemas during the
   transition (temporarily relax the validate check).
2. Wait at least one full release-cadence window for players to update.
3. Publish a manifest at the new schema version.
4. After two cadence windows, the next launcher release can drop the
   compatibility branch.

For changes that don't fit the additive-optional shape, prefer the
new-launcher path over a schema bump — e.g. adding a new manifest URL
prefix and pointing new launcher releases at it, while keeping the old
URL serving the old schema for existing installs.

#### Minimum launcher version (`min_launcher`)

An optional top-level field, so no schema bump (a launcher older than
the field ignores it):

```json
{ "schema": 1, "min_launcher": "launcher-20261002-bbbbbbb", "seed": { … }, "patches": [ … ] }
```

A launcher release older than this tag refuses to play that release and
says "Update the launcher before playing this release". Order: a different
day decides by the tag's date. On the same day, the required release is
newer when it was published after the running launcher was built; when
the launcher cannot tell, it lets the player through. A value that is not
a launcher tag is ignored. Development builds are never gated.

Publish the launcher release first, then the manifest that requires it.
The field is signed with the rest of the manifest.

### Step 4 — Sign + publish to GitHub Releases

The hosting layout uses two release-tag families on the repo:

```text
content-current                ← rolling tag, overwritten each publish
├── manifest.json              ← the manifest
└── manifest.json.sig          ← Ed25519 detached signature (hex)

content-2026-05-20-001         ← immutable, one per content drop
├── seed.zip                   (if this drop ships a new seed)
└── 002-mercury-config.zip     (per-patch zips)
content-2026-05-20-002
├── 003-quest-fixes.zip
└── ...
```

The manifest references the immutable tag's assets by **absolute URL**,
e.g. `https://github.com/<org>/<repo>/releases/download/content-2026-05-20-001/seed.zip`.
The launcher's `blob_url` (in `manifest.rs`) passes absolute URLs through
unchanged; relative `blob` fields still work (resolved against the manifest
URL's container) for legacy / mixed hosting setups.

A rough operator workflow using PowerShell + `gh`:

```powershell

# 1. Build a patch zip from the changed files.
Compress-Archive `
  -Path 'Binaries\res\server\mercury.xml' `
  -DestinationPath '002-mercury-config.zip' `
  -CompressionLevel Optimal

# 2. Compute sha + size for the manifest entry.
$sha  = (Get-FileHash 002-mercury-config.zip -Algorithm SHA256).Hash.ToLower()
$size = (Get-Item 002-mercury-config.zip).Length
"sha:  $sha"
"size: $size"

# 3. Append a patch entry to manifest.json with the absolute URL for

#    where the asset will live in the immutable release. Pick the tag

#    you're about to create.
$tag = "content-$(Get-Date -Format yyyy-MM-dd)-001"
$blob = "https://github.com/SandboxServers/Cimmeria/releases/download/$tag/002-mercury-config.zip"

# (edit manifest.json by hand or via jq)

# 4. Sign the manifest with the offline private key, producing

#    manifest.json.sig (Ed25519 over the exact manifest bytes, hex), and

#    check it against the public key in the LAUNCHER_MANIFEST_PUBKEY_HEX

#    secret.
cargo run -p cimmeria-patchset -- sign manifest.json --key <path to manifest-signing.key>
cargo run -p cimmeria-patchset -- verify manifest.json --pubkey <public key hex>

# 5. Create the immutable release first (asset must exist before the

#    manifest references it).
gh release create "$tag" --notes "Content drop $tag" --prerelease `
  002-mercury-config.zip

# 6. Update the rolling manifest pointer. --clobber overwrites in place.
gh release upload content-current --clobber `
  manifest.json manifest.json.sig
```

**Ordering matters.** Create the immutable per-publication release
(step 5) **before** updating `content-current` (step 6). A launcher
that fetches `manifest.json` between the two would see an entry
referencing a blob that doesn't exist yet and fail with a 404.

The launcher verifies the signature against the compile-time-embedded
`MANIFEST_SIGNING_PUBKEY` (`verify_manifest_signature` in `manifest.rs`).
`cimmeria-patchset pubkey --key <file>` prints the public key for a key
file, to check it matches the secret.

Full operator setup (signing keypair generation, GitHub secrets) lives in
[launcher-distribution-setup.md](launcher-distribution-setup.md).

### Append-only invariants

Once you publish a patch, **never mutate it**:

- Don't change its `blob` path (players who already installed under the
  old path would refetch).
- Don't change its `sha256` (a launcher mid-rerun would see a hash
  mismatch and refuse the install).
- Don't change its `id` (the installed-state ledger uses ids to know what
  it already applied).

**To fix a broken patch, publish a new patch that overwrites the
affected files.** The manifest is append-only at the patch level.

**Retiring a patch doesn't undo it.** Removing an entry from the
manifest stops fresh installs from getting it, but installs that already
applied it keep its files. A patch that supersedes a retired one should
repeat the ops it keeps, pinned to the same result hashes, so the apply
step skips them on installs that already have the result. For example,
`007-castle-armory-ring` replaced `002-castle-ring-transport` (retired
2026-09-29) and repeats only 002's Armory op. Changing a file back to
stock needs a delta whose result is the stock bytes. That works when the
patched file differs from stock by a small edit. It doesn't work when
the patch rewrote the whole file: 002's stasis-hall map is stored
uncompressed, and the stock map is LZO-compressed, so a delta back to
stock would carry about half of the stock map verbatim.

The seed is replaceable, but it's a heavy operation: a new
`seed.sha256` forces every installed player to re-download the full
seed and re-apply every patch. Reserve seed bumps for major-version
flips.

### Future automation

`cimmeria-patchset` builds patch sets and signs manifests; publishing is
still by hand. A reasonable next step once patches ship regularly is a
GitHub Actions workflow that rebuilds the patch zips, checks they match
the committed ones, and publishes on a `/release-patch` ChatOps comment,
mirroring the `/release-launcher` pattern from
[.github/workflows/launcher-release-on-comment.yml](../../.github/workflows/launcher-release-on-comment.yml).
Signing stays offline.

---

## Troubleshooting

Common problems, with first steps. Every action in the launcher reports a
status line under its button; copy that text into a bug report if the steps
here don't help.

### Windows Defender or SmartScreen blocks the launcher

Launcher releases are not Authenticode-signed, so Windows may warn about the
launcher or quarantine an extracted helper. The embedded launch helper starts
`SGW.exe` and loads the client patches, which antivirus heuristics can flag.
The signed updater ZIP verifies release integrity; it does not suppress
SmartScreen or antivirus warnings.

- **SmartScreen ("Windows protected your PC")** on
  `cimmeria-launcher-desktop.exe`: choose **More info → Run anyway**. Only do
  this for a launcher you downloaded from the stable link.
- **A helper is quarantined:** check **Windows Security → Virus & threat
  protection → Protection history** and review the detection. The launcher
  repairs missing or corrupt ordinary cache files from its embedded copy on
  the next start. Repeated quarantine needs the detection resolved first;
  downloading the same ZIP again does not prevent it.
- **Payload preparation fails:** close the launcher and try again. If it still
  fails, report the visible error and launcher version for support. Do not
  substitute helper or DLL files from another release.

### A launcher update failed, or the launcher is gone after an update

If **Apply and restart** fails before the new launcher starts, the launcher
puts its old files back and the section says "The update could not start. The
original launcher was restored when its files could be verified." Click
**Recheck update status**, then **Check for updates** to try again. A download that
failed its signature check was not installed; check again for a fresh offer.

Two cases need a hand fix, because no launcher is left to fix them:

- **The launcher exe is missing** (the PC turned off mid-update, or antivirus
  quarantined the new exe). Rename `cimmeria-launcher-desktop.exe.old` back
  to `cimmeria-launcher-desktop.exe`, and do the same for any `.old` file in
  the `windows` and `graphics` folders. The next start rolls back the rest.
- **The updated launcher closes as soon as it starts.** Delete the new
  `cimmeria-launcher-desktop.exe`, then rename the `.old` files back as above.

Or download the zip from the stable link and unzip it over the folder. A
leftover `.old` file next to a working launcher is safe to delete; the
launcher deletes them itself at its next start.

### "Another launcher is using these settings"

A second copy of the launcher is open. Starting the launcher again normally
brings the open window to the front instead. Close the other window and try
again.

### Install fails or stops

- **Not enough space:** the install needs about 14 GB free on the install
  drive while it unpacks.
- **A download or checksum error:** remove the partial installation
  (**Remove partial installation…**) and install again. If it keeps failing on
  the same file, the release or archive.org may be out of step; report it.
- **The folder is refused:** pick an empty folder, or a new folder inside an
  existing one.
- **Use an existing game folder is refused:** the launcher lists the files and
  what to do; see [If the launcher refuses the folder](#if-the-launcher-refuses-the-folder).

### "Your ini (..\SGWGame\Config\SGWEditor.ini) file is outdated"

The game asks this on launch when a `Working\SGWGame\Config\Default*.ini`
file's modified time differs from the one it recorded the last time it
ran. Unreal Engine 3 builds your own settings files in
`Documents\My Games\Firesky\SGWGame\Config\SGW*.ini` from the
`Default*.ini` files, and keeps the timestamps of those in each file's
`[INIVersion]` section. A different timestamp looks like a newer
`Default*.ini`.

Old launchers released before a fix wrote every client file with the install
time instead of the 2009-06-30 date in the installer cabinets. If you had run
SGW on this PC before, from any install, the first launch after such an
install shows this dialog. The current launcher keeps the cabinets' dates, as
the original installer does.

- **Yes** (or **Yes to all**) is safe. The game rebuilds the
  `SGW*.ini` files from the `Default*.ini` files and records the new
  timestamps, so it does not ask again. It resets your settings in those
  files (resolution, keys, audio) to the defaults.
- **No** keeps your settings, and the game asks again on the next
  launch.

### The game shows the gate backdrop but never the login screen

The old launcher released on 2026-09-29 (`launcher-20260929-f518b57`)
renamed `Working\SGWGame\Content\UI\Startup\EULA\EULA.lua` to
`eula.lua` while applying the `005-login-delay` patch. The game looks
that file up case-sensitively, so the EULA screen never loads, and the
login screen comes after it. The client log (and telemetry's
`client.ui.cegui_log`) shows `SGWResourceProvider: 'EULA.lua' does not
exist in group lua`.

- **Fix**: play from the current launcher. Before every Play it renames the
  file back to its stock name.
- **By hand**: rename `eula.lua` to `EULA.lua` in that folder. Keep the
  file; it is the patched one.

Patch sets now keep the on-disk name of every file they change, so a new
install is not affected.

### The Black Market window does not open

The Black Market window needs the client patches.

- Check **Play → Client patches**: **Load client patches** must be on.
- Check **Play → Changes to your client**: it lists the DLLs Play loads.
- If Defender removed a file from the launcher's `windows` folder, see
  [Windows Defender or SmartScreen blocks the launcher](#windows-defender-or-smartscreen-blocks-the-launcher).
- The patches write `cimmeria-client-patches.log` next to `SGW.exe`. A line
  ending `nothing installed` there means this `SGW.exe` is not the build the
  patches know. Include that file in a bug report.

### SGW.exe starts but can't reach the server

- Check **Settings → Login servers**: one `Name = http://host:8081` line per
  server, with your operator's host, then **Save servers**.
- Pick the right server in the login screen's list.
- Don't edit `LoginInternal.lua` by hand: Play rewrites it from the saved
  list.

### The game shows old or missing content after a server change

Click **Settings → Reset client cache**, then log in again; the server sends
its data again. If the game will not start at all or its settings are broken,
**Reset all client state…** starts the client fresh. See also
[A cooked-data category went empty](../troubleshooting.md#a-cooked-data-category-went-empty-after-logging-in-to-another-server).

### Log upload problems

- **"No new log lines since the last upload"** is not an error: everything
  was already sent. Play a new session to make new lines.
- **"Sent X of Y lines. The server asked us to wait; press again in about N min."**
  The server asked the launcher to wait (a per-install or per-day limit), or you closed
  the launcher mid-upload. Wait the minutes shown, then press again; it
  carries on from the last batch the server accepted. Each press sends at most
  40,000 lines, and a day allows about two full presses.
- **"The upload did not reach the log store."** The launcher could not reach
  the server. Check your connection and press again.
- **"Debug logs can't be sent: this install has no telemetry server."** The
  saved login-server list is not valid, for example after a hand edit of
  `launcher-config.json`. Save a valid list under **Login servers**, or clear
  it to use the default server.
- **"No logs to upload yet: the game has not written any."** There is nothing
  to read yet.

### Where to find logs to share

- The status line under the button you pressed.
- **Settings → Debug logs → Upload debug logs** for the launcher's and the game's log lines.
- `game\Working\Binaries\SGWDebugLog.log` in your install folder.
- The launcher's own log: `%APPDATA%\app.cimmeria.launcher.desktop\logs\launcher.<date>.log`; on Linux, `~/.local/share/app.cimmeria.launcher.desktop/logs/`; on macOS, `~/Library/Application Support/app.cimmeria.launcher.desktop/logs/`. There is one file a day, 7 are kept, and the home folder is written as `~`. **Upload debug logs** sends the new lines of these files, up to 20,000 per press. Before the upload, paths are scrubbed and IP literals are replaced with `[ip]`, and the server scrubs each message again word by word.

---

## Cross-references

- [Playing on a Mac](../guides/macos.md) — the macOS launcher
- [Desktop launcher README](../../crates/launcher/desktop/README.md) — how the launcher works, for contributors
- [launcher-self-update.md](launcher-self-update.md) — how launcher updates are signed, verified and swapped
- [launcher-distribution-setup.md](launcher-distribution-setup.md) — operator setup: content releases and the two signing keys
- [Desktop launcher release](../../crates/launcher/desktop/docs/release.md) — what `/release-launcher` publishes
- [sgw-launcher.md](sgw-launcher.md) — the retired old launcher's design, kept for history
