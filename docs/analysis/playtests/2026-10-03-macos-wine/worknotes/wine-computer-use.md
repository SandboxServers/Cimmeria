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

Default Play is unchanged. Nothing has been verified against the game or
against the computer-use tool; both are the coordinator's gates below.

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

Expect one entry of type `Foreground` whose executable path ends in
`Stargate Worlds.app/Contents/MacOS/wine`, and one or more of type `UIElement`.
No entry at all means Play fell back; the reason is on standard error.

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
3. The tool binding the window owner rather than the windowless `explorer.exe`,
   which shares the identifier and registers first. If the screenshot is empty
   or `getApp` binds a windowless application, the minimal upstream requirement
   is that the tool prefer the regular-activation-policy process among running
   applications with one bundle identifier, or accept a PID or window ID.
4. The bundle identifier and display name were chosen to match the launcher's
   `app.cimmeria.*` convention and are one constant each; confirm or rename.

Only after gates 1 to 3 pass should the opt-in become the default.
