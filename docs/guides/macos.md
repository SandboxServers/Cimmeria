---
title: "Playing on a Mac (experimental)"
type: how-to
audience: players and testers on Apple Silicon Macs; no programming needed
status: experimental (the macOS launcher has not yet been confirmed by a player, 2026-10-11)
last_updated: 2026-10-11
companion_docs:
  - ../client/launcher-guide.md
  - ../../crates/launcher/desktop/docs/release.md
  - unified-uat.md
  - ../operations/telemetry.md
---

# Playing on a Mac (experimental)

Stargate Worlds is a 32-bit Windows game (Unreal Engine 3, Direct3D 9).
There is no Mac client. The Cimmeria launcher for macOS is a Mac app that
installs the Windows game and runs it inside Wine for you, so you don't set up
Wine yourself. Nothing on the server changes: a Mac player connects to the same
Cimmeria server as everyone else.

**Status: experimental.** The macOS launcher is built and published by the
release pipeline, but no player has yet confirmed the whole path from a fresh
Mac to the game world with it. Follow the steps in order and report what
happens at each checkpoint.

This page replaces the earlier setup, which ran the old Windows launcher
(`sgw-launcher.exe`) inside a WoWSilicon profile. That launcher was retired on
2026-10-11. You no longer need WoWSilicon or CrossOver: the Mac app downloads
the Wine runtime it needs on its own.

## What you need

- An Apple Silicon Mac (M1 or later) on macOS 12 or newer. There is no Intel
  Mac build.
- Rosetta 2, which runs the 32-bit Windows game. If it is not installed yet, in
  Terminal: `softwareupdate --install-rosetta`
- About 20 GB of free disk space. The game download is 4.1 GB and the install
  needs about 14 GB while it unpacks, plus a Wine runtime of about 64 MB.
- A Cimmeria account. Accounts are created by the server owner; ask for one.

## Step 1: download and open the launcher

1. Download
   [`StargateWorlds-Launcher-macos-arm64.zip`](https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-macos-arm64.zip).
   This link always holds the newest release.
2. Double-click the zip to unzip it, then drag **Stargate Worlds Launcher** into
   **Applications** in Finder. Run it from there, not from Downloads: macOS
   runs apps opened straight from Downloads from a hidden read-only copy.
3. Open it. macOS refuses the first time, because the app is not notarized by
   Apple (it is signed ad hoc only). Close the message without moving the app
   to the Trash.
4. Open **System Settings → Privacy & Security**, scroll down to the message
   about "Stargate Worlds Launcher", click **Open Anyway** and enter your password.
   You do this once; after that the app opens normally.

**Checkpoint A.** The launcher window opens with the **Play** and **Patch
Notes** tabs. Open **Settings** (the ⚙ button): the line under the **Game
settings** heading shows the launcher version. A black or empty window here is
important to report.

## Step 2: install the game

1. In **Settings**, **Local install directory** shows where the game goes.
   Leave it, or click **Choose folder…** and pick an empty folder.
2. Turn on **Send game diagnostics while playing**. This is what lets us see a
   Mac problem remotely without asking you for logs. It is off by default.
3. Click **Install Stargate Worlds** and wait. The launcher downloads the Wine
   runtime once, then the original 2009 client from archive.org, unpacks it with
   Wine and applies the Cimmeria patches. When the content is in place it goes
   on to the compatibility setup the game needs (PhysX under Wine). If you close
   the launcher in between, it offers **Continue installation** next time.

**Checkpoint B.** The progress bar is gone and **Play** is enabled. Note how
long it took.
If it stops with an error, copy the status text exactly.

macOS may ask whether the launcher can access a folder (Documents, or the folder
you install into). Allow it. Because the app is signed ad hoc, macOS asks again
after every new launcher version.

## Step 3: start the game

1. Click **Play**. Keep **Settings → Client patches → Load client patches** on.
2. The first start can take a minute; shaders are compiled on every start on
   macOS. There is an intro, then the gate courtyard, and after about 20
   seconds the login box.

**Checkpoint C.** The login box appears over the gate. Report instead: a crash
or error box (copy its text), a black or garbled screen, or the courtyard with
no login box after a minute.

The launcher runs the game with stock Rosetta 2. It does not carry the faster
x87 translator ("rosettax87") that WoWSilicon offers, so frame rates in busy
areas may be lower than under WoWSilicon. Tell us how it feels.

## Step 4: log in and play

1. Log in with your account and pick the server in the list.
2. Create a character or pick one, and press **Play**.

**Checkpoint D.** You arrive in the world with the HUD, minimap and nearby
characters visible. The first arrival can take a minute to finish drawing.
Walk around for a few minutes; note frame rate and any stutter.

## Step 5: the Black Market window

This checks that the client patches loaded, which is the part most likely to
behave differently under Wine.

1. Find **Machra**, the Black Market auctioneer, by the exit doorway of the
   stasis room new characters start in, or ask the server owner where he is
   for your character.
2. Walk right up to him and right-click him.

**Checkpoint E.** The Black Market window opens with a list of auctions. If
only a chat line appears ("The auctioneer opens the Black Market. (No
window? …)"), the client patches did not load: say so.

## Updating the launcher

macOS updates are manual. On a Mac the **Launcher updates** section in Settings
says launcher updates are unavailable, and **Check for updates** is greyed out. When a new launcher is released, download the zip from
the same link again, quit the launcher, and replace the app in Applications
with the new one. Your settings and the installed game stay. Expect the
folder-access question again after each new version.

## Coming from the old launcher under WoWSilicon

If you installed the game with the old Windows launcher inside a WoWSilicon
profile, you can install fresh (simplest), or bring that install over:
**Settings → Import legacy launcher settings → Select legacy folders…** asks
first for the folder that held the old launcher `.exe`, then for its game
folder (both are inside the WoWSilicon profile's Wine prefix). After
**Confirm settings import**, **Verified copy adoption** makes a verified copy of
the game in a new folder; your original stays untouched. An adopted copy plays
without the game diagnostics module for now, so testers should install fresh.
The details are in
the [migration contract](../../crates/launcher/desktop/docs/migration.md#adopting-an-existing-folder).

## What to report

Copy this, fill it in, and send it to the server owner:

```text
Mac model / RAM:
macOS version:
Launcher version (under the Game settings heading):
Game diagnostics turned on:   yes / no
Date and time of each attempt (with time zone):

A launcher opens:                           pass / fail  -
B install and setup finish:                 pass / fail  - minutes:
C login box appears:                        pass / fail  -
D in the world, HUD and characters drawn:   pass / fail  - fps feel:
E Black Market window opens:                pass / fail  -

Error texts (copied exactly):
Anything else odd:
```

With game diagnostics on, also send your install id: it is the `install_id`
value in `~/Library/Application Support/app.cimmeria.launcher.desktop/state/game-telemetry.json`
(in Finder, **Go → Go to Folder…** and paste the folder path). It is a random
id that lets us find your diagnostics, nothing else.

## For the server owner: finding a Mac tester's telemetry

With the tester's `install_id` and the times they reported, query SigNoz:

```text
service.name = 'cimmeria-client' AND install_id = '<install_id>'
```

Useful rows: `client_target = 'client.dll.attached'` (the telemetry DLL
started), `client_target = 'client.hooks.fingerprint'` (hooks installed, or
`fingerprint_usable = false`), and `client.ui.cegui_log` errors. A session
with launcher rows but no `client.dll.attached` means the DLLs were not
loaded into the game under Wine. See [telemetry.md](../operations/telemetry.md)
for the full list.

## Known unknowns

- **The packaged app on a fresh Mac.** The release job builds, signs and checks
  the app, but nobody has yet downloaded it on a Mac that never ran it and gone
  from Open Anyway to the world (acceptance step 13 of the
  [launcher consolidation](../analysis/launcher-consolidation/README.md)).
- **Frame rate without rosettax87.** See Step 3.
- **Shadow resolution.** A community guide for another fan server reports the
  client writing a corrupted shadow-resolution value under DXVK. Report it if
  shadows look wrong or the game stalls; the client's shadow settings are
  mapped in [render-thread shadow options RE](../reverse-engineering/README.md).
- **Resetting client state.** **Reset client cache** and **Reset all client
  state…** are not built for macOS yet: the Wine client keeps its settings
  inside the launcher's Wine prefix, and the buttons say the client's folder
  could not be found.
