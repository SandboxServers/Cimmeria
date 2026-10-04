---
title: "Launcher Guide"
type: how-to
audience: players, operators
last_updated: 2026-10-03
---

# Launcher Guide

How the Stargate Worlds launcher works from the user's seat, and how
operators prepare and publish patches for it to consume.

This is the practical, day-to-day doc. For architecture and rationale
see [sgw-launcher.md](sgw-launcher.md). For the one-time operator
backend setup (GitHub Releases for content, manifest signing keypair,
Azure Blob SAS for log uploads) see
[launcher-distribution-setup.md](launcher-distribution-setup.md).

> **Audience split:**
> [Part 1 — Players](#part-1--for-players) is for anyone running the
> launcher to install and play the game.
> [Part 2 — Operators](#part-2--for-operators-publishing-patches) is for
> whoever publishes patches to the Azure container that players' launchers
> pull from. Skip the one that isn't you.

---

## Contents

- [Part 1 — For players](#part-1--for-players)
  - [The launcher window](#the-launcher-window)
  - [Install and Play](#install-and-play)
  - [Share diagnostic logs](#share-diagnostic-logs)
  - [Patch Notes](#patch-notes)
  - [Game settings](#game-settings)
  - [Advanced](#advanced)
  - [What it does (end-to-end)](#what-it-does-end-to-end)
  - [State files](#state-files)
  - [What "Install / Update" does internally](#what-install--update-does-internally)
  - [How the client finds its server](#how-the-client-finds-its-server)
  - [Install layout](#install-layout)
  - [Debug launches](#debug-launches)
  - [Uploading debug logs](#uploading-debug-logs)
  - [Launcher updates](#launcher-updates)
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

### The launcher window

The launcher is one dark window for one game, Stargate Worlds. It stays
dark even when Windows uses light mode. It opens at 1030 × 720 and you
can resize it down to 560 × 520.

- **On the left**, a panel with the gate. When the window is narrower
  than 760 pixels, the panel folds away and the main column takes the
  whole window.
- **At the top of the main column**, the launcher update banner when a
  newer launcher exists (see [Launcher updates](#launcher-updates)), then
  a heading that says where you stand.
- **Two tabs**, **Play** and **Patch Notes**, and a gear (⚙) at their
  right. The gear opens **Game settings** above whichever tab you are on;
  click it again, or **Close**, to hide them.
- **At the bottom of every view**, the **Share diagnostic logs** choice.
  It stays there on both tabs and with the settings open.

### Install and Play

The Play tab has one status card and one big button. The button is
whatever comes next, so you never pick from a list of launch modes:

| You see | What it means | The button does |
|---|---|---|
| **Choose where to install** | No install folder is set | **Choose install folder** opens Game settings at the folder box |
| **One setup. Then just Play.** | The folder has no game yet | **Install Stargate Worlds** downloads the game (the card gives its size), applies Cimmeria's patches and sets up the login servers |
| **Stargate Worlds found in this folder** | The folder has `SGW.exe` that this launcher did not install | **Use this installation** adopts it: no download, Cimmeria's patches go on top. Its existing files are not checked |
| **An update is ready** | New patches, or a new base game, are published | **Update** applies them. **Play without updating** starts the game you have |
| **Installing** | An install or update is running | **Cancel** (see below) |
| **● Ready to play** | Everything published is installed | **Play** |
| **Starting Stargate Worlds…** | You pressed Play | Nothing; the button reads **Starting…** |
| **● Game running** | Stargate Worlds is running from this install folder | Nothing; the button reads **Playing** |
| **One more step before you play** | Something needs fixing first; the card says what | **Retry** when the game content list could not load, otherwise **Open settings** |

Every click changes the card at once, before the work starts.

**Installing.** Downloading and installing files are progress phases on
the same card, with a progress bar and the file being worked on. You can
switch tabs or open the settings while it runs. **Cancel** stops after
the current step: what finished is kept, and the next **Install** or
**Update** picks up from there (a half-downloaded file resumes). The
card then says "Installation cancelled. What finished is kept; Install
picks up from there." A failed install says "Installation failed: …"
with the reason, and the button goes back to **Install** or **Update**,
so you retry with one click. After any install, failed or not, the
launcher reads its install record again, so the card always matches
what is really on disk.

**Playing.** **Play** prepares the client (login servers, ASLR, stock
file names; see [What "Install / Update" does internally](#what-install--update-does-internally)),
starts `SGW.exe` with Cimmeria's client patches, and shows **Game
running** until the game closes. Then the card goes back to **Ready to
play** and says "The game closed." If the game ended with an error code,
the card gives the code and suggests **Upload Debug Logs** in
Settings › Advanced. This works whether or not you share diagnostic logs.

- Closing the launcher does not close the game.
- If you open the launcher while the game is already running (you
  closed the launcher and reopened it, or started the game another
  way), it sees `SGW.exe` running from the install folder and shows
  **Game running** too. It checks every two seconds, so the card goes
  back to **Ready to play** shortly after the game closes.
- While the game runs, or an install runs, everything that changes the
  game's files is off: Install, Update, adopt, changing the install
  folder and the client-state resets. A second Play is off too. If a
  click gets through anyway, the launcher refuses it and the card says
  `Not started: …` with the reason.

**View details** under the button lists the install folder, whether
`SGW.exe` was found, who installed it (this launcher, or an adopted copy
that was not verified), the content manifest, and whether the client
patches and diagnostic logs are on. A first install also reminds you
that you need a server account to sign in inside the game.

### Share diagnostic logs

The checkbox at the bottom of the window is **off** until you turn it
on. Installing, updating or playing never turns it on for you. **What
is sent?** under it explains it:

While the game runs, the launcher uploads the client's log files, and a
small telemetry module (`cimmeria-client-telemetry.dll`) loaded into the
game records in-game events: the game messages it receives, interface
errors and its own status. Both go, with this install's random id, to
the Cimmeria server, so crashes and bugs can be traced. Nothing is sent,
and the module is not loaded, while the box is off.

- **It is saved the moment you click it**, and it is the same on every
  tab and after a restart. If it cannot be saved, a red line under it
  says so; the choice then holds only until you close the launcher.
- **It takes effect at your next game launch.** A game that is already
  running keeps what it was started with. The caption under the box says
  which applies: for example "Off from your next game launch. The game
  running now keeps sending diagnostics until it closes." For a game the
  launcher did not start itself, it says only that a change applies on
  your next launch.

The launcher no longer asks about this in a one-time prompt; the
checkbox is always there instead. The telemetry design is in
[client-telemetry.md](../architecture/client-telemetry.md) and
[operations/telemetry.md](../operations/telemetry.md).

### Patch Notes

The Patch Notes tab lists the client patches the server publishes right
now, in the order the content manifest lists them. Each row shows the
patch's title (or its id when it has none) and its description, or
"No description provided in the manifest."

- The list comes only from the content manifest, after the launcher has
  checked its signature. Nothing else is shown as a patch note.
- It is **not** a list of what is installed on your computer. For that,
  see **Changes to your client** in [Advanced](#advanced).
- **Refresh** fetches the manifest again. If that fails, the old list
  stays, marked "Could not refresh: … Showing the last list that
  verified." If the first fetch fails, the tab says "Could not load
  patch notes: …".

### Game settings

The gear opens **Game settings**:

- **Install folder.** The folder the launcher installs to and plays from
  (by default `%LOCALAPPDATA%\Stargate Worlds`).
- **Open in Explorer ↗** shows the folder. The button never creates one:
  a folder that does not exist is reported instead.
- **Change folder…** points the launcher at another folder:
  1. Type the full path, such as `C:\Games\Stargate Worlds`, and press
     **Check folder**. The launcher refuses a relative path, a drive
     root, a file, or the folder you already use.
  2. It says what the folder holds: empty (Install downloads the game
     there), a game this launcher manages, a game it has not adopted
     (the Play tab offers to adopt it), or other files (the game installs
     alongside them, and nothing there is removed). Checking creates
     nothing.
  3. **Use this folder** saves the change, and the launcher creates the
     folder if it does not exist. The old installation stays where it
     is; nothing is moved or deleted.

  **Change folder…** and **Use this folder** are off while an install
  runs or the game is running.
- **Repair game** and **Uninstall…** are shown but **not available yet**;
  they arrive in a later launcher update. Until then, **Install / Update**
  in Advanced re-applies only what the launcher has not recorded as
  installed. It does not replace a file that was deleted or damaged after
  it was installed. See
  [`SGW.exe` is missing, or a game file is damaged](#sgwexe-is-missing-or-a-game-file-is-damaged).

### Advanced

**Advanced**, at the bottom of Game settings, is closed by default. It
holds every configuration and troubleshooting tool the launcher had
before its redesign, each in its own section:

| Section | What it holds |
|---|---|
| Login servers | The `Name = URL` list written into the client's `LoginInternal.lua`, and **Save login servers** (see [How the client finds its server](#how-the-client-finds-its-server)) |
| Content manifest | The manifest URL, **Refresh**, and whether the manifest's signature verified. An edited URL is used only after **Refresh** |
| Install, update and adopt | Manual **Install / Update** and **Cancel**, **Adopt existing install**, and what the install record says is still to apply |
| Client patches | **Load client patches (restores the Black Market window)** |
| Debug launches | **Launch Atera Debug**, **Launch Atera + Telemetry**, **Fix ASLR** (see [Debug launches](#debug-launches)) |
| Debug logs | **Upload Debug Logs** (see [Uploading debug logs](#uploading-debug-logs)) |
| Client state | **Reset client cache** and **Reset all client state…** |
| Launcher updates | The running launcher's version and **Check for updates** |
| Changes to your client | Every change Cimmeria makes to the stock client (see [What the launcher changes in your client](#what-the-launcher-changes-in-your-client)) |
| Activity log | The last 100 lines of what the launcher did; copy them into a bug report |

Their warnings and confirmations are unchanged. The ones that change the
game's files (Install / Update, adopt, the two resets) are also off
while the game runs.

### What it does (end-to-end)

```text
1. Run sgw-launcher.exe (single ~5 MB file).
2. The launcher fetches the content manifest (default: the GitHub
   Release `content-current` tag,
   https://github.com/SandboxServers/Cimmeria/releases/download/content-current/manifest.json)
   and checks its signature. Only a manifest that verified is used.
3. It compares the manifest against <install_dir>/launcher-installed.json
   (the install record) and picks the Play tab's button:
     a. No record and no SGW.exe   → "Install Stargate Worlds"
     b. SGW.exe but no record      → "Use this installation" (adopt)
     c. Seed differs or a patch is missing → "Update" (+ "Play without updating")
     d. Everything matches          → "Play"
4. Install / Update:
     - Download the seed (resumable via HTTP Range) → verify sha256 → unpack.
       The seed is a zip, or the archive.org client RAR, whose installer
       cabinets are expanded straight into the installed layout
     - For each missing patch in declared order: download → verify → unpack
       (overlay files, and/or deltas rebuilt from your own stock files)
     - Client setup: restore stock file names (EULA.lua), write
       LoginInternal.lua, switch ASLR off in SGW.exe (also done before
       every launch)
5. Play: client setup again, then SGW.exe starts with the client patches
   (and, if you share diagnostic logs, the telemetry module). The
   launcher follows the game until it exits.
6. After a crash, Settings › Advanced › Upload Debug Logs zips and
   uploads the client's logs in one shot.
```

### State files

Four JSON files persist across runs:

| File | Lives | Contents |
|------|-------|----------|
| `<exe>/launcher-config.json` | next to `launcher.exe` | `schema_version`, `install_path`, `login_servers` (`[{name, url}]`), `manifest_url`, `client_patches` (`enabled`, on by default), and a `telemetry` object (`opted_in`, default `false`, the **Share diagnostic logs** box; `prompt_answered`, set whenever that choice is saved and kept from the retired one-time prompt; `auth_url`; `auth_url` defaults to the public server's login port, `http://play.cimmeria.app:8081/api`). An old `server_host` field is ignored. Schema 2 moved that default: a schema-1 file whose `auth_url` is exactly the old `http://localhost:8443/api` is rewritten to the new default once. |
| `<install>/launcher-installed.json` | in the game dir | `seed_sha256`, `applied_patches: ["001-dialog-portraits", …]`, `seed_adopted`. An old `patched_host` field is ignored. |
| `<exe>/uploaded.json` | next to `launcher.exe` | `[{sha256, blob_name, uploaded_at}, …]` — log-upload dedupe ledger |
| `<exe>/telemetry-state.json` | next to `launcher.exe` | per-session telemetry runtime state, kept separate from the config so config rewrites don't churn it |

Putting the installed-state file **inside the game directory** is
deliberate: reinstall the launcher and your install is still recognized;
copy the game directory to another machine and it's still recognized.

### What "Install / Update" does internally

Pseudocode of [`crates/launcher/src/install.rs`](../../crates/launcher/src/install.rs)::`install_all()`:

```text
1. Load <install>/launcher-installed.json (or default if absent).

2. If state.seed_sha256 != manifest.seed.sha256:
     - GET <manifest_base>/<manifest.seed.blob>, sending HTTP `Range: bytes=N-`
       when a .tmp file from a previous attempt exists.
     - Stream the body into <install>/.tmp-seed-<sha-prefix>.download.
       A 416 reply for a full-length file means it is already downloaded.
     - SHA-256 verify against manifest.seed.sha256. On mismatch, delete the
       download and bail (a kept bad file would fail every retry).
     - Unpack into <install>/ (overwrites colliding files), picking the
       format from the file's magic bytes:
         zip → extract directly.
         RAR → extract into <install>/.tmp-unpack/. If that holds a MakeCAB
               set (an .INF with a [cabinet list], e.g. Data\DATA.INF +
               DATA1-4.CAB), expand the cabinets into <install>/ with
               Windows' cabinet.dll; otherwise move the files across.
               Then delete .tmp-unpack/.
       Every extracted file keeps the modified time the archive records
       for it (the cabinets date the client 2009-06-30), the way the stock
       installer does. See "Your ini file is outdated" under
       Troubleshooting for why that matters.
       Hashing and unpacking run on a blocking thread.
     - state.seed_sha256 = manifest.seed.sha256
     - state.applied_patches = []   ← reseeding invalidates patch history
     - Persist state.

3. Rename Working\SGWGame\Cache.en-US to SourceCache.en-us, unless a
   SourceCache.en-us already exists (see "Install layout"). Runs every
   time, so adopted installs get it too.

4. For each patch in manifest.patches (declared order):
     - Skip if patch.id is already in state.applied_patches.
     - GET <manifest_base>/<patch.blob> → tmp → SHA-256 verify → unpack.
       A patch zip with a cimmeria-patch.json recipe is a patch set: see
       "Patch sets" below.
     - Append patch.id to state.applied_patches.
     - Persist state after every patch (survives mid-update crash).

5. Client setup (crates/launcher/src/client_setup/), when SGW.exe exists:
     - Rename any file a patch set writes back to its stock spelling if
       only its case differs (eula.lua -> EULA.lua).
     - Write Working\SGWGame\Content\UI\Startup\Login\LoginInternal.lua
       from the login-server list, if its content changed.
     - Clear IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE in SGW.exe's PE
       header (one byte, 0x186 in the 0.8348 build), if set.
```

### How the client finds its server

The client's login screen lists the servers `LoginMod.loadServerSystems()`
defines in `LoginInternal.lua`. The stock file points at CME's dead QA and
production login servers; the launcher replaces it with the configured
list, CRLF and ASCII like the rest of the client's UI Lua, at the end of
every install, on **Save login servers** (Settings › Advanced), and
before every launch. Names and URLs may not
contain quotes or backslashes, so a setting can't break out of the Lua
string. A Cimmeria auth server listens on `http://<host>:8081`.

Earlier launcher builds instead byte-patched `www.stargateworlds.com` in
`SGW.exe`'s `.rdata`. That string is only the SOAP namespace
`http://www.stargateworlds.com/xml/sgwlogin`, so the patch redirected
nothing and would have broken the namespace the auth server expects. It is
gone; no released launcher ever ran it.

ASLR is switched off because the client-patches DLL and every RE address
in `docs/` assume `SGW.exe` loads at `0x00400000`. The stock exe with that
one byte cleared is byte-identical to a known-good QA client's.

Hitting **Cancel** flips a `CancellationToken` that the download stream
checks on every chunk, and the unpack steps check between files. A
cancelled install leaves the `.tmp-*.download` file on disk, and every
seed or patch that finished stays recorded in `launcher-installed.json`,
so the next **Install** or **Update** picks up where it stopped via
`Range: bytes=N-`.

Installing from the archive.org client RAR needs roughly 14 GB free on
the install drive at its peak: the 4.1 GB download, the 4.1 GB staged
installer, and the expanded client.

### Install layout

A full install has the original installer's layout:

```text
<install>\Common\...
<install>\Resources\...
<install>\Working\Binaries\SGW.exe   (plus its DLLs, SGWDebugLog.log, sessions\)
<install>\Working\SGWGame\...
```

[`crates/launcher/src/install_layout.rs`](../../crates/launcher/src/install_layout.rs)::`binaries_dir`
finds the directory holding `SGW.exe`, and every launch, the hostname
patch, adoption and the log upload go through it. It also accepts an
install path that points straight at `Binaries` (holds `SGW.exe`) or at
`Working`, so configs from before this layout keep working.

The client reads cooked data (`Cooked*.pak`, `TextStrings.pak`, …) from
two tiers, set in `Working\Engine\Config\GameplayEngine.ini`:

| Tier | INI key | Where it is | Who writes it |
|---|---|---|---|
| Writable | `CachePath=..\SGWGame\Cache` | `Documents\My Games\Firesky\SGWGame\Cache.en-US` | The server's version push, on every connect |
| Bundled, read-only | `SourceCachePath=..\SGWGame\SourceCache` | `<install>\Working\SGWGame\SourceCache.en-us` | Nothing at runtime |

The 2009 cabinets put the bundled PAKs in
`Working\SGWGame\Cache.en-US`, which is neither path. A stock install
logs this once per cooked-data category in `SGWDebugLog.log`:

```text
WARN common - Non-existent source archive directory: <install>\Working\SGWGame\SourceCache.en-US
```

So after the seed is unpacked, the launcher renames that folder to
`SourceCache.en-us`, which silences the warning. Testers had already
done the same by hand, and their `SourceCache.en-us` PAKs are
byte-identical to the cabinets' `Cache.en-US`. It is also why a PAK
placed by hand in `SourceCache.en-us` stays put, while one placed in the
Documents `Cache.en-US` is rewritten the next time the client connects.
The launcher's **Reset client cache** button wipes only the Documents
tier.

### Debug launches

**Play** is the launch for players. Settings › Advanced › **Debug
launches** has three more buttons for developers and modders. Each is on
only when its files are in the folder that holds `SGW.exe`, and, like
Play, only while no install runs and the game is not already running
(see [`crates/launcher/src/app/advanced_panel.rs`](../../crates/launcher/src/app/advanced_panel.rs)::`show_debug_launches`):

| Button | On when | What it runs |
|--------|------------|--------------|
| **Launch Atera Debug** | `AteraLoader.exe` **and** `AtreaGameDebug.bat` both present | `cmd /C AtreaGameDebug.bat` (cwd = binaries dir) |
| **Launch Atera + Telemetry** | Atera available **and** you share diagnostic logs **and** the launcher identity loaded | Same as Atera Debug, plus the dev-session telemetry pipeline — see [telemetry.md](../operations/telemetry.md) |
| **Fix ASLR** | `AtreaFixASLR.bat` present | `cmd /C AtreaFixASLR.bat` |

The Atera bat starts `SGW.exe` itself, so the launcher cannot follow
that game the way it follows one Play started. It notices the running
`SGW.exe` instead, within two seconds, and the Play tab shows **Game
running** until it closes.

**Load client patches (restores the Black Market window)**, in
Settings › Advanced › **Client patches**, controls
`cimmeria-client-patches.dll`, which the launcher loads into the game on
**Play**. It is on by default, saved as soon as you change it, and has
nothing to do with sharing diagnostic logs. Turn it off only to rule the
patches out when something misbehaves; the Black Market window will not
open without them. The activity log says on every launch when the
patches were not loaded, and why. Atera debug launches never load them.

When you share diagnostic logs, **Play** also loads the telemetry module
after the client patches (see [Share diagnostic logs](#share-diagnostic-logs)).
Telemetry goes to the server's login port, the same address the game
logs in to, so it needs no setup. The module changes nothing in the
game. If the launcher cannot reach the telemetry server, or the module
is missing, the activity log says so and the game starts without it.

### What the launcher changes in your client

Stargate Worlds needs the stock 2009 client, and Cimmeria changes it.
The **Changes to your client** section, in Settings › Advanced, lists
every change: the login
servers, ASLR flag and stock file names it sets before every launch, the
folder it renames
at install, each patch from the manifest and whether it is applied yet,
what it adds to `SGW.exe` at launch, and what the server sends while you
play. The same list applies whether the launcher downloaded the client
or you adopted your own copy. Details are in
[sgw-launcher.md](sgw-launcher.md#changes-to-your-client).

The Atera files are **not** shipped by the launcher or any of its
patches. Developers and modders drop the Atera tarball into the install
directory themselves; the launcher detects the files and surfaces the
buttons.

The Atera debug build requires ASLR disabled on `SGW.exe`. The launcher
switches it off itself before every launch, Atera ones included, so
**Fix ASLR** is only needed for a client the launcher has never
prepared.

For what Atera actually does at runtime see
[../technical/atrealoader-exe.md](../technical/atrealoader-exe.md) and
[../technical/atrealoader-config.md](../technical/atrealoader-config.md).

### Uploading debug logs

The **Upload Debug Logs** button, in Settings › Advanced › **Debug
logs**, collects:

- `<binaries>/sgwdebuglog*`, matched case-blind (the client writes
  `SGWDebugLog.log`) — BigWorld Mercury unicode log
- `<binaries>/sessions/**` — per-session logs

The zip stores them under `Binaries/` whatever the directory is called
on disk.

Zips them in memory and PUTs the zip in a single HTTP request to the
Azure storage account, named `logs/<hostname>-<utc>-<digest12>.zip`.

Three rules to keep storage costs minimal:

1. **One PUT per click**, never one-per-file. The whole zip goes up in
   one request.
2. **Content-digest dedupe**: the launcher hashes the *input files*
   (sorted filename + bytes), not the zip itself, and records the hash
   in `<exe>/uploaded.json`. Already-uploaded digest → zero HTTP
   requests, button reports "Already uploaded".
3. **Never automatic**: only fires on explicit button click.

If the launcher you're running has no upload SAS baked in (i.e. it was
built from a PR or a local dev build), the button is disabled with a
friendly note. Only the official release pipeline injects the SAS.

### Launcher updates

The launcher updates itself. At startup it asks GitHub once, in the
background, whether a newer launcher release exists; the window opens
straight away. Settings › Advanced › **Launcher updates** shows the
running version (`Launcher launcher-YYYYMMDD-<sha>`) and a **Check for
updates** button.

When a newer release exists, a banner at the top of the window reads
**Launcher update available (launcher-…)** with an **Update now** button
and a link to the release notes. One click:

1. Downloads the new launcher into the launcher's own folder, with a
   progress bar in the banner.
2. Checks the download against the size and the SHA-256 published in
   the same GitHub release. A download that does not match is deleted
   and nothing else changes.
3. Renames the running launcher to `<its name>.old`, puts the new one at
   the old name, and starts it with the same arguments. If the new one
   cannot be put in place or will not start, the old one is renamed back.
4. Closes the old launcher as soon as the new one has started, without
   waiting for you to touch its window. The new launcher's activity log
   says `Launcher updated from … to …`, and it deletes the `.old` file.

The launcher keeps its file name, whatever you renamed it to, and the
settings files beside it (`launcher-config.json` and the rest) are left
alone. **Update now** is off while an install is running. Updating closes
the launcher, so if a game launched with telemetry is still running, that
telemetry session ends; update between sessions.

A launcher you built yourself (or from a pull request) never updates: the
version line reads `development build — updates disabled`.

The check is one request to the GitHub API, which allows 60 anonymous
requests an hour per address. When you are offline or over that limit,
the activity log says `Could not check for launcher updates: …` and the
launcher carries on; the next start checks again.

> **Launcher `launcher-20260929-f518b57` and older have no updater.**
> Download the newest `sgw-launcher-launcher-….exe` from the
> [releases page](https://github.com/SandboxServers/Cimmeria/releases)
> by hand once. It updates itself from then on.

**Required updates.** The server's operator can require a minimum
launcher version (the manifest's
[`min_launcher`](#minimum-launcher-version-min_launcher)). When yours is
older, a red line at the top of the main column says so, the Play tab
says "This launcher is too old for the current game content. Update the
launcher first.", Install, Update and every launch button are off, and
the banner offers the update. When the update
check could not run, it links to the releases page instead.

---

## Part 2 — For operators (publishing patches)

The launcher doesn't generate patches — it consumes them. Operators own
the publishing pipeline. There's no automation yet; this section
describes the manual procedure.

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
serves `Range` requests, so the 4.1 GB download resumes.

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
runs at the end of `install_all`, after all manifest patches.

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
**Changes to your client** list. Add the same text to the launcher's
`builtin_description` in `crates/launcher/src/client_changes.rs`: a
test checks the two match.

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

The launcher extracts the patch zip over the install directory using
`zip::ZipArchive`. Existing files at the same paths are **overwritten**;
files not present in the patch are left alone.

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

Rules the launcher enforces (see
[`crates/launcher/src/manifest.rs`](../../crates/launcher/src/manifest.rs)::`validate`):

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
`if self.schema != 1` in `Manifest::validate`
([`crates/launcher/src/manifest.rs:176`](../../crates/launcher/src/manifest.rs)),
not a named constant. There is no multi-schema support today. Bumping the
schema is therefore a hard cutover that requires every player to be on a
launcher build that understands the new schema, and the bump means editing
that literal.

The recommended bump procedure:

1. Cut a new launcher release whose `SUPPORTED_SCHEMA` is the new
   number. The new launcher must accept *both* schemas during the
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

A launcher release older than this tag turns off **Install / Update** and
the launch buttons and shows a mandatory update banner. Order: a
different day decides by the tag's date. On the same day, the required
release is newer when it was published after the running launcher was
built; the launcher looks the tag up in the release list it just
fetched, and lets the player through when it could not fetch it. A value
that is not a launcher tag is ignored, with a WARN in the launcher's log.
Development builds are never gated.

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
The launcher's [`blob_url`](../../crates/launcher/src/manifest.rs)
passes absolute URLs through unchanged; relative `blob` fields still
work (resolved against the manifest URL's container) for legacy / mixed
hosting setups.

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
`MANIFEST_SIGNING_PUBKEY`
([`crates/launcher/src/manifest.rs`](../../crates/launcher/src/manifest.rs)::`verify_manifest_signature`).
`cimmeria-patchset pubkey --key <file>` prints the public key for a key
file, to check it matches the secret.

Full operator setup (signing keypair generation, GitHub secrets,
log-upload SAS) lives in
[launcher-distribution-setup.md](launcher-distribution-setup.md).

### Append-only invariants

Once you publish a patch, **never mutate it**:

- Don't change its `blob` path (players who already installed under the
  old path would refetch).
- Don't change its `sha256` (a launcher mid-rerun would see a hash
  mismatch and refuse the install).
- Don't change its `id` (the installed-state file uses ids to know what
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

Common issues players hit, with first-line diagnostic steps. Every
error message also lands in the launcher's activity log (Settings ›
Advanced › **Activity log**) — copy-paste that into a bug report if
first-line fixes don't help.

### "Could not load the game content list", or "Manifest error: …"

The launcher couldn't fetch, verify or parse `manifest.json`. The Play
tab says "Could not load the game content list. Check your connection
and retry." with a **Retry** button, Patch Notes says "Could not load
patch notes: …", and Settings › Advanced › **Content manifest** shows
`Manifest error: …` with the reason. A game that is already installed
still shows **Play**: the launcher just cannot check it for updates.
Press **Retry** (or **Refresh** on Patch Notes) once the cause is fixed.

- **`error sending request` / DNS failures**: check your internet
  connection. Verify the manifest URL in Settings › Advanced › **Content
  manifest** matches what your server operator published. Changing the
  URL and pressing **Refresh** drops the list from the old URL at once.
- **HTTP 404**: the manifest URL is wrong, or the release asset it
  points at was never uploaded. Operators: confirm `manifest.json` is
  attached to the `content-current` release tag, and that the tag
  itself exists.
- **HTTP 403**: the release or repository isn't publicly readable.
  Operators: content is served from GitHub Releases, so a private
  repository will 403 anonymous fetches.
- **JSON parse error**: the manifest is malformed. Operators: validate
  the file with `jq . manifest.json` before uploading.
- **"Unsupported manifest schema N"**: this launcher binary is older
  than the manifest. Download a newer launcher release.

### "Install failed: Hash mismatch for seed/patch …"

The download completed but the file's SHA-256 didn't match the manifest.

- Click **Install** (or **Update**) again — the download that failed its
  hash was deleted, so it starts that file again; anything that already
  finished is kept.
- If it persists: the CDN or the manifest is out of sync. Operators
  should re-publish the affected blob and verify the manifest hash
  matches.

### "Install failed: Unexpected HTTP &lt;status&gt; for …"

The seed or patch blob URL returned a non-success status (anything outside
`2xx` plus the explicit resume case of `206 Partial Content`).

- **404**: a manifest entry references a blob that wasn't uploaded.
  Operators should verify the upload order (patch blob first, then
  manifest — see the storage runbook).
- **403**: blob exists but isn't public.
- **5xx**: Azure transient error. Retry; if it sticks, check the Azure
  status page.

### "Could not start the game: …" or "Launch failed: File not found"

The launcher tried to launch SGW.exe / a batch file that isn't actually
on disk.

- Check the install folder in Game settings (the gear) matches where the
  game is installed. **Open in Explorer** shows it.
- If `SGW.exe` itself is gone, see the next entry.
- For Atera-debug launches: confirm `AteraLoader.exe` and
  `AtreaGameDebug.bat` are both in the install dir alongside SGW.exe.
  These files are not shipped by the launcher.

### `SGW.exe` is missing, or a game file is damaged

The Play tab says "SGW.exe is missing from the install folder", or the
game misbehaves after a file was deleted or changed by hand.

The launcher's install record (`launcher-installed.json`) says what it
installed, not what is still on disk, and **Install / Update** skips
everything the record lists. So it does not bring back a deleted
`SGW.exe` or replace a damaged file. **Repair game** will, in a later
launcher update; until then:

- **Install into a new folder.** In Game settings, **Change folder…** to
  an empty folder, **Use this folder**, then **Install Stargate Worlds**.
  The old folder is left as it is; delete it yourself when the new one
  works. This downloads the whole game again.
- **Or reinstall in place.** Close the game, delete
  `launcher-installed.json` from the install folder, and restart the
  launcher. With `SGW.exe` gone, it offers **Install Stargate Worlds**,
  which downloads the base game again over the folder and re-applies
  every patch. With `SGW.exe` still there, it offers **Use this
  installation** instead, which does not replace any file; use a new
  folder then.

### Play says "Game running" but the game is closed

The launcher shows **Game running** while any `SGW.exe` runs from the
install folder, including one it did not start, and also when it cannot
read a running `SGW.exe`'s path (another Windows user's game, for
example). It would rather wait than change files under a running game.

- Wait two seconds: the launcher checks again every two seconds.
- Open Task Manager (**Ctrl+Shift+Esc**), **Details** tab, and look for
  `SGW.exe`. A game that crashed can stay there, frozen, with no window.
  End it, and the Play tab goes back to **Ready to play**.
- A game running from another folder does not count, so a second client
  elsewhere does not block this one.

### "Not started: …"

The launcher refused a command because something else was running. The
reason follows the colon:

- **`an installation is in progress; wait for it to finish`**: let the
  install finish, or **Cancel** it, then try again.
- **`the game is already starting`**: you pressed Play twice. The first
  press is starting the game; wait for it.
- **`Stargate Worlds is running (pid N); close the game first`**: an
  `SGW.exe` from this install folder is running. Close it (see the
  previous entry if you think it is closed).

Nothing was changed. The buttons are normally off in these cases; this
message means a click got in just as the state changed.

### "The install folder is not writable"

The Play tab says "The install folder is not writable. Choose another
folder in Settings." The launcher cannot write into the folder, usually
because it is under `C:\Program Files` or another folder that needs
admin rights. Use Game settings › **Change folder…** to pick a folder of
your own, such as `%LOCALAPPDATA%\Stargate Worlds`. Running the
launcher as administrator is not needed and not recommended.

### "Could not open the folder: …"

**Open in Explorer** shows only a folder that exists; it never creates
one, and neither does the launcher's writability check. A new install
folder appears when **Install Stargate Worlds** starts. Before that,
there is nothing to open. If the folder should exist, check that its
drive is connected and that you have rights there.

### "Could not save this choice …" under Share diagnostic logs

The launcher could not write `launcher-config.json` beside itself,
usually because the launcher sits in a folder you cannot write to. The
choice holds until you close the launcher, then reverts. Move the
launcher and the files beside it to a folder of your own, and set the
choice again.

### "Your ini (..\SGWGame\Config\SGWEditor.ini) file is outdated"

The game asks this on launch when a `Working\SGWGame\Config\Default*.ini`
file's modified time differs from the one it recorded the last time it
ran. Unreal Engine 3 builds your own settings files in
`Documents\My Games\Firesky\SGWGame\Config\SGW*.ini` from the
`Default*.ini` files, and keeps the timestamps of those in each file's
`[INIVersion]` section. A different timestamp looks like a newer
`Default*.ini`.

Launchers released before this fix wrote every client file with the
install time instead of the 2009-06-30 date in the installer cabinets.
If you had run SGW on this PC before, from any install, the first launch
after such an install shows this dialog. A launcher with the fix keeps
the cabinets' dates, as the original installer does, so the dialog does
not appear after a fresh install.

- **Yes** (or **Yes to all**) is safe. The game rebuilds the
  `SGW*.ini` files from the `Default*.ini` files and records the new
  timestamps, so it does not ask again. It resets your settings in those
  files (resolution, keys, audio) to the defaults.
- **No** keeps your settings, and the game asks again on the next
  launch.
- To stop the dialog without losing settings, reinstall with a fixed
  launcher into an empty folder (Game settings › **Change folder…**,
  then **Install Stargate Worlds**). The files get
  their 2009 dates back and match what your `SGW*.ini` files recorded.
  If you already answered **Yes** after an older launcher's install,
  your `SGW*.ini` files recorded that install's dates instead, so the
  first launch after the reinstall asks once more.

### "Launcher update failed: …"

The update stopped before it changed anything, or it put the old
launcher back. The message says why:

- **`… is not writable …`**: the launcher is in a folder you cannot write
  to without admin rights, such as `C:\Program Files`. Move the launcher
  (and the files beside it) to a folder of your own, such as
  `%LOCALAPPDATA%\Stargate Worlds Launcher`, or download the new version
  from the linked release page.
- **`… does not match the release's SHA-256 …`** or **`… bytes, the
  release lists …`**: the download was damaged or cut short, and it was
  deleted. Click **Try again**.
- **`… untrusted address …`** or **`… does not trust …`**: the download
  was sent somewhere other than GitHub, so it was refused. Download by
  hand from the release page.
- **`… could not remove the previous update's leftover …`**: an old
  `<launcher>.exe.old` is locked, usually by antivirus scanning it. Wait
  a minute, or delete the `.old` file by hand, then try again.
- **`the new launcher would not start …`**: the old launcher was put
  back. Antivirus usually quarantined the new one; see the next entry.

The link under the message opens the release page, where you can
download the new exe by hand and replace yours with it.

### "Another Stargate Worlds Launcher instance appears to be running" right after an update

Updating **from** `launcher-20260929-676f314` can show this box once. That
launcher did not close its window by itself after starting the new one,
so the new launcher found it still running. The update itself worked:
click **OK** and start the launcher again. It shows the new version and
deletes the leftover `.old` file. Updates from later launchers close the
old launcher straight away, and a new launcher started by an update ends
a previous launcher that stays open too long instead of showing this
box.

If you see **"The launcher was updated, but the previous launcher window
is still open"**, close the old launcher window (or end
`sgw-launcher…exe` in Task Manager), then start the launcher again.

Outside an update, this box means what it says: a launcher is already
open. Switch to it, or close it and retry.

### Antivirus or SmartScreen after a launcher update

Every launcher release is a new unsigned exe. Windows may warn about it
the first time the updated launcher runs, and Defender may scan or
quarantine it. If no launcher window comes back after **Update now**,
look in **Windows Security → Virus & threat protection → Protection
history**, restore the file, and add the folder exclusion described in
[Windows Defender or SmartScreen blocks the launcher](#windows-defender-or-smartscreen-blocks-the-launcher).
If the launcher's own name is missing from its folder, rename
`<launcher>.exe.old` back to it. A leftover `.old` file next to a
working launcher is safe to delete; the launcher also deletes it at its
next start.

### Windows Defender or SmartScreen blocks the launcher

Launcher releases are not code-signed, so Windows may warn about the launcher or
quarantine `sgw-start32.exe`. That small helper sits beside
`sgw-launcher.exe` and loads the client patches into the game. It starts
`SGW.exe` and writes into it, which is what antivirus heuristics look
for in an unsigned program.

- **SmartScreen ("Windows protected your PC")** on `sgw-launcher.exe`:
  choose **More info → Run anyway**. Only do this for a launcher you
  downloaded from the project's GitHub Releases page.
- **Defender removed or blocked `sgw-start32.exe`**: the activity log shows
  `Client patches: not loaded (… sgw-start32.exe …)` and the game starts
  without them. Restore it from **Windows Security → Virus & threat
  protection → Protection history**, then add an exclusion for the
  folder that holds the launcher (**Virus & threat protection settings →
  Exclusions → Add an exclusion → Folder**). The helper keeps one fixed
  name in that folder across launcher updates, so the exclusion keeps
  working. The launcher never writes it to a temporary folder. From an
  elevated PowerShell:

  ```powershell
  Add-MpPreference -ExclusionPath "C:\path\to\the\launcher\folder"
  ```

Code signing would remove the need for this; it is deferred for now.

### The Black Market window does not open

The Black Market window needs the client patches. Check the activity log
from the launch:

- **`Client patches: off (launcher setting)`**: tick **Load client
  patches** and launch again.
- **`Client patches: unavailable: …`**: this launcher build has no DLL
  bundled (a dev build) and there is no `cimmeria-client-patches.dll`
  beside it. Use a released launcher.
- **`Client patches: not loaded (… sgw-start32.exe …)`**: the launcher
  could not run its 32-bit helper, `sgw-start32.exe`, which loads the
  patches into the 32-bit game. A dev build has none unless it sits
  beside the launcher; a released launcher carries it. If security
  software removed it, see
  [Windows Defender or SmartScreen blocks the launcher](#windows-defender-or-smartscreen-blocks-the-launcher).
- **`Client patches: not loaded (… remote_load_failed …)`**: the helper
  ran but the DLL did not load into the game. Include the activity log in
  a bug report.
- **No such line**: the patches were loaded. They write
  `cimmeria-client-patches.log` next to `SGW.exe`; a line ending
  `nothing installed` there means this `SGW.exe` is not the build the
  patches know. Include that file in a bug report.

### The game shows the gate backdrop but never the login screen

The launcher released on 2026-09-29 (`launcher-20260929-f518b57`)
renamed `Working\SGWGame\Content\UI\Startup\EULA\EULA.lua` to
`eula.lua` while applying the `005-login-delay` patch. The game looks
that file up case-sensitively, so the EULA screen never loads, and the
login screen comes after it. The client log (and telemetry's
`client.ui.cegui_log`) shows `SGWResourceProvider: 'EULA.lua' does not
exist in group lua`.

- **Fix**: update the launcher. Before every launch, and after every
  **Install / Update**, it renames the file back; the activity log says
  "Renamed ... eula.lua back to its stock name EULA.lua".
- **By hand**: rename `eula.lua` to `EULA.lua` in that folder. Keep the
  file; it is the patched one.

Patch sets now keep the on-disk name of every file they change, so a new
install is not affected.

### SGW.exe launches but can't reach the server

- Check the login servers in Settings › Advanced › **Login servers**:
  one `Name = http://host:8081` line per server, with your operator's
  host. Click **Save login servers**; the activity log says "Wrote the
  login server list" when the file changed.
- Open `Working\SGWGame\Content\UI\Startup\Login\LoginInternal.lua` and
  check it lists your server. The launcher rewrites it before every
  launch, so edit the list in the launcher, not the file.
- Pick the right server in the login screen's dropdown.

### "Log upload failed: 4xx/5xx"

- **403 AuthorizationFailure**: the SAS in the launcher binary has
  expired. Players need to download a newer launcher release.
- **403 AuthenticationFailed**: the SAS was malformed. Operators should
  rotate (see storage runbook).
- **413 RequestBodyTooLarge**: the log zip exceeded Azure's 256 MB
  single-PUT block-blob limit. Should not happen in practice (sessions
  rarely produce >100 MB of logs); if it does, report it.

### "Log upload skipped: Already uploaded this exact log set"

This is **not an error** — it means the launcher detected via local
ledger that the same log contents were already uploaded. To force a
re-upload, edit `<launcher.exe dir>/uploaded.json` and remove the
relevant entry, or play a new session to generate fresh logs.

### Where to find logs to share

When asking for support:

- The launcher's activity log (Settings › Advanced › **Activity log**) —
  copy the relevant lines.
- `<launcher.exe dir>/launcher-config.json` — your install path, server
  host, and manifest URL (the relevant config the launcher is using).
- `<install_dir>/launcher-installed.json` — what the launcher thinks is
  installed (seed sha + applied patches).
- For game crashes (not launcher issues): use the **Upload Debug Logs**
  button. Logs land at `logs/<host>-<utc>-<digest>.zip` in the
  operator's storage container.

---

## Cross-references

- [sgw-launcher.md](sgw-launcher.md) — full design and architecture rationale
- [launcher-distribution-setup.md](launcher-distribution-setup.md) — operator setup: GH Releases publishing, manifest signing key, Azure Blob SAS
- [`crates/launcher/`](../../crates/launcher/) — source code
- [.github/workflows/launcher-release.yml](../../.github/workflows/launcher-release.yml) — release pipeline
