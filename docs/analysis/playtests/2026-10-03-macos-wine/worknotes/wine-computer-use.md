# Wine computer-use compatibility

> **Type:** Reference
> **Audience:** Launcher integrators and coordinator UAT
> **Last updated:** 2026-10-04

## Outcome

The first candidate, a foreground wrapper application that started Wine as a
separate process, is rejected and removed. It cannot make the game window
selectable: the wrapper owns no window. It is replaced by an opt-in that runs
the real Wine loader from inside an application bundle, so the process that
owns the game window carries the bundle identifier itself. The contract,
evidence table and rollback are in
[launch.md](../../../../../crates/launcher/desktop/docs/launch.md#optional-mac-application-identity-for-the-game-window).

Default Play is unchanged. The first game run with the opt-in rendered and
reached the tool, but the tool could not select the game; see
[First game run](#first-game-run-and-the-follow-up). The follow-up fix has not
been verified against the game or the tool; both are the coordinator's gates
below.

## Why the wrapper could not work

- A native fixture of the same shape (bundled foreground parent, separate
  window-owning child) showed the parent with the bundle identifier and zero
  windows, and the child with the window and no identifier. Reading the
  parent's accessibility windows failed outright; it runs no event loop.
- In the pinned Wine every guest process, `SGW.exe` included, executes the
  loader beside the `ntdll.so` it loaded. No guest process is ever the wrapper.
- The wrapper commit also did not compile on macOS, could not parse its own
  launch spec, and made Play fail whenever its unbundled host binary was absent.
  None of that was worth repairing once the design was ruled out.

## What replaced it

`engine/src/mac_wine/app_identity.rs` in the desktop workspace. With
`CIMMERIA_WINE_APP_IDENTITY=1` in the launcher's environment, Play stages
`wine-app-identity/Stargate Worlds.app` under the launcher state root from the
verified runtime and starts the launch worker with the loader inside it. No new
binary, bundled resource, `Resources` field or shell change is involved. Any
staging failure falls back to the stock loader.

## First game run and the follow-up

On 2026-10-04 the coordinator ran the game with the opt-in. `lsappinfo` showed
two running applications with `app.cimmeria.stargate-worlds`, both executing the
staged loader: `explorer.exe /desktop`, type UIElement, registered first, and
`SGW.exe`, type Foreground, registered second. The tool's `getApp` timed out
after five seconds for the identifier and for the bundle path. No x87
accelerator was staged, so gate 2 below was not exercised.

Wine starts the desktop host from the loader of the first process that asks for
the desktop window. That was the game. The follow-up makes Play start a keeper
from the stock loader first, so the desktop host is a stock `wine` process with
no identifier, and only a process that shows a window from the bundle has the
game's identifier. Contract and evidence:
[launch.md](../../../../../crates/launcher/desktop/docs/launch.md#the-desktop-host-stays-on-the-stock-loader).

What the follow-up established, with Wine's Notepad in an isolated runtime copy
and throwaway prefix, never with the game:

- Staged loader alone reproduces the two applications.
- With the keeper, exactly one running application has the identifier: the
  32-bit Notepad started by a 32-bit parent. It owns the window.
- Releasing the keeper while Notepad runs leaves the desktop host in place.
  Closing Notepad ends the desktop host and the Wine session with no process
  left. A keeper released with no game ends the session the same way.
- The keeper ends when its input closes, which includes the launcher exiting.

Alternatives ruled out: starting `explorer.exe /desktop` directly (it never
closes if no user ever joins, and offers no ready signal); letting
`explorer.exe` start the keeper (Wine opens a visible console window for it);
`start /wait` in the keeper (it reports success when the program is missing).

One more observation for the tool run. While the active Space was another
application's full-screen Space, the accessibility window lists of the game and
of a fixture Notepad were both empty. The Notepad's list had one window once
its Space was active. Bring the game's Space to the front before `getApp`.

## Coordinator verification

Not done by this packet. Start the launcher from a terminal so the variable
reaches it and its standard error is visible:

```bash
CIMMERIA_WINE_APP_IDENTITY=1 "<launcher bundle>/Contents/MacOS/cimmeria-launcher-desktop"
```

Press Play, wait for the login screen, then check the identity natively before
involving the tool:

```bash
lsappinfo list | grep -B2 -A6 'app.cimmeria.stargate-worlds'
```

Expect exactly one entry, of type `Foreground`, whose executable path ends in
`Stargate Worlds.app/Contents/MacOS/wine`. A second entry of type `UIElement`
means the desktop host still came from the bundle. No entry at all means Play
fell back; the reason is on standard error. The desktop host is now a separate
`wine` entry with no bundle identifier:

```bash
ps -axo pid,args | grep 'explorer.exe /desktop'
lsappinfo info -only bundleID,name "$(lsappinfo find pid=<that pid>)"
```

Then, with the existing tool:

```javascript
await cua.listApps(); // expect Stargate Worlds / app.cimmeria.stargate-worlds
const game = await cua.getApp("app.cimmeria.stargate-worlds");
// or cua.getApp("Stargate Worlds"), or the bundle path from lsappinfo
await game.getAXStateAndScreenshot(); // must show the login screen
```

`getApp("SGW.exe")` is not expected to resolve.

## Remaining gates

1. The game under the staged loader: rendering, patch injection, login and the
   30 FPS limit. Only Wine's Notepad has run this way.
2. The x87 accelerator path, if staged: confirm the `Foreground` entry's
   executable path is still inside the bundle.
3. The tool binding the game now that it is the only running application with
   the identifier. If `getApp` still times out with one entry and the game's
   Space in front, the cause is not the duplicate, and the minimal upstream
   requirement is that the tool accept a PID or window ID.
4. The game with a stock desktop host: full-screen and display mode changes,
   clipboard, input, quit. The desktop host and the game run the same loader
   bytes from different paths; only Notepad has run that way.
5. The bundle identifier and display name were chosen to match the launcher's
   `app.cimmeria.*` convention and are one constant each; confirm or rename.

Only after gates 1 to 4 pass should the opt-in become the default.
