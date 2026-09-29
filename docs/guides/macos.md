---
title: "Playing on a Mac (experimental)"
type: how-to
audience: players and testers on Apple Silicon Macs; no programming needed
status: experimental (not yet confirmed with the Cimmeria launcher, 2026-09-29)
last_updated: 2026-09-29
companion_docs:
  - ../client/launcher-guide.md
  - unified-uat.md
  - ../operations/telemetry.md
---

# Playing on a Mac (experimental)

Stargate Worlds is a 32-bit Windows game (Unreal Engine 3, Direct3D 9).
There is no Mac client. On a Mac you run the Windows game, and the Cimmeria
launcher, inside Wine. Nothing on the server changes: a Mac player connects to
the same Cimmeria server as everyone else.

**Status: experimental.** The Wine setup below is based on a community guide
that runs this client on Apple Silicon for another fan server
([pavlista.cz/stargate](https://www.pavlista.cz/stargate/), by Michael
Pavlista). Nobody has confirmed it with *our* launcher yet. Follow the steps
in order and report what happens at each checkpoint.

Why run our launcher and not the game directly: the launcher installs the
game, applies the Cimmeria patches, points the client at our server, loads
the client patches (the Black Market window needs them) and, if you opt in,
sends telemetry that lets us see what went wrong without asking you for logs.

What that guide found, and why this page uses WoWSilicon:

- A virtual machine (VMware Fusion) crashed on larger maps.
- CrossOver ran, but slowly. Old 32-bit games use x87 floating-point
  instructions, which Rosetta 2 translates slowly.
- [WoWSilicon](https://wowsilicon.github.io/) bundles Wine, DXVK and a
  faster x87 translation ("rosettax87 JIT"), and runs the game well.
- The other server's launcher shows only a black screen under Wine because it
  is built on WebView2. The Cimmeria launcher does not use WebView2, so this
  should not happen with ours, but it is untested.

## What you need

- An Apple Silicon Mac (M1 or later) on macOS 15 or newer.
- Rosetta 2. In Terminal: `softwareupdate --install-rosetta`
- [WoWSilicon](https://wowsilicon.github.io/) **3.0.2 or newer**, installed in
  `/Applications`. Older versions cannot make the HTTPS connections the
  launcher needs to download the game and its patches.
- About 20 GB of free disk space. The installer download is 4.1 GB and the
  install needs about 14 GB while it unpacks.
- A Cimmeria account. Accounts are created by the server owner; ask for one.

## Step 1: make a WoWSilicon profile

1. Download the newest `sgw-launcher-launcher-<date>-<commit>.exe` from the
   project's [GitHub Releases](https://github.com/SandboxServers/Cimmeria/releases)
   (the release marked **Latest** whose name starts with "Launcher"). Put it in
   a folder of its own; the launcher keeps its settings next to itself.
2. In WoWSilicon, create a **Non-WoW game (32-bit D3D9)** profile.
3. Point the profile at the launcher `.exe`.
4. Set the graphics backend to **d9vk** and leave x87 translation on
   **rosettax87 JIT**.
5. Press **Patch**, then **Play**.

**Checkpoint A.** The "Stargate Worlds Launcher" window opens, shows
`Launcher launcher-<date>-<commit>` under the title, and after a few seconds
lists the manifest ("Manifest schema 1, seed … N patch(es) declared").
If the window stays on "Fetching manifest…", move the mouse over it once.
A black or empty window here is important to report.

## Step 2: install the game

1. Leave **Install dir** as it is (inside the Wine prefix, the prefix's own
   `AppData\Local\Stargate Worlds`).
2. Click **Turn on telemetry** in the "Help us fix bugs?" box. This is what
   lets us diagnose a Mac problem remotely.
3. Click **Install / Update** and wait. It downloads the original 2009 client
   from archive.org, unpacks it and applies the Cimmeria patches. You do not
   need to run the old `SetupQA.exe` installer yourself.

**Checkpoint B.** The launcher shows **"✔ Install is up to date"**. Note how
long it took. If it stops with an error, copy the red status text. The unpack
step uses Windows' cabinet code, which Wine provides; a failure here is one
of the things we want to know about.

## Step 3: start the game

1. Click **Launch SGW.exe**. Keep **Load client patches** ticked.
2. The first start can take a minute; shaders are compiled on every start on
   macOS. There is an intro, then the gate courtyard, and after about 20
   seconds the login box.

**Checkpoint C.** The login box appears over the gate. Report instead: a
crash or error box (copy its text), a black or garbled screen, or the
courtyard with no login box after a minute.

If the picture is black or wrong, the likely cause is that the game did not
pick up DXVK's `d3d9.dll`. The community launcher copies that file into the
game's `Working\binaries` folder before starting it; ours does not yet. Tell
us, and we will add that step (see "Known unknowns").

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

## What to report

Copy this, fill it in, and send it to the server owner:

```text
Mac model / RAM:
macOS version:
WoWSilicon version:
Profile type / graphics / x87:  Non-WoW 32-bit D3D9 / d9vk / rosettax87 JIT
Launcher version (under the title):
Telemetry turned on:          yes / no
Date and time of each attempt (with time zone):

A launcher opens and lists the manifest:   pass / fail  -
B install finishes:                         pass / fail  - minutes:
C login box appears:                        pass / fail  -
D in the world, HUD and characters drawn:   pass / fail  - fps feel:
E Black Market window opens:                pass / fail  -

Error texts (copied exactly):
Anything else odd:
```

Also attach `install.json` from the launcher's folder (it holds only this
install's random id, which lets us find your telemetry).

## Fallback: CrossOver

If WoWSilicon will not run the launcher at all, try
[CrossOver](https://www.codeweavers.com/crossover) (free trial): make a
Windows 10 64-bit bottle, run the launcher `.exe` in it with **Run Command…**,
and follow the same checkpoints. Expect lower frame rates: CrossOver has no
fast x87 translation. Try its DXVK graphics option if the picture is wrong,
and report which settings you used.

## For the server owner: finding a Mac tester's telemetry

With the tester's `install_id` (from `install.json`) and the times they
reported, query SigNoz:

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

- **The launcher window under Wine.** It is drawn with OpenGL (egui), not
  WebView2, so the community guide's black-screen problem should not apply.
  Untested.
- **DXVK's `d3d9.dll`.** The community launcher copies it into
  `Working\binaries`. If the game only renders with that file present, the
  launcher should do the same when it detects Wine.
- **Loading the client patches.** The launcher starts the game suspended and
  injects its DLLs with a small 32-bit helper (`sgw-start32.exe`). Wine
  normally supports this, but it has not been tried here, nor under the x87
  translator.
- **Shadow resolution.** The community guide reports the client writing a
  corrupted shadow-resolution value under DXVK (its launcher has a checkbox
  that fixes it). The Cimmeria launcher does not correct this yet. Report it
  if shadows look wrong or the game stalls; the client's shadow settings are
  mapped in [render-thread shadow options RE](../reverse-engineering/README.md).
- **Launcher self-update.** Replacing the launcher `.exe` while it runs should
  work inside Wine, but is untested there.
